//! Reviewable reply drafting with host-owned generation and a real text editor.
use crate::{
    components::textarea::{Textarea, TextareaStyle},
    primitives::{
        brand_mark::brand_mark,
        control::UiLanguage,
        family::{Paint, PhaseClock, Tone, ease, face},
        field::{Field, FieldEvent},
    },
};
use gpui::{prelude::*, *};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplyMessage {
    pub sender: SharedString,
    pub subject: SharedString,
    pub body: SharedString,
}
impl ReplyMessage {
    pub fn new(
        sender: impl Into<SharedString>,
        subject: impl Into<SharedString>,
        body: impl Into<SharedString>,
    ) -> Self {
        Self {
            sender: sender.into(),
            subject: subject.into(),
            body: body.into(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplyHint {
    pub id: SharedString,
    pub label: SharedString,
}
impl ReplyHint {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReplyTone {
    #[default]
    Neutral,
    Warm,
    Formal,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReplyLength {
    Short,
    #[default]
    Medium,
    Long,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReplyStatus {
    #[default]
    Idle,
    Generating,
    Ready,
    Review,
    Stopped,
    Failed,
    Inserted,
}
#[derive(Clone, Debug)]
pub struct ReplyRequest {
    pub id: u64,
    pub message: ReplyMessage,
    pub tone: ReplyTone,
    pub length: ReplyLength,
    pub hint: Option<ReplyHint>,
    pub previous: SharedString,
}
#[derive(Clone, Debug)]
pub enum ReplyDraftEvent {
    Requested(ReplyRequest),
    Cancelled(u64),
    Insert(SharedString),
}
/// Generation is performed by the host. This control never sends a message.
pub struct ReplyDraft {
    message: ReplyMessage,
    sender_image: Option<ImageSource>,
    model: Option<SharedString>,
    hints: Vec<ReplyHint>,
    hint: Option<usize>,
    tone: ReplyTone,
    length: ReplyLength,
    status: ReplyStatus,
    draft: Entity<Textarea>,
    programmatic: Option<SharedString>,
    revision: u64,
    start_revision: u64,
    start_text: SharedString,
    request_seq: u64,
    active: Option<u64>,
    candidate: String,
    error: SharedString,
    tone_focus: [FocusHandle; 3],
    length_focus: [FocusHandle; 3],
    motion: PhaseClock,
    action_focus: FocusHandle,
    _sub: Subscription,
}
impl EventEmitter<ReplyDraftEvent> for ReplyDraft {}
impl ReplyDraft {
    pub fn new(
        message: ReplyMessage,
        hints: Vec<ReplyHint>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let l = UiLanguage::current(cx);
        let draft = cx.new(|cx| {
            Field::<TextareaStyle>::new(
                "reply-draft-text",
                l.text("Reply draft", "回复草稿"),
                w,
                cx,
            )
        });
        let sub = cx.subscribe_in(&draft, w, |s: &mut Self, _, event: &FieldEvent, _, cx| {
            let FieldEvent::Changed(value) = event else {
                return;
            };
            if s.programmatic.take().as_ref() == Some(value) {
                return;
            }
            s.revision = s.revision.wrapping_add(1);
            if s.status == ReplyStatus::Inserted {
                s.status = ReplyStatus::Ready;
            }
            cx.notify();
        });
        Self {
            message,
            sender_image: None,
            model: None,
            hints,
            hint: None,
            tone: ReplyTone::Neutral,
            length: ReplyLength::Medium,
            status: ReplyStatus::Idle,
            draft,
            programmatic: None,
            revision: 0,
            start_revision: 0,
            start_text: "".into(),
            request_seq: 0,
            active: None,
            candidate: String::new(),
            error: "".into(),
            tone_focus: [
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
            ],
            length_focus: [
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
            ],
            motion: Default::default(),
            action_focus: cx.focus_handle().tab_stop(true),
            _sub: sub,
        }
    }
    pub fn editor(&self) -> Entity<Textarea> {
        self.draft.clone()
    }
    /// Set the portrait shown with the source message.
    pub fn with_sender_image(mut self, image: impl Into<ImageSource>) -> Self {
        self.sender_image = Some(image.into());
        self
    }
    /// Name the host-selected model in the draft.
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = Some(model.into());
        self
    }
    pub fn status(&self) -> ReplyStatus {
        self.status
    }
    pub fn active_request(&self) -> Option<u64> {
        self.active
    }
    /// Replacing the source message cancels the old request and keeps any edited draft.
    pub fn set_message(&mut self, message: ReplyMessage, cx: &mut Context<Self>) {
        if self.message == message {
            return;
        }
        self.cancel(cx);
        self.message = message;
        self.candidate.clear();
        self.status = if self.value(cx).is_empty() {
            ReplyStatus::Idle
        } else {
            ReplyStatus::Ready
        };
        cx.notify();
    }
    pub fn value<'a>(&'a self, cx: &'a App) -> &'a str {
        self.draft.read(cx).value(cx)
    }
    fn cancel(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.active.take() {
            cx.emit(ReplyDraftEvent::Cancelled(id));
        }
    }
    /// Requests a new draft; existing text stays editable and remains untouched during generation.
    pub fn generate(&mut self, cx: &mut Context<Self>) {
        self.cancel(cx);
        self.request_seq = self.request_seq.wrapping_add(1).max(1);
        self.active = Some(self.request_seq);
        self.start_revision = self.revision;
        self.start_text = self.value(cx).to_owned().into();
        self.candidate.clear();
        self.error = "".into();
        self.status = ReplyStatus::Generating;
        self.motion.restart(cx);
        let previous = self.value(cx).to_owned().into();
        cx.emit(ReplyDraftEvent::Requested(ReplyRequest {
            id: self.request_seq,
            message: self.message.clone(),
            tone: self.tone,
            length: self.length,
            hint: self.hint.and_then(|i| self.hints.get(i)).cloned(),
            previous,
        }));
        cx.notify();
    }
    pub fn set_tone(&mut self, tone: ReplyTone, cx: &mut Context<Self>) {
        if self.tone != tone {
            self.tone = tone;
            self.generate(cx);
        }
    }
    pub fn set_length(&mut self, length: ReplyLength, cx: &mut Context<Self>) {
        if self.length != length {
            self.length = length;
            self.generate(cx);
        }
    }
    pub fn choose_hint(&mut self, id: &str, cx: &mut Context<Self>) -> bool {
        let Some(index) = self.hints.iter().position(|hint| hint.id.as_ref() == id) else {
            return false;
        };
        self.hint = Some(index);
        self.generate(cx);
        true
    }
    /// Late chunks after cancellation or supersession are rejected.
    pub fn append(&mut self, request: u64, chunk: &str, cx: &mut Context<Self>) -> bool {
        if self.active != Some(request) {
            return false;
        }
        self.candidate.push_str(chunk);
        cx.notify();
        true
    }
    pub fn finish(&mut self, request: u64, w: &Window, cx: &mut Context<Self>) -> bool {
        if self.active != Some(request) {
            return false;
        }
        self.active = None;
        if self.candidate.trim().is_empty() {
            self.error = UiLanguage::current(cx)
                .text(
                    "No draft was returned. Try again.",
                    "没有收到草稿，请重试。",
                )
                .into();
            self.status = ReplyStatus::Failed;
            self.motion.restart(cx);
            cx.notify();
            return true;
        }
        let editor = self.draft.read(cx).editor();
        if editor.read(cx).is_composing()
            || editor.read(cx).focus.is_focused(w)
            || self.revision != self.start_revision
            || self.value(cx) != self.start_text.as_ref()
        {
            self.status = ReplyStatus::Review;
        } else {
            self.apply_candidate(w, cx);
        }
        self.motion.restart(cx);
        cx.notify();
        true
    }
    fn apply_candidate(&mut self, w: &Window, cx: &mut Context<Self>) {
        let value: SharedString = self
            .candidate
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .into();
        // Editor::set_value emits no Changed event when the value is unchanged.
        // Do not leave a stale suppression marker for a later user edit.
        self.programmatic = (self.value(cx) != value.as_ref()).then_some(value.clone());
        self.draft
            .update(cx, |field, cx| field.set_value(value.to_string(), w, cx));
        self.candidate.clear();
        self.status = ReplyStatus::Ready;
    }
    /// Explicitly accept or discard a candidate that arrived during editing.
    pub fn review(&mut self, accept: bool, w: &Window, cx: &mut Context<Self>) -> bool {
        if self.status != ReplyStatus::Review {
            return false;
        }
        let editor = self.draft.read(cx).editor();
        if editor.read(cx).is_composing() {
            return false;
        }
        if accept {
            self.apply_candidate(w, cx);
        } else {
            self.candidate.clear();
            self.status = ReplyStatus::Ready;
        }
        self.motion.restart(cx);
        cx.notify();
        true
    }
    pub fn fail(
        &mut self,
        request: u64,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.active != Some(request) {
            return false;
        }
        self.active = None;
        self.error = message.into();
        self.status = ReplyStatus::Failed;
        self.motion.restart(cx);
        cx.notify();
        true
    }
    pub fn stop(&mut self, cx: &mut Context<Self>) {
        if self.active.is_some() {
            self.cancel(cx);
            self.status = ReplyStatus::Stopped;
            self.motion.restart(cx);
            cx.notify();
        }
    }
    /// Emits the current edited value for the host composer; it never sends on the user's behalf.
    pub fn insert(&mut self, cx: &mut Context<Self>) -> bool {
        let editor = self.draft.read(cx).editor();
        if editor.read(cx).is_composing() {
            return false;
        }
        let value: SharedString = self.value(cx).to_owned().into();
        if value.trim().is_empty() {
            return false;
        }
        self.cancel(cx);
        self.candidate.clear();
        self.status = ReplyStatus::Inserted;
        cx.emit(ReplyDraftEvent::Insert(value));
        self.motion.restart(cx);
        cx.notify();
        true
    }
}
impl Render for ReplyDraft {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let l = UiLanguage::current(cx);
        let radius = crate::foundation::surface_radius(cx);
        let t = |en: &'static str, zh: &'static str| l.text(en, zh);
        let selected_hint = p.panel.blend(p.bright.opacity(0.09));
        let selected_hint_hover = p.panel.blend(p.bright.opacity(0.13));
        let selected_edge = p.line.blend(p.bright.opacity(0.55));
        let mut hints = div().flex().flex_wrap().gap(px(8.));
        for (i, hint) in self.hints.iter().enumerate() {
            let id = hint.id.clone();
            let selected = self.hint == Some(i);
            hints = hints.child(
                face(
                    ("reply-hint", i),
                    hint.label.clone(),
                    32.,
                    radius.into(),
                    Tone::Surface,
                    cx,
                )
                .px(px(12.))
                .border_1()
                .border_color(if selected { selected_edge } else { p.line })
                .line_height(px(20.))
                .aria_toggled(selected.into())
                .when(selected, |v| v.bg(selected_hint))
                .hover(move |s| {
                    if selected {
                        s.bg(selected_hint_hover)
                    } else {
                        s.bg(p.tray)
                    }
                })
                .child(hint.label.clone())
                .on_click(cx.listener(move |s, _, _, cx| {
                    s.choose_hint(id.as_ref(), cx);
                })),
            );
        }
        let tones = [
            (ReplyTone::Neutral, t("Neutral", "自然")),
            (ReplyTone::Warm, t("Warm", "亲切")),
            (ReplyTone::Formal, t("Formal", "正式")),
        ];
        let mut tone_group = div()
            .id("reply-tones")
            .role(Role::RadioGroup)
            .aria_label(t("Tone", "语气"))
            .flex()
            .flex_wrap()
            .gap(px(2.));
        for (i, (tone, label)) in tones.into_iter().enumerate() {
            let active = self.tone == tone;
            tone_group = tone_group.child(
                face(
                    ("reply-tone", i),
                    label,
                    28.,
                    radius.into(),
                    Tone::Clear,
                    cx,
                )
                .px(px(12.))
                .text_size(px(12.))
                .line_height(px(16.))
                .track_focus(&self.tone_focus[i])
                .tab_index(if active { 0 } else { -1 })
                .role(Role::RadioButton)
                .aria_toggled(active.into())
                .when(active, |v| {
                    v.bg(p.panel)
                        .text_color(p.ink)
                        .border_1()
                        .border_color(p.line)
                })
                .when(!active, |v| {
                    v.text_color(p.secondary)
                        .hover(|s| s.bg(p.ink.opacity(0.05)).text_color(p.ink))
                })
                .child(label)
                .on_click(cx.listener(move |s, _, w, cx| {
                    s.set_tone(tone, cx);
                    s.tone_focus[i].focus(w, cx);
                }))
                .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                    let next = match e.keystroke.key.as_str() {
                        "left" | "up" => (i + 2) % 3,
                        "right" | "down" => (i + 1) % 3,
                        "home" => 0,
                        "end" => 2,
                        _ => return,
                    };
                    s.set_tone(
                        [ReplyTone::Neutral, ReplyTone::Warm, ReplyTone::Formal][next],
                        cx,
                    );
                    s.tone_focus[next].focus(w, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                })),
            );
        }
        let lengths = [
            (ReplyLength::Short, t("Short", "简短")),
            (ReplyLength::Medium, t("Medium", "适中")),
            (ReplyLength::Long, t("Long", "详细")),
        ];
        let mut length_group = div()
            .id("reply-lengths")
            .role(Role::RadioGroup)
            .aria_label(t("Length", "长度"))
            .flex()
            .flex_wrap()
            .gap(px(2.));
        for (i, (length, label)) in lengths.into_iter().enumerate() {
            let active = self.length == length;
            length_group = length_group.child(
                face(
                    ("reply-length", i),
                    label,
                    28.,
                    radius.into(),
                    Tone::Clear,
                    cx,
                )
                .px(px(12.))
                .text_size(px(12.))
                .line_height(px(16.))
                .track_focus(&self.length_focus[i])
                .tab_index(if active { 0 } else { -1 })
                .role(Role::RadioButton)
                .aria_toggled(active.into())
                .when(active, |v| {
                    v.bg(p.panel)
                        .text_color(p.ink)
                        .border_1()
                        .border_color(p.line)
                })
                .when(!active, |v| {
                    v.text_color(p.secondary)
                        .hover(|s| s.bg(p.ink.opacity(0.05)).text_color(p.ink))
                })
                .child(label)
                .on_click(cx.listener(move |s, _, w, cx| {
                    s.set_length(length, cx);
                    s.length_focus[i].focus(w, cx);
                }))
                .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                    let next = match e.keystroke.key.as_str() {
                        "left" | "up" => (i + 2) % 3,
                        "right" | "down" => (i + 1) % 3,
                        "home" => 0,
                        "end" => 2,
                        _ => return,
                    };
                    s.set_length(
                        [ReplyLength::Short, ReplyLength::Medium, ReplyLength::Long][next],
                        cx,
                    );
                    s.length_focus[next].focus(w, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                })),
            );
        }
        let status = match self.status {
            ReplyStatus::Idle => t("Ready to draft", "可以生成草稿"),
            ReplyStatus::Generating => t("Drafting", "正在起草"),
            ReplyStatus::Ready => t("Draft ready", "草稿已就绪"),
            ReplyStatus::Review => t("Review new draft", "请确认新草稿"),
            ReplyStatus::Stopped => t("Stopped", "已停止"),
            ReplyStatus::Failed => t("Failed", "失败"),
            ReplyStatus::Inserted => t("Inserted", "已插入"),
        };
        let age = ease(self.motion.time(0.16, w, cx) / 0.16);
        let mut actions = div().flex().flex_wrap().gap(px(6.));
        if self.active.is_some() {
            actions = actions.child(
                face(
                    "reply-action",
                    t("Stop", "停止"),
                    36.,
                    radius.into(),
                    Tone::Surface,
                    cx,
                )
                .track_focus(&self.action_focus)
                .px(px(10.))
                .child(t("Stop", "停止"))
                .on_click(cx.listener(|s, _, _, cx| s.stop(cx))),
            );
        } else {
            let label = if matches!(self.status, ReplyStatus::Failed | ReplyStatus::Stopped) {
                t("Try again", "重试")
            } else {
                t("Generate", "生成草稿")
            };
            actions = actions.child(
                face("reply-action", label, 36., radius.into(), Tone::Surface, cx)
                    .track_focus(&self.action_focus)
                    .px(px(10.))
                    .child(label)
                    .on_click(cx.listener(|s, _, _, cx| s.generate(cx))),
            );
        }
        actions = actions.child(
            face(
                "reply-insert",
                t("Insert draft", "插入草稿"),
                36.,
                radius.into(),
                Tone::Primary,
                cx,
            )
            .min_w(px(88.))
            .px(px(12.))
            .child(t("Insert", "插入"))
            .on_click(cx.listener(|s, _, _, cx| {
                s.insert(cx);
            })),
        );
        let mut root = div()
            .id("reply-draft")
            .role(Role::Article)
            .aria_label(format!(
                "{}: {}",
                t("Message from", "来信"),
                self.message.sender
            ))
            .w_full()
            .max_w(px(600.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .p(px(24.))
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(
                div()
                    .text_size(px(16.))
                    .line_height(px(24.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(self.message.subject.clone()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        div()
                            .w(px(36.))
                            .h(px(36.))
                            .rounded(px(18.))
                            .bg(p.tray)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .overflow_hidden()
                            .when_some(self.sender_image.clone(), |portrait, image| {
                                portrait.child(
                                    img(image)
                                        .size_full()
                                        .rounded_full()
                                        .object_fit(ObjectFit::Cover),
                                )
                            })
                            .when(self.sender_image.is_none(), |portrait| {
                                portrait.child(
                                    self.message
                                        .sender
                                        .chars()
                                        .next()
                                        .map(|c| c.to_string())
                                        .unwrap_or_default(),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(self.message.sender.clone()),
                    ),
            )
            .child(
                div()
                    .pl(px(48.))
                    .text_size(px(14.))
                    .line_height(px(22.))
                    .text_color(p.secondary)
                    .child(self.message.body.clone()),
            )
            .child(
                div()
                    .mt(px(8.))
                    .pt(px(24.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(hints)
                    .when_some(self.model.clone(), |draft, model| {
                        draft.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(12.))
                                .text_color(p.secondary)
                                .children(brand_mark(&model, 14., cx))
                                .child(model),
                        )
                    })
                    .child(self.draft.clone())
                    .child(
                        div()
                            .border_t_1()
                            .border_color(p.line)
                            .p(px(8.))
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(8.))
                            .child(tone_group)
                            .child(div().w(px(1.)).h(px(16.)).bg(p.line))
                            .child(length_group),
                    ),
            );
        if !self.candidate.is_empty()
            && matches!(
                self.status,
                ReplyStatus::Generating | ReplyStatus::Review | ReplyStatus::Stopped
            )
        {
            root = root.child(
                div()
                    .rounded(radius)
                    .bg(p.tray)
                    .p(px(10.))
                    .text_size(px(14.))
                    .line_height(px(22.))
                    .text_color(p.ink)
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.secondary)
                            .child(t("New draft preview", "新草稿预览")),
                    )
                    .child(self.candidate.clone()),
            );
        }
        if self.status == ReplyStatus::Review {
            root = root.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(8.))
                    .child(
                        face(
                            "reply-accept",
                            t("Use new draft", "使用新草稿"),
                            36.,
                            radius.into(),
                            Tone::Surface,
                            cx,
                        )
                        .px(px(10.))
                        .child(t("Use new draft", "使用新草稿"))
                        .on_click(cx.listener(|s, _, w, cx| {
                            s.review(true, w, cx);
                        })),
                    )
                    .child(
                        face(
                            "reply-keep",
                            t("Keep my draft", "保留我的草稿"),
                            36.,
                            radius.into(),
                            Tone::Clear,
                            cx,
                        )
                        .px(px(10.))
                        .child(t("Keep my draft", "保留我的草稿"))
                        .on_click(cx.listener(|s, _, w, cx| {
                            s.review(false, w, cx);
                        })),
                    ),
            );
        }
        if self.status == ReplyStatus::Failed {
            root = root.child(
                div()
                    .id("reply-error")
                    .role(Role::Alert)
                    .text_size(px(14.))
                    .line_height(px(22.))
                    .text_color(p.danger)
                    .child(self.error.clone()),
            );
        }
        root.child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .id("reply-status")
                        .role(Role::Status)
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(p.secondary)
                        .opacity(age)
                        .child(status),
                )
                .child(actions),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{ReplyDraft, ReplyHint, ReplyMessage, ReplyStatus, ReplyTone};
    use gpui::{EntityInputHandler, TestAppContext};
    fn message() -> ReplyMessage {
        ReplyMessage::new("Sam", "Meeting", "Are you free tomorrow?")
    }
    #[test]
    fn cancelled_results_are_ignored_and_manual_edits_are_preserved() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) = app.add_window_view(|w, cx| {
            ReplyDraft::new(message(), vec![ReplyHint::new("yes", "Yes")], w, cx)
        });
        w.update(|_, cx| {
            control.update(cx, |draft, cx| {
                draft.generate(cx);
            })
        });
        let first = w.update(|_, cx| control.read(cx).active_request().unwrap());
        w.update(|_, cx| {
            control.update(cx, |draft, cx| {
                draft.set_tone(ReplyTone::Warm, cx);
            })
        });
        let second = w.update(|_, cx| control.read(cx).active_request().unwrap());
        assert_ne!(first, second);
        w.update(|w, cx| {
            let editor = control.read(cx).editor();
            editor.update(cx, |field, cx| field.set_value("My own reply", w, cx));
            control.update(cx, |draft, cx| {
                assert!(!draft.append(first, "late", cx));
                assert!(draft.append(second, "Generated reply", cx));
            });
        });
        w.update(|w, cx| {
            control.update(cx, |draft, cx| {
                assert!(draft.finish(second, w, cx));
                assert_eq!(draft.status(), ReplyStatus::Review);
                assert_eq!(draft.value(cx), "My own reply");
                assert!(draft.review(false, w, cx));
                assert_eq!(draft.value(cx), "My own reply");
                assert!(draft.insert(cx));
            })
        });
    }
    #[test]
    fn result_does_not_replace_a_change_with_a_pending_edit_event() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) = app.add_window_view(|w, cx| ReplyDraft::new(message(), vec![], w, cx));
        w.update(|_, cx| control.update(cx, |draft, cx| draft.generate(cx)));
        let request = w.update(|_, cx| control.read(cx).active_request().unwrap());
        w.update(|w, cx| {
            control.update(cx, |draft, cx| {
                assert!(draft.append(request, "Generated", cx));
                draft
                    .editor()
                    .update(cx, |field, cx| field.set_value("Edited", w, cx));
                assert!(draft.finish(request, w, cx));
                assert_eq!(draft.status(), ReplyStatus::Review);
                assert_eq!(draft.value(cx), "Edited");
            })
        });
    }

    #[test]
    fn unchanged_generated_text_does_not_suppress_the_next_edit() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) = app.add_window_view(|w, cx| ReplyDraft::new(message(), vec![], w, cx));
        w.update(|w, cx| {
            control
                .read(cx)
                .editor()
                .update(cx, |field, cx| field.set_value("Same", w, cx))
        });
        w.update(|_, cx| control.update(cx, |draft, cx| draft.generate(cx)));
        let request = w.update(|_, cx| control.read(cx).active_request().unwrap());
        w.update(|w, cx| {
            control.update(cx, |draft, cx| {
                assert!(draft.append(request, "Same", cx));
                assert!(draft.finish(request, w, cx));
                assert!(draft.programmatic.is_none());
            })
        });
        let before = w.update(|_, cx| control.read(cx).revision);
        w.update(|w, cx| {
            control
                .read(cx)
                .editor()
                .update(cx, |field, cx| field.set_value("My edit", w, cx))
        });
        w.update(|_, cx| assert!(control.read(cx).revision > before));
    }
    #[test]
    fn composition_blocks_replacement_and_insertion() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) = app.add_window_view(|w, cx| ReplyDraft::new(message(), vec![], w, cx));
        w.update(|_, cx| control.update(cx, |draft, cx| draft.generate(cx)));
        let request = w.update(|_, cx| control.read(cx).active_request().unwrap());
        let editor = w.update(|_, cx| control.read(cx).editor().read(cx).editor());
        w.update(|w, cx| {
            editor.update(cx, |editor, cx| {
                editor.focus(w, cx);
                editor.replace_and_mark_text_in_range(None, "ni", Some(2..2), w, cx);
            })
        });
        w.update(|w, cx| {
            control.update(cx, |draft, cx| {
                assert!(draft.append(request, "Generated reply", cx));
                assert!(draft.finish(request, w, cx));
                assert_eq!(draft.status(), ReplyStatus::Review);
                assert_eq!(draft.value(cx), "ni");
                assert!(!draft.review(true, w, cx));
                assert!(!draft.insert(cx));
            })
        });
    }
}
