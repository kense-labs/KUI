//! Host-provided meeting notes, citations and task delivery.
use crate::primitives::{
    clipboard,
    family::{Paint, PhaseClock, Slide, Tone, ease, face},
    glyph::{Glyph, icon, transformed},
    nav::tr,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};

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
    credit: SharedString,
    transcript_words: Option<usize>,
    reveal: PhaseClock,
    loading: PhaseClock,
    measured_body: Rc<Cell<Pixels>>,
    body_height: Slide,
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
            credit: "".into(),
            transcript_words: None,
            reveal: PhaseClock::default(),
            loading: PhaseClock::default(),
            measured_body: Rc::default(),
            body_height: Slide::new(0.),
        }
    }
    pub fn targets(mut self, targets: Vec<SharedString>) -> Self {
        self.targets = targets;
        self
    }
    pub fn with_transcript(mut self, credit: impl Into<SharedString>, words: usize) -> Self {
        self.credit = credit.into();
        self.transcript_words = Some(words);
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
        self.copied = None;
        if self.status == MeetingStatus::Summarizing {
            self.reveal.restart(cx);
        }
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
        if status == MeetingStatus::Summarizing {
            self.loading.restart(cx);
        }
        if status != MeetingStatus::Ready {
            self.quote = None;
            self.reveal = PhaseClock::default();
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

fn stack() -> Div {
    div().flex().flex_col().min_w_0()
}

fn avatar(name: &str, size: f32, p: Paint) -> Div {
    let colors = [0x8468ab, 0x328b85, 0xbb7856, 0x5b78ab];
    let index = name
        .bytes()
        .fold(0usize, |sum, byte| sum + usize::from(byte))
        % colors.len();
    let color: Hsla = rgb(colors[index]).into();
    div()
        .size(px(size))
        .flex_none()
        .rounded_full()
        .border_1()
        .border_color(p.panel)
        .bg(p.panel.blend(color.opacity(0.24)))
        .text_color(p.ink)
        .text_size(px(size * 0.40))
        .flex()
        .items_center()
        .justify_center()
        .child(
            name.chars()
                .next()
                .map_or_else(String::new, |ch| ch.to_uppercase().collect()),
        )
}

impl MeetingSummary {
    fn citation(
        &self,
        point: &MeetingPoint,
        enabled: bool,
        cx: &Context<Self>,
    ) -> Option<Stateful<Div>> {
        let quote = point.quote.as_ref()?;
        let id = point.id.clone();
        let open = self.quote.as_ref() == Some(&id);
        Some(
            face(
                (ElementId::from("meeting-quote"), id.clone()),
                format!("{} · {}", quote.time, point.text),
                22.,
                5.,
                Tone::Clear,
                cx,
            )
            .flex_none()
            .px(px(5.))
            .text_size(px(11.))
            .text_color(Paint::get(cx).faint)
            .aria_expanded(open)
            .child(quote.time.clone())
            .when(!enabled, |button| {
                button.opacity(0.).tab_index(-1).cursor_default()
            })
            .when(enabled, |button| {
                button.on_click(cx.listener(move |s, _, _, cx| {
                    s.quote = (s.quote.as_ref() != Some(&id)).then(|| id.clone());
                    cx.notify();
                }))
            }),
        )
    }

    fn transcript(&self, point: &MeetingPoint, p: Paint) -> Option<Div> {
        if self.quote.as_ref() != Some(&point.id) {
            return None;
        }
        let quote = point.quote.as_ref()?;
        Some(
            stack()
                .ml(px(22.))
                .mt(px(4.))
                .py(px(7.))
                .pl(px(10.))
                .border_l_2()
                .border_color(p.line)
                .text_color(p.secondary)
                .text_size(px(12.))
                .line_height(px(18.))
                .child(
                    div()
                        .text_color(p.ink)
                        .child(format!("{} · {}", quote.speaker, quote.time)),
                )
                .child(quote.text.clone()),
        )
    }

    fn delivery(
        &self,
        item: &MeetingAction,
        target: &SharedString,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let p = Paint::get(cx);
        let key = (item.point.id.clone(), target.clone());
        let (mark, ink, detail, blocked) = match self.send_states.get(&key) {
            Some(SendState::Pending(_)) => {
                (Glyph::Clock, p.faint, tr(cx, "Sending…", "发送中…"), true)
            }
            Some(SendState::Sent(reference)) => (Glyph::Check, p.success, reference.as_ref(), true),
            Some(SendState::Failed(message)) => (Glyph::Alert, p.danger, message.as_ref(), false),
            None => {
                let mark = if target.as_ref() == "Linear" {
                    Glyph::Slash
                } else {
                    Glyph::Book
                };
                (mark, p.secondary, target.as_ref(), false)
            }
        };
        face(
            (
                ElementId::from("meeting-send"),
                SharedString::from(format!("{}:{}", key.0, key.1)),
            ),
            format!("{} · {} · {}", target, detail, item.point.text),
            28.,
            6.,
            Tone::Clear,
            cx,
        )
        .w(px(28.))
        .flex_none()
        .when(blocked, |button| button.tab_index(-1).cursor_default())
        .child(icon(mark, 14., ink))
        .when(!blocked, |button| {
            button
                .on_click(cx.listener(move |s, _, _, cx| s.send(key.0.clone(), key.1.clone(), cx)))
        })
    }

    fn notes(&self, age: f32, cx: &Context<Self>) -> Div {
        let p = Paint::get(cx);
        let mut body = stack()
            .pt(px(16.))
            .px(px(20.))
            .pb(px(16.))
            .gap(px(16.))
            .text_size(px(13.))
            .line_height(px(19.));
        let mut offset = 0.;
        for (title, points, decision) in [
            (tr(cx, "Summary", "摘要"), &self.data.summary, false),
            (tr(cx, "Decisions", "决定"), &self.data.decisions, true),
        ] {
            if points.is_empty() || age < offset {
                continue;
            }
            let mut section = stack()
                .gap(px(6.))
                .child(div().font_weight(FontWeight::MEDIUM).child(title));
            for entry in points {
                let time = age - offset;
                offset += 0.17;
                if time < 0. {
                    break;
                }
                let mark = if decision {
                    div()
                        .size(px(14.))
                        .child(icon(Glyph::Check, 13., p.success))
                } else {
                    div()
                        .size(px(14.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(div().size(px(4.)).rounded_full().bg(p.faint))
                };
                section = section.child(
                    stack()
                        .opacity(ease((time / 0.14).clamp(0., 1.)))
                        .child(
                            div()
                                .flex()
                                .items_start()
                                .gap(px(8.))
                                .child(mark.mt(px(2.)).flex_none())
                                .child(div().flex_1().min_w_0().child(reveal_text(
                                    &entry.text,
                                    time,
                                    p.ink,
                                )))
                                .children(self.citation(entry, time >= 0.14, cx)),
                        )
                        .children(self.transcript(entry, p)),
                );
            }
            body = body.child(section);
            offset += 0.08;
            if age < offset {
                return body;
            }
        }
        if !self.data.actions.is_empty() && age >= offset {
            let done = self.data.actions.iter().filter(|item| item.done).count();
            let mut actions = stack().child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .pb(px(6.))
                    .child(div().font_weight(FontWeight::MEDIUM).child(tr(
                        cx,
                        "Action items",
                        "行动事项",
                    )))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(p.faint)
                            .child(format!("{done}/{}", self.data.actions.len())),
                    ),
            );
            for item in &self.data.actions {
                let time = age - offset;
                offset += 0.17;
                if time < 0. {
                    break;
                }
                let id = item.point.id.clone();
                let ink = if item.done { p.secondary } else { p.ink };
                let check = face(
                    (ElementId::from("meeting-check"), id.clone()),
                    item.point.text.clone(),
                    16.,
                    4.,
                    Tone::Clear,
                    cx,
                )
                .w(px(16.))
                .flex_none()
                .role(Role::CheckBox)
                .aria_toggled(item.done.into())
                .border_1()
                .border_color(if item.done { p.bright } else { p.line })
                .when(item.done, |box_| {
                    box_.bg(p.bright).child(icon(Glyph::Check, 12., p.inverse))
                })
                .on_click(cx.listener(move |s, _, _, cx| {
                    if let Some(item) = s.data.actions.iter_mut().find(|item| item.point.id == id) {
                        item.done = !item.done;
                        cx.emit(MeetingSummaryEvent::ActionChanged {
                            id: id.clone(),
                            done: item.done,
                        });
                        cx.notify();
                    }
                }));
                let meta = div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(11.))
                    .text_color(p.faint)
                    .child(avatar(&item.owner, 17., p))
                    .child(item.owner.clone())
                    .when(!item.due.is_empty(), |row| {
                        row.child("·").child(item.due.clone())
                    })
                    .children(self.citation(&item.point, time >= 0.14, cx));
                actions = actions.child(
                    stack()
                        .py(px(7.))
                        .border_t_1()
                        .border_color(p.line)
                        .opacity(ease((time / 0.14).clamp(0., 1.)))
                        .child(
                            div()
                                .flex()
                                .items_start()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .size(px(16.))
                                        .flex_none()
                                        .mt(px(2.))
                                        .when(time >= 0.14, |slot| slot.child(check)),
                                )
                                .child(
                                    stack()
                                        .flex_1()
                                        .gap(px(3.))
                                        .text_color(ink)
                                        .child(reveal_text(&item.point.text, time, ink))
                                        .child(meta),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .justify_end()
                                        .flex_none()
                                        .gap(px(2.))
                                        .min_w(px(
                                            (self.targets.len().min(2) as f32 * 30. - 2.).max(0.)
                                        ))
                                        .max_w(relative(0.34))
                                        .when(time >= 0.14, |row| {
                                            row.children(
                                                self.targets
                                                    .iter()
                                                    .map(|target| self.delivery(item, target, cx)),
                                            )
                                        }),
                                ),
                        )
                        .children(self.transcript(&item.point, p)),
                );
            }
            body = body.child(actions);
        }
        body
    }
}

