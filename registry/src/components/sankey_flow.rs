//! Host-provided staged flows rendered as proportional, interactive ribbons.
use crate::primitives::{
    family::{Paint, PhaseClock, Slide, Tone, disabled_motion, face},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Debug)]
pub struct FlowNode {
    pub id: SharedString,
    pub label: SharedString,
    pub column: Option<usize>,
    pub color: Hsla,
}
impl FlowNode {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>, color: Hsla) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            column: None,
            color,
        }
    }
    pub fn with_column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }
}
#[derive(Clone, Debug)]
pub struct FlowLink {
    pub source: SharedString,
    pub target: SharedString,
    pub value: f32,
}
impl FlowLink {
    pub fn new(
        source: impl Into<SharedString>,
        target: impl Into<SharedString>,
        value: f32,
    ) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            value,
        }
    }
}
#[derive(Clone, Debug)]
pub struct FlowInspect {
    pub node: Option<SharedString>,
    pub source: Option<SharedString>,
    pub target: Option<SharedString>,
    pub value: f32,
    pub total_share: f32,
}
#[derive(Clone, Debug)]
pub struct FlowPeriodChange {
    pub index: usize,
}
#[derive(Clone, Copy, Default)]
struct Bar {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}
#[derive(Clone, Copy, Default)]
struct Band {
    source: usize,
    target: usize,
    source_y: f32,
    target_y: f32,
    thickness: f32,
}
#[derive(Clone, Default)]
struct Layout {
    bars: Vec<Bar>,
    bands: Vec<Band>,
    columns: Vec<usize>,
    values: Vec<f64>,
    total: f64,
    vertical: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Focus {
    Node(usize),
    Link(usize),
}
pub struct SankeyFlow {
    title: SharedString,
    description: SharedString,
    unit: SharedString,
    columns: Vec<SharedString>,
    periods: Vec<SharedString>,
    period: usize,
    nodes: Vec<FlowNode>,
    links: Vec<FlowLink>,
    table_rows: Vec<String>,
    layout: Layout,
    old: Layout,
    display: Layout,
    dirty: bool,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    selected: Option<Focus>,
    pointer: Rc<Cell<Option<(f32, f32)>>>,
    tip_x: Slide,
    tip_y: Slide,
    pinned: bool,
    visible: bool,
    particles: bool,
    started: PhaseClock,
    elapsed: f32,
    last_age: f32,
    animation: Slide,
    last_size: (f32, f32),
}
impl EventEmitter<FlowInspect> for SankeyFlow {}
impl EventEmitter<FlowPeriodChange> for SankeyFlow {}
impl SankeyFlow {
    pub fn new(title: impl Into<SharedString>, nodes: Vec<FlowNode>, links: Vec<FlowLink>) -> Self {
        Self {
            title: title.into(),
            description: "".into(),
            unit: "".into(),
            columns: vec![],
            periods: vec![],
            period: 0,
            nodes,
            links,
            table_rows: Vec::new(),
            layout: Layout::default(),
            old: Layout::default(),
            display: Layout::default(),
            dirty: true,
            bounds: Default::default(),
            selected: None,
            pointer: Default::default(),
            tip_x: Slide::new(0.),
            tip_y: Slide::new(0.),
            pinned: false,
            visible: true,
            particles: true,
            started: PhaseClock::default(),
            elapsed: 0.,
            last_age: 0.,
            animation: Slide::new(0.),
            last_size: (0., 0.),
        }
    }
    pub fn with_columns(mut self, columns: Vec<SharedString>) -> Self {
        self.columns = columns;
        self
    }
    pub fn with_description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = text.into();
        self
    }
    pub fn with_periods(mut self, periods: Vec<SharedString>, selected: usize) -> Self {
        self.period = selected.min(periods.len().saturating_sub(1));
        self.periods = periods;
        self
    }
    pub fn set_period(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.periods.len() && self.period != index {
            self.period = index;
            cx.emit(FlowPeriodChange { index });
            cx.notify();
        }
    }
    pub fn with_unit(mut self, unit: impl Into<SharedString>) -> Self {
        self.unit = unit.into();
        self
    }
    pub fn set_data(&mut self, nodes: Vec<FlowNode>, links: Vec<FlowLink>, cx: &mut Context<Self>) {
        let current = transition(
            &self.old,
            &self.layout,
            self.animation.value(cx).clamp(0., 1.),
        );
        let bars = nodes
            .iter()
            .map(|node| {
                self.nodes
                    .iter()
                    .position(|old| old.id == node.id)
                    .and_then(|i| current.bars.get(i).copied())
                    .unwrap_or_default()
            })
            .collect();
        let bands = links
            .iter()
            .map(|link| {
                self.links
                    .iter()
                    .position(|old| old.source == link.source && old.target == link.target)
                    .and_then(|i| current.bands.get(i).copied())
                    .unwrap_or_default()
            })
            .collect();
        self.layout.bars = bars;
        self.layout.bands = bands;
        self.old = self.layout.clone();
        self.animation.reset(1.);
        self.nodes = nodes;
        self.links = links;
        self.selected = None;
        self.pinned = false;
        self.dirty = true;
        cx.notify();
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            self.started.restart(cx);
            self.last_age = 0.;
            if visible {
                cx.notify();
            }
        }
    }
    pub fn set_particles(&mut self, on: bool, cx: &mut Context<Self>) {
        if self.particles != on {
            self.particles = on;
            self.started.restart(cx);
            self.last_age = 0.;
            cx.notify();
        }
    }
    fn set_focus(&mut self, focus: Option<Focus>, cx: &mut Context<Self>) {
        if self.selected == focus {
            return;
        }
        self.selected = focus;
        if let Some(focus) = focus {
            match focus {
                Focus::Node(i) => {
                    if let Some(node) = self.nodes.get(i) {
                        let v = self.layout.values.get(i).copied().unwrap_or(0.);
                        cx.emit(FlowInspect {
                            node: Some(node.id.clone()),
                            source: None,
                            target: None,
                            value: v.min(f64::from(f32::MAX)) as f32,
                            total_share: if self.layout.total > 0. {
                                (v / self.layout.total) as f32
                            } else {
                                0.
                            },
                        });
                    }
                }
                Focus::Link(i) => {
                    if let Some(link) = self.links.get(i) {
                        cx.emit(FlowInspect {
                            node: None,
                            source: Some(link.source.clone()),
                            target: Some(link.target.clone()),
                            value: link.value,
                            total_share: if self.layout.total > 0. {
                                (f64::from(link.value) / self.layout.total) as f32
                            } else {
                                0.
                            },
                        });
                    }
                }
            }
        }
        cx.notify();
    }
    fn build(&self, width: f32, height: f32) -> Layout {
        let n = self.nodes.len();
        let vertical = width < 420.;
        let (major, minor) = if vertical {
            (height, width)
        } else {
            (width, height)
        };
        let mut cols: Vec<usize> = self
            .nodes
            .iter()
            .map(|v| v.column.unwrap_or(0).min(n.saturating_sub(1)))
            .collect();
        let pairs: Vec<Option<(usize, usize)>> = self
            .links
            .iter()
            .map(|l| {
                if !l.value.is_finite() || l.value <= 0. {
                    return None;
                }
                Some((
                    self.nodes.iter().position(|n| n.id == l.source)?,
                    self.nodes.iter().position(|n| n.id == l.target)?,
                ))
            })
            .collect();
        for _ in 0..n {
            let mut changed = false;
            for (a, b) in pairs.iter().flatten() {
                if self.nodes[*b].column.is_none() && cols[*b] <= cols[*a] && cols[*a] + 1 < n {
                    cols[*b] = cols[*a] + 1;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let count = cols.iter().copied().max().unwrap_or(0) + 1;
        let mut incoming = vec![0f64; n];
        let mut outgoing = vec![0f64; n];
        for (i, pair) in pairs.iter().enumerate() {
            if let Some((a, b)) = pair
                && cols[*b] > cols[*a]
            {
                outgoing[*a] += f64::from(self.links[i].value);
                incoming[*b] += f64::from(self.links[i].value);
            }
        }
        let values: Vec<f64> = incoming
            .iter()
            .zip(&outgoing)
            .map(|(a, b)| f64::max(*a, *b))
            .collect();
        let total: f64 = values
            .iter()
            .zip(&cols)
            .filter(|(_, c)| **c == 0)
            .map(|(v, _)| v)
            .sum();
        let mut column_totals = vec![0f64; count];
        let mut column_count = vec![0usize; count];
        for (i, &c) in cols.iter().enumerate() {
            column_totals[c] += values[i];
            column_count[c] += 1;
        }
        let scale = (0..count)
            .filter(|&c| column_totals[c] > 0.)
            .map(|c| {
                f64::from((minor - 8. - column_count[c].saturating_sub(1) as f32 * 14.).max(1.))
                    / column_totals[c]
            })
            .fold(f64::INFINITY, f64::min);
        let scale = if scale.is_finite() { scale } else { 0. };
        let mut offset = vec![4f32; count];
        let mut bars = vec![];
        for i in 0..n {
            let c = cols[i];
            let pos = if count <= 1 {
                (major - 10.) / 2.
            } else {
                c as f32 * (major - 10.).max(1.) / (count - 1) as f32
            };
            let len = (values[i] * scale) as f32;
            bars.push(Bar {
                x: pos,
                y: offset[c],
                w: 10.,
                h: len,
            });
            offset[c] += len + 14.;
        }
        let mut src = bars.iter().map(|v| v.y).collect::<Vec<_>>();
        let mut dst = src.clone();
        let mut bands = vec![Band::default(); self.links.len()];
        for (i, pair) in pairs.iter().enumerate() {
            if let Some((a, b)) = pair
                && cols[*b] > cols[*a]
            {
                let thickness = (f64::from(self.links[i].value) * scale) as f32;
                bands[i] = Band {
                    source: *a,
                    target: *b,
                    source_y: src[*a],
                    target_y: dst[*b],
                    thickness,
                };
                src[*a] += thickness;
                dst[*b] += thickness;
            }
        }
        Layout {
            bars,
            bands,
            columns: cols,
            values,
            total,
            vertical,
        }
    }
}
fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
fn transition(old: &Layout, new: &Layout, t: f32) -> Layout {
    let mut result = new.clone();
    for (i, bar) in result.bars.iter_mut().enumerate() {
        let previous = old.bars.get(i).copied().unwrap_or(Bar {
            x: bar.x,
            y: bar.y + bar.h / 2.,
            w: bar.w,
            h: 0.,
        });
        bar.x = mix(previous.x, bar.x, t);
        bar.y = mix(previous.y, bar.y, t);
        bar.h = mix(previous.h, bar.h, t);
    }
    for (i, band) in result.bands.iter_mut().enumerate() {
        let previous = old.bands.get(i).copied().unwrap_or(Band {
            source: band.source,
            target: band.target,
            source_y: band.source_y + band.thickness / 2.,
            target_y: band.target_y + band.thickness / 2.,
            thickness: 0.,
        });
        band.source_y = mix(previous.source_y, band.source_y, t);
        band.target_y = mix(previous.target_y, band.target_y, t);
        band.thickness = mix(previous.thickness, band.thickness, t);
    }
    result
}
fn connected(layout: &Layout, links: &[FlowLink], focus: Option<Focus>) -> (Vec<bool>, Vec<f32>) {
    let n = layout.bars.len();
    if focus.is_none() {
        return (vec![true; n], vec![1.; layout.bands.len()]);
    }
    let mut incoming = vec![0f64; n];
    let mut outgoing = vec![0f64; n];
    for (i, band) in layout.bands.iter().enumerate() {
        if band.thickness > 0. {
            incoming[band.target] += f64::from(links[i].value);
            outgoing[band.source] += f64::from(links[i].value);
        }
    }
    let mut up = vec![0f64; n];
    let mut down = vec![0f64; n];
    let mut trace = vec![0f32; layout.bands.len()];
    match focus {
        Some(Focus::Node(i)) if i < n => {
            up[i] = incoming[i];
            down[i] = outgoing[i];
        }
        Some(Focus::Link(i)) if i < layout.bands.len() && layout.bands[i].thickness > 0. => {
            let band = layout.bands[i];
            trace[i] = 1.;
            up[band.source] = f64::from(links[i].value);
            down[band.target] = f64::from(links[i].value);
        }
        _ => {}
    }
    let max = layout.columns.iter().copied().max().unwrap_or(0);
    for col in (0..=max).rev() {
        for (i, band) in layout.bands.iter().enumerate() {
            if band.thickness <= 0.
                || layout.columns[band.target] != col
                || incoming[band.target] <= 0.
            {
                continue;
            }
            let share = (up[band.target] / incoming[band.target]).clamp(0., 1.);
            trace[i] = trace[i].max(share as f32);
            up[band.source] += f64::from(links[i].value) * share;
        }
    }
    for col in 0..=max {
        for (i, band) in layout.bands.iter().enumerate() {
            if band.thickness <= 0.
                || layout.columns[band.source] != col
                || outgoing[band.source] <= 0.
            {
                continue;
            }
            let share = (down[band.source] / outgoing[band.source]).clamp(0., 1.);
            trace[i] = trace[i].max(share as f32);
            down[band.target] += f64::from(links[i].value) * share;
        }
    }
    let mut nodes: Vec<_> = up
        .iter()
        .zip(&down)
        .map(|(a, b)| *a > 0. || *b > 0.)
        .collect();
    if let Some(Focus::Node(i)) = focus
        && i < n
    {
        nodes[i] = true;
    }
    (nodes, trace)
}
fn chart_point(major: f32, minor: f32, vertical: bool, b: Bounds<Pixels>) -> Point<Pixels> {
    if vertical {
        point(b.origin.x + px(minor), b.origin.y + px(major))
    } else {
        point(b.origin.x + px(major), b.origin.y + px(minor))
    }
}
fn curve(a: f32, b: f32, t: f32) -> f32 {
    let eased = t * t * (3. - 2. * t);
    mix(a, b, eased)
}
impl SankeyFlow {
    fn hit(&self, position: Point<Pixels>) -> Option<Focus> {
        let b = self.bounds.get();
        let x = f32::from(position.x - b.origin.x);
        let y = f32::from(position.y - b.origin.y);
        let (major, minor) = if self.display.vertical {
            (y, x)
        } else {
            (x, y)
        };
        for (i, bar) in self.display.bars.iter().enumerate() {
            if major >= bar.x - 8.
                && major <= bar.x + bar.w + 8.
                && minor >= bar.y
                && minor <= bar.y + bar.h
            {
                return Some(Focus::Node(i));
            }
        }
        for (i, band) in self.display.bands.iter().enumerate().rev() {
            if band.thickness <= 0. {
                continue;
            }
            let a = self.display.bars[band.source];
            let z = self.display.bars[band.target];
            let start = a.x + a.w;
            let end = z.x;
            if major < start || major > end || end <= start {
                continue;
            }
            let t = (major - start) / (end - start);
            let top = curve(band.source_y, band.target_y, t);
            if minor >= top && minor <= top + band.thickness {
                return Some(Focus::Link(i));
            }
        }
        None
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
impl Render for SankeyFlow {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let bounds = self.bounds.get();
        let on_screen =
            self.visible && w.is_visible() && intersects_viewport(bounds, w.viewport_size());
        let measured_width = f32::from(bounds.size.width);
        let measured_height = f32::from(bounds.size.height);
        let width = if measured_width > 0. {
            measured_width
        } else {
            600.
        };
        let height = if measured_height > 0. {
            measured_height
        } else {
            340.
        };
        if self.dirty || self.last_size != (width, height) {
            self.old = transition(
                &self.old,
                &self.layout,
                self.animation.value(cx).clamp(0., 1.),
            );
            self.layout = self.build(width, height);
            let mut outgoing = vec![0f64; self.nodes.len()];
            for (i, band) in self.layout.bands.iter().enumerate() {
                if band.thickness > 0. {
                    outgoing[band.source] += f64::from(self.links[i].value);
                }
            }
            self.table_rows = self
                .layout
                .bands
                .iter()
                .enumerate()
                .filter_map(|(i, band)| {
                    if band.thickness <= 0. {
                        return None;
                    }
                    let link = &self.links[i];
                    let source = &self.nodes[band.source].label;
                    let target = &self.nodes[band.target].label;
                    let share = if outgoing[band.source] > 0. {
                        f64::from(link.value) / outgoing[band.source] * 100.
                    } else {
                        0.
                    };
                    Some(format!(
                        "{source} → {target} · {:.0}{} · {:.1}%",
                        link.value, self.unit, share
                    ))
                })
                .collect();
            self.last_size = (width, height);
            self.dirty = false;
            self.animation.reset(0.);
            let seconds = if self.old.bars.is_empty() { 0.95 } else { 0.34 };
            self.animation.duration(1., seconds, cx);
        }
        let progress = if on_screen {
            self.animation.get(w, cx).clamp(0., 1.)
        } else {
            1.
        };
        let layout = transition(&self.old, &self.layout, progress);
        self.display = layout.clone();
        let (focus_nodes, focus_bands) = connected(&layout, &self.links, self.selected);
        let moving = on_screen
            && self.particles
            && !disabled_motion(cx)
            && layout.bands.iter().any(|band| band.thickness > 0.);
        if moving {
            if !self.started.age(cx).is_finite() {
                self.started.restart(cx);
            }
            let age = self.started.age(cx);
            self.elapsed += (age - self.last_age).clamp(0., 0.1);
            self.last_age = age;
            w.request_animation_frame();
        } else {
            self.started.restart(cx);
            self.last_age = 0.;
        }
        let phase = self.elapsed;
        let nodes = self.nodes.clone();
        let graph_bounds = self.bounds.clone();
        let paint_layout = layout.clone();
        let selected = self.selected;
        let details = match selected {
            Some(Focus::Node(i)) => self.nodes.get(i).map(|v| {
                format!(
                    "{} · {:.0}{}",
                    v.label,
                    layout.values.get(i).copied().unwrap_or(0.),
                    self.unit
                )
            }),
            Some(Focus::Link(i)) => self.links.get(i).map(|v| {
                let source = self
                    .nodes
                    .iter()
                    .find(|n| n.id == v.source)
                    .map_or(v.source.as_ref(), |n| n.label.as_ref());
                let target = self
                    .nodes
                    .iter()
                    .find(|n| n.id == v.target)
                    .map_or(v.target.as_ref(), |n| n.label.as_ref());
                format!("{} → {} · {:.0}{}", source, target, v.value, self.unit)
            }),
            None => None,
        }
        .unwrap_or_else(|| {
            tr(cx, "Hover or tap a flow to inspect", "悬停或点击查看流向").to_string()
        });
        let chart_height = if width < 420. { 480. } else { 340. };
        if self.selected.is_some()
            && let Some((x, y)) = self.pointer.get()
        {
            let width = f32::from(self.bounds.get().size.width);
            let height = f32::from(self.bounds.get().size.height);
            let mut left = x + 14.;
            if left + 168. > width {
                left = (x - 14. - 168.).max(0.);
            }
            let mut top = y - 14. - 88.;
            if top < 0. {
                top = y + 14.;
            }
            self.tip_x.duration(left.max(0.), 0.26, cx);
            self.tip_y
                .duration(top.clamp(0., (height - 88.).max(0.)), 0.26, cx);
        }
        let tip_x = self.tip_x.get(w, cx);
        let tip_y = self.tip_y.get(w, cx);
        let mut graph = div()
            .id("sankey-flow")
            .role(Role::Figure)
            .tab_index(0)
            .relative()
            .w_full()
            .h(px(chart_height))
            .overflow_hidden()
            .aria_label(format!(
                "{}. {}. {}",
                self.title,
                tr(
                    cx,
                    "Use arrow keys to inspect nodes; escape clears selection",
                    "使用方向键查看节点，Esc 清除选择"
                ),
                details.clone()
            ))
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                let b = s.bounds.get();
                s.pointer.set(Some((
                    f32::from(e.position.x - b.origin.x),
                    f32::from(e.position.y - b.origin.y),
                )));
                if !s.pinned {
                    let value = s.hit(e.position);
                    s.set_focus(value, cx);
                }
            }))
            .on_hover(cx.listener(|s, over: &bool, _, cx| {
                if !over {
                    s.pointer.set(None);
                    if !s.pinned {
                        s.set_focus(None, cx);
                    }
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, _, cx| {
                    let next = s.hit(e.position);
                    if s.pinned && s.selected == next {
                        s.pinned = false;
                        s.set_focus(None, cx);
                    } else {
                        s.pinned = next.is_some();
                        s.set_focus(next, cx);
                    }
                }),
            )
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                if s.nodes.is_empty() {
                    return;
                }
                let index = match s.selected {
                    Some(Focus::Node(i)) => i,
                    _ => 0,
                };
                let col = s.layout.columns.get(index).copied().unwrap_or(0);
                let siblings: Vec<usize> = s
                    .layout
                    .columns
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| (*c == col).then_some(i))
                    .collect();
                let pos = siblings.iter().position(|i| *i == index).unwrap_or(0);
                let vertical = s.layout.vertical;
                let key = e.keystroke.key.as_str();
                let next = match key {
                    "escape" => {
                        s.pinned = false;
                        s.set_focus(None, cx);
                        w.prevent_default();
                        cx.stop_propagation();
                        return;
                    }
                    k if (k == "up" && !vertical) || (k == "left" && vertical) => {
                        siblings.get(pos.saturating_sub(1)).copied()
                    }
                    k if (k == "down" && !vertical) || (k == "right" && vertical) => siblings
                        .get((pos + 1).min(siblings.len().saturating_sub(1)))
                        .copied(),
                    k if (k == "left" && !vertical) || (k == "up" && vertical) => s
                        .layout
                        .columns
                        .iter()
                        .enumerate()
                        .find_map(|(i, c)| (*c == col.saturating_sub(1)).then_some(i)),
                    k if (k == "right" && !vertical) || (k == "down" && vertical) => s
                        .layout
                        .columns
                        .iter()
                        .enumerate()
                        .find_map(|(i, c)| (*c == col + 1).then_some(i)),
                    "home" => Some(0),
                    "end" => s.nodes.len().checked_sub(1),
                    _ => return,
                };
                if let Some(i) = next {
                    s.pinned = true;
                    s.set_focus(Some(Focus::Node(i)), cx);
                }
                w.prevent_default();
                cx.stop_propagation();
            }))
            .child(
                canvas(
                    move |b, w, _| {
                        let previous = graph_bounds.get();
                        let viewport = w.viewport_size();
                        if previous.size != b.size
                            || intersects_viewport(previous, viewport)
                                != intersects_viewport(b, viewport)
                        {
                            w.request_animation_frame();
                        }
                        graph_bounds.set(b);
                    },
                    move |b, _, w, cx| {
                        for (i, band) in paint_layout.bands.iter().enumerate() {
                            if band.thickness <= 0. {
                                continue;
                            }
                            let a = paint_layout.bars[band.source];
                            let z = paint_layout.bars[band.target];
                            let start = a.x + a.w;
                            let end = z.x;
                            let mid = (start + end) / 2.;
                            let at =
                                |major, minor| chart_point(major, minor, paint_layout.vertical, b);
                            let mut path = PathBuilder::fill();
                            path.move_to(at(start, band.source_y));
                            path.cubic_bezier_to(
                                at(mid, band.source_y),
                                at(mid, band.target_y),
                                at(end, band.target_y),
                            );
                            path.line_to(at(end, band.target_y + band.thickness));
                            path.cubic_bezier_to(
                                at(mid, band.target_y + band.thickness),
                                at(mid, band.source_y + band.thickness),
                                at(start, band.source_y + band.thickness),
                            );
                            path.close();
                            let color = nodes.get(band.source).map_or(p.bright, |n| n.color);
                            if let Ok(path) = path.build() {
                                let fill = if selected.is_none() {
                                    p.ink.opacity(0.09)
                                } else if focus_bands[i] > 0. {
                                    color.opacity(0.36)
                                } else {
                                    p.ink.opacity(0.05)
                                };
                                w.paint_path(path, fill);
                            }
                            if moving && focus_bands[i] > 0. {
                                let count = ((band.thickness / 8.).ceil() as usize).clamp(1, 12);
                                for j in 0..count {
                                    let t = (phase * 40. / (end - start).max(1.)
                                        + j as f32 / count as f32
                                        + i as f32 * 0.137)
                                        .fract();
                                    let x = mix(start, end, t);
                                    let y = curve(band.source_y, band.target_y, t)
                                        + band.thickness
                                            * (0.2 + 0.6 * ((j as f32 * 0.618).fract()));
                                    let center = at(x, y);
                                    let mut dot = PathBuilder::fill();
                                    for k in 0..=8 {
                                        let a = k as f32 * std::f32::consts::TAU / 8.;
                                        let q = point(
                                            center.x + px(a.cos() * 1.1),
                                            center.y + px(a.sin() * 1.1),
                                        );
                                        if k == 0 {
                                            dot.move_to(q)
                                        } else {
                                            dot.line_to(q)
                                        }
                                    }
                                    dot.close();
                                    if let Ok(dot) = dot.build() {
                                        w.paint_path(dot, color.opacity(0.8));
                                    }
                                }
                            }
                        }
                        for (i, bar) in paint_layout.bars.iter().enumerate() {
                            if bar.h <= 0. {
                                continue;
                            }
                            let color = nodes.get(i).map_or(p.bright, |n| n.color);
                            let at =
                                |major, minor| chart_point(major, minor, paint_layout.vertical, b);
                            let mut path = PathBuilder::fill();
                            path.move_to(at(bar.x, bar.y));
                            path.line_to(at(bar.x + bar.w, bar.y));
                            path.line_to(at(bar.x + bar.w, bar.y + bar.h));
                            path.line_to(at(bar.x, bar.y + bar.h));
                            path.close();
                            if let Ok(path) = path.build() {
                                let fill = if selected.is_none() {
                                    p.panel.blend(p.ink.opacity(0.72))
                                } else if focus_nodes[i] {
                                    color
                                } else {
                                    p.panel.blend(p.ink.opacity(0.28))
                                };
                                w.paint_path(path, fill);
                            }
                            if let Some(node) = nodes.get(i) {
                                let name: String = node.label.chars().take(16).collect();
                                let name = if node.label.chars().count() > 16 {
                                    format!("{name}…")
                                } else {
                                    name
                                };
                                let value = paint_layout.values[i];
                                let value = if value >= 1_000_000_000. {
                                    format!("{value:.1e}")
                                } else {
                                    format!("{value:.0}")
                                };
                                let short = bar.h < 28.;
                                let name_color = if focus_nodes[i] { p.ink } else { p.secondary };
                                let mut name_run = w.text_style().to_run(name.len());
                                name_run.color = name_color;
                                let name_line = w.text_system().shape_line(
                                    name.clone().into(),
                                    px(12.),
                                    &[name_run],
                                    None,
                                );
                                let value_size = if short { 12. } else { 14. };
                                let mut value_style = w.text_style();
                                value_style.font_weight = FontWeight::MEDIUM;
                                let mut value_run = value_style.to_run(value.len());
                                value_run.color = p.ink;
                                let value_line = w.text_system().shape_line(
                                    value.into(),
                                    px(value_size),
                                    &[value_run],
                                    None,
                                );
                                let pos = if paint_layout.vertical {
                                    at(bar.x + bar.w + 2., bar.y)
                                } else {
                                    at(bar.x + bar.w + 4., bar.y)
                                };
                                let widest = name_line.width().max(value_line.width());
                                let right =
                                    (f32::from(b.size.width) - f32::from(widest) - 4.).max(4.);
                                let x = f32::from(pos.x - b.origin.x).clamp(4., right);
                                let origin = point(b.origin.x + px(x), pos.y);
                                if short {
                                    let _ = name_line.paint(origin, px(14.4), TextAlign::Left, None, w, cx);
                                    let _ = value_line.paint(
                                        point(origin.x + name_line.width() + px(6.), origin.y),
                                        px(14.4),
                                        TextAlign::Left,
                                        None,
                                        w,
                                        cx,
                                    );
                                } else {
                                    let _ = name_line.paint(origin, px(14.4), TextAlign::Left, None, w, cx);
                                    let _ = value_line.paint(
                                        point(origin.x, origin.y + px(15.4)),
                                        px(16.8),
                                        TextAlign::Left,
                                        None,
                                        w,
                                        cx,
                                    );
                                }
                            }
                        }
                    },
                )
                .absolute()
                .inset_0(),
            );
        if self.selected.is_some() {
            graph = graph.child(
                div()
                    .absolute()
                    .left(px(tip_x))
                    .top(px(tip_y))
                    .min_w(px(168.))
                    .max_w(px(240.))
                    .px(px(12.))
                    .py(px(10.))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(p.line)
                    .bg(p.panel)
                    .shadow_lg()
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(20.))
                            .text_color(p.secondary)
                            .child(details.clone()),
                    ),
            );
        }
        let rows = self.table_rows.clone();
        column()
            .relative()
            .w_full()
            .max_w(px(760.))
            .gap(px(20.))
            .p(px(24.))
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
                    .flex_wrap()
                    .child(
                        column()
                            .gap_1()
                            .child(title(self.title.clone(), p))
                            .when(!self.description.is_empty(), |v| {
                                v.child(hint(self.description.clone(), p))
                            }),
                    )
                    .when(!self.periods.is_empty(), |v| {
                        v.child(row().gap_1().children(self.periods.iter().enumerate().map(
                            |(i, label)| {
                                face(
                                    ("flow-period", i),
                                    label.clone(),
                                    30.,
                                    crate::foundation::surface_radius(cx).into(),
                                    if self.period == i {
                                        Tone::Primary
                                    } else {
                                        Tone::Clear
                                    },
                                    cx,
                                )
                                .px(px(10.))
                                .child(label.clone())
                                .aria_toggled((self.period == i).into())
                                .on_click(cx.listener(move |s, _, _, cx| s.set_period(i, cx)))
                                .on_key_down(cx.listener(
                                    move |s, e: &KeyDownEvent, w, cx| {
                                        if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                            && !e.is_held
                                        {
                                            s.set_period(i, cx);
                                            w.prevent_default();
                                            cx.stop_propagation();
                                        }
                                    },
                                ))
                            },
                        )))
                    }),
            )
            .when(width >= 420. && !self.columns.is_empty(), |view| {
                view.child(
                    row()
                        .justify_between()
                        .children(self.columns.iter().map(|label| hint(label.clone(), p))),
                )
            })
            .child(graph)
            .child(hint(details, p))
            .when(!disabled_motion(cx), |view| {
                view.child(
                    row().items_center().child(
                        face(
                            "flow-particles",
                            tr(cx, "Flow", "流动"),
                            34.,
                            crate::foundation::surface_radius(cx).into(),
                            if self.particles {
                                Tone::Primary
                            } else {
                                Tone::Surface
                            },
                            cx,
                        )
                        .px(px(10.))
                        .child(tr(cx, "Flow", "流动"))
                        .on_click(cx.listener(|s, _, _, cx| s.set_particles(!s.particles, cx)))
                        .on_key_down(cx.listener(
                            |s, e: &KeyDownEvent, w, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                    && !e.is_held
                                {
                                    s.set_particles(!s.particles, cx);
                                    w.prevent_default();
                                    cx.stop_propagation();
                                }
                            },
                        )),
                    ),
                )
            })
            .child(
                div()
                    .id("flow-data")
                    .absolute()
                    .size(px(1.))
                    .overflow_hidden()
                    .role(Role::Table)
                    .aria_label(tr(cx, "Flow data", "流向数据"))
                    .children(rows.into_iter().enumerate().map(|(i, value)| {
                        div()
                            .id(("flow-data-row", i))
                            .role(Role::Row)
                            .aria_label(value)
                    })),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{Band, Bar, FlowLink, FlowNode, Focus, Layout, SankeyFlow, connected};
    use gpui::{Bounds, point, px, rgb, size};
    #[test]
    fn trace_propagates_mixed_share_through_both_sources() {
        let links = vec![
            FlowLink::new("a", "c", 60.),
            FlowLink::new("b", "c", 40.),
            FlowLink::new("c", "d", 50.),
            FlowLink::new("c", "e", 50.),
        ];
        let band = |source, target| Band {
            source,
            target,
            source_y: 0.,
            target_y: 0.,
            thickness: 1.,
        };
        let layout = Layout {
            bars: vec![Bar::default(); 5],
            bands: vec![band(0, 2), band(1, 2), band(2, 3), band(2, 4)],
            columns: vec![0, 0, 1, 2, 2],
            values: vec![],
            total: 100.,
            vertical: false,
        };
        let (_, shares) = connected(&layout, &links, Some(Focus::Link(2)));
        assert_eq!(shares, vec![0.5, 0.5, 1., 0.]);
    }
    #[test]
    fn layout_ignores_invalid_missing_and_reverse_links() {
        let color = rgb(0x8899aa).into();
        let nodes = vec![
            FlowNode::new("a", "A", color).with_column(0),
            FlowNode::new("b", "B", color).with_column(1),
        ];
        let links = vec![
            FlowLink::new("a", "b", 6.),
            FlowLink::new("a", "b", f32::NAN),
            FlowLink::new("a", "b", -3.),
            FlowLink::new("missing", "b", 2.),
            FlowLink::new("b", "a", 1.),
        ];
        let layout = SankeyFlow::new("test", nodes, links).build(600., 340.);
        assert_eq!(layout.values, vec![6., 6.]);
        assert_eq!(layout.total, 6.);
        assert!(layout.bands[0].thickness > 0.);
        assert!(layout.bands[1..].iter().all(|band| band.thickness == 0.));
    }
    #[test]
    fn large_links_do_not_overflow_layout() {
        let color = rgb(0x8899aa).into();
        let nodes = vec![
            FlowNode::new("a", "A", color).with_column(0),
            FlowNode::new("b", "B", color).with_column(1),
        ];
        let links = vec![
            FlowLink::new("a", "b", f32::MAX),
            FlowLink::new("a", "b", f32::MAX),
        ];
        let layout = SankeyFlow::new("test", nodes, links).build(600., 340.);
        assert!(layout.total.is_finite());
        assert!(
            layout
                .bands
                .iter()
                .all(|band| band.thickness.is_finite() && band.thickness > 0.)
        );
    }
    #[test]
    fn hit_testing_distinguishes_node_and_ribbon() {
        let color = rgb(0x8899aa).into();
        let nodes = vec![
            FlowNode::new("a", "A", color).with_column(0),
            FlowNode::new("b", "B", color).with_column(1),
        ];
        let mut chart = SankeyFlow::new("test", nodes, vec![FlowLink::new("a", "b", 20.)]);
        chart.display = chart.build(600., 340.);
        chart.bounds.set(Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(600.), px(340.)),
        });
        let bar = chart.display.bars[0];
        assert_eq!(
            chart.hit(point(px(bar.x + 2.), px(bar.y + 2.))),
            Some(Focus::Node(0))
        );
        let end = chart.display.bars[1];
        let band = chart.display.bands[0];
        let mid = (bar.x + bar.w + end.x) / 2.;
        let y = (band.source_y + band.target_y + band.thickness) / 2.;
        assert_eq!(chart.hit(point(px(mid), px(y))), Some(Focus::Link(0)));
        assert!(chart.hit(point(px(0.), px(0.))).is_none());
    }
}
