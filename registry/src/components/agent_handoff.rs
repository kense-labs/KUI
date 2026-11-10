//! Host-controlled assistant-to-person handoff. This view never contacts a support service.
use crate::primitives::{
    brand_mark::brand_mark,
    editor::{EditEvent, Editor, InputMode},
    family::{Paint, Slide, Tone, face},
    glyph::{Glyph, icon},
    nav::tr,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandoffStage {
    Assistant,
    Transferring,
    Connected,
    Unavailable,
    Resolved,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandoffAuthor {
    Customer,
    Assistant,
    Agent,
}
#[derive(Clone)]
pub struct HandoffMessage {
    pub id: SharedString,
    pub author: HandoffAuthor,
    pub text: SharedString,
    pub time: SharedString,
    pub after_handoff: bool,
}
impl HandoffMessage {
    pub fn new(
        id: impl Into<SharedString>,
        author: HandoffAuthor,
        text: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            author,
            text: text.into(),
            time: "".into(),
            after_handoff: false,
        }
    }
    pub fn after_handoff(mut self) -> Self {
        self.after_handoff = true;
        self
    }
    pub fn time(mut self, time: impl Into<SharedString>) -> Self {
        self.time = time.into();
        self
    }
}
#[derive(Clone)]
pub struct HandoffSummary {
    pub issue: SharedString,
    pub fields: Vec<(SharedString, SharedString)>,
}
impl HandoffSummary {
    pub fn new(issue: impl Into<SharedString>) -> Self {
        Self {
            issue: issue.into(),
            fields: Vec::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentHandoffEvent {
    Request { ticket: u64 },
    Cancel { ticket: u64 },
    Send { ticket: u64, text: SharedString },
    CancelSend { ticket: u64 },
}
#[derive(Default)]
struct RequestGate {
    serial: u64,
    active: Option<u64>,
}
impl RequestGate {
    fn begin(&mut self) -> u64 {
        self.serial = self.serial.wrapping_add(1);
        self.active = Some(self.serial);
        self.serial
    }
    fn cancel(&mut self) -> Option<u64> {
        self.active.take()
    }
    fn settle(&mut self, ticket: u64) -> bool {
        if self.active == Some(ticket) {
            self.active = None;
            true
        } else {
            false
        }
    }
}
fn draft_is_unchanged(sent: Option<&str>, current: &str) -> bool {
    sent == Some(current)
}
pub struct AgentHandoff {
    title: SharedString,
    subtitle: SharedString,
    assistant_model: Option<SharedString>,
    task: SharedString,
    summary: HandoffSummary,
    messages: Vec<HandoffMessage>,
    stage: HandoffStage,
    agent: SharedString,
    estimate: SharedString,
    error: SharedString,
    summary_open: bool,
    summary_motion: Slide,
    summary_height: Rc<Cell<Pixels>>,
    transfer: RequestGate,
    sending: RequestGate,
    sent_text: Option<String>,
    send_error: SharedString,
    composer: Entity<Editor>,
    scroll: ScrollHandle,
    previous_count: usize,
    _subscription: Subscription,
}
impl EventEmitter<AgentHandoffEvent> for AgentHandoff {}
impl AgentHandoff {
    pub fn new(
        title: impl Into<SharedString>,
        task: impl Into<SharedString>,
        summary: HandoffSummary,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let composer = cx.new(|cx| {
            let mut editor = Editor::new(tr(cx, "Message", "消息"), InputMode::InlineMulti, w, cx);
            editor.set_placeholder(tr(cx, "Write a message…", "输入消息…"));
            editor
        });
        let sub = cx.subscribe_in(&composer, w, |s: &mut Self, _, event: &EditEvent, _, cx| {
            if *event == EditEvent::Submitted {
                s.send(cx);
            }
        });
        Self {
            title: title.into(),
            subtitle: "".into(),
            assistant_model: None,
            task: task.into(),
            summary,
            messages: Vec::new(),
            stage: HandoffStage::Assistant,
            agent: "".into(),
            estimate: "".into(),
            error: "".into(),
            summary_open: true,
            summary_motion: Slide::new(1.),
            summary_height: Rc::new(Cell::new(px(80.))),
            transfer: RequestGate::default(),
            sending: RequestGate::default(),
            sent_text: None,
            send_error: "".into(),
            composer,
            scroll: ScrollHandle::new(),
            previous_count: 0,
            _subscription: sub,
        }
    }
    pub fn subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
        self.subtitle = subtitle.into();
        self
    }
    /// Name the model shown on assistant messages.
    pub fn with_assistant_model(mut self, model: impl Into<SharedString>) -> Self {
        self.assistant_model = Some(model.into());
        self
    }
    pub fn estimate(mut self, estimate: impl Into<SharedString>) -> Self {
        self.estimate = estimate.into();
        self
    }
    pub fn set_messages(&mut self, messages: Vec<HandoffMessage>, cx: &mut Context<Self>) {
        self.messages = messages;
        cx.notify();
    }
    /// Append a message once by stable host ID.
    pub fn append_message(&mut self, message: HandoffMessage, cx: &mut Context<Self>) -> bool {
        if self
            .messages
            .iter()
            .any(|existing| existing.id == message.id)
        {
            return false;
        }
        self.messages.push(message);
        cx.notify();
        true
    }
    pub fn set_summary(&mut self, summary: HandoffSummary, cx: &mut Context<Self>) {
        self.summary = summary;
        cx.notify();
    }
    pub fn stage(&self) -> HandoffStage {
        self.stage
    }
    pub fn request_human(&mut self, cx: &mut Context<Self>) {
        if !matches!(
            self.stage,
            HandoffStage::Assistant | HandoffStage::Unavailable
        ) || self.transfer.active.is_some()
        {
            return;
        }
        let ticket = self.transfer.begin();
        self.stage = HandoffStage::Transferring;
        self.error = "".into();
        cx.emit(AgentHandoffEvent::Request { ticket });
        cx.notify();
    }
    pub fn cancel_transfer(&mut self, cx: &mut Context<Self>) {
        if let Some(ticket) = self.transfer.cancel() {
            self.stage = HandoffStage::Assistant;
            cx.emit(AgentHandoffEvent::Cancel { ticket });
            cx.notify();
        }
    }
    /// The host must return the request ticket; late completions after cancellation are ignored.
    pub fn connect(
        &mut self,
        ticket: u64,
        agent: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.stage != HandoffStage::Transferring || !self.transfer.settle(ticket) {
            return false;
        }
        self.agent = agent.into();
        self.stage = HandoffStage::Connected;
        cx.notify();
        true
    }
    pub fn fail_transfer(
        &mut self,
        ticket: u64,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.stage != HandoffStage::Transferring || !self.transfer.settle(ticket) {
            return false;
        }
        self.error = error.into();
        self.stage = HandoffStage::Unavailable;
        cx.notify();
        true
    }
    pub fn set_resolved(&mut self, resolved: bool, cx: &mut Context<Self>) {
        if matches!(self.stage, HandoffStage::Connected | HandoffStage::Resolved) {
            self.stage = if resolved {
                HandoffStage::Resolved
            } else {
                HandoffStage::Connected
            };
            cx.notify();
        }
    }
    fn send(&mut self, cx: &mut Context<Self>) {
        if self.stage == HandoffStage::Resolved || self.sending.active.is_some() {
            return;
        }
        let draft = self.composer.read(cx).value().to_owned();
        let text = draft.trim().to_owned();
        if text.is_empty() {
            return;
        }
        let ticket = self.sending.begin();
        self.sent_text = Some(draft);
        self.send_error = "".into();
        cx.emit(AgentHandoffEvent::Send {
            ticket,
            text: text.into(),
        });
        cx.notify();
    }
    fn cancel_send(&mut self, cx: &mut Context<Self>) {
        if let Some(ticket) = self.sending.cancel() {
            self.sent_text = None;
            cx.emit(AgentHandoffEvent::CancelSend { ticket });
            cx.notify();
        }
    }
    /// On success clear only an unchanged draft; failure keeps it available for retry.
    pub fn finish_send(
        &mut self,
        ticket: u64,
        result: Result<(), SharedString>,
        w: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.sending.settle(ticket) {
            return false;
        }
        let sent = self.sent_text.take();
        match result {
            Ok(()) => {
                if draft_is_unchanged(sent.as_deref(), self.composer.read(cx).value()) {
                    self.composer.update(cx, |e, cx| e.set_value("", w, cx));
                }
                self.send_error = "".into();
            }
            Err(error) => self.send_error = error,
        }
        cx.notify();
        true
    }
}
fn message_view(
    message: &HandoffMessage,
    agent: &str,
    model: Option<&str>,
    p: Paint,
    radius: Pixels,
    cx: &App,
) -> impl IntoElement {
    let own = message.author == HandoffAuthor::Customer;
    let name: SharedString = match message.author {
        HandoffAuthor::Customer => tr(cx, "You", "你").into(),
        HandoffAuthor::Assistant => tr(cx, "Assistant", "智能助手").into(),
        HandoffAuthor::Agent => agent.to_owned().into(),
    };
    div()
        .flex()
        .flex_col()
        .when(own, |v| v.items_end())
        .child(
            div()
                .min_h(px(28.))
                .flex()
                .items_center()
                .text_size(px(12.))
                .line_height(px(16.))
                .text_color(p.secondary)
                .gap(px(6.))
                .children(if message.author == HandoffAuthor::Assistant {
                    model.and_then(|model| brand_mark(model, 14., cx))
                } else {
                    None
                })
                .child(format!(
                    "{} {} {}",
                    name,
                    if message.author == HandoffAuthor::Assistant {
                        model.unwrap_or_default()
                    } else {
                        ""
                    },
                    message.time
                )),
        )
        .child(
            div()
                .max_w(px(440.))
                .min_h(px(36.))
                .py(px(8.))
                .px(px(12.))
                .rounded(radius)
                .bg(if own { p.bright } else { p.tray })
                .text_color(if own { p.inverse } else { p.ink })
                .child(message.text.clone()),
        )
}
impl Render for AgentHandoff {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let progress = self.summary_motion.get(w, cx).clamp(0., 1.);
        if self.messages.len() != self.previous_count {
            if self.scroll.max_offset().y + self.scroll.offset().y < px(24.) {
                self.scroll.scroll_to_bottom();
            }
            self.previous_count = self.messages.len();
        }
        let label = match self.stage {
            HandoffStage::Assistant => tr(cx, "Assistant", "智能助手"),
            HandoffStage::Transferring => tr(cx, "Connecting", "正在转接"),
            HandoffStage::Connected => tr(cx, "Connected", "已接入"),
            HandoffStage::Unavailable => tr(cx, "No one available", "暂时无人接入"),
            HandoffStage::Resolved => tr(cx, "Resolved", "已解决"),
        };
        let mut thread = div()
            .id("handoff-thread")
            .role(Role::Region)
            .aria_label(tr(cx, "Conversation", "会话记录"))
            .tab_index(0)
            .track_scroll(&self.scroll)
            .overflow_y_scroll()
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .pt(px(20.))
            .pr(px(20.))
            .pb(px(20.))
            .pl(px(12.));
        for message in self
            .messages
            .iter()
            .filter(|message| !message.after_handoff)
        {
            thread = thread.child(message_view(
                message,
                &self.agent,
                self.assistant_model.as_deref(),
                p,
                radius,
                cx,
            ));
        }
        if self.stage == HandoffStage::Assistant {
            thread = thread.child(
                face(
                    "handoff-request",
                    tr(cx, "Talk to a person", "联系人工"),
                    36.,
                    8.,
                    Tone::Surface,
                    cx,
                )
                .ml_auto()
                .pl(px(12.))
                .pr(px(14.))
                .gap(px(6.))
                .hover(|s| s.bg(p.tray))
                .child(tr(cx, "Talk to a person", "联系人工"))
                .on_click(cx.listener(|s, _, _, cx| s.request_human(cx))),
            );
        }
        if self.stage != HandoffStage::Assistant {
            let mut card = div()
                .w_full()
                .rounded(radius)
                .border_1()
                .border_color(p.line)
                .bg(p.panel)
                .flex()
                .flex_col()
                .child(
                    div()
                        .min_h(px(62.))
                        .pt(px(12.))
                        .pr(px(12.))
                        .pb(px(12.))
                        .pl(px(16.))
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .text_color(p.ink)
                        .child(
                            div()
                                .size(px(28.))
                                .rounded(radius)
                                .bg(p.stage)
                                .flex()
                                .items_center()
                                .justify_center()
                                .flex_none()
                                .child(icon(
                                    match self.stage {
                                        HandoffStage::Transferring => Glyph::Clock,
                                        HandoffStage::Unavailable => Glyph::Alert,
                                        HandoffStage::Resolved => Glyph::Check,
                                        _ => Glyph::User,
                                    },
                                    16.,
                                    p.secondary,
                                )),
                        )
                        .child(label),
                );
            if self.stage == HandoffStage::Transferring {
                card = card
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.secondary)
                            .px(px(16.))
                            .child(self.estimate.clone()),
                    )
                    .child(
                        face(
                            "handoff-cancel",
                            tr(cx, "Cancel transfer", "取消转接"),
                            32.,
                            8.,
                            Tone::Clear,
                            cx,
                        )
                        .px_2()
                        .child(tr(cx, "Cancel", "取消"))
                        .on_click(cx.listener(|s, _, _, cx| s.cancel_transfer(cx))),
                    );
            }
            if self.stage == HandoffStage::Unavailable {
                card = card
                    .child(
                        div()
                            .id("handoff-error")
                            .role(Role::Alert)
                            .text_color(p.danger)
                            .text_size(px(12.))
                            .px(px(16.))
                            .child(self.error.clone()),
                    )
                    .child(
                        face(
                            "handoff-retry",
                            tr(cx, "Try again", "重试"),
                            32.,
                            8.,
                            Tone::Surface,
                            cx,
                        )
                        .px_2()
                        .child(tr(cx, "Try again", "重试"))
                        .on_click(cx.listener(|s, _, _, cx| s.request_human(cx))),
                    );
            }
            if matches!(self.stage, HandoffStage::Connected | HandoffStage::Resolved) {
                card = card.child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.secondary)
                        .px(px(16.))
                        .child(self.agent.clone()),
                );
            }
            let open = self.summary_open;
            card = card.child(
                face(
                    "handoff-summary-toggle",
                    tr(cx, "Shared summary", "共享摘要"),
                    32.,
                    8.,
                    Tone::Clear,
                    cx,
                )
                .aria_expanded(open)
                .w_full()
                .py(px(8.))
                .pl(px(16.))
                .pr(px(12.))
                .justify_between()
                .text_color(p.secondary)
                .hover(|s| s.bg(p.tray.opacity(0.6)).text_color(p.ink))
                .child(tr(cx, "Shared summary", "共享摘要"))
                .on_click(cx.listener(|s, _, _, cx| {
                    s.summary_open = !s.summary_open;
                    s.summary_motion
                        .duration(f32::from(s.summary_open), 0.24, cx);
                    cx.notify();
                })),
            );
            if progress > 0.001 {
                let height = self.summary_height.clone();
                let mut body = div()
                    .relative()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .pt(px(12.))
                    .px(px(16.))
                    .pb(px(16.))
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(p.secondary)
                    .child(self.summary.issue.clone());
                for (name, value) in &self.summary.fields {
                    body = body.child(format!("{}: {}", name, value));
                }
                body = body.child(
                    canvas(
                        move |b, w, _| {
                            if (height.get() - b.size.height).abs() > px(0.5) {
                                height.set(b.size.height);
                                w.request_animation_frame();
                            }
                        },
                        |_, _, _, _| (),
                    )
                    .absolute()
                    .inset_0(),
                );
                card = card.child(
                    div()
                        .h(self.summary_height.get() * progress)
                        .overflow_hidden()
                        .child(body),
                );
            }
            thread = thread.child(card);
        }
        for message in self.messages.iter().filter(|message| message.after_handoff) {
            thread = thread.child(message_view(
                message,
                &self.agent,
                self.assistant_model.as_deref(),
                p,
                radius,
                cx,
            ));
        }
        let mut root = div()
            .id("agent-handoff")
            .role(Role::Group)
            .aria_label(self.title.clone())
            .w_full()
            .max_w(px(560.))
            .h(px(600.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_color(p.ink)
            .flex()
            .flex_col()
            .child(
                div()
                    .min_h(px(64.))
                    .pt(px(12.))
                    .pr(px(12.))
                    .pb(px(12.))
                    .pl(px(20.))
                    .border_b_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .line_height(px(20.))
                                    .child(self.title.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .text_color(p.secondary)
                                    .child(self.subtitle.clone()),
                            ),
                    )
                    .child(
                        div()
                            .id("handoff-status")
                            .role(Role::Status)
                            .h(px(32.))
                            .flex()
                            .items_center()
                            .px(px(8.))
                            .gap_2()
                            .rounded(radius)
                            .bg(p.stage)
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.secondary)
                            .child(div().size(px(6.)).flex_none().rounded_full().bg(
                                match self.stage {
                                    HandoffStage::Connected | HandoffStage::Resolved => p.success,
                                    HandoffStage::Unavailable => p.danger,
                                    HandoffStage::Transferring => p.bright,
                                    _ => p.secondary,
                                },
                            ))
                            .child(label),
                    ),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(p.secondary)
                    .px(px(20.))
                    .pt(px(8.))
                    .child(self.task.clone()),
            )
            .child(thread);
        if self.stage != HandoffStage::Resolved {
            let pending = self.sending.active.is_some();
            let action = if pending {
                tr(cx, "Cancel sending", "取消发送")
            } else if self.send_error.is_empty() {
                tr(cx, "Send", "发送")
            } else {
                tr(cx, "Retry send", "重试发送")
            };
            let bar = div()
                .h(px(42.))
                .pl(px(16.))
                .pr(px(4.))
                .flex()
                .items_center()
                .gap(px(8.))
                .rounded(radius)
                .border_1()
                .border_color(p.line)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .h_full()
                        .child(self.composer.clone()),
                )
                .child(
                    face("handoff-send", action, 32., 8., Tone::Surface, cx)
                        .size(px(32.))
                        .rounded(px(8.))
                        .p_0()
                        .child(icon(Glyph::Send, 16., p.ink))
                        .on_click(cx.listener(move |s, _, _, cx| {
                            if pending {
                                s.cancel_send(cx)
                            } else {
                                s.send(cx)
                            }
                        })),
                );
            let mut footer = div().m(px(12.)).flex().flex_col().gap(px(8.));
            if !self.send_error.is_empty() {
                footer = footer.child(
                    div()
                        .id("handoff-send-error")
                        .role(Role::Alert)
                        .text_size(px(12.))
                        .text_color(p.danger)
                        .child(self.send_error.clone()),
                );
            }
            root = root.child(footer.child(bar));
        }
        root
    }
}
#[cfg(test)]
mod tests {
    use super::{RequestGate, draft_is_unchanged};
    #[test]
    fn cancelled_or_superseded_transfer_cannot_complete() {
        let mut gate = RequestGate::default();
        let first = gate.begin();
        assert_eq!(gate.cancel(), Some(first));
        let second = gate.begin();
        assert!(!gate.settle(first));
        assert!(gate.settle(second));
        assert!(!gate.settle(second));
    }
    #[test]
    fn successful_send_preserves_a_draft_edited_while_waiting() {
        assert!(draft_is_unchanged(Some("hello"), "hello"));
        assert!(!draft_is_unchanged(Some("hello"), "hello there"));
        assert!(!draft_is_unchanged(Some("hello"), " hello"));
        assert!(!draft_is_unchanged(None, "hello"));
    }
}
