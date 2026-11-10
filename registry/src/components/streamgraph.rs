//! Layered streams on a wiggle baseline that morph between ranges.
use crate::primitives::{
    family::{Paint, Slide, Tone, disabled_motion, face, measure},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const GRID: usize = 161;
const PAD: f32 = 10.;
const PLOT_H: f32 = 260.;
const AXIS_H: f32 = 26.;
const TIP_GAP: f32 = 14.;
/// How the stack is centred: `Wiggle` flows, `Silhouette` centres, `Zero` is flat.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StreamOffset {
    #[default]
    Wiggle,
    Silhouette,
    Zero,
}
#[derive(Clone, Debug)]
pub struct StreamLayer {
    pub id: SharedString,
    pub label: SharedString,
    pub color: Hsla,
}
impl StreamLayer {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>, color: Hsla) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            color,
        }
    }
}
/// Values follow the order of the layers passed to `Streamgraph::new`.
#[derive(Clone, Debug)]
pub struct StreamPoint {
    pub label: SharedString,
    /// Short label on the time axis; points without one keep the axis quiet.
    pub axis_label: Option<SharedString>,
    pub values: Vec<f32>,
}
impl StreamPoint {
    pub fn new(label: impl Into<SharedString>, values: Vec<f32>) -> Self {
        Self {
            label: label.into(),
            axis_label: None,
            values,
        }
    }
    pub fn with_axis_label(mut self, label: Option<SharedString>) -> Self {
        self.axis_label = label;
        self
    }
}
#[derive(Clone, Debug)]
pub struct StreamInspect {
    pub index: usize,
    pub label: SharedString,
    pub total: f32,
    pub values: Vec<(SharedString, f32)>,
}
pub struct Streamgraph {
    title: SharedString,
    unit: SharedString,
    layers: Vec<StreamLayer>,
    points: Vec<StreamPoint>,
    hidden: Vec<SharedString>,
    offset: StreamOffset,
    direct_labels: bool,
    samples: Vec<Vec<Slide>>,
    reveal: Slide,
    revealed: bool,
    active: Option<usize>,
    active_layer: Option<usize>,
    legend_focus: Option<usize>,
    pointer: Option<(f32, f32)>,
    spots: Vec<Option<usize>>,
    cursor: Slide,
    tip_x: Slide,
    tip_y: Slide,
    tip_show: Slide,
    dragging: bool,
    focus: Option<FocusHandle>,
    focus_subscriptions: Vec<Subscription>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    touching: Rc<Cell<bool>>,
    initialized: bool,
    visible: bool,
}
impl EventEmitter<StreamInspect> for Streamgraph {}
impl Streamgraph {
    pub fn new(
        title: impl Into<SharedString>,
        layers: Vec<StreamLayer>,
        points: Vec<StreamPoint>,
    ) -> Self {
        Self {
            title: title.into(),
            unit: "".into(),
            layers,
            points,
            hidden: vec![],
            offset: StreamOffset::default(),
            direct_labels: true,
            samples: vec![],
            reveal: Slide::new(0.),
            revealed: false,
            active: None,
            active_layer: None,
            legend_focus: None,
            pointer: None,
            spots: vec![],
            cursor: Slide::new(0.),
            tip_x: Slide::new(0.),
            tip_y: Slide::new(0.),
            tip_show: Slide::new(0.),
            dragging: false,
            focus: None,
            focus_subscriptions: vec![],
            bounds: Default::default(),
            touching: Default::default(),
            initialized: false,
            visible: true,
        }
    }
    /// Unit printed after each value, such as "tickets".
    pub fn with_unit(mut self, unit: impl Into<SharedString>) -> Self {
        self.unit = unit.into();
        self
    }
    pub fn with_offset(mut self, offset: StreamOffset) -> Self {
        self.offset = offset;
        self
    }
    /// Names each layer inside its widest stretch (on by default).
    pub fn with_direct_labels(mut self, enabled: bool) -> Self {
        self.direct_labels = enabled;
        self
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
    pub fn set_points(&mut self, points: Vec<StreamPoint>, cx: &mut Context<Self>) {
        self.points = points;
        self.active = None;
        self.active_layer = None;
        self.reconcile(cx);
        cx.notify();
    }
    /// Fritsch–Carlson monotone cubic resample onto the shared grid: curves
    /// never swing past their neighbouring samples or below zero.
    fn resample(points: &[StreamPoint], layer: usize) -> Vec<f32> {
        let raw: Vec<f64> = points
            .iter()
            .map(|p| {
                p.values
                    .get(layer)
                    .copied()
                    .filter(|v| v.is_finite())
                    .unwrap_or(0.)
                    .max(0.) as f64
            })
            .collect();
        let mut out = vec![0f32; GRID];
        if raw.is_empty() {
            return out;
        }
        if raw.len() == 1 {
            out.fill(raw[0] as f32);
            return out;
        }
        let last = raw.len() - 1;
        let slopes: Vec<f64> = raw.windows(2).map(|w| w[1] - w[0]).collect();
        let mut tangents = vec![0f64; raw.len()];
        for i in 1..last {
            let (s0, s1) = (slopes[i - 1], slopes[i]);
            let combo = s0.signum() + s1.signum();
            tangents[i] = if combo == 0. {
                0.
            } else {
                combo * ((s0 + s1).abs() / 4.).min(s0.abs()).min(s1.abs())
            };
        }
        if last > 1 {
            tangents[0] = (3. * slopes[0] - tangents[1]) / 2.;
            tangents[last] = (3. * slopes[last - 1] - tangents[last - 1]) / 2.;
        } else {
            tangents[0] = slopes[0];
            tangents[1] = slopes[0];
        }
        for (k, cell) in out.iter_mut().enumerate() {
            let at = k as f64 / (GRID - 1) as f64 * last as f64;
            let seg = (at.floor() as usize).min(last - 1);
            let t = at - seg as f64;
            let t2 = t * t;
            let t3 = t2 * t;
            let (a, b) = (raw[seg], raw[seg + 1]);
            let value = (2. * t3 - 3. * t2 + 1.) * a
                + (t3 - 2. * t2 + t) * tangents[seg]
                + (-2. * t3 + 3. * t2) * b
                + (t3 - t2) * tangents[seg + 1];
            *cell = value.clamp(a.min(b), a.max(b)) as f32;
        }
        out
    }
    fn reconcile(&mut self, cx: &App) {
        while self.samples.len() < self.layers.len() {
            self.samples
                .push((0..GRID).map(|_| Slide::new(0.)).collect());
        }
        self.samples.truncate(self.layers.len());
        self.spots.resize(self.layers.len(), None);
        for i in 0..self.layers.len() {
            let visible = !self.hidden.contains(&self.layers[i].id);
            let target = if visible {
                Self::resample(&self.points, i)
            } else {
                vec![0.; GRID]
            };
            for (j, slide) in self.samples[i].iter_mut().enumerate() {
                slide.duration(target[j], 0.4, cx);
            }
        }
        self.initialized = true;
    }
    fn visible_layers(&self) -> Vec<usize> {
        (0..self.layers.len())
            .filter(|&i| !self.hidden.contains(&self.layers[i].id))
            .collect()
    }
    fn select(&mut self, next: Option<usize>, cx: &mut Context<Self>) {
        self.select_layer(next, self.active_layer, cx);
    }
    fn select_layer(&mut self, next: Option<usize>, layer: Option<usize>, cx: &mut Context<Self>) {
        if self.active == next && self.active_layer == layer {
            return;
        }
        self.active = next;
        self.active_layer = layer;
        if let Some((i, point)) = next.and_then(|i| self.points.get(i).map(|p| (i, p))) {
            let values: Vec<_> = self
                .visible_layers()
                .into_iter()
                .map(|l| {
                    (
                        self.layers[l].id.clone(),
                        point
                            .values
                            .get(l)
                            .copied()
                            .filter(|v| v.is_finite())
                            .unwrap_or(0.)
                            .max(0.),
                    )
                })
                .collect();
            cx.emit(StreamInspect {
                index: i,
                label: point.label.clone(),
                total: values.iter().map(|(_, v)| v).sum(),
                values,
            });
        }
        cx.notify();
    }
    fn toggle(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(layer) = self.layers.get(index) {
            if let Some(i) = self.hidden.iter().position(|id| *id == layer.id) {
                self.hidden.remove(i);
            } else if self.layers.len().saturating_sub(self.hidden.len()) > 1 {
                self.hidden.push(layer.id.clone());
            } else {
                return;
            }
            self.reconcile(cx);
            cx.notify();
        }
    }
    fn format_value(&self, value: f32) -> String {
        let text = grouped(f64::from(value.max(0.)));
        if self.unit.is_empty() {
            text
        } else {
            format!("{text} {}", self.unit)
        }
    }
}
/// The paint order of layers: the first sits in the middle of the stream and
/// the rest alternate above and below it; a flat baseline stacks in order.
fn stack_order(count: usize, mode: StreamOffset) -> Vec<usize> {
    if mode == StreamOffset::Zero {
        return (0..count).collect();
    }
    let mut list: Vec<usize> = vec![];
    for index in 0..count {
        if index == 0 || index % 2 == 1 {
            list.push(index);
        } else {
            list.insert(0, index);
        }
    }
    list
}
/// The baseline under the stack. Wiggle follows Byron and Wattenberg, then
/// removes its linear drift so the stream stays level across the plot.
fn baseline(layers: &[&[f32]], mode: StreamOffset) -> Vec<f32> {
    let totals: Vec<f64> = (0..GRID)
        .map(|j| layers.iter().map(|l| f64::from(l[j])).sum())
        .collect();
    let mut g = vec![0f64; GRID];
    if layers.is_empty() || mode == StreamOffset::Zero {
        return vec![0.; GRID];
    }
    if mode == StreamOffset::Silhouette {
        return totals.iter().map(|t| (-t / 2.) as f32).collect();
    }
    for j in 1..GRID {
        let mut s1 = 0.;
        let mut s2 = 0.;
        let mut below = 0.;
        for layer in layers {
            let change = f64::from(layer[j]) - f64::from(layer[j - 1]);
            s1 += f64::from(layer[j]);
            s2 += (below + change / 2.) * f64::from(layer[j]);
            below += change;
        }
        g[j] = g[j - 1] - if s1 != 0. { s2 / s1 } else { 0. };
    }
    // Least squares line through the centre of the stream, subtracted so the
    // stream never tilts off the plot.
    let centers: Vec<f64> = g
        .iter()
        .zip(&totals)
        .map(|(value, total)| value + total / 2.)
        .collect();
    let mx = (GRID - 1) as f64 / 2.;
    let my = centers.iter().sum::<f64>() / GRID as f64;
    let mut num = 0.;
    let mut den = 0.;
    for (j, value) in centers.iter().enumerate() {
        num += (j as f64 - mx) * (value - my);
        den += (j as f64 - mx).powi(2);
    }
    let slope = if den != 0. { num / den } else { 0. };
    g.iter()
        .enumerate()
        .map(|(j, value)| value - (my + slope * (j as f64 - mx)))
        .map(|v| v as f32)
        .collect()
}
/// Axis labels that fit: at most one per 72 pixels, counted back from the latest.
fn axis_picks(points: &[StreamPoint], width: f32) -> Vec<usize> {
    let labeled: Vec<usize> = points
        .iter()
        .enumerate()
        .filter(|(_, p)| p.axis_label.is_some())
        .map(|(i, _)| i)
        .collect();
    if width <= 0. || labeled.is_empty() {
        return vec![];
    }
    if labeled.len() == 1 {
        return labeled;
    }
    let stride = ((labeled.len() as f32) / (width / 72.).floor().max(2.)).ceil() as usize;
    labeled
        .iter()
        .rev()
        .enumerate()
        .filter(|(rank, _)| rank % stride.max(1) == 0)
        .map(|(_, &i)| i)
        .rev()
        .collect()
}
fn sample_layers(
    samples: &[Vec<Slide>],
    layers: &[StreamLayer],
    now: Instant,
    reduced: bool,
) -> (Vec<(Vec<f32>, Hsla)>, bool) {
    let mut active = false;
    let values = samples
        .iter()
        .zip(layers)
        .map(|(slides, layer)| {
            let values = slides
                .iter()
                .map(|slide| {
                    let (value, moving) = slide.sample_batch(now, reduced);
                    active |= moving;
                    value
                })
                .collect();
            (values, layer.color)
        })
        .collect();
    (values, active)
}
fn request_if_active(on_screen: bool, active: bool, request: impl FnOnce()) {
    if on_screen && active {
        request();
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
/// Grouped thousands with at most one fraction digit (`18,400`, `345.6`).
fn grouped(v: f64) -> String {
    let rounded = (v * 10.).round() / 10.;
    let int = rounded.trunc().abs() as i64;
    let mut digits = int.to_string();
    let mut insert = 3;
    while digits.len() > insert {
        digits.insert(digits.len() - insert, ',');
        insert += 4;
    }
    let sign = if rounded < 0. { "-" } else { "" };
    let frac = (rounded - rounded.trunc()).abs();
    if frac > 1e-9 {
        format!("{sign}{digits}.{}", (frac * 10.).round() as u64)
    } else {
        format!("{sign}{digits}")
    }
}
impl Render for Streamgraph {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus = self.focus.get_or_insert_with(|| cx.focus_handle()).clone();
        if self.focus_subscriptions.is_empty() {
            // The reference opens on the latest point when the plot is focused.
            self.focus_subscriptions
                .push(cx.on_focus(&focus, w, |s, _, cx| {
                    if s.active.is_none() && !s.points.is_empty() && s.pointer.is_none() {
                        s.select(Some(s.points.len() - 1), cx);
                    }
                }));
        }
        if !self.initialized {
            self.reconcile(cx);
        }
        let measured = self.bounds.get();
        let on_screen =
            self.visible && w.is_visible() && intersects_viewport(measured, w.viewport_size());
        if !self.revealed && on_screen {
            self.revealed = true;
            self.reveal.duration(1., 0.9, cx);
        }
        let now = cx.background_executor().now();
        let reduced = disabled_motion(cx);
        let (reveal, revealing) = self.reveal.sample_batch(now, reduced);
        let reveal = reveal.clamp(0., 1.);
        let p = Paint::get(cx);
        let (layers, changing) = sample_layers(&self.samples, &self.layers, now, reduced);
        request_if_active(on_screen, revealing || changing, || {
            w.request_animation_frame()
        });
        let empty = self.points.is_empty();
        // The stack geometry for this frame, shared by painting, hover, labels.
        let stack = stack_order(self.layers.len(), self.offset);
        let refs: Vec<&[f32]> = layers.iter().map(|(v, _)| v.as_slice()).collect();
        let g = baseline(&refs, self.offset);
        let mut lower = vec![vec![0f32; GRID]; self.layers.len()];
        let mut upper = vec![vec![0f32; GRID]; self.layers.len()];
        let mut running = g.clone();
        for &i in &stack {
            lower[i] = running.clone();
            for (j, cell) in running.iter_mut().enumerate() {
                *cell += layers[i].0[j];
            }
            upper[i] = running.clone();
        }
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        for j in 0..GRID {
            lo = lo.min(g[j]);
            hi = hi.max(running[j]);
        }
        if !(lo.is_finite() && hi.is_finite()) || hi - lo < 1e-9 {
            lo = -1.;
            hi = 1.;
        }
        let span = hi - lo;
        let centers: Vec<f32> = (0..GRID).map(|j| (g[j] + running[j]) / 2.).collect();
        let to_y = |value: f32| PAD + (1. - (value - lo) / span) * (PLOT_H - PAD * 2.);
        let plot_w = f32::from(measured.size.width);
        let n = self.points.len();
        // The layer under the pointer, found in the frame that is on screen.
        let (hover_index, hover_layer) = match self.pointer.filter(|_| !empty && plot_w > 0.) {
            Some((x, y)) => {
                let share = (x / plot_w).clamp(0., 1.);
                let j = (share * (GRID - 1) as f32).round() as usize;
                let mut hit: Option<usize> = None;
                for i in 0..self.layers.len() {
                    if self.hidden.contains(&self.layers[i].id) {
                        continue;
                    }
                    let top = to_y(upper[i][j]);
                    let bottom = to_y(lower[i][j]);
                    if y >= top - 1. && y <= bottom + 1. {
                        hit = Some(i);
                    }
                }
                (
                    Some((share * n.saturating_sub(1) as f32).round() as usize),
                    hit,
                )
            }
            None => (None, None),
        };
        // Hover wins while the pointer is inside; the legend and keyboard follow.
        let display_layer = hover_layer
            .or(self.legend_focus)
            .or(self.active_layer)
            .filter(|&i| !self.hidden.contains(&self.layers[i].id));
        let reading = hover_index.or(self.active).filter(|_| !empty);
        let labels: Vec<SharedString> = self.layers.iter().map(|l| l.label.clone()).collect();
        // The label sits where its layer is thickest, and holds its spot until
        // another stretch is clearly thicker.
        let mut label_pos: Vec<(f32, f32, bool)> = vec![];
        if self.direct_labels {
            for i in 0..self.layers.len() {
                let mut best = 4;
                for j in 4..GRID - 4 {
                    if upper[i][j] - lower[i][j] > upper[i][best] - lower[i][best] {
                        best = j;
                    }
                }
                let held = self.spots.get(i).copied().flatten().filter(|&h| {
                    upper[i][h] - lower[i][h] >= (upper[i][best] - lower[i][best]) * 0.8
                });
                let j = held.unwrap_or(best);
                self.spots[i] = Some(j);
                let thickness = (upper[i][j] - lower[i][j]) / span * (PLOT_H - PAD * 2.) * reveal;
                let text_w = if plot_w > 0. {
                    measure(&labels[i], 12., w)
                } else {
                    0.
                };
                let show = !self.hidden.contains(&self.layers[i].id)
                    && thickness >= 19.
                    && text_w + 16. < plot_w;
                let x = if plot_w > 0. {
                    (plot_w * j as f32 / (GRID - 1) as f32).clamp(
                        text_w / 2. + 6.,
                        (plot_w - text_w / 2. - 6.).max(text_w / 2. + 6.),
                    )
                } else {
                    0.
                };
                let y = to_y((upper[i][j] + lower[i][j]) * 0.5);
                label_pos.push((x, y, show));
            }
        }
        // The crosshair glides between points; the tooltip follows on its own
        // spring and never leaves the plot.
        if let Some(i) = reading.filter(|_| plot_w > 0. && n > 0) {
            let x = if n > 1 {
                plot_w * i as f32 / (n - 1) as f32
            } else {
                plot_w / 2.
            };
            self.cursor.duration(x, 0.26, cx);
            let visible_rows = self.visible_layers().len();
            let th = 40. + visible_rows as f32 * 24.;
            // The tooltip rides the middle of the focused layer at the crosshair.
            let share = (self.cursor.value(cx) / plot_w.max(1.)).clamp(0., 1.);
            let j = (share * (GRID - 1) as f32).round() as usize;
            let focus_y = display_layer
                .map(|l| to_y((upper[l][j] + lower[l][j]) * 0.5))
                .unwrap_or(PLOT_H / 2.);
            let ty = (focus_y - th / 2.).clamp(0., (PLOT_H - th).max(0.));
            let tw = 156.;
            let mut left = self.cursor.value(cx) + TIP_GAP;
            if left + tw > plot_w {
                left = (self.cursor.value(cx) - TIP_GAP - tw).max(0.);
            }
            left = left.clamp(0., (plot_w - tw).max(0.));
            self.tip_x.duration(left, 0.26, cx);
            self.tip_y.duration(ty, 0.26, cx);
            self.tip_show.duration(1., 0.16, cx);
        } else {
            self.tip_show.duration(0., 0.16, cx);
        }
        let cursor_x = self.cursor.get(w, cx);
        let tip_x = self.tip_x.get(w, cx);
        let tip_y = self.tip_y.get(w, cx);
        let tip_t = self.tip_show.get(w, cx).clamp(0., 1.);
        let bounds = self.bounds.clone();
        let touching = self.touching.clone();
        let touch = cx.listener(|s, e: &TouchDragEvent, _, cx| {
            if s.points.is_empty() {
                return;
            }
            let b = s.bounds.get();
            let t = (f32::from(e.position.x - b.origin.x) / f32::from(b.size.width).max(1.))
                .clamp(0., 1.);
            s.select(
                Some((t * s.points.len().saturating_sub(1) as f32).round() as usize),
                cx,
            );
        });
        let paint_layers = layers;
        let paint_lower = lower;
        let paint_upper = upper;
        let paint_centers = centers;
        let paint_labels = labels.clone();
        let paint_spots = label_pos;
        let paint_dim = display_layer;
        let paint_min = lo;
        let paint_span = span;
        let active_reading = reading;
        let graph = div()
            .id("streamgraph")
            .role(Role::Figure)
            .tab_index(if empty { -1 } else { 0 })
            .track_focus(&focus)
            .relative()
            .w_full()
            .h(px(PLOT_H))
            .aria_label(format!(
                "{}. {}",
                self.title,
                tr(
                    cx,
                    "Use left and right arrows to move through time, up and down to change layer",
                    "使用左右方向键移动时间，上下方向键切换图层",
                )
            ))
            .on_hover(cx.listener(|s, h: &bool, w, cx| {
                // Keyboard focus suppresses gpui hover, so a keypress would
                // otherwise clear a reading the reference keeps.
                if !*h && !w.last_input_was_keyboard() {
                    s.pointer = None;
                    s.select_layer(None, None, cx);
                }
            }))
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                if s.points.is_empty() {
                    return;
                }
                let b = s.bounds.get();
                let local = (
                    f32::from(e.position.x - b.origin.x),
                    f32::from(e.position.y - b.origin.y),
                );
                if s.pointer != Some(local) {
                    s.pointer = Some(local);
                    let width = f32::from(b.size.width).max(1.);
                    let t = (local.0 / width).clamp(0., 1.);
                    s.select(
                        Some((t * s.points.len().saturating_sub(1) as f32).round() as usize),
                        cx,
                    );
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, _, cx| {
                    s.dragging = true;
                    cx.notify();
                    if s.points.is_empty() {
                        return;
                    }
                    let b = s.bounds.get();
                    let width = f32::from(b.size.width).max(1.);
                    let t = (f32::from(e.position.x - b.origin.x) / width).clamp(0., 1.);
                    s.select(
                        Some((t * s.points.len().saturating_sub(1) as f32).round() as usize),
                        cx,
                    );
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|s, _, _, _| s.dragging = false),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|s, _, _, _| s.dragging = false),
            )
            .when(self.dragging, |graph| {
                let moved = cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                    if e.pressed_button != Some(MouseButton::Left) {
                        s.dragging = false;
                        cx.notify();
                        return;
                    }
                    if s.points.is_empty() {
                        return;
                    }
                    let b = s.bounds.get();
                    let t = (f32::from(e.position.x - b.origin.x)
                        / f32::from(b.size.width).max(1.))
                    .clamp(0., 1.);
                    s.select(
                        Some((t * s.points.len().saturating_sub(1) as f32).round() as usize),
                        cx,
                    );
                });
                let released = cx.listener(|s, _: &MouseUpEvent, _, cx| {
                    s.dragging = false;
                    cx.notify();
                });
                graph.child(
                    canvas(
                        |_, _, _| (),
                        move |_, _, w, _| {
                            w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                                if phase.capture() {
                                    moved(e, w, cx);
                                }
                            });
                            w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                                if phase.capture() && e.button == MouseButton::Left {
                                    released(e, w, cx);
                                }
                            });
                        },
                    )
                    .absolute()
                    .size(px(0.)),
                )
            })
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                let n = s.points.len();
                if n == 0 {
                    return;
                }
                let from = s.active.unwrap_or(n - 1);
                let keys = s.visible_layers();
                let at = s
                    .active_layer
                    .and_then(|l| keys.iter().position(|&k| k == l));
                let page = ((n as f32) / 6.).round().max(1.) as usize;
                let key = e.keystroke.key.as_str();
                let next: Option<(Option<usize>, Option<usize>)> = match key {
                    "left" => Some((Some(from.saturating_sub(1).min(n - 1)), s.active_layer)),
                    "right" => Some((
                        Some(if s.active.is_none() {
                            n - 1
                        } else {
                            (from + 1).min(n - 1)
                        }),
                        s.active_layer,
                    )),
                    "pageup" => Some((Some((from + page).min(n - 1)), s.active_layer)),
                    "pagedown" => Some((Some(from.saturating_sub(page)), s.active_layer)),
                    "home" => Some((Some(0), s.active_layer)),
                    "end" => Some((Some(n - 1), s.active_layer)),
                    "up" => {
                        let target = at.map_or(keys.len() - 1, |a| (a + 1).min(keys.len() - 1));
                        Some((Some(from), keys.get(target).copied()))
                    }
                    "down" => {
                        let target = at.map_or(0, |a| a.saturating_sub(1));
                        Some((Some(from), keys.get(target).copied()))
                    }
                    "escape" if s.active.is_some() || s.active_layer.is_some() => {
                        Some((None, None))
                    }
                    _ => return,
                };
                if let Some((index, layer)) = next {
                    w.prevent_default();
                    cx.stop_propagation();
                    s.pointer = None;
                    s.select_layer(index, layer, cx);
                }
            }))
            .when(empty, |graph| {
                graph.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(13.))
                        .text_color(p.faint)
                        .child(tr(cx, "No data for this range", "该范围暂无数据")),
                )
            })
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
                    move |b, _, w, cx| {
                        let width = f32::from(b.size.width);
                        let origin = b.origin;
                        let pt = |i: usize, y: f32| {
                            let shown = paint_centers[i] + (y - paint_centers[i]) * reveal;
                            point(
                                origin.x + px(width * i as f32 / (GRID - 1) as f32),
                                origin.y
                                    + px(PAD
                                        + (1. - (shown - paint_min) / paint_span)
                                            * (PLOT_H - PAD * 2.)),
                            )
                        };
                        // Neighbouring layers are separated by a 2px surface stroke.
                        for (i, (_, color)) in paint_layers.iter().enumerate() {
                            let mut fill = PathBuilder::fill();
                            fill.move_to(pt(0, paint_lower[i][0]));
                            for (j, &y) in paint_upper[i].iter().enumerate() {
                                fill.line_to(pt(j, y));
                            }
                            for (j, &y) in paint_lower[i].iter().enumerate().rev() {
                                fill.line_to(pt(j, y));
                            }
                            fill.close();
                            let dim = paint_dim.is_some() && paint_dim != Some(i);
                            let alpha = if paint_dim == Some(i) {
                                1.
                            } else if dim {
                                0.82 * 0.34
                            } else {
                                0.82
                            };
                            if let Ok(path) = fill.build() {
                                w.paint_path(path, color.opacity(alpha));
                            }
                            let mut gap = PathBuilder::stroke(px(2.));
                            gap.move_to(pt(0, paint_upper[i][0]));
                            for (j, &y) in paint_upper[i].iter().enumerate() {
                                gap.line_to(pt(j, y));
                            }
                            for (j, &y) in paint_lower[i].iter().enumerate().rev() {
                                gap.line_to(pt(j, y));
                            }
                            gap.close();
                            if let Ok(path) = gap.build() {
                                w.paint_path(path, p.panel);
                            }
                        }
                        // Direct labels ride the thickest stretch of each layer,
                        // outlined so they read over any neighbour.
                        let mut style = w.text_style();
                        style.font_weight = FontWeight::MEDIUM;
                        for (i, (x, y, show)) in paint_spots.iter().enumerate() {
                            if !show {
                                continue;
                            }
                            let label = paint_labels[i].clone();
                            let center = point(origin.x + px(*x), origin.y + px(*y) - px(8.));
                            let mut outline_run = style.to_run(label.len());
                            outline_run.color = p.panel;
                            let outline = w.text_system().shape_line(
                                label.clone(),
                                px(12.),
                                &[outline_run],
                                None,
                            );
                            for (dx, dy) in [
                                (0., -2.2),
                                (0., 2.2),
                                (-2.2, 0.),
                                (2.2, 0.),
                                (-1.6, -1.6),
                                (1.6, -1.6),
                                (-1.6, 1.6),
                                (1.6, 1.6),
                            ] {
                                let _ = outline.paint(
                                    point(
                                        center.x - outline.width / 2. + px(dx),
                                        center.y + px(dy),
                                    ),
                                    px(16.),
                                    TextAlign::Left,
                                    None,
                                    w,
                                    cx,
                                );
                            }
                            let mut run = style.to_run(label.len());
                            run.color = p.ink;
                            let shaped = w.text_system().shape_line(label, px(12.), &[run], None);
                            let _ = shaped.paint(
                                point(center.x - shaped.width / 2., center.y),
                                px(16.),
                                TextAlign::Left,
                                None,
                                w,
                                cx,
                            );
                        }
                        if active_reading.is_some() && width > 0. {
                            let mut line = PathBuilder::stroke(px(1.));
                            line.move_to(point(origin.x + px(cursor_x), origin.y));
                            line.line_to(point(origin.x + px(cursor_x), origin.y + px(PLOT_H)));
                            if let Ok(path) = line.build() {
                                w.paint_path(path, p.ink.opacity(0.5));
                            }
                        }
                    },
                )
                .size_full(),
            )
            .child(
                canvas(
                    |b, w, _| w.insert_hitbox(b, HitboxBehavior::Normal),
                    move |_, hit, w, _| {
                        w.on_mouse_event(move |e: &TouchDragEvent, phase, w, cx| {
                            if !phase.bubble() {
                                return;
                            }
                            if e.phase == TouchPhase::Started {
                                touching.set(hit.is_hovered(w));
                            }
                            if !touching.get() {
                                return;
                            }
                            let dx = f32::from(e.position.x - e.start_position.x).abs();
                            let dy = f32::from(e.position.y - e.start_position.y).abs();
                            if e.phase == TouchPhase::Moved && dx > 4. && dx > dy {
                                w.prevent_default();
                                touch(e, w, cx);
                                cx.stop_propagation();
                            } else if e.phase == TouchPhase::Ended && dx < 4. && dy < 4. {
                                touch(e, w, cx);
                            }
                            if matches!(e.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                                touching.set(false);
                            }
                        });
                    },
                )
                .absolute()
                .inset_0(),
            );
        // The tooltip trails the crosshair, listing layers top to bottom.
        let mut tip = div()
            .absolute()
            .top(px(tip_y))
            .left(px(tip_x))
            .min_w(px(156.))
            .max_w(px(256.))
            .px(px(12.))
            .py(px(10.))
            .flex()
            .flex_col()
            .gap_1()
            .rounded(px(14.))
            .border_1()
            .border_color(p.line)
            .bg(p.tray)
            .shadow_lg()
            .opacity(tip_t);
        if let Some(point) = reading.and_then(|i| self.points.get(i)) {
            tip = tip.child(
                div()
                    .mb(px(2.))
                    .text_size(px(12.))
                    .line_height(px(20.))
                    .text_color(p.secondary)
                    .child(point.label.clone()),
            );
            for &li in stack.iter().rev() {
                if self.hidden.contains(&self.layers[li].id) {
                    continue;
                }
                let layer = &self.layers[li];
                let value = point
                    .values
                    .get(li)
                    .copied()
                    .filter(|v| v.is_finite())
                    .unwrap_or(0.)
                    .max(0.);
                let dim = display_layer.is_some() && display_layer != Some(li);
                tip = tip.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .opacity(if dim { 0.5 } else { 1. })
                        .child(div().w(px(10.)).h(px(2.5)).rounded(px(2.)).bg(layer.color))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_color(if display_layer == Some(li) {
                                    p.ink
                                } else {
                                    p.secondary
                                })
                                .child(layer.label.clone()),
                        )
                        .child(
                            div()
                                .ml(px(8.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(self.format_value(value)),
                        ),
                );
            }
        }
        let graph = graph.child(tip);
        // The time axis stays quiet except where labels fit.
        let picks = axis_picks(&self.points, plot_w);
        let axis = div()
            .relative()
            .w_full()
            .h(px(AXIS_H))
            .children(picks.into_iter().map(|at| {
                let share = if n > 1 {
                    at as f32 / (n - 1) as f32
                } else {
                    0.5
                };
                let text = self.points[at].axis_label.clone().unwrap_or_default();
                let text_w = measure(&text, 12., w);
                let left = if plot_w > 0. {
                    (plot_w * share - text_w / 2.).clamp(0., (plot_w - text_w).max(0.))
                } else {
                    0.
                };
                div()
                    .absolute()
                    .top(px(8.))
                    .left(px(left))
                    .text_size(px(12.))
                    .line_height(px(12.))
                    .text_color(p.faint)
                    .child(text)
            }));
        let legend = row()
            .flex_wrap()
            .gap_1()
            .children(self.layers.iter().enumerate().map(|(i, l)| {
                let hidden = self.hidden.contains(&l.id);
                let focused = display_layer == Some(i);
                let dim = display_layer.is_some() && display_layer != Some(i);
                face(
                    ("stream-layer", i),
                    l.label.clone(),
                    32.,
                    crate::foundation::surface_radius(cx).into(),
                    Tone::Clear,
                    cx,
                )
                .pl(px(8.))
                .pr(px(10.))
                .line_height(px(20.))
                .text_size(px(14.))
                .text_color(if hidden {
                    p.faint
                } else if focused {
                    p.ink
                } else {
                    p.secondary
                })
                .when(focused, |pill| pill.bg(p.ink.opacity(0.05)))
                .when(dim, |pill| pill.opacity(0.56))
                .when(hidden, |pill| pill.line_through())
                .hover(|s| s.text_color(p.ink))
                .aria_toggled((!hidden).into())
                .gap(px(8.))
                .child(
                    div()
                        .size(px(10.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .size(px(if hidden { 6. } else { 10. }))
                                .rounded(px(if hidden { 2. } else { 3. }))
                                .bg(l.color.opacity(if hidden { 0.35 } else { 1. })),
                        ),
                )
                .child(l.label.clone())
                .on_click(cx.listener(move |s, _, _, cx| s.toggle(i, cx)))
                .on_hover(cx.listener(move |s, h: &bool, _, cx| {
                    s.legend_focus = if *h { Some(i) } else { None };
                    cx.notify();
                }))
            }));
        let detail = reading
            .and_then(|i| self.points.get(i))
            .map(|point| {
                let data = self
                    .visible_layers()
                    .into_iter()
                    .map(|li| {
                        format!(
                            "{} {}",
                            self.layers[li].label,
                            grouped(f64::from(
                                point
                                    .values
                                    .get(li)
                                    .copied()
                                    .filter(|v| v.is_finite())
                                    .unwrap_or(0.)
                                    .max(0.)
                            ))
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" · ");
                format!("{}  {}", point.label, data)
            })
            .unwrap_or_else(|| tr(cx, "Select a point to inspect", "选择数据点查看").to_string());
        let readings = div()
            .id("stream-readings")
            .absolute()
            .size(px(1.))
            .overflow_hidden()
            .role(Role::Table)
            .aria_label(self.title.clone())
            .children(self.points.iter().enumerate().map(|(i, point)| {
                let data = self
                    .visible_layers()
                    .into_iter()
                    .map(|li| {
                        format!(
                            "{} {}",
                            self.layers[li].label,
                            grouped(f64::from(
                                point
                                    .values
                                    .get(li)
                                    .copied()
                                    .filter(|v| v.is_finite())
                                    .unwrap_or(0.)
                                    .max(0.)
                            ))
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                div()
                    .id(("stream-reading", i))
                    .role(Role::Row)
                    .aria_label(format!("{}. {}", point.label, data))
            }));
        column()
            .w(px(560.))
            .max_w_full()
            .gap(px(16.))
            .p_4()
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(title(self.title.clone(), p))
            .child(legend)
            .child(column().gap(px(4.)).child(graph).child(axis))
            .child(hint(detail, p))
            .when(!empty, |card| card.child(readings))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GRID, StreamLayer, StreamOffset, StreamPoint, axis_picks, baseline, request_if_active,
        sample_layers, stack_order,
    };
    use crate::primitives::family::Slide;
    use gpui::{TestAppContext, rgb};
    use std::time::Duration;

    #[test]
    fn resample_stays_inside_its_samples() {
        let points: Vec<StreamPoint> = (0..8)
            .map(|i| {
                StreamPoint::new(
                    format!("p{i}"),
                    vec![if i == 3 { 90. } else { 10. + i as f32 * 2. }],
                )
            })
            .collect();
        let curve = super::Streamgraph::resample(&points, 0);
        assert_eq!(curve.len(), GRID);
        assert_eq!(curve[0], 10.);
        assert_eq!(curve[GRID - 1], 24.);
        // The monotone clamp keeps the spike inside its neighbours' range.
        assert!(curve.iter().all(|v| (0. ..=90.).contains(v)));
        assert!(curve.iter().copied().fold(f32::NEG_INFINITY, f32::max) >= 89.);
    }
    #[test]
    fn wiggle_baseline_stays_level() {
        let ramp: Vec<Vec<f32>> = (0..3)
            .map(|l| {
                (0..GRID)
                    .map(|j| 10. + l as f32 * 5. + j as f32 * 0.4)
                    .collect()
            })
            .collect();
        let refs: Vec<&[f32]> = ramp.iter().map(|v| v.as_slice()).collect();
        let g = baseline(&refs, StreamOffset::Wiggle);
        let centers: Vec<f32> = (0..GRID)
            .map(|j| {
                let total: f32 = ramp.iter().map(|l| l[j]).sum();
                g[j] + total / 2.
            })
            .collect();
        let drift = (centers[0] - centers[GRID - 1]).abs();
        assert!(
            drift < 2.,
            "linear drift should be removed, got {drift} across {GRID} samples"
        );
        let flat = baseline(&refs, StreamOffset::Zero);
        assert!(flat.iter().all(|v| *v == 0.));
        let sil = baseline(&refs, StreamOffset::Silhouette);
        let total0: f32 = ramp.iter().map(|l| l[0]).sum();
        assert!((sil[0] + total0 / 2.).abs() < 1e-4);
    }
    #[test]
    fn stack_order_alternates_around_the_first_layer() {
        assert_eq!(stack_order(4, StreamOffset::Zero), vec![0, 1, 2, 3]);
        assert_eq!(stack_order(4, StreamOffset::Wiggle), vec![2, 0, 1, 3]);
        assert_eq!(stack_order(5, StreamOffset::Wiggle), vec![4, 2, 0, 1, 3]);
    }
    #[test]
    fn axis_picks_keep_a_72px_stride_from_the_latest() {
        let points: Vec<StreamPoint> = (0..12)
            .map(|i| {
                StreamPoint::new(format!("p{i}"), vec![1.])
                    .with_axis_label(Some(format!("a{i}").into()))
            })
            .collect();
        let picks = axis_picks(&points, 850.);
        assert!(!picks.is_empty() && picks.len() <= 12);
        assert_eq!(picks.last().copied().unwrap_or(0), 11);
        let wide = axis_picks(&points, 100.);
        assert!(wide.len() < picks.len(), "narrow plots drop labels");
    }
    #[test]
    fn many_active_springs_schedule_once_and_hidden_schedules_none() {
        let app = TestAppContext::single();
        let (mut samples, layers) = app.update(|cx| {
            let samples = (0..4)
                .map(|_| {
                    (0..GRID)
                        .map(|_| {
                            let mut slide = Slide::new(0.);
                            slide.duration(1., 0.55, cx);
                            slide
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let layers = (0..4)
                .map(|i| StreamLayer::new(format!("{i}"), "Layer", rgb(0x8899aa).into()))
                .collect::<Vec<_>>();
            (samples, layers)
        });
        app.executor().advance_clock(Duration::from_millis(80));
        app.update(|cx| {
            let now = cx.background_executor().now();
            let (values, active) = sample_layers(&samples, &layers, now, false);
            assert!(active);
            let mut requests = 0;
            request_if_active(true, active, || requests += 1);
            assert_eq!(requests, 1);
            request_if_active(false, active, || requests += 1);
            assert_eq!(requests, 1);
            let before = values[0].0[0];
            samples[0][0].duration(0., 0.55, cx);
            let (reversed, _) = sample_layers(&samples, &layers, now, false);
            assert_eq!(reversed[0].0[0], before);
            request_if_active(true, true, || requests += 1);
            assert_eq!(requests, 2);
            let (_, active) = sample_layers(&samples, &layers, now, true);
            assert!(!active);
        });
    }
}