fn reveal_text(text: &SharedString, age: f32, ink: Hsla) -> StyledText {
    if age >= 0.24 {
        return StyledText::new(text.clone());
    }
    let count = text.chars().count();
    let mut starts = [text.len(); 6];
    starts[0] = 0;
    let mut part = 1;
    for (index, (byte, _)) in text.char_indices().enumerate() {
        while part < 5 && index >= count * part / 5 {
            starts[part] = byte;
            part += 1;
        }
    }
    let highlights = starts
        .windows(2)
        .enumerate()
        .filter(|(_, bytes)| bytes[0] < bytes[1])
        .map(|(index, bytes)| {
            let delay = index as f32 * 0.032;
            let opacity = ease(((age - delay) / 0.11).clamp(0., 1.));
            (
                bytes[0]..bytes[1],
                HighlightStyle {
                    color: Some(ink.opacity(opacity)),
                    ..Default::default()
                },
            )
        });
    StyledText::new(text.clone()).with_highlights(highlights)
}

impl Render for MeetingSummary {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let ready = self.status == MeetingStatus::Ready;
        let mut people = div().flex().items_center().flex_none();
        for (index, name) in self.data.attendees.iter().take(4).enumerate() {
            people = people.child(avatar(name, 25., p).when(index > 0, |node| node.ml(px(-6.))));
        }
        let extra = self.data.attendees.len().saturating_sub(4);
        if extra > 0 {
            people = people.child(
                div()
                    .ml(px(-4.))
                    .size(px(25.))
                    .rounded_full()
                    .border_1()
                    .border_color(p.panel)
                    .bg(p.tray)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(10.))
                    .child(format!("+{extra}")),
            );
        }
        let label = tr(cx, "Regenerate", "重新生成");
        let regenerate = face("meeting-regenerate", label, 30., 6., Tone::Clear, cx)
            .w(px(30.))
            .child(icon(Glyph::Redo, 15., p.secondary))
            .on_click(cx.listener(|_, _, _, cx| cx.emit(MeetingSummaryEvent::Regenerate)));
        let copy_label = match self.copied {
            Some(true) => tr(cx, "Copied", "已复制"),
            Some(false) => tr(cx, "Copy failed — retry", "复制失败，重试"),
            None => tr(cx, "Copy notes", "复制纪要"),
        };
        let copy = face("meeting-copy", copy_label, 30., 6., Tone::Clear, cx)
            .w(px(30.))
            .when(!ready || self.copying, |button| {
                button.tab_index(-1).cursor_default()
            })
            .child(icon(
                match self.copied {
                    Some(true) => Glyph::Check,
                    Some(false) => Glyph::Alert,
                    None => Glyph::Copy,
                },
                15.,
                p.secondary,
            ))
            .when(ready, |button| {
                button.on_click(cx.listener(|s, _, _, cx| s.copy(cx)))
            });
        let body = match self.status {
            MeetingStatus::Ready => {
                let count =
                    self.data.summary.len() + self.data.decisions.len() + self.data.actions.len();
                let age = self.reveal.time(count as f32 * 0.17 + 0.40, w, cx);
                self.notes(age, cx)
            }
            MeetingStatus::Summarizing => {
                let age = self.loading.time(1.5, w, cx);
                stack()
                    .min_h(px(102.))
                    .py(px(15.))
                    .px(px(20.))
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(13.))
                            .text_color(p.secondary)
                            .child(transformed(
                                Glyph::Spinner,
                                14.,
                                p.secondary,
                                age * 340.,
                                1.,
                                (0., 0.),
                                1.,
                            ))
                            .child(tr(cx, "Summarizing transcript…", "正在整理会议记录…")),
                    )
                    .children([0.80, 0.59].into_iter().enumerate().map(|(i, width)| {
                        let x = ((age * 3. - i as f32 * 0.42).cos() + 1.) / 2.;
                        div()
                            .w(relative(width))
                            .h(px(7.))
                            .rounded(px(4.))
                            .bg(p.line.opacity(0.42 + x * 0.50))
                    }))
            }
            MeetingStatus::Error => stack()
                .min_h(px(102.))
                .p(px(20.))
                .justify_center()
                .text_size(px(13.))
                .line_height(px(20.))
                .text_color(p.danger)
                .child(
                    div()
                        .id("meeting-error")
                        .role(Role::Alert)
                        .child(self.error.clone()),
                ),
            MeetingStatus::NoTranscript => stack()
                .min_h(px(102.))
                .p(px(20.))
                .justify_center()
                .text_size(px(13.))
                .text_color(p.secondary)
                .child(tr(
                    cx,
                    "No transcript is available for this meeting.",
                    "这场会议没有转录记录。",
                )),
        };
        let measured = self.measured_body.clone();
        let height = measured.get();
        if self.body_height.value(cx) == 0. {
            self.body_height.reset(f32::from(height));
        } else {
            self.body_height.duration(f32::from(height), 0.24, cx);
        }
        let current = self.body_height.get(w, cx).max(0.);
        div()
            .id("meeting-summary")
            .role(Role::Group)
            .aria_label(self.data.title.clone())
            .w_full()
            .max_w(px(640.))
            .rounded(crate::foundation::surface_radius(cx))
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_color(p.ink)
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                if e.keystroke.key == "escape" && s.quote.take().is_some() {
                    cx.notify();
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .pt(px(18.))
                    .px(px(20.))
                    .pb(px(16.))
                    .border_b_1()
                    .border_color(p.line)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .flex_1()
                            .min_w(px(160.))
                            .child(div().size(px(20.)).flex_none().child(icon(
                                Glyph::Monitor,
                                20.,
                                p.secondary,
                            )))
                            .child(
                                stack()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .text_size(px(15.))
                                            .line_height(px(22.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(self.data.title.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .line_height(px(18.))
                                            .text_color(p.secondary)
                                            .child(self.data.meta.clone()),
                                    ),
                            ),
                    )
                    .child(people),
            )
            .child(
                div()
                    .w_full()
                    .overflow_hidden()
                    .when(height > px(0.), |view| view.h(px(current)))
                    .flex()
                    .flex_col()
                    .on_children_prepainted(move |bounds, w, _| {
                        if let Some(bounds) = bounds.first()
                            && bounds.size.height != measured.get()
                        {
                            measured.set(bounds.size.height);
                            w.request_animation_frame();
                        }
                    })
                    .child(body.flex_none()),
            )
            .child(
                div()
                    .min_h(px(53.))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(4.))
                    .pl(px(20.))
                    .pr(px(12.))
                    .py(px(6.))
                    .border_t_1()
                    .border_color(p.line)
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(6.))
                            .text_size(px(11.))
                            .text_color(p.faint)
                            .child(self.credit.clone())
                            .when_some(self.transcript_words, |row, words| {
                                row.child(format!("· {words} {}", tr(cx, "words", "词")))
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(copy)
                            .when(self.status != MeetingStatus::Summarizing, |row| {
                                row.child(regenerate)
                            }),
                    ),
            )
    }
}
