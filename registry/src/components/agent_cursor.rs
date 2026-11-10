//! Host-driven read-only document review with an illustrative collaborator cursor.
use crate::primitives::{
    family::{Paint, PhaseClock, Slide, Tone, ease, face},
    flyout::TriggerBounds,
    glyph::{Glyph, icon},
    nav::tr,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, ops::Range, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocBlockKind {
    Title,
    Heading,
    Paragraph,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocBlock {
    pub id: SharedString,
    pub kind: DocBlockKind,
    pub text: SharedString,
}
impl DocBlock {
    pub fn new(
        id: impl Into<SharedString>,
        kind: DocBlockKind,
        text: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            text: text.into(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentCursorStatus {
    Idle,
    Thinking,
    Editing,
    Review,
    Accepted,
    Reverted,
    Stopped,
    Empty,
    Error,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentCursorEvent {
    Run { ticket: u64 },
    Stop { ticket: u64 },
    Accept(Vec<DocBlock>),
    Undo(Vec<DocBlock>),
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
    fn is_active(&self, ticket: u64) -> bool {
        self.active == Some(ticket)
    }
    fn cancel(&mut self) -> Option<u64> {
        self.active.take()
    }
    fn settle(&mut self, ticket: u64) -> bool {
        if self.is_active(ticket) {
            self.active = None;
            true
        } else {
            false
        }
    }
}
pub struct AgentCursor {
    label: SharedString,
    agent: SharedString,
    collaborator: Option<(SharedString, Hsla)>,
    selection_motion: Slide,
    meta: Option<SharedString>,
    color: Option<Hsla>,
    blocks: Vec<DocBlock>,
    original: Vec<DocBlock>,
    status: AgentCursorStatus,
    error: SharedString,
    gate: RequestGate,
    target: Option<SharedString>,
    selection: bool,
    selection_range: Option<Range<usize>>,
    correction: bool,
    activity: Option<SharedString>,
    additions: Vec<(Range<usize>, PhaseClock)>,
    review_clock: PhaseClock,
    peer_position: Rc<Cell<Option<Point<Pixels>>>>,
    cursor_ready: bool,
    x: Slide,
    y: Slide,
    visible: bool,
    in_view: bool,
    doc_bounds: TriggerBounds,
    last_target: Rc<Cell<Option<Point<Pixels>>>>,
    doc_size: Rc<Cell<Size<Pixels>>>,
}
impl EventEmitter<AgentCursorEvent> for AgentCursor {}
impl AgentCursor {
    pub fn new(label: impl Into<SharedString>, blocks: Vec<DocBlock>) -> Self {
        Self {
            label: label.into(),
            agent: "Kense".into(),
            collaborator: None,
            selection_motion: Slide::new(0.),
            meta: None,
            color: None,
            original: blocks.clone(),
            blocks,
            status: AgentCursorStatus::Idle,
            error: "".into(),
            gate: RequestGate::default(),
            target: None,
            selection: false,
            selection_range: None,
            correction: false,
            activity: None,
            additions: Vec::new(),
            review_clock: PhaseClock::default(),
            peer_position: Rc::default(),
            cursor_ready: false,
            x: Slide::new(0.),
            y: Slide::new(0.),
            visible: true,
            in_view: true,
            doc_bounds: TriggerBounds::default(),
            last_target: Default::default(),
            doc_size: Default::default(),
        }
    }
    pub fn agent(mut self, name: impl Into<SharedString>, color: Hsla) -> Self {
        self.agent = name.into();
        self.color = Some(color);
        self
    }
    /// Show a second, decorative cursor while the agent is editing the document.
    pub fn with_collaborator(mut self, name: impl Into<SharedString>, color: Hsla) -> Self {
        self.collaborator = Some((name.into(), color));
        self
    }
    /// Adds a muted byline below the first title.
    pub fn with_meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = Some(meta.into());
        self
    }
    pub fn status(&self) -> AgentCursorStatus {
        self.status
    }
    pub fn blocks(&self) -> &[DocBlock] {
        &self.blocks
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
    /// Whether the host and the current viewport both allow playback.
    pub fn is_visible(&self) -> bool {
        self.visible && self.in_view
    }
    /// Replace host data only when no edit is in flight or awaiting review.
    pub fn set_blocks(&mut self, blocks: Vec<DocBlock>, cx: &mut Context<Self>) -> bool {
        if self.gate.active.is_some() || self.status == AgentCursorStatus::Review {
            return false;
        }
        self.original = blocks.clone();
        self.blocks = blocks;
        cx.notify();
        true
    }
    /// Positions are local to the document surface, in logical pixels.
    pub fn set_position(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        self.target = None;
        self.selection = false;
        self.last_target.set(None);
        self.move_to(position, cx);
    }
    fn move_to(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let x = f32::from(position.x).max(0.);
        let y = f32::from(position.y).max(0.);
        if !self.cursor_ready {
            self.x.reset(x);
            self.y.reset(y);
            self.cursor_ready = true;
            cx.notify();
            return;
        }
        let dx = x - self.x.value(cx);
        let dy = y - self.y.value(cx);
        let distance = (dx * dx + dy * dy).sqrt();
        if distance < 1. {
            self.x.reset(x);
            self.y.reset(y);
        } else {
            let seconds = (220. + 0.7 * distance).clamp(280., 600.) / 1000.;
            self.x.duration(x, seconds, cx);
            self.y.duration(y, seconds, cx);
        }
        cx.notify();
    }
    /// Apply a position only while the corresponding run is still active.
    pub fn set_position_for(
        &mut self,
        ticket: u64,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.gate.is_active(ticket) {
            return false;
        }
        self.set_position(position, cx);
        true
    }
    /// The cursor follows this block's measured position as the container changes size.
    pub fn set_target(&mut self, id: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.target = Some(id.into());
        self.selection = false;
        self.selection_range = None;
        self.correction = false;
        self.additions.clear();
        self.last_target.set(None);
        cx.notify();
    }
    pub fn set_target_for(
        &mut self,
        ticket: u64,
        id: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.gate.is_active(ticket) {
            return false;
        }
        self.set_target(id, cx);
        true
    }
    /// Selects the target's text for the active run before a streamed replacement.
    pub fn select_target_for(
        &mut self,
        ticket: u64,
        id: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.set_target_for(ticket, id, cx) {
            return false;
        }
        self.selection = true;
        self.selection_motion.reset(0.);
        self.selection_motion.duration(1., 0.28, cx);
        true
    }
    /// Focus a UTF-8 subrange and underline it before replacing a correction.
    pub fn mark_correction_for(
        &mut self,
        ticket: u64,
        id: impl Into<SharedString>,
        range: Range<usize>,
        cx: &mut Context<Self>,
    ) -> bool {
        let id = id.into();
        if !self.blocks.iter().any(|block| {
            block.id == id && !range.is_empty() && block.text.get(range.clone()).is_some()
        }) || !self.set_target_for(ticket, id, cx)
        {
            return false;
        }
        self.selection_range = Some(range);
        self.correction = true;
        cx.notify();
        true
    }
    pub fn set_activity_for(
        &mut self,
        ticket: u64,
        activity: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.gate.is_active(ticket) {
            return false;
        }
        self.activity = Some(activity.into());
        cx.notify();
        true
    }
    pub fn set_status_for(
        &mut self,
        ticket: u64,
        status: AgentCursorStatus,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.gate.is_active(ticket)
            || !matches!(
                status,
                AgentCursorStatus::Thinking | AgentCursorStatus::Editing
            )
        {
            return false;
        }
        self.status = status;
        cx.notify();
        true
    }
    /// Host streaming updates are ignored after Stop, failure or completion.
    pub fn set_blocks_for(
        &mut self,
        ticket: u64,
        blocks: Vec<DocBlock>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.gate.is_active(ticket) {
            return false;
        }
        if let Some(id) = &self.target
            && let Some(next) = blocks.iter().find(|block| &block.id == id)
        {
            let previous = self
                .blocks
                .iter()
                .find(|block| &block.id == id)
                .map(|block| block.text.as_ref())
                .unwrap_or("");
            let range = changed_text(previous, &next.text);
            if !range.is_empty() {
                if range.start != previous.len() {
                    self.additions.clear();
                }
                let mut clock = PhaseClock::default();
                clock.restart(cx);
                self.additions.retain(|(_, clock)| clock.age(cx) < 0.14);
                self.additions.push((range, clock));
            }
        }
        self.blocks = blocks;
        self.selection = false;
        self.selection_range = None;
        self.correction = false;
        self.status = AgentCursorStatus::Editing;
        cx.notify();
        true
    }
    pub fn run(&mut self, cx: &mut Context<Self>) {
        if self.gate.active.is_some() || self.status == AgentCursorStatus::Review {
            return;
        }
        self.original = self.blocks.clone();
        self.error = "".into();
        self.target = self.blocks.first().map(|block| block.id.clone());
        self.selection = false;
        self.selection_range = None;
        self.correction = false;
        self.activity = None;
        self.additions.clear();
        self.cursor_ready = false;
        self.last_target.set(None);
        self.status = AgentCursorStatus::Thinking;
        let ticket = self.gate.begin();
        cx.emit(AgentCursorEvent::Run { ticket });
        cx.notify();
    }
    pub fn stop(&mut self, cx: &mut Context<Self>) {
        if let Some(ticket) = self.gate.cancel() {
            self.blocks = self.original.clone();
            self.target = None;
            self.status = AgentCursorStatus::Stopped;
            cx.emit(AgentCursorEvent::Stop { ticket });
            cx.notify();
        }
    }
    pub fn finish(&mut self, ticket: u64, blocks: Vec<DocBlock>, cx: &mut Context<Self>) -> bool {
        if !self.gate.settle(ticket) {
            return false;
        }
        self.blocks = blocks;
        self.target = None;
        self.review_clock.restart(cx);
        self.status = if self.blocks == self.original {
            AgentCursorStatus::Empty
        } else {
            AgentCursorStatus::Review
        };
        cx.notify();
        true
    }
    pub fn fail(
        &mut self,
        ticket: u64,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.gate.settle(ticket) {
            return false;
        }
        self.blocks = self.original.clone();
        self.target = None;
        self.error = error.into();
        self.status = AgentCursorStatus::Error;
        cx.notify();
        true
    }
    pub fn accept(&mut self, cx: &mut Context<Self>) {
        if self.status != AgentCursorStatus::Review {
            return;
        }
        self.original = self.blocks.clone();
        self.status = AgentCursorStatus::Accepted;
        cx.emit(AgentCursorEvent::Accept(self.blocks.clone()));
        cx.notify();
    }
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if self.status != AgentCursorStatus::Review {
            return;
        }
        self.blocks = self.original.clone();
        self.status = AgentCursorStatus::Reverted;
        cx.emit(AgentCursorEvent::Undo(self.blocks.clone()));
        cx.notify();
    }
}
fn changed_text(original: &str, edited: &str) -> Range<usize> {
    let prefix = original
        .chars()
        .zip(edited.chars())
        .take_while(|(a, b)| a == b)
        .map(|(ch, _)| ch.len_utf8())
        .sum::<usize>();
    let suffix = original[prefix..]
        .chars()
        .rev()
        .zip(edited[prefix..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(ch, _)| ch.len_utf8())
        .sum::<usize>();
    prefix..edited.len() - suffix
}

fn avatar(label: &str, color: Hsla, p: Paint) -> Div {
    div()
        .size(px(28.))
        .rounded_full()
        .border_2()
        .border_color(p.panel)
        .bg(p.panel.blend(color.opacity(0.26)))
        .text_color(p.ink)
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(11.))
        .font_weight(FontWeight::SEMIBOLD)
        .child(
            label
                .chars()
                .next()
                .map_or_else(String::new, |ch| ch.to_uppercase().collect()),
        )
}

impl Render for AgentCursor {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let ink = self.color.unwrap_or(p.bright);
        let active = self.gate.active.is_some();
        let visible = self.is_visible() && w.is_visible();
        let review = self.status == AgentCursorStatus::Review;
        let inset = if self.doc_size.get().width > px(0.) && self.doc_size.get().width < px(480.) {
            px(16.)
        } else {
            px(32.)
        };
        let selection_progress = if visible && self.selection {
            self.selection_motion.get(w, cx).clamp(0., 1.)
        } else {
            0.
        };
        let x = if visible {
            self.x.get(w, cx)
        } else {
            self.x.value(cx)
        };
        let y = if visible {
            self.y.get(w, cx)
        } else {
            self.y.value(cx)
        };
        let status = self.activity.clone().filter(|_| active).unwrap_or_else(|| {
            match self.status {
                AgentCursorStatus::Idle => tr(cx, "Ready", "准备就绪"),
                AgentCursorStatus::Thinking if self.selection => {
                    tr(cx, "Selecting text", "选择文字")
                }
                AgentCursorStatus::Thinking => tr(cx, "Reading the document", "正在阅读文档"),
                AgentCursorStatus::Editing => tr(cx, "Editing", "编辑中"),
                AgentCursorStatus::Review => tr(cx, "Review changes", "审核修改"),
                AgentCursorStatus::Accepted => tr(cx, "Accepted", "已接受"),
                AgentCursorStatus::Reverted => tr(cx, "Undone", "已撤销"),
                AgentCursorStatus::Stopped => tr(cx, "Stopped", "已停止"),
                AgentCursorStatus::Empty => tr(cx, "No changes", "没有修改"),
                AgentCursorStatus::Error => tr(cx, "Connection lost", "连接中断"),
            }
            .into()
        });
        let viewport = w.viewport_size();
        let weak = cx.weak_entity();
        let mut root = div()
            .on_children_prepainted(move |bounds, _, cx| {
                let visible = bounds.iter().any(|b| {
                    b.left() < viewport.width
                        && b.top() < viewport.height
                        && b.right() > px(0.)
                        && b.bottom() > px(0.)
                });
                let _ = weak.update(cx, |s, cx| {
                    if s.in_view != visible {
                        s.in_view = visible;
                        cx.notify();
                    }
                });
            })
            .id("agent-cursor")
            .role(Role::Group)
            .aria_label(self.label.clone())
            .w_full()
            .max_w(px(680.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .text_color(p.ink)
            .flex()
            .flex_col();
        let action = if active {
            tr(cx, "Stop", "停止")
        } else if self.status == AgentCursorStatus::Error {
            tr(cx, "Retry", "重试")
        } else {
            tr(cx, "Ask agent", "让助手修改")
        };
        let avatars = div()
            .flex()
            .items_center()
            .flex_none()
            .child(avatar(&self.agent, ink, p))
            .when_some(self.collaborator.as_ref(), |row, (name, color)| {
                row.child(avatar(name, *color, p).ml(px(-7.)))
            });
        root = root.child(
            div()
                .h(px(56.))
                .pl(px(16.))
                .pr(px(12.))
                .border_b_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.))
                .child(avatars)
                .child(
                    div()
                        .id("cursor-status")
                        .role(Role::Status)
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .items_center()
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.secondary)
                        .child(status),
                )
                .when(!review, |header| {
                    header.child(
                        face("cursor-run", action, 36., 8., Tone::Surface, cx)
                            .flex_none()
                            .px_2()
                            .gap(px(5.))
                            .when(active, |button| {
                                button.child(div().size(px(7.)).rounded(px(1.5)).bg(p.secondary))
                            })
                            .when(!active, |button| {
                                button.child(icon(Glyph::Play, 12., p.secondary))
                            })
                            .child(action)
                            .on_click(cx.listener(
                                move |s, _, _, cx| if active { s.stop(cx) } else { s.run(cx) },
                            )),
                    )
                }),
        );
        if self.status == AgentCursorStatus::Error {
            root = root.child(
                div()
                    .id("cursor-error")
                    .role(Role::Alert)
                    .px(inset)
                    .py(px(10.))
                    .bg(p.danger.opacity(0.06))
                    .text_color(p.danger)
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .child(self.error.clone()),
            );
        }
        let measured_size = self.doc_size.clone();
        let mut doc = div()
            .relative()
            .w_full()
            .pt(px(24.))
            .px(inset)
            .pb(px(80.))
            .flex()
            .flex_col()
            .child(self.doc_bounds.measure())
            .child(
                canvas(
                    move |bounds, w, _| {
                        if measured_size.get() != bounds.size {
                            measured_size.set(bounds.size);
                            w.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            );
        let peer_index = self
            .blocks
            .iter()
            .position(|block| block.kind == DocBlockKind::Paragraph);
        for (i, block) in self.blocks.iter().enumerate() {
            let target = self.target.as_ref() == Some(&block.id);
            let weak = cx.weak_entity();
            let doc_bounds = self.doc_bounds.clone();
            let last = self.last_target.clone();
            let previous = i.checked_sub(1).and_then(|index| self.blocks.get(index));
            let (size, line, pad) = match block.kind {
                DocBlockKind::Title => (20., 28., 0.),
                DocBlockKind::Heading => (15., 24., 24.),
                DocBlockKind::Paragraph => (
                    15.,
                    24.,
                    match previous.map(|block| block.kind) {
                        Some(DocBlockKind::Heading) => 4.,
                        Some(DocBlockKind::Title) => 16.,
                        _ => 12.,
                    },
                ),
            };
            let previous_text = self
                .original
                .iter()
                .find(|original| original.id == block.id)
                .map_or("", |block| block.text.as_ref());
            let changed = changed_text(previous_text, &block.text);
            let selected = active && target && self.selection;
            let correction = active && target && self.correction;
            let base_range = self.selection_range.clone().unwrap_or(0..block.text.len());
            let selected_to = if selected {
                block
                    .text
                    .get(base_range.clone())
                    .map_or(base_range.start, |text| {
                        base_range.start
                            + text
                                .char_indices()
                                .nth((text.chars().count() as f32 * selection_progress).ceil()
                                    as usize)
                                .map_or(text.len(), |(i, _)| i)
                    })
            } else {
                base_range.end
            };
            let emphasis = if selected || correction {
                base_range.start..selected_to
            } else {
                changed.clone()
            };
            let mut breaks = vec![emphasis.start, emphasis.end];
            let mut additions = Vec::new();
            if active && visible && target && !selected && !correction {
                for (range, clock) in &self.additions {
                    let fade = ease(clock.time(0.14, w, cx) / 0.14);
                    if fade < 1. && range.end <= block.text.len() {
                        breaks.extend([range.start, range.end]);
                        additions.push((range, fade));
                    }
                }
            }
            breaks.sort_unstable();
            breaks.dedup();
            let highlights = breaks.windows(2).filter_map(|segment| {
                let [start, end] = [segment[0], segment[1]];
                let on = emphasis.contains(&start);
                let fade = additions
                    .iter()
                    .find(|(range, _)| range.contains(&start))
                    .map(|(_, fade)| *fade);
                (on || fade.is_some()).then_some((
                    start..end,
                    HighlightStyle {
                        color: fade.map(|fade| p.ink.opacity(fade)),
                        background_color: (on && !correction).then(|| {
                            ink.opacity(if selected {
                                0.28
                            } else {
                                0.16 * fade.unwrap_or(1.)
                            })
                        }),
                        underline: (on && correction).then_some(UnderlineStyle {
                            thickness: px(1.),
                            color: Some(p.danger),
                            wavy: true,
                        }),
                        ..Default::default()
                    },
                ))
            });
            let label = StyledText::new(block.text.clone()).with_highlights(highlights);
            let layout = label.layout().clone();
            let follow = if selected || correction {
                Some(selected_to)
            } else if target && self.status == AgentCursorStatus::Editing {
                Some(changed.end)
            } else {
                None
            };
            let peer_anchor = active && self.collaborator.is_some() && peer_index == Some(i);
            let peer = self.peer_position.clone();
            let text_end = block.text.len();
            let line = div()
                .on_children_prepainted(move |children, w, cx| {
                    let Some(bounds) = children.first() else {
                        return;
                    };
                    let origin = doc_bounds.get().origin;
                    if peer_anchor {
                        let pos =
                            layout.position_for_index(text_end).unwrap_or(bounds.origin) - origin;
                        if peer.get() != Some(pos) {
                            peer.set(Some(pos));
                            w.request_animation_frame();
                        }
                    }
                    if target {
                        let position = follow
                            .and_then(|index| layout.position_for_index(index))
                            .unwrap_or(bounds.origin)
                            - origin;
                        if last.get() != Some(position) {
                            last.set(Some(position));
                            let _ = weak.update(cx, |s, cx| s.move_to(position, cx));
                        }
                    }
                })
                .id((ElementId::from("cursor-block"), block.id.clone()))
                .role(match block.kind {
                    DocBlockKind::Title | DocBlockKind::Heading => Role::Heading,
                    DocBlockKind::Paragraph => Role::Paragraph,
                })
                .when(block.kind == DocBlockKind::Title, |e| e.aria_level(1))
                .when(block.kind == DocBlockKind::Heading, |e| e.aria_level(2))
                .pt(px(pad))
                .text_size(px(size))
                .line_height(px(line))
                .text_color(p.ink)
                .when(block.kind != DocBlockKind::Paragraph, |e| {
                    e.font_weight(FontWeight::SEMIBOLD)
                })
                .child(label);
            doc = doc.child(line);
            if i == 0
                && block.kind == DocBlockKind::Title
                && let Some(meta) = &self.meta
            {
                doc = doc.child(
                    div()
                        .pt(px(4.))
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.secondary)
                        .child(meta.clone()),
                );
            }
        }
        if active && visible && self.cursor_ready {
            let bounds = self.doc_bounds.get();
            let width = f32::from(bounds.size.width);
            let height = f32::from(bounds.size.height);
            let marker = |x: f32, y: f32, label: SharedString, color: Hsla| {
                let x = x.clamp(0., (width - 20.).max(0.));
                let y = y.clamp(0., (height - 44.).max(0.));
                div()
                    .absolute()
                    .left(px(x))
                    .top(px(y))
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |bounds, _, w, _| {
                                let o = bounds.origin;
                                let mut outline = PathBuilder::stroke(px(2.));
                                let mut path = PathBuilder::fill();
                                outline.move_to(o);
                                path.move_to(o);
                                for (x, y) in [(2.8, 18.), (8.6, 12.5), (16., 10.5)] {
                                    let point = o + point(px(x), px(y));
                                    outline.line_to(point);
                                    path.line_to(point);
                                }
                                outline.close();
                                path.close();
                                if let Ok(outline) = outline.build() {
                                    w.paint_path(outline, p.panel);
                                }
                                if let Ok(path) = path.build() {
                                    w.paint_path(path, color);
                                }
                            },
                        )
                        .size(px(20.)),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(px(18.))
                            .when(x + 96. > width, |v| v.right(px(6.)))
                            .when(x + 96. <= width, |v| v.left(px(12.)))
                            .h(px(24.))
                            .max_w(px(88.))
                            .px_2()
                            .rounded(radius)
                            .border_1()
                            .border_color(p.line.blend(color.opacity(0.34)))
                            .bg(p.panel.blend(color.opacity(0.25)))
                            .shadow_sm()
                            .text_color(p.ink)
                            .text_size(px(12.))
                            .line_height(px(24.))
                            .whitespace_nowrap()
                            .child(label),
                    )
            };
            if let Some((name, color)) = &self.collaborator
                && let Some(pos) = self.peer_position.get()
            {
                doc = doc.child(marker(pos.x.into(), pos.y.into(), name.clone(), *color));
            }
            let (x, y) = if self.selection
                && let Some(pos) = self.last_target.get()
            {
                (pos.x.into(), pos.y.into())
            } else {
                (x, y)
            };
            doc = doc.child(marker(x, y, self.agent.clone(), ink));
        }
        if review {
            let progress = if visible {
                ease(self.review_clock.time(0.20, w, cx) / 0.20)
            } else {
                1.
            };
            let count = self
                .blocks
                .iter()
                .filter(|block| !self.original.contains(block))
                .count()
                + self
                    .original
                    .iter()
                    .filter(|original| !self.blocks.iter().any(|block| block.id == original.id))
                    .count();
            doc = doc.child(
                div()
                    .absolute()
                    .left(inset)
                    .right(inset)
                    .bottom(px(16. - 8. * (1. - progress)))
                    .opacity(progress)
                    .rounded(radius)
                    .border_1()
                    .border_color(p.line)
                    .bg(p.panel)
                    .shadow_sm()
                    .p_1()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .px_2()
                            .text_sm()
                            .text_color(p.secondary)
                            .child(format!(
                                "{count} {} · {}",
                                tr(cx, "changes", "项修改"),
                                self.agent
                            )),
                    )
                    .child(
                        face(
                            "cursor-undo",
                            tr(cx, "Undo", "撤销"),
                            32.,
                            8.,
                            Tone::Surface,
                            cx,
                        )
                        .px_2()
                        .child(tr(cx, "Undo", "撤销"))
                        .on_click(cx.listener(|s, _, _, cx| s.undo(cx))),
                    )
                    .child(
                        face(
                            "cursor-accept",
                            tr(cx, "Accept changes", "接受修改"),
                            32.,
                            8.,
                            Tone::Primary,
                            cx,
                        )
                        .px_2()
                        .child(tr(cx, "Accept changes", "接受修改"))
                        .on_click(cx.listener(|s, _, _, cx| s.accept(cx))),
                    ),
            );
        }
        root.child(doc)
    }
}
#[cfg(test)]
mod tests {
    use super::{RequestGate, changed_text};

    #[test]
    fn diff_preserves_shared_utf8_prefix_and_suffix() {
        let edited = "先在工作区开放";
        assert_eq!(&edited[changed_text("先在工区开放", edited)], "作");
        assert!(changed_text(edited, edited).is_empty());
        assert!(changed_text("a!b", "ab").is_empty());
    }

    #[test]
    fn stop_and_retry_exclude_previous_edits() {
        let mut gate = RequestGate::default();
        let first = gate.begin();
        assert_eq!(gate.cancel(), Some(first));
        let second = gate.begin();
        assert!(!gate.is_active(first));
        assert!(!gate.settle(first));
        assert!(gate.settle(second));
        assert!(!gate.settle(second));
    }
}
