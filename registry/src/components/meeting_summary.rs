//! Host-provided meeting notes, citations and task delivery.
use crate::primitives::{
    clipboard,
    family::{Paint, Tone, face},
    nav::tr,
};
use gpui::{prelude::*, *};
use std::collections::HashMap;

#[derive(Clone)]
pub struct MeetingQuote {
    pub speaker: SharedString,
    pub time: SharedString,
    pub text: SharedString,
}
impl MeetingQuote {
    pub fn new(
        speaker: impl Into<SharedString>,
        time: impl Into<SharedString>,
        text: impl Into<SharedString>,
    ) -> Self {
        Self {
            speaker: speaker.into(),
            time: time.into(),
            text: text.into(),
        }
    }
}
#[derive(Clone)]
pub struct MeetingPoint {
    pub id: SharedString,
    pub text: SharedString,
    pub quote: Option<MeetingQuote>,
}
impl MeetingPoint {
    pub fn new(id: impl Into<SharedString>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            quote: None,
        }
    }
    pub fn quote(mut self, quote: MeetingQuote) -> Self {
        self.quote = Some(quote);
        self
    }
}
#[derive(Clone)]
pub struct MeetingAction {
    pub point: MeetingPoint,
    pub owner: SharedString,
    pub due: SharedString,
    pub done: bool,
}
impl MeetingAction {
    pub fn new(point: MeetingPoint, owner: impl Into<SharedString>) -> Self {
        Self {
            point,
            owner: owner.into(),
            due: "".into(),
            done: false,
        }
    }
}
#[derive(Clone)]
pub struct MeetingData {
    pub title: SharedString,
    pub meta: SharedString,
    pub attendees: Vec<SharedString>,
    pub summary: Vec<MeetingPoint>,
    pub decisions: Vec<MeetingPoint>,
    pub actions: Vec<MeetingAction>,
}
impl MeetingData {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            meta: "".into(),
            attendees: Vec::new(),
            summary: Vec::new(),
            decisions: Vec::new(),
            actions: Vec::new(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeetingStatus {
    Summarizing,
    Ready,
    Error,
    NoTranscript,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MeetingSummaryEvent {
    ActionChanged {
        id: SharedString,
        done: bool,
    },
    Send {
        ticket: u64,
        id: SharedString,
        target: SharedString,
    },
    Cancel(u64),
    Regenerate,
    CopyResult(bool),
}
#[derive(Clone)]
enum SendState {
    Pending(u64),
    Sent(SharedString),
    Failed(SharedString),
}
pub struct MeetingSummary {
    data: MeetingData,
    status: MeetingStatus,
    error: SharedString,
    send_states: HashMap<(SharedString, SharedString), SendState>,
    targets: Vec<SharedString>,
    ticket: u64,
    quote: Option<SharedString>,
    copying: bool,
    copied: Option<bool>,
}
impl EventEmitter<MeetingSummaryEvent> for MeetingSummary {}
impl MeetingSummary {
    pub fn new(data: MeetingData) -> Self {
        Self {
            data,
            status: MeetingStatus::Ready,
            error: "".into(),
            send_states: HashMap::new(),
            targets: vec!["Linear".into(), "Notion".into()],
            ticket: 0,
            quote: None,
            copying: false,
            copied: None,
        }
    }
    pub fn targets(mut self, targets: Vec<SharedString>) -> Self {
        self.targets = targets;
        self
    }
    pub fn set_data(&mut self, data: MeetingData, cx: &mut Context<Self>) {
        for state in self.send_states.values() {
            if let SendState::Pending(ticket) = state {
                cx.emit(MeetingSummaryEvent::Cancel(*ticket));
            }
        }
        self.send_states.clear();
        self.quote = None;
        self.data = data;
        self.status = MeetingStatus::Ready;
        cx.notify();
    }
    pub fn set_status(
        &mut self,
        status: MeetingStatus,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        if status != MeetingStatus::Ready {
            self.send_states.retain(|_, state| {
                if let SendState::Pending(ticket) = state {
                    cx.emit(MeetingSummaryEvent::Cancel(*ticket));
                    false
                } else {
                    true
                }
            });
        }
        self.status = status;
        self.error = error.into();
        cx.notify();
    }
    pub fn finish_send(
        &mut self,
        ticket: u64,
        result: Result<SharedString, SharedString>,
        cx: &mut Context<Self>,
    ) {
        if let Some(state) = self
            .send_states
            .values_mut()
            .find(|s| matches!(s,SendState::Pending(active) if *active==ticket))
        {
            *state = match result {
                Ok(reference) => SendState::Sent(reference),
                Err(message) => SendState::Failed(message),
            };
            cx.notify();
        }
    }
    fn send(&mut self, id: SharedString, target: SharedString, cx: &mut Context<Self>) {
        let key = (id.clone(), target.clone());
        if matches!(
            self.send_states.get(&key),
            Some(SendState::Pending(_) | SendState::Sent(_))
        ) {
            return;
        }
        self.ticket = self.ticket.wrapping_add(1);
        let ticket = self.ticket;
        self.send_states.insert(key, SendState::Pending(ticket));
        cx.emit(MeetingSummaryEvent::Send { ticket, id, target });
        cx.notify();
    }
    fn copy(&mut self, cx: &mut Context<Self>) {
        if self.copying {
            return;
        }
        self.copying = true;
        let mut text = format!("{}\n{}\n", self.data.title, self.data.meta);
        for point in &self.data.summary {
            text.push_str(&format!("\n• {}", point.text));
        }
        for point in &self.data.decisions {
            text.push_str(&format!("\n• {}", point.text));
        }
        for action in &self.data.actions {
            text.push_str(&format!(
                "\n{} {} — {}",
                if action.done { "[x]" } else { "[ ]" },
                action.point.text,
                action.owner
            ));
        }
        let write = clipboard::write(&text, cx);
        cx.spawn(async move |this, cx| {
            let ok = write.await;
            let _ = this.update(cx, |s, cx| {
                s.copying = false;
                s.copied = Some(ok);
                cx.emit(MeetingSummaryEvent::CopyResult(ok));
                cx.notify();
            });
        })
        .detach();
    }
}
impl Render for MeetingSummary {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let mut root = div()
            .id("meeting-summary")
            .role(Role::Group)
            .aria_label(self.data.title.clone())
            .w_full()
            .max_w(px(640.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_color(p.ink)
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                if e.keystroke.key == "escape" && s.quote.is_some() {
                    s.quote = None;
                    cx.notify();
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .pt(px(18.))
                    .px(px(20.))
                    .pb(px(16.))
                    .border_b_1()
                    .border_color(p.line)
                    .child(
                        div()
                            .text_size(px(15.))
                            .line_height(px(22.))
                            .child(self.data.title.clone()),
                    )
                    .child(
                        div()
                            .mt(px(2.))
                            .flex()
                            .flex_wrap()
                            .gap(px(8.))
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(p.secondary)
                            .child(self.data.meta.clone())
                            .child(
                                self.data
                                    .attendees
                                    .iter()
                                    .map(|v| v.as_ref())
                                    .collect::<Vec<_>>()
                                    .join(" · "),
                            ),
                    ),
            );
        if self.status == MeetingStatus::Summarizing {
            return root.child(
                div()
                    .min_h(px(58.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .py(px(18.))
                    .px(px(20.))
                    .text_size(px(14.))
                    .line_height(px(22.))
                    .text_color(p.secondary)
                    .child(tr(cx, "Summarizing transcript…", "正在整理会议记录…")),
            );
        }
        if self.status == MeetingStatus::NoTranscript {
            return root.child(
                div()
                    .min_h(px(100.))
                    .p(px(20.))
                    .text_size(px(14.))
                    .text_color(p.secondary)
                    .child(tr(
                        cx,
                        "No transcript is available for this meeting.",
                        "这场会议没有转录记录。",
                    )),
            );
        }
        if self.status == MeetingStatus::Error {
            return root
                .child(
                    div()
                        .id("meeting-error")
                        .role(Role::Alert)
                        .pt(px(18.))
                        .px(px(20.))
                        .pb(px(20.))
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.danger)
                        .child(self.error.clone()),
                )
                .child(
                    face(
                        "meeting-retry",
                        tr(cx, "Try again", "重试"),
                        32.,
                        8.,
                        Tone::Surface,
                        cx,
                    )
                    .px_2()
                    .child(tr(cx, "Try again", "重试"))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(MeetingSummaryEvent::Regenerate))),
                );
        }
        let sections = [
            (tr(cx, "Summary", "摘要"), &self.data.summary),
            (tr(cx, "Decisions", "决定"), &self.data.decisions),
        ];
        let mut notes = div()
            .flex()
            .flex_col()
            .gap(px(18.))
            .pt(px(16.))
            .px(px(20.))
            .pb(px(12.));
        for (section, points) in sections {
            let mut list = div()
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(div().text_size(px(13.)).line_height(px(20.)).child(section));
            for (index, point) in points.iter().enumerate() {
                let mut row = div().flex().flex_col().gap_1();
                let mut heading = div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .px(px(8.))
                    .py(px(5.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .child(div().size(px(5.)).rounded_full().bg(p.secondary))
                    .child(div().flex_1().child(point.text.clone()));
                if let Some(quote) = &point.quote {
                    let id = point.id.clone();
                    let time = quote.time.clone();
                    let open = self.quote.as_ref() == Some(&id);
                    heading = heading.child(
                        face(
                            (section, index),
                            format!("{} · {}", time, point.text),
                            22.,
                            7.,
                            Tone::Clear,
                            cx,
                        )
                        .px(px(6.))
                        .text_size(px(12.))
                        .aria_expanded(open)
                        .child(time)
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.quote = if s.quote.as_ref() == Some(&id) {
                                None
                            } else {
                                Some(id.clone())
                            };
                            cx.notify();
                        })),
                    );
                }
                row = row.child(heading);
                if self.quote.as_ref() == Some(&point.id)
                    && let Some(quote) = &point.quote
                {
                    row = row.child(
                        div()
                            .max_w(px(440.))
                            .pt(px(10.))
                            .px(px(12.))
                            .pb(px(11.))
                            .rounded(crate::foundation::surface_radius(cx))
                            .border_1()
                            .border_color(p.line)
                            .bg(p.tray)
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(p.secondary)
                            .child(format!(
                                "{} · {} — {}",
                                quote.speaker, quote.time, quote.text
                            )),
                    );
                }
                list = list.child(row);
            }
            if !points.is_empty() {
                notes = notes.child(list);
            }
        }
        if !self.data.actions.is_empty() {
            let mut actions = div().flex().flex_col().gap(px(4.)).child(
                div().text_size(px(13.)).line_height(px(20.)).child(tr(
                    cx,
                    "Action items",
                    "行动事项",
                )),
            );
            for (index, item) in self.data.actions.iter().enumerate() {
                let id = item.point.id.clone();
                let action_id = id.clone();
                let checked = item.done;
                let mut line = div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(10.))
                    .px(px(8.))
                    .py(px(6.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .child(
                        face(
                            ("meeting-check", index),
                            item.point.text.clone(),
                            16.,
                            5.,
                            Tone::Clear,
                            cx,
                        )
                        .role(Role::CheckBox)
                        .aria_toggled(checked.into())
                        .child(if checked { "☑" } else { "□" })
                        .on_click(cx.listener(move |s, _, _, cx| {
                            if let Some(item) =
                                s.data.actions.iter_mut().find(|a| a.point.id == action_id)
                            {
                                item.done = !item.done;
                                cx.emit(MeetingSummaryEvent::ActionChanged {
                                    id: action_id.clone(),
                                    done: item.done,
                                });
                                cx.notify();
                            }
                        })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(120.))
                            .text_color(if checked { p.secondary } else { p.ink })
                            .child(item.point.text.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.secondary)
                            .child(format!("{} {}", item.owner, item.due)),
                    );
                for (target_index, target) in self.targets.iter().enumerate() {
                    let target = target.clone();
                    let key = (id.clone(), target.clone());
                    let state = self.send_states.get(&key);
                    let name = match state {
                        Some(SendState::Pending(_)) => tr(cx, "Sending…", "发送中…").to_string(),
                        Some(SendState::Sent(reference)) => format!("✓ {reference}"),
                        Some(SendState::Failed(message)) => {
                            format!("{} · {}", tr(cx, "Retry", "重试"), message)
                        }
                        None => target.to_string(),
                    };
                    let item_id = id.clone();
                    let target_id = target.clone();
                    line = line.child(
                        face(
                            ("meeting-send", index * 16 + target_index),
                            format!("{} · {}", name, item.point.text),
                            28.,
                            8.,
                            Tone::Clear,
                            cx,
                        )
                        .px_2()
                        .child(name)
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.send(item_id.clone(), target_id.clone(), cx)
                        })),
                    );
                }
                if let Some(quote) = &item.point.quote {
                    let quote_id = item.point.id.clone();
                    let open = self.quote.as_ref() == Some(&quote_id);
                    line = line.child(
                        face(
                            ("meeting-action-quote", index),
                            format!("{} · {}", quote.time, item.point.text),
                            22.,
                            7.,
                            Tone::Clear,
                            cx,
                        )
                        .px(px(6.))
                        .text_size(px(12.))
                        .aria_expanded(open)
                        .child(quote.time.clone())
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.quote = if s.quote.as_ref() == Some(&quote_id) {
                                None
                            } else {
                                Some(quote_id.clone())
                            };
                            cx.notify();
                        })),
                    );
                }
                let mut entry = div().flex().flex_col().gap_1().child(line);
                if self.quote.as_ref() == Some(&item.point.id)
                    && let Some(quote) = &item.point.quote
                {
                    entry = entry.child(
                        div()
                            .max_w(px(440.))
                            .pt(px(10.))
                            .px(px(12.))
                            .pb(px(11.))
                            .rounded(crate::foundation::surface_radius(cx))
                            .border_1()
                            .border_color(p.line)
                            .bg(p.tray)
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(p.secondary)
                            .child(format!(
                                "{} · {} — {}",
                                quote.speaker, quote.time, quote.text
                            )),
                    );
                }
                actions = actions.child(entry);
            }
            notes = notes.child(actions);
        }
        root = root.child(notes);
        let copy = match self.copied {
            Some(true) => tr(cx, "Copied", "已复制"),
            Some(false) => tr(cx, "Copy failed — retry", "复制失败，重试"),
            None => tr(cx, "Copy notes", "复制纪要"),
        };
        root.child(
            div()
                .min_h(px(48.))
                .flex()
                .flex_wrap()
                .items_center()
                .justify_end()
                .gap(px(12.))
                .pt(px(8.))
                .pr(px(12.))
                .pb(px(8.))
                .pl(px(20.))
                .border_t_1()
                .border_color(p.line)
                .child(
                    face("meeting-copy", copy, 32., 8., Tone::Surface, cx)
                        .px_2()
                        .child(copy)
                        .on_click(cx.listener(|s, _, _, cx| s.copy(cx))),
                )
                .child(
                    face(
                        "meeting-regenerate",
                        tr(cx, "Regenerate", "重新生成"),
                        32.,
                        8.,
                        Tone::Surface,
                        cx,
                    )
                    .px_2()
                    .child(tr(cx, "Regenerate", "重新生成"))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(MeetingSummaryEvent::Regenerate))),
                ),
        )
    }
}
