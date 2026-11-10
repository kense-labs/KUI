//! Host-driven read-only document review with an illustrative collaborator cursor.
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
    flyout::TriggerBounds,
    glyph::{Glyph, icon},
    nav::tr,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

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
        self.blocks = blocks;
        self.selection = false;
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
        self.target = None;
        self.selection = false;
        self.x.reset(16.);
        self.y.reset(0.);
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
        let status = match self.status {
            AgentCursorStatus::Idle => tr(cx, "Ready", "准备就绪"),
            AgentCursorStatus::Thinking if self.selection => tr(cx, "Selecting text", "选择文字"),
            AgentCursorStatus::Thinking => tr(cx, "Reading the document", "正在阅读文档"),
            AgentCursorStatus::Editing => tr(cx, "Editing", "编辑中"),
            AgentCursorStatus::Review => tr(cx, "Review changes", "审核修改"),
            AgentCursorStatus::Accepted => tr(cx, "Accepted", "已接受"),
            AgentCursorStatus::Reverted => tr(cx, "Undone", "已撤销"),
            AgentCursorStatus::Stopped => tr(cx, "Stopped", "已停止"),
            AgentCursorStatus::Empty => tr(cx, "No changes", "没有修改"),
            AgentCursorStatus::Error => tr(cx, "Connection lost", "连接中断"),
        };
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
                .child(
                    div()
                        .size(px(28.))
                        .flex_none()
                        .rounded_full()
                        .bg(p.tray)
                        .border_2()
                        .border_color(p.panel)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icon(Glyph::User, 16., ink)),
                )
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
                    .pt(px(12.))
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
            let changed = self
                .original
                .iter()
                .find(|original| original.id == block.id)
                .is_none_or(|original| original.text != block.text || original.kind != block.kind);
            let selected = active && target && self.selection;
            let label = StyledText::new(block.text.clone());
            let label = if (changed || selected) && !block.text.is_empty() {
                label.with_highlights([(
                    0..if selected {
                        let count = block.text.chars().count();
                        block
                            .text
                            .char_indices()
                            .nth((count as f32 * selection_progress).ceil() as usize)
                            .map_or(block.text.len(), |(index, _)| index)
                    } else {
                        block.text.len()
                    },
                    HighlightStyle {
                        background_color: Some(ink.opacity(if selected { 0.28 } else { 0.16 })),
                        ..Default::default()
                    },
                )])
            } else {
                label
            };
            let layout = label.layout().clone();
            let track_tail = target && active && self.status == AgentCursorStatus::Editing;
            let text_end = block.text.len();
            let line = div()
                .on_children_prepainted(move |children, _, cx| {
                    if target && let Some(b) = children.first() {
                        let origin = doc_bounds.get().origin;
                        let position = if track_tail {
                            layout.position_for_index(text_end).unwrap_or(b.origin) - origin
                        } else {
                            b.origin - origin
                        };
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
        if active && visible {
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
                                let mut path = PathBuilder::fill();
                                path.move_to(o);
                                path.line_to(o + point(px(2.8), px(20.)));
                                path.line_to(o + point(px(8.6), px(14.)));
                                path.line_to(o + point(px(17.), px(11.5)));
                                path.close();
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
                            .bg(p.panel.blend(color.opacity(0.14)))
                            .shadow_sm()
                            .text_color(p.ink)
                            .text_size(px(12.))
                            .line_height(px(24.))
                            .whitespace_nowrap()
                            .child(label),
                    )
            };
            if let Some((name, color)) = &self.collaborator {
                doc = doc.child(marker(width * 0.72, height * 0.66, name.clone(), *color));
            }
            doc = doc.child(marker(x, y, self.agent.clone(), ink));
        }
        if review {
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
                    .bottom(px(16.))
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
                            .child(format!("{count} {}", tr(cx, "changes", "项修改"))),
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
    use super::RequestGate;
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
