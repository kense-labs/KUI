//! Host-provided staged flows rendered as proportional, interactive ribbons.
use crate::{
    components::color_math::oklab_mix,
    primitives::{
        family::{Paint, PhaseClock, Slide, Tone, disabled_motion, face, measure},
        presentation::{column, hint, row, tr},
    },
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
/// A request from the chart's Flow toggle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlowParticlesChange {
    pub enabled: bool,
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
impl Layout {
    fn upright_label_bounds(
        &self,
        index: usize,
        plot_width: f32,
        text_width: f32,
        previous_end: f32,
    ) -> (f32, f32) {
        let bar = self.bars[index];
        // The upright layout borrows 8px from the 14px gap after each bar.
        let right = (bar.y + bar.h + 8.).min(plot_width + 8.);
        let left = if right >= plot_width + 7.99 {
            // Use spare space before the last label when it would hit the card edge.
            bar.y.min((right - text_width).max(previous_end + 6.))
        } else {
            bar.y
        };
        (left, right.max(left))
    }

    fn nodes(&self) -> impl DoubleEndedIterator<Item = (usize, usize)> + '_ {
        self.columns
            .iter()
            .zip(&self.bars)
            .enumerate()
            .filter_map(|(i, (col, bar))| (bar.h > 0.).then_some((i, *col)))
    }
    fn edge_node(&self, last: bool) -> Option<usize> {
        if last {
            self.nodes()
                .max_by_key(|&(i, col)| (col, i))
                .map(|(i, _)| i)
        } else {
            self.nodes()
                .min_by_key(|&(i, col)| (col, i))
                .map(|(i, _)| i)
        }
    }
    fn key_node(&self, index: usize, key: &str) -> Option<usize> {
        let col = *self.columns.get(index)?;
        let cross = if self.vertical {
            ("up", "down")
        } else {
            ("left", "right")
        };
        let within = if self.vertical {
            ("left", "right")
        } else {
            ("up", "down")
        };
        if key == within.0 {
            return self
                .nodes()
                .filter(|&(i, c)| c == col && i < index)
                .map(|(i, _)| i)
                .next_back()
                .or(Some(index));
        }
        if key == within.1 {
            return self
                .nodes()
                .find(|&(i, c)| c == col && i > index)
                .map(|(i, _)| i)
                .or(Some(index));
        }
        if key != cross.0 && key != cross.1 {
            return None;
        }
        let center = |i: usize| self.bars[i].y + self.bars[i].h * 0.5;
        let current = center(index);
        self.nodes()
            .filter(|&(_, c)| if key == cross.0 { c < col } else { c > col })
            .min_by(|&(a, ca), &(b, cb)| {
                ca.abs_diff(col).cmp(&cb.abs_diff(col)).then_with(|| {
                    (center(a) - current)
                        .abs()
                        .total_cmp(&(center(b) - current).abs())
                })
            })
            .map(|(i, _)| i)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Focus {
    Node(usize),
    Link(usize),
}
struct FlowTip {
    title: String,
    value: String,
    note: String,
}
type ValueFormatter = Box<dyn Fn(f64) -> String>;
pub struct SankeyFlow {
    title: SharedString,
    description: SharedString,
    unit: SharedString,
    total_label: Option<SharedString>,
    columns: Vec<SharedString>,
    periods: Vec<SharedString>,
    period_details: Vec<SharedString>,
    period: usize,
    nodes: Vec<FlowNode>,
    links: Vec<FlowLink>,
    table_rows: Vec<String>,
    node_values: Vec<SharedString>,
    formatter: Option<ValueFormatter>,
    empty_label: Option<SharedString>,
    plot_height: f32,
    layout: Layout,
    old: Layout,
    display: Layout,
    dirty: bool,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    selected: Option<Focus>,
    focus: Option<FocusHandle>,
    focus_subscriptions: Vec<Subscription>,
    node_trace: Vec<Slide>,
    trace: Vec<Slide>,
    trace_source: Option<usize>,
    dim: Slide,
    pointer: Rc<Cell<Option<(f32, f32)>>>,
    tip_size: Rc<Cell<Size<Pixels>>>,
    tip_x: Slide,
    tip_y: Slide,
    pinned: bool,
    visible: bool,
    particles: bool,
    particles_controlled: bool,
    particle_toggle: bool,
    started: PhaseClock,
    elapsed: f32,
    last_age: f32,
    animation: Slide,
    last_size: (f32, f32),
}
impl EventEmitter<FlowInspect> for SankeyFlow {}
impl EventEmitter<FlowPeriodChange> for SankeyFlow {}
impl EventEmitter<FlowParticlesChange> for SankeyFlow {}
impl SankeyFlow {
    pub fn new(title: impl Into<SharedString>, nodes: Vec<FlowNode>, links: Vec<FlowLink>) -> Self {
        Self {
            title: title.into(),
            description: "".into(),
            unit: "".into(),
            total_label: None,
            columns: vec![],
            periods: vec![],
            period_details: vec![],
            period: 0,
            nodes,
            links,
            table_rows: Vec::new(),
            node_values: Vec::new(),
            formatter: None,
            empty_label: None,
            plot_height: 340.,
            layout: Layout::default(),
            old: Layout::default(),
            display: Layout::default(),
            dirty: true,
            bounds: Default::default(),
            selected: None,
            focus: None,
            focus_subscriptions: Vec::new(),
            node_trace: Vec::new(),
            trace: Vec::new(),
            trace_source: None,
            dim: Slide::snappy(0.),
            pointer: Default::default(),
            tip_size: Default::default(),
            tip_x: Slide::new(0.),
            tip_y: Slide::new(0.),
            pinned: false,
            visible: true,
            particles: true,
            particles_controlled: false,
            particle_toggle: true,
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
    /// Longer period captions shown in the summary line, such as "Jul to Sep 2026".
    pub fn with_period_details(mut self, details: Vec<SharedString>) -> Self {
        self.period_details = details;
        self
    }
    /// Names the first column's total in the summary line, such as "signups".
    pub fn with_total_label(mut self, label: impl Into<SharedString>) -> Self {
        self.total_label = Some(label.into());
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
    /// Set the plot height in pixels. Upright plots use at least 480px.
    pub fn with_height(mut self, height: f32) -> Self {
        if height.is_finite() && height > 0. {
            self.plot_height = height;
        }
        self
    }
    /// Customize value formatting for node labels, tooltips and the flow table.
    pub fn with_value_formatter(mut self, formatter: impl Fn(f64) -> String + 'static) -> Self {
        self.formatter = Some(Box::new(formatter));
        self
    }
    /// Override the placeholder for data with no drawable links.
    pub fn with_empty_label(mut self, label: impl Into<SharedString>) -> Self {
        self.empty_label = Some(label.into());
        self
    }
    /// Set the initial state of the locally managed particle stream.
    pub fn with_particles(mut self, enabled: bool) -> Self {
        self.particles = enabled;
        self.particles_controlled = false;
        self
    }
    /// Let the host apply toggle requests with `set_particles`.
    pub fn with_controlled_particles(mut self, enabled: bool) -> Self {
        self.particles = enabled;
        self.particles_controlled = true;
        self
    }
    /// Choose whether to show the Flow toggle.
    pub fn with_particle_toggle(mut self, visible: bool) -> Self {
        self.particle_toggle = visible;
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
        self.node_trace.clear();
        self.trace.clear();
        self.trace_source = None;
        self.dim.set(0., cx);
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
    fn toggle_particles(&mut self, cx: &mut Context<Self>) {
        let enabled = !self.particles;
        if !self.particles_controlled {
            self.set_particles(enabled, cx);
        }
        cx.emit(FlowParticlesChange { enabled });
    }
    fn format_value(&self, value: f64) -> String {
        self.formatter
            .as_ref()
            .map_or_else(|| thousands(value), |format| format(value))
    }
    fn tip(&self, focus: Focus, cx: &App) -> Option<FlowTip> {
        let percentage = |value, total| (fraction(value, total) * 100.).round();
        Some(match focus {
            Focus::Node(i) => {
                let value = *self.layout.values.get(i)?;
                let share = percentage(value, self.layout.total);
                let label = self
                    .total_label
                    .as_deref()
                    .unwrap_or_else(|| tr(cx, "the total", "总量"));
                let note = if tr(cx, "en", "zh") == "zh" {
                    format!("占{label}的 {share}%")
                } else {
                    format!("{share}% of {label}")
                };
                FlowTip {
                    title: self.nodes.get(i)?.label.to_string(),
                    value: format!("{}{}", self.format_value(value), self.unit),
                    note,
                }
            }
            Focus::Link(i) => {
                let band = self.layout.bands.get(i)?;
                let value = f64::from(self.links.get(i)?.value);
                let source = &self.nodes.get(band.source)?.label;
                let target = &self.nodes.get(band.target)?.label;
                let from_share = percentage(value, self.layout.values[band.source]);
                let to_share = percentage(value, self.layout.values[band.target]);
                let (title, note) = if tr(cx, "en", "zh") == "zh" {
                    (
                        format!("{source} → {target}"),
                        format!("占{source}的 {from_share}%，占{target}的 {to_share}%"),
                    )
                } else {
                    (
                        format!("{source} to {target}"),
                        format!("{from_share}% of {source}, {to_share}% of {target}"),
                    )
                };
                FlowTip {
                    title,
                    value: format!("{}{}", self.format_value(value), self.unit),
                    note,
                }
            }
        })
    }
    fn set_focus(&mut self, focus: Option<Focus>, cx: &mut Context<Self>) {
        if self.selected == focus {
            return;
        }
        self.selected = focus;
        self.tip_size.set(Size::default());
        self.retarget_trace(cx);
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
    fn retarget_trace(&mut self, cx: &App) {
        let (nodes, shares) = connected(&self.layout, &self.links, self.selected);
        if let Some(focus) = self.selected {
            let source = match focus {
                Focus::Node(i) => Some(i),
                Focus::Link(i) => self.layout.bands.get(i).map(|band| band.source),
            };
            self.trace_source = source.filter(|&i| self.layout.columns.get(i) == Some(&0));
        }
        self.node_trace.resize(nodes.len(), Slide::snappy(0.));
        for (spring, share) in self.node_trace.iter_mut().zip(nodes) {
            spring.set(share, cx);
        }
        self.trace.resize(shares.len(), Slide::snappy(0.));
        for (spring, share) in self.trace.iter_mut().zip(shares) {
            spring.set(share, cx);
        }
        self.dim.set(f32::from(self.selected.is_some()), cx);
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
                f64::from((minor - 4. - column_count[c].saturating_sub(1) as f32 * 14.).max(1.))
                    / column_totals[c]
            })
            .fold(f64::INFINITY, f64::min);
        let scale = if scale.is_finite() { scale } else { 0. };
        // Columns align to the bottom edge; shorter columns leave the slack on top.
        let mut offset: Vec<f32> = (0..count)
            .map(|c| {
                let content = column_totals[c] as f32 * scale as f32
                    + 14. * column_count[c].saturating_sub(1) as f32;
                (minor - content).max(0.)
            })
            .collect();
        let mut bars = vec![];
        for i in 0..n {
            let c = cols[i];
            // The last column's labels sit to its right; reserve that strip so
            // the reference stage headers can align with each column's origin.
            let usable = if vertical { major - 50. } else { major - 110. };
            let pos = if count <= 1 {
                (major - 10.) / 2.
            } else {
                c as f32 * usable.max(1.) / (count - 1) as f32
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
/// Group thousands the way the reference node values read, as in "5,200".
fn thousands(value: f64) -> String {
    let rounded = format!("{value:.0}");
    let (sign, digits) = if let Some(rest) = rounded.strip_prefix('-') {
        ("-", rest)
    } else {
        ("", rounded.as_str())
    };
    let mut grouped = String::from(sign);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    grouped
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
fn connected(layout: &Layout, links: &[FlowLink], focus: Option<Focus>) -> (Vec<f32>, Vec<f32>) {
    let n = layout.bars.len();
    if focus.is_none() {
        return (vec![0.; n], vec![0.; layout.bands.len()]);
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
    let nodes = (0..n)
        .map(|i| fraction(up[i].max(down[i]), incoming[i].max(outgoing[i])))
        .collect();
    (nodes, trace)
}
fn fraction(value: f64, total: f64) -> f32 {
    if total > 0. {
        (value / total).clamp(0., 1.) as f32
    } else {
        0.
    }
}
fn chart_point(major: f32, minor: f32, vertical: bool, b: Bounds<Pixels>) -> Point<Pixels> {
    if vertical {
        point(b.origin.x + px(minor), b.origin.y + px(major))
    } else {
        point(b.origin.x + px(major), b.origin.y + px(minor))
    }
}
fn curve(a: f32, b: f32, major_fraction: f32) -> f32 {
    // Both control points are at the major-axis midpoint. Solve for the
    // parameter at the pointer's major coordinate before evaluating the edge.
    let mut t = major_fraction;
    for _ in 0..3 {
        let u = t - 0.5;
        let x = 0.5 + u * (0.75 + u * u);
        t -= (x - major_fraction) / (0.75 + 3. * u * u);
    }
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
        let focus = self
            .focus
            .get_or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        if self.focus_subscriptions.is_empty() {
            self.focus_subscriptions
                .push(cx.on_focus(&focus, w, |s, _, cx| {
                    if s.selected.is_none() && s.pointer.get().is_none() {
                        s.set_focus(s.layout.edge_node(false).map(Focus::Node), cx);
                    }
                }));
            self.focus_subscriptions
                .push(cx.on_blur(&focus, w, |s, _, cx| {
                    if s.pointer.get().is_none() && !s.pinned {
                        s.set_focus(None, cx);
                    }
                }));
        }
        let bounds = self.bounds.get();
        let viewport = w.viewport_size();
        let on_screen = self.visible && w.is_visible() && intersects_viewport(bounds, viewport);
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
            self.plot_height
        };
        if self.dirty || self.last_size != (width, height) {
            self.old = transition(
                &self.old,
                &self.layout,
                self.animation.value(cx).clamp(0., 1.),
            );
            self.layout = self.build(width, height);
            self.node_values = self
                .layout
                .values
                .iter()
                .map(|&value| {
                    if self.formatter.is_none() && value >= 1_000_000_000. {
                        format!("{value:.1e}").into()
                    } else {
                        self.format_value(value).into()
                    }
                })
                .collect();
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
                    let share =
                        fraction(f64::from(link.value), self.layout.values[band.source]) * 100.;
                    Some(format!(
                        "{source} → {target} · {}{} · {:.1}%",
                        self.format_value(f64::from(link.value)),
                        self.unit,
                        share
                    ))
                })
                .collect();
            self.last_size = (width, height);
            self.dirty = false;
            self.animation.reset(0.);
            // First reveal grows in (spring.smooth); data swaps morph (spring.morph).
            let seconds = if self.old.bars.is_empty() { 0.4 } else { 0.42 };
            self.animation.duration(1., seconds, cx);
            self.retarget_trace(cx);
        }
        let progress = if on_screen {
            self.animation.get(w, cx).clamp(0., 1.)
        } else {
            1.
        };
        let layout = transition(&self.old, &self.layout, progress);
        self.display = layout.clone();
        let node_values = self.node_values.clone();
        let now = cx.background_executor().now();
        let reduced = disabled_motion(cx);
        let (dim, dim_active) = self.dim.sample_batch(now, reduced);
        let dim = dim.clamp(0., 1.);
        let mut trace_active = dim_active;
        let mut sample_trace = |springs: &[Slide]| {
            springs
                .iter()
                .map(|spring| {
                    let (share, active) = spring.sample_batch(now, reduced);
                    trace_active |= active;
                    share.clamp(0., 1.)
                })
                .collect::<Vec<_>>()
        };
        let trace = sample_trace(&self.trace);
        let node_trace = sample_trace(&self.node_trace);
        let trace_color = self
            .trace_source
            .and_then(|i| self.nodes.get(i))
            .map_or_else(|| oklab_mix(p.panel, p.ink, 0.785), |n| n.color);
        if on_screen && trace_active {
            w.request_animation_frame();
        }
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
        let tip = selected.and_then(|focus| self.tip(focus, cx));
        let details = tip.as_ref().map_or_else(
            || tr(cx, "Hover or tap a flow to inspect", "悬停或点击查看流向").to_string(),
            |tip| format!("{}. {}. {}", tip.title, tip.value, tip.note),
        );
        let chart_height = if width < 420. {
            self.plot_height.max(480.)
        } else {
            self.plot_height
        };
        let measured_tip = self.tip_size.get();
        let mut tip_width = f32::from(measured_tip.width).min(width);
        let tip_height = if measured_tip.height > px(0.) {
            f32::from(measured_tip.height)
        } else {
            82.4
        };
        if let Some(tip) = &tip {
            if tip_width <= 0. {
                let estimate =
                    |text: &str, size| measure(&SharedString::from(text.to_owned()), size, w);
                tip_width = (estimate(&tip.title, 12.)
                    .max(estimate(&tip.value, 16.))
                    .max(estimate(&tip.note, 12.))
                    + 26.)
                    .min(width);
            }
            let anchor = self.pointer.get().or_else(|| {
                let Focus::Node(i) = selected? else {
                    return None;
                };
                let bar = layout.bars.get(i)?;
                let major = bar.x + bar.w * 0.5;
                let minor = bar.y + bar.h * 0.5;
                Some(if layout.vertical {
                    (minor, major)
                } else {
                    (major, minor)
                })
            });
            if let Some((x, y)) = anchor {
                let left = if x + 14. + tip_width > width {
                    x - 14. - tip_width
                } else {
                    x + 14.
                };
                self.tip_x
                    .set(left.clamp(0., (width - tip_width).max(0.)), cx);
                self.tip_y.set(
                    (y - 14. - tip_height).clamp(0., (height - tip_height).max(0.)),
                    cx,
                );
            }
        }
        let tip_x = self.tip_x.get(w, cx).clamp(0., (width - tip_width).max(0.));
        let tip_y = self
            .tip_y
            .get(w, cx)
            .clamp(0., (height - tip_height).max(0.));
        let mut graph = div()
            .id("sankey-flow")
            .role(Role::Figure)
            .track_focus(&focus)
            .tab_index(0)
            .relative()
            .w_full()
            .h(px(chart_height))
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
                if !s.pinned {
                    let b = s.bounds.get();
                    let previous = s.pointer.replace(Some((
                        f32::from(e.position.x - b.origin.x),
                        f32::from(e.position.y - b.origin.y),
                    )));
                    let value = s.hit(e.position);
                    if value == s.selected && value.is_some() && previous != s.pointer.get() {
                        cx.notify();
                    }
                    s.set_focus(value, cx);
                }
            }))
            .on_hover(cx.listener(|s, over: &bool, _, cx| {
                if !over && !s.pinned && s.pointer.take().is_some() {
                    s.set_focus(None, cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, w, cx| {
                    let next = s.hit(e.position);
                    let b = s.bounds.get();
                    s.pointer.set(Some((
                        f32::from(e.position.x - b.origin.x),
                        f32::from(e.position.y - b.origin.y),
                    )));
                    let touch = w.last_input_was_touch();
                    if touch && s.pinned && s.selected == next {
                        s.pinned = false;
                        s.set_focus(None, cx);
                    } else {
                        s.pinned = touch && next.is_some();
                        s.set_focus(next, cx);
                    }
                }),
            )
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                let key = e.keystroke.key.as_str();
                let next = match key {
                    "escape" => None,
                    "home" => s.layout.edge_node(false),
                    "end" => s.layout.edge_node(true),
                    "left" | "right" | "up" | "down" => {
                        let Some(i) = s.layout.edge_node(false) else {
                            return;
                        };
                        match s.selected {
                            Some(Focus::Node(index)) => {
                                s.layout.key_node(index, key).or(Some(index))
                            }
                            _ => Some(i),
                        }
                    }
                    _ => return,
                };
                let moved = s.pointer.take().is_some();
                s.pinned = false;
                s.set_focus(next.map(Focus::Node), cx);
                if moved {
                    cx.notify();
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
                            let ribbon = |thickness: f32| {
                                let mut path = PathBuilder::fill();
                                path.move_to(at(start, band.source_y));
                                path.cubic_bezier_to(
                                    at(end, band.target_y),
                                    at(mid, band.source_y),
                                    at(mid, band.target_y),
                                );
                                path.line_to(at(end, band.target_y + thickness));
                                path.cubic_bezier_to(
                                    at(start, band.source_y + thickness),
                                    at(mid, band.target_y + thickness),
                                    at(mid, band.source_y + thickness),
                                );
                                path.close();
                                path.build()
                            };
                            let color = nodes.get(band.source).map_or(p.bright, |n| n.color);
                            if let Ok(path) = ribbon(band.thickness) {
                                let fill = if paint_layout.columns[band.source] == 0 {
                                    color.opacity(mix(0.22, 0.07, dim))
                                } else {
                                    p.ink.opacity(mix(0.09, 0.05, dim))
                                };
                                w.paint_path(path, fill);
                            }
                            let traced_width = band.thickness * trace[i];
                            if traced_width > 0.001
                                && let Ok(path) = ribbon(traced_width)
                            {
                                w.paint_path(path, trace_color.opacity(0.34));
                            }
                            // With nothing selected the stream runs on every ribbon;
                            // a selection focuses the stream onto its connected bands.
                            if moving && (selected.is_none() || traced_width > 0.001) {
                                let count = ((band.thickness / 8.).ceil() as usize).clamp(1, 34);
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
                                        let particle_color = if selected.is_some() {
                                            trace_color
                                        } else {
                                            color
                                        };
                                        w.paint_path(dot, particle_color.opacity(0.8));
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
                            let source_column = paint_layout.columns[i] == 0;
                            let paint_bar = |height: f32, fill_color, w: &mut Window| {
                                let bounds = Bounds::from_corners(
                                    at(bar.x, bar.y),
                                    at(bar.x + bar.w, bar.y + height),
                                );
                                let radius = 2.5f32.min(bar.w * 0.5).min(height * 0.5);
                                w.paint_quad(fill(bounds, fill_color).corner_radii(px(radius)));
                            };
                            let base = if source_column {
                                oklab_mix(p.panel, color, mix(1., 0.4, dim))
                            } else {
                                oklab_mix(p.panel, p.ink, mix(0.72, 0.28, dim))
                            };
                            paint_bar(bar.h, base, w);
                            let traced_height = bar.h * node_trace[i];
                            if traced_height > 0.001 {
                                paint_bar(traced_height, trace_color, w);
                            }
                            if !paint_layout.vertical
                                && let Some(node) = nodes.get(i)
                            {
                                let name = node.label.clone();
                                let value = node_values[i].clone();
                                let short = bar.h < 28.;
                                let name_color = p.secondary;
                                let mut name_run = w.text_style().to_run(name.len());
                                name_run.color = name_color;
                                let name_line =
                                    w.text_system().shape_line(name, px(12.), &[name_run], None);
                                let value_size = if short { 12. } else { 14. };
                                let mut value_style = w.text_style();
                                value_style.font_weight = FontWeight::MEDIUM;
                                let mut value_run = value_style.to_run(value.len());
                                value_run.color = p.ink;
                                let value_line = w.text_system().shape_line(
                                    value,
                                    px(value_size),
                                    &[value_run],
                                    None,
                                );
                                let half = if short { 7.2 } else { 16.1 };
                                let pos =
                                    at(bar.x + bar.w + 7., (bar.y + bar.h * 0.5 - half).max(0.));
                                let widest = if short {
                                    name_line.width() + px(6.) + value_line.width()
                                } else {
                                    name_line.width().max(value_line.width())
                                };
                                let x = pos
                                    .x
                                    .min((b.right() - widest - px(4.)).max(b.left() + px(4.)));
                                let origin = point(x, pos.y);
                                if short {
                                    let _ = name_line.paint(
                                        origin,
                                        px(14.4),
                                        TextAlign::Left,
                                        None,
                                        w,
                                        cx,
                                    );
                                    let _ = value_line.paint(
                                        point(origin.x + name_line.width() + px(6.), origin.y),
                                        px(14.4),
                                        TextAlign::Left,
                                        None,
                                        w,
                                        cx,
                                    );
                                } else {
                                    let _ = name_line.paint(
                                        origin,
                                        px(14.4),
                                        TextAlign::Left,
                                        None,
                                        w,
                                        cx,
                                    );
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
            )
            .when(layout.vertical, |graph| {
                let mut label_ends = vec![0.; layout.bars.len()];
                graph.children(layout.nodes().map(|(i, stage)| {
                    let bar = layout.bars[i];
                    let name = &self.nodes[i].label;
                    let value = &self.node_values[i];
                    let text_width = measure(name, 12., w).max(measure(value, 14., w)).ceil() + 1.;
                    let (left, right) =
                        layout.upright_label_bounds(i, width, text_width, label_ends[stage]);
                    label_ends[stage] = right.min(left + text_width);
                    column()
                        .absolute()
                        .left(px(left))
                        .top(px(bar.x + bar.w + 6.))
                        .w(px(right - left))
                        .gap(px(1.))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(12.))
                                .line_height(px(14.4))
                                .text_color(p.secondary)
                                .child(name.clone()),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(px(14.))
                                .line_height(px(16.8))
                                .font_weight(FontWeight::MEDIUM)
                                .child(value.clone()),
                        )
                }))
            });
        if let Some(tip) = tip {
            let tip_size = self.tip_size.clone();
            graph = graph.child(
                div()
                    .id("flow-tooltip")
                    .debug_selector(|| "flow-tooltip".into())
                    .absolute()
                    .left(px(tip_x))
                    .top(px(tip_y))
                    .max_w(px(width))
                    .px(px(12.))
                    .py(px(10.))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(p.line)
                    .bg(p.stage)
                    .shadow_lg()
                    .child(
                        canvas(
                            move |bounds, w, _| {
                                let measured =
                                    size(bounds.size.width + px(2.), bounds.size.height + px(2.));
                                if tip_size.replace(measured) != measured {
                                    w.request_animation_frame();
                                }
                            },
                            |_, _, _, _| (),
                        )
                        .absolute()
                        .inset_0(),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(px(12.))
                            .line_height(px(16.8))
                            .text_color(p.secondary)
                            .child(tip.title),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(px(16.))
                            .line_height(px(20.8))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tip.value),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(px(12.))
                            .line_height(px(16.8))
                            .text_color(p.faint)
                            .child(tip.note),
                    ),
            );
        }
        if layout.total <= 0. {
            let label = self
                .empty_label
                .clone()
                .unwrap_or_else(|| tr(cx, "No flows yet", "暂无流向").into());
            graph = graph.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(hint(label, p)),
            );
        }
        let rows = self.table_rows.clone();
        // Summary line under the title: "11,200 signups, Jul to Sep 2026".
        let total = thousands(self.display.total);
        let total_label = self
            .total_label
            .clone()
            .unwrap_or_else(|| tr(cx, "the total", "总量").to_string().into());
        let meta = match self.period_details.get(self.period) {
            Some(detail) => format!("{total} {total_label}, {detail}"),
            None => format!("{total} {total_label}"),
        };
        let column_x = |c: usize| {
            if self.columns.len() <= 1 {
                0.
            } else {
                c as f32 * (width - 110.).max(1.) / (self.columns.len() - 1) as f32
            }
        };
        column()
            .relative()
            .w_full()
            .max_w(px(706.))
            .gap(px(20.))
            .p(px(24.))
            .when(viewport.width < px(420.), |view| view.px(px(16.)))
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
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .line_height(px(17.6))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(self.title.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .line_height(px(19.6))
                                    .text_color(p.secondary)
                                    .child(meta),
                            )
                            .when(!self.description.is_empty(), |v| {
                                v.child(hint(self.description.clone(), p))
                            }),
                    )
                    .when(!self.periods.is_empty(), |v| {
                        v.child(
                            row()
                                .p(px(3.))
                                .gap(px(2.))
                                .bg(p.tray)
                                .rounded(crate::foundation::surface_radius(cx))
                                .children(self.periods.iter().enumerate().map(|(i, label)| {
                                    face(
                                        ("flow-period", i),
                                        label.clone(),
                                        36.,
                                        crate::foundation::surface_radius(cx).into(),
                                        Tone::Clear,
                                        cx,
                                    )
                                    .when(self.period == i, |v| v.bg(p.panel))
                                    .px(px(12.))
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if self.period == i { p.ink } else { p.secondary })
                                    .hover(|s| s.text_color(p.ink))
                                    .child(label.clone())
                                    .aria_toggled((self.period == i).into())
                                    .on_click(cx.listener(move |s, _, _, cx| s.set_period(i, cx)))
                                    .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                                        if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                            && !e.is_held
                                        {
                                            s.set_period(i, cx);
                                            w.prevent_default();
                                            cx.stop_propagation();
                                        }
                                    }))
                                })),
                        )
                    }),
            )
            .when(width >= 420. && !self.columns.is_empty(), |view| {
                view.child(div().relative().h(px(20.)).mt(px(2.)).children(
                    self.columns.iter().enumerate().map(|(c, label)| {
                        div()
                            .absolute()
                            .top_0()
                            .left(px(column_x(c)))
                            .text_size(px(12.))
                            .line_height(px(20.))
                            .text_color(p.faint)
                            .child(label.clone())
                    }),
                ))
            })
            .child(graph.mt(px(-8.)))
            .when(
                self.particle_toggle && !reduced && layout.total > 0.,
                |view| {
                    // Capsule toggle with a three-bar flow glyph, tucked bottom-right.
                    let dot = if self.particles { p.ink } else { p.secondary };
                    view.child(
                        row().justify_end().mt(px(-8.)).child(
                            face(
                                "flow-particles",
                                tr(cx, "Flow", "流动"),
                                32.,
                                999.,
                                Tone::Clear,
                                cx,
                            )
                            .bg(p.panel)
                            .rounded_full()
                            .pl(px(10.))
                            .pr(px(12.))
                            .gap(px(8.))
                            .child(
                                row()
                                    .items_center()
                                    .gap(px(1.))
                                    .when(self.particles, |r| r.opacity(1.))
                                    .children((0..3).map(|_| {
                                        div().w(px(4.)).h(px(3.)).rounded(px(1.)).bg(dot)
                                    })),
                            )
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .line_height(px(14.))
                                    .text_color(p.ink)
                                    .child(tr(cx, "Flow", "流动")),
                            )
                            .aria_toggled(self.particles.into())
                            .on_click(cx.listener(|s, _, _, cx| s.toggle_particles(cx)))
                            .on_key_down(cx.listener(
                                |s, e: &KeyDownEvent, w, cx| {
                                    if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                        && !e.is_held
                                    {
                                        s.toggle_particles(cx);
                                        w.prevent_default();
                                        cx.stop_propagation();
                                    }
                                },
                            )),
                        ),
                    )
                },
            )
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
    use super::{
        Band, Bar, FlowLink, FlowNode, Focus, Layout, SankeyFlow, connected, curve, transition,
    };
    use gpui::{AppContext, Bounds, TestAppContext, point, px, rgb, size};
    use std::time::Duration;
    #[test]
    fn ribbon_sampling_matches_its_bezier_at_a_quarter_of_the_path() {
        // At t=0.25 the cubic's normalized major and minor coordinates differ.
        assert!((curve(0., 100., 0.296_875) - 15.625).abs() < 0.001);
    }
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
        let (nodes, shares) = connected(&layout, &links, Some(Focus::Link(2)));
        assert_eq!(shares, vec![0.5, 0.5, 1., 0.]);
        assert_eq!(nodes, vec![0.5, 0.5, 0.5, 1., 0.]);
    }
    #[test]
    fn trace_pours_a_partial_band_and_drains_from_its_current_width() {
        let app = TestAppContext::single();
        app.update(crate::init);
        let chart = app.update(|cx| {
            cx.new(|_| {
                let ink = rgb(0x8899aa).into();
                let nodes = ["a", "b", "c", "d"]
                    .into_iter()
                    .map(|id| FlowNode::new(id, id, ink))
                    .collect();
                let mut chart = SankeyFlow::new(
                    "test",
                    nodes,
                    vec![
                        FlowLink::new("a", "b", 100.),
                        FlowLink::new("b", "c", 40.),
                        FlowLink::new("b", "d", 60.),
                    ],
                )
                .with_unit(" people")
                .with_value_formatter(|value| format!("{value:.1}"));
                chart.layout = chart.build(600., 340.);
                chart
            })
        });
        app.update(|cx| chart.update(cx, |chart, cx| chart.set_focus(Some(Focus::Link(1)), cx)));
        let sample = || app.update(|cx| chart.read(cx).trace[0].value(cx));
        assert_eq!(sample(), 0.);
        app.executor().advance_clock(Duration::from_millis(120));
        let pouring = sample();
        assert!(pouring > 0. && pouring < 0.4);
        app.update(|cx| chart.update(cx, |chart, cx| chart.set_focus(None, cx)));
        assert_eq!(sample(), pouring);
        app.executor().advance_clock(Duration::from_millis(120));
        let draining = sample();
        assert!(draining > 0. && draining < pouring);
        app.executor().advance_clock(Duration::from_secs(2));
        assert_eq!(sample(), 0.);
        app.update(|cx| chart.update(cx, |chart, cx| chart.set_focus(Some(Focus::Link(1)), cx)));
        app.executor().advance_clock(Duration::from_secs(2));
        assert_eq!(sample(), 0.4);
        app.update(|cx| {
            let state = chart.read(cx);
            assert_eq!(state.trace_source, None);
            assert_eq!(state.node_trace[0].value(cx), 0.4);
            assert_eq!(state.node_trace[2].value(cx), 1.);
            let tip = state.tip(Focus::Link(1), cx).unwrap();
            assert_eq!(tip.value, "40.0 people");
            assert_eq!(tip.note, "40% of b, 100% of c");
        });
        app.update(|cx| {
            chart.update(cx, |chart, cx| {
                chart.set_focus(Some(Focus::Link(0)), cx);
                assert_eq!(chart.trace_source, Some(0));
                chart.set_focus(None, cx);
                assert_eq!(chart.trace_source, Some(0));
            });
        });
    }

    #[test]
    fn keyboard_enters_the_plot_and_crosses_to_the_nearest_node() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (chart, window) = app.add_window_view(|_, _| {
            let color = rgb(0x8899aa).into();
            let nodes = ["a", "b", "c", "d"]
                .into_iter()
                .map(|id| FlowNode::new(id, id, color))
                .collect();
            SankeyFlow::new(
                "Flow",
                nodes,
                vec![FlowLink::new("a", "c", 50.), FlowLink::new("b", "d", 50.)],
            )
            .with_particles(false)
        });
        window.update(|w, cx| {
            w.activate_window();
            w.draw(cx).clear(cx);
        });
        window.run_until_parked();
        for (key, expected) in [
            ("tab", Some(0)),
            ("down", Some(1)),
            ("right", Some(3)),
            ("home", Some(0)),
            ("end", Some(3)),
            ("escape", None),
        ] {
            if key == "tab" {
                window.update(|w, cx| w.focus_next(cx));
            } else {
                window.simulate_keystrokes(key);
            }
            window.update(|w, cx| {
                w.simulate_next_frame(cx);
                w.refresh();
                w.draw(cx).clear(cx);
            });
            window.run_until_parked();
            window.update(|_, cx| {
                assert_eq!(chart.read(cx).selected, expected.map(Focus::Node), "{key}")
            });
        }
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
    fn upright_labels_keep_a_gap_through_a_period_change() {
        let color = rgb(0x8899aa).into();
        let chart = |pro, team| {
            SankeyFlow::new(
                "Journey",
                vec![
                    FlowNode::new("a", "Activated", color),
                    FlowNode::new("p", "Pro", color),
                    FlowNode::new("t", "Team", color),
                ],
                vec![FlowLink::new("a", "p", pro), FlowLink::new("a", "t", team)],
            )
        };
        for width in [238., 310.] {
            let q2 = chart(1560., 530.).build(width, 480.);
            let q3 = chart(2310., 840.).build(width, 480.);
            for progress in [0., 0.25, 0.5, 0.75, 1.] {
                let plot = transition(&q2, &q3, progress);
                let (pro_left, pro_right) = plot.upright_label_bounds(1, width, 38., 0.);
                let pro_end = pro_right.min(pro_left + 38.);
                let (team_left, team_right) = plot.upright_label_bounds(2, width, 37., pro_end);
                assert!(team_left - pro_end >= 5.99);
                assert!(team_right - team_left >= 36.99);
                assert!(team_right <= width + 8.01);
            }
        }
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
