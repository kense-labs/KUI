//! Host-driven data-flow illustration with optional decorative motion.
use crate::components::animated_counter::AnimatedCounter;
use crate::primitives::{
    family::{Paint, PhaseClock, Tone, disabled_motion, face},
    glyph::{Glyph, icon},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, f32::consts::TAU, rc::Rc};

type NodeMark = Box<dyn Fn() -> AnyElement>;
#[derive(Clone, Debug)]
pub struct DataFlowNode {
    pub id: SharedString,
    pub label: SharedString,
    pub meta: SharedString,
    pub detail: SharedString,
    pub share: f32,
}
impl DataFlowNode {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            meta: "".into(),
            detail: "".into(),
            share: 1.,
        }
    }
    pub fn with_meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = meta.into();
        self
    }
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = detail.into();
        self
    }
    pub fn with_share(mut self, share: f32) -> Self {
        self.share = if share.is_finite() {
            share.clamp(0., 1.)
        } else {
            0.
        };
        self
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataFlowStatus {
    Running,
    Paused,
    Error,
}
#[derive(Clone, Debug)]
pub struct DataFlowSnapshot {
    pub processed: u64,
    pub status: DataFlowStatus,
    pub message: SharedString,
}
impl DataFlowSnapshot {
    pub fn new(processed: u64, status: DataFlowStatus) -> Self {
        Self {
            processed,
            status,
            message: "".into(),
        }
    }
    pub fn with_message(mut self, message: impl Into<SharedString>) -> Self {
        self.message = message.into();
        self
    }
}
#[derive(Clone, Debug)]
pub struct DataFlowInspect {
    pub id: SharedString,
}
#[derive(Clone, Debug)]
pub struct DataFlowPlaybackChanged {
    pub paused: bool,
}
pub struct DataFlow {
    sources: Vec<DataFlowNode>,
    core: DataFlowNode,
    destinations: Vec<DataFlowNode>,
    snapshot: DataFlowSnapshot,
    icons: HashMap<SharedString, Glyph>,
    marks: HashMap<SharedString, NodeMark>,
    counter: Option<Entity<AnimatedCounter>>,
    selected: Option<usize>,
    hovered: Option<usize>,
    paused: bool,
    visible: bool,
    clock: PhaseClock,
    elapsed: f32,
    last_age: f32,
    bounds: Rc<Cell<Bounds<Pixels>>>,
}
impl EventEmitter<DataFlowInspect> for DataFlow {}
impl EventEmitter<DataFlowPlaybackChanged> for DataFlow {}
impl DataFlow {
    pub fn new(
        sources: Vec<DataFlowNode>,
        core: DataFlowNode,
        destinations: Vec<DataFlowNode>,
        snapshot: DataFlowSnapshot,
    ) -> Self {
        Self {
            sources,
            core,
            destinations,
            snapshot,
            icons: HashMap::new(),
            marks: HashMap::new(),
            counter: None,
            selected: None,
            hovered: None,
            paused: false,
            visible: true,
            clock: PhaseClock::default(),
            elapsed: 0.,
            last_age: 0.,
            bounds: Default::default(),
        }
    }
    /// Supplies an optional icon for a host-defined node ID.
    pub fn with_node_icon(mut self, id: impl Into<SharedString>, glyph: Glyph) -> Self {
        self.icons.insert(id.into(), glyph);
        self
    }
    /// Supplies a decorative mark for a node, taking precedence over its glyph.
    pub fn with_node_mark(
        mut self,
        id: impl Into<SharedString>,
        mark: impl Fn() -> AnyElement + 'static,
    ) -> Self {
        self.marks.insert(id.into(), Box::new(mark));
        self
    }
    pub fn set_snapshot(&mut self, snapshot: DataFlowSnapshot, cx: &mut Context<Self>) {
        if self.snapshot.processed == snapshot.processed
            && self.snapshot.status == snapshot.status
            && self.snapshot.message == snapshot.message
        {
            return;
        }
        let old_value = self.snapshot.processed.min(1_000_000_000_000);
        let new_value = snapshot.processed.min(1_000_000_000_000);
        if old_value != new_value
            && let Some(counter) = &self.counter
        {
            counter.update(cx, |counter, cx| counter.set_value(new_value as f64, cx));
        }
        self.snapshot = snapshot;
        cx.notify();
    }
    pub fn set_nodes(
        &mut self,
        sources: Vec<DataFlowNode>,
        core: DataFlowNode,
        destinations: Vec<DataFlowNode>,
        cx: &mut Context<Self>,
    ) {
        let selected = self
            .selected
            .and_then(|i| self.node(i).map(|n| n.id.clone()));
        self.sources = sources;
        self.core = core;
        self.destinations = destinations;
        self.selected = selected.and_then(|id| {
            (0..self.total()).find(|i| self.node(*i).is_some_and(|node| node.id == id))
        });
        self.hovered = None;
        cx.notify();
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            self.clock.restart(cx);
            self.last_age = 0.;
            if visible {
                cx.notify();
            }
        }
    }
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        if self.paused != paused {
            self.paused = paused;
            self.clock.restart(cx);
            self.last_age = 0.;
            cx.emit(DataFlowPlaybackChanged { paused });
            cx.notify();
        }
    }
    fn total(&self) -> usize {
        self.sources.len() + 1 + self.destinations.len()
    }
    fn node(&self, index: usize) -> Option<&DataFlowNode> {
        if index < self.sources.len() {
            self.sources.get(index)
        } else if index == self.sources.len() {
            Some(&self.core)
        } else {
            self.destinations.get(index - self.sources.len() - 1)
        }
    }
    fn select(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        if self.selected == index {
            return;
        }
        self.selected = index;
        if let Some(node) = index.and_then(|i| self.node(i)) {
            cx.emit(DataFlowInspect {
                id: node.id.clone(),
            });
        }
        cx.notify();
    }
}
fn chip_rect(
    width: f32,
    height: f32,
    index: usize,
    count: usize,
    side: usize,
) -> (f32, f32, f32, f32) {
    let count = count.max(1);
    let narrow = width > 0. && width < 600.;
    if !narrow {
        let chip_w = (width * 0.18).clamp(132., 184.);
        let core = (width * 0.1).clamp(84., 104.);
        let center_y = height / 2. - 10.;
        if side == 1 {
            return ((width - core) / 2., center_y - core / 2., core, core);
        }
        let spread = (height * 0.29).min(112.);
        let offset = if count == 1 {
            0.
        } else {
            (index as f32 / (count - 1) as f32 - 0.5) * 2. * spread
        };
        let y = center_y + offset - 26.;
        let x = if side == 0 { 1. } else { width - 1. - chip_w };
        (x, y, chip_w, 52.)
    } else {
        let core = 84.;
        if side == 1 {
            return ((width - core) / 2., (height - core) / 2., core, core);
        }
        let chip_w = ((width - 2. - 8. * (count - 1) as f32).max(1.)) / count as f32;
        let x = 1. + index as f32 * (chip_w + 8.);
        let y = if side == 0 { 1. } else { height - 1. - 64. };
        (x, y, chip_w, 64.)
    }
}
fn between(a: Point<Pixels>, b: Point<Pixels>, t: f32, vertical: bool) -> Point<Pixels> {
    let k = t * t * (3. - 2. * t);
    if vertical {
        let y = 1.5 * t * (1. - t) + t * t * t;
        point(a.x + (b.x - a.x) * k, a.y + (b.y - a.y) * y)
    } else {
        point(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * k)
    }
}
fn marker(pos: Point<Pixels>, solid: bool, color: Hsla, w: &mut Window) {
    let mut path = if solid {
        PathBuilder::fill()
    } else {
        PathBuilder::stroke(px(1.5))
    };
    let radius = 3.5;
    if solid {
        path.move_to(point(pos.x - px(radius), pos.y - px(radius)));
        path.line_to(point(pos.x + px(radius), pos.y - px(radius)));
        path.line_to(point(pos.x + px(radius), pos.y + px(radius)));
        path.line_to(point(pos.x - px(radius), pos.y + px(radius)));
    } else {
        for i in 0..=16 {
            let a = TAU * i as f32 / 16.;
            let q = point(pos.x + px(a.cos() * radius), pos.y + px(a.sin() * radius));
            if i == 0 {
                path.move_to(q)
            } else {
                path.line_to(q)
            }
        }
    }
    path.close();
    if let Ok(path) = path.build() {
        w.paint_path(path, color);
    }
}
fn intersects_viewport(bounds: Bounds<Pixels>, viewport: Size<Pixels>) -> bool {
    bounds.size.width > px(0.)
        && bounds.size.height > px(0.)
        && bounds.origin.x < viewport.width
        && bounds.origin.y < viewport.height
        && bounds.origin.x + bounds.size.width > px(0.)
        && bounds.origin.y + bounds.size.height > px(0.)
}
impl Render for DataFlow {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let reduced = disabled_motion(cx);
        let playing = self.visible
            && w.is_visible()
            && intersects_viewport(self.bounds.get(), w.viewport_size())
            && !self.paused
            && self.snapshot.status == DataFlowStatus::Running
            && (!self.sources.is_empty() || !self.destinations.is_empty())
            && !reduced;
        if playing {
            if !self.clock.age(cx).is_finite() {
                self.clock.restart(cx);
            }
            let now = self.clock.age(cx);
            self.elapsed += (now - self.last_age).clamp(0., 0.1);
            self.last_age = now;
            w.request_animation_frame();
        } else {
            self.clock.restart(cx);
            self.last_age = 0.;
        }
        let age = self.elapsed;
        let observed = self.hovered.or(self.selected);
        let width = f32::from(self.bounds.get().size.width);
        let narrow = width > 0. && width < 600.;
        let stage_h = if width <= 0. {
            360.
        } else if narrow {
            (width * 0.97).clamp(400., 510.)
        } else {
            (width * 0.4).clamp(300., 400.)
        };
        let source_count = self.sources.len();
        let dest_count = self.destinations.len();
        let dests = self.destinations.clone();
        let bounds = self.bounds.clone();
        let status = match self.snapshot.status {
            DataFlowStatus::Running => tr(cx, "Running", "运行中"),
            DataFlowStatus::Paused => tr(cx, "Paused", "已暂停"),
            DataFlowStatus::Error => tr(cx, "Error", "异常"),
        };
        let counter = self
            .counter
            .get_or_insert_with(|| {
                cx.new(|_| {
                    AnimatedCounter::new(self.snapshot.processed.min(1_000_000_000_000) as f64)
                        .with_size(14.)
                })
            })
            .clone();
        let description = format!(
            "{} → {} → {}. {}: {}. {} {}",
            self.sources
                .iter()
                .map(|v| v.label.as_ref())
                .collect::<Vec<_>>()
                .join(", "),
            self.core.label,
            self.destinations
                .iter()
                .map(|v| v.label.as_ref())
                .collect::<Vec<_>>()
                .join(", "),
            tr(cx, "Status", "状态"),
            status,
            tr(cx, "Processed", "已处理"),
            self.snapshot.processed
        );
        let graph = div()
            .id("data-flow-figure")
            .role(Role::Group)
            .tab_index(0)
            .aria_label(description)
            .relative()
            .w_full()
            .h(px(stage_h))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                let total = s.total();
                if total == 0 {
                    return;
                }
                let i = s.selected.unwrap_or(0);
                let next = match e.keystroke.key.as_str() {
                    "left" | "up" => (i + total - 1) % total,
                    "right" | "down" => (i + 1) % total,
                    "home" => 0,
                    "end" => total - 1,
                    "escape" => {
                        s.select(None, cx);
                        cx.stop_propagation();
                        return;
                    }
                    _ => return,
                };
                s.select(Some(next), cx);
                cx.stop_propagation();
            }))
            .child(
                canvas(
                    move |b, w, _| {
                        let previous = bounds.get();
                        let viewport = w.viewport_size();
                        if previous.size != b.size
                            || intersects_viewport(previous, viewport)
                                != intersects_viewport(b, viewport)
                        {
                            w.request_animation_frame();
                        }
                        bounds.set(b);
                    },
                    move |b, _, w, _| {
                        let width = f32::from(b.size.width);
                        let height = f32::from(b.size.height);
                        let core_box = chip_rect(width, height, 0, 1, 1);
                        let core_mid = point(
                            b.origin.x + px(core_box.0 + core_box.2 / 2.),
                            b.origin.y + px(core_box.1 + core_box.3 / 2.),
                        );
                        for (side, count) in [(0, source_count), (2, dest_count)] {
                            for i in 0..count {
                                let rect = chip_rect(width, height, i, count, side);
                                let fan = if count <= 1 {
                                    0.
                                } else {
                                    (i as f32 / (count - 1) as f32 - 0.5)
                                        * 2.
                                        * 14_f32.min(core_box.2 * 0.16)
                                };
                                let endpoint = if narrow {
                                    point(
                                        b.origin.x + px(rect.0 + rect.2 / 2.),
                                        b.origin.y
                                            + px(if side == 0 { rect.1 + rect.3 } else { rect.1 }),
                                    )
                                } else if side == 0 {
                                    point(
                                        b.origin.x + px(rect.0 + rect.2),
                                        b.origin.y + px(rect.1 + rect.3 / 2.),
                                    )
                                } else {
                                    point(
                                        b.origin.x + px(rect.0),
                                        b.origin.y + px(rect.1 + rect.3 / 2.),
                                    )
                                };
                                let core = if narrow {
                                    point(
                                        core_mid.x + px(fan),
                                        b.origin.y
                                            + px(if side == 0 {
                                                core_box.1
                                            } else {
                                                core_box.1 + core_box.3
                                            }),
                                    )
                                } else {
                                    point(
                                        if side == 0 {
                                            b.origin.x + px(core_box.0)
                                        } else {
                                            b.origin.x + px(core_box.0 + core_box.2)
                                        },
                                        core_mid.y + px(fan),
                                    )
                                };
                                let (a, z) = if side == 0 {
                                    (endpoint, core)
                                } else {
                                    (core, endpoint)
                                };
                                let index = if side == 0 { i } else { source_count + 1 + i };
                                let alpha =
                                    if observed.is_none_or(|x| x == index || x == source_count) {
                                        0.58
                                    } else {
                                        0.16
                                    };
                                let mut path = PathBuilder::stroke(px(1.25));
                                path.move_to(a);
                                for step in 1..=30 {
                                    path.line_to(between(a, z, step as f32 / 30., narrow));
                                }
                                if let Ok(path) = path.build() {
                                    w.paint_path(path, p.line.blend(p.bright.opacity(alpha)));
                                }
                                marker(endpoint, false, p.secondary.opacity(0.42), w);
                                {
                                    for packet in 0..6 {
                                        let phase = if reduced {
                                            packet as f32 / 6.
                                        } else {
                                            (age / 6.6 + packet as f32 / 6.).fract()
                                        };
                                        let serial = (age / 6.6).floor() as usize * 6 + packet;
                                        if side == 0
                                            && phase < 0.5
                                            && source_count > 0
                                            && serial % source_count == i
                                        {
                                            marker(
                                                between(a, z, phase * 2., narrow),
                                                false,
                                                p.bright.opacity(alpha),
                                                w,
                                            );
                                        }
                                        if side == 2 && phase >= 0.5 {
                                            let share =
                                                dests.get(i).map_or(0., |d| d.share.clamp(0., 1.));
                                            if ((serial * 37 + i * 13) % 100) as f32 / 100. < share
                                            {
                                                marker(
                                                    between(a, z, (phase - 0.5) * 2., narrow),
                                                    true,
                                                    p.bright.opacity(alpha),
                                                    w,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                )
                .absolute()
                .inset_0(),
            );
        let mut node_elements = Vec::new();
        let core_side_caption = narrow && width >= 360.;
        for index in 0..self.total() {
            let (side, within, count) = if index < source_count {
                (0, index, source_count)
            } else if index == source_count {
                (1, 0, 1)
            } else {
                (2, index - source_count - 1, dest_count)
            };
            let Some(node) = self.node(index).cloned() else {
                continue;
            };
            let rect = chip_rect(width.max(1.), stage_h, within, count, side);
            let label = node.label.clone();
            let meta = node.meta.clone();
            node_elements.push(
                div()
                    .id(("data-flow-node", index))
                    .role(Role::Button)
                    .tab_index(-1)
                    .aria_label(format!("{} {}", label, node.detail))
                    .when(self.selected == Some(index), |v| v.aria_active_descendant())
                    .absolute()
                    .left(px(rect.0))
                    .top(px(rect.1))
                    .w(px(rect.2))
                    .h(px(rect.3))
                    .rounded(crate::foundation::surface_radius(cx))
                    .bg(p.panel)
                    .border_1()
                    .border_color(if self.selected == Some(index) {
                        p.line.blend(p.bright.opacity(0.55))
                    } else if self.hovered == Some(index) {
                        p.line.blend(p.bright.opacity(0.40))
                    } else {
                        p.line
                    })
                    .flex()
                    .when(narrow || side == 1, |v| {
                        v.flex_col().justify_center().gap(px(6.)).px(px(8.))
                    })
                    .when(!narrow && side != 1, |v| {
                        v.flex_row().items_center().gap(px(12.)).px(px(16.))
                    })
                    .items_center()
                    .when(observed.is_some() && observed != Some(index), |v| {
                        v.opacity(0.5)
                    })
                    .when_some(
                        self.marks.get(&node.id).map(|mark| mark()).or_else(|| {
                            self.icons
                                .get(&node.id)
                                .map(|&glyph| icon(glyph, 16., p.secondary).into_any_element())
                        }),
                        |node, mark| node.child(mark),
                    )
                    .when(side != 1 || !core_side_caption, |v| {
                        v.child(
                            div()
                                .max_w_full()
                                .truncate()
                                .text_size(px(if narrow { 12. } else { 14. }))
                                .line_height(px(if narrow { 16. } else { 20. }))
                                .font_weight(FontWeight::MEDIUM)
                                .child(label),
                        )
                    })
                    .when(!narrow && side != 1 && !meta.is_empty(), |v| {
                        v.child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(20.))
                                .text_color(p.faint)
                                .child(meta),
                        )
                    })
                    .on_hover(cx.listener(move |s, over: &bool, _, cx| {
                        s.hovered = if *over { Some(index) } else { None };
                        cx.notify();
                    }))
                    .on_click(cx.listener(move |s, _, _, cx| {
                        s.select(
                            if s.selected == Some(index) {
                                None
                            } else {
                                Some(index)
                            },
                            cx,
                        )
                    })),
            );
        }
        let details = observed
            .and_then(|i| self.node(i))
            .map(|node| {
                if node.detail.is_empty() {
                    node.label.clone()
                } else {
                    node.detail.clone()
                }
            })
            .unwrap_or_else(|| tr(cx, "Select a node to inspect", "选择节点查看说明").into());
        let core_rect = chip_rect(width.max(1.), stage_h, 0, 1, 1);
        let summary = column()
            .min_w_0()
            .gap_0()
            .when(core_side_caption, |v| {
                v.child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(self.core.label.clone()),
                )
            })
            .child(
                row()
                    .flex_wrap()
                    .gap_1()
                    .items_center()
                    .when(self.snapshot.processed <= 1_000_000_000_000, |v| {
                        v.child(counter)
                    })
                    .when(self.snapshot.processed > 1_000_000_000_000, |v| {
                        v.child(self.snapshot.processed.to_string())
                    })
                    .child(hint(tr(cx, "processed", "条已处理"), p)),
            );
        let graph = graph.children(node_elements);
        let (graph, summary) = if core_side_caption {
            (
                graph.child(
                    summary
                        .absolute()
                        .left(px(core_rect.0 + core_rect.2 + 10.))
                        .right_0()
                        .top(px(core_rect.1 + 16.)),
                ),
                None,
            )
        } else {
            (graph, Some(summary))
        };
        column()
            .w_full()
            .max_w(px(920.))
            .gap_2()
            .p(px(16.))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(
                row()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(title(tr(cx, "Data flow", "数据流"), p))
                    .child(hint(status, p)),
            )
            .child(graph)
            .children(summary)
            .child(
                row()
                    .justify_between()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .text_color(p.secondary)
                            .child(details),
                    )
                    .child(
                        face(
                            "data-flow-pause",
                            if self.paused {
                                tr(cx, "Resume animation", "继续动画")
                            } else {
                                tr(cx, "Pause animation", "暂停动画")
                            },
                            36.,
                            crate::foundation::surface_radius(cx).into(),
                            Tone::Surface,
                            cx,
                        )
                        .px(px(10.))
                        .aria_toggled(self.paused.into())
                        .child(if self.paused {
                            tr(cx, "Resume animation", "继续动画")
                        } else {
                            tr(cx, "Pause animation", "暂停动画")
                        })
                        .on_click(cx.listener(|s, _, _, cx| s.set_paused(!s.paused, cx))),
                    ),
            )
            .when(!self.snapshot.message.is_empty(), |v| {
                v.child(hint(self.snapshot.message.clone(), p))
            })
    }
}
#[cfg(test)]
mod tests {
    use super::{DataFlowNode, DataFlowSnapshot, DataFlowStatus, chip_rect, intersects_viewport};
    use gpui::{Bounds, point, px, size};
    #[test]
    fn destination_shares_are_bounded() {
        assert_eq!(DataFlowNode::new("n", "Node").with_share(-1.).share, 0.);
        assert_eq!(DataFlowNode::new("n", "Node").with_share(2.).share, 1.);
        assert_eq!(
            DataFlowNode::new("n", "Node").with_share(f32::NAN).share,
            0.
        );
    }
    #[test]
    fn layout_rotates_for_narrow_hosts_and_snapshot_keeps_real_count() {
        let source = chip_rect(800., 320., 0, 3, 0);
        let core = chip_rect(800., 320., 0, 1, 1);
        assert!(source.0 + source.2 < core.0);
        let stacked = chip_rect(400., 500., 0, 3, 0);
        let stacked_core = chip_rect(400., 500., 0, 1, 1);
        assert!(stacked.1 + stacked.3 < stacked_core.1);
        assert_eq!(
            DataFlowSnapshot::new(42, DataFlowStatus::Paused).processed,
            42
        );
    }
    #[test]
    fn viewport_intersection_handles_partial_and_fully_hidden_graphs() {
        let viewport = size(px(300.), px(200.));
        let bounds = |x, y, width, height| Bounds {
            origin: point(px(x), px(y)),
            size: size(px(width), px(height)),
        };
        assert!(intersects_viewport(bounds(10., -20., 100., 100.), viewport));
        assert!(!intersects_viewport(
            bounds(10., -100., 100., 100.),
            viewport
        ));
        assert!(!intersects_viewport(bounds(300., 0., 20., 20.), viewport));
        assert!(!intersects_viewport(bounds(0., 0., 0., 20.), viewport));
    }
}
