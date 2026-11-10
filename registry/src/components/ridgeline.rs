//! Overlapping sampled distributions with keyboard inspection.
use crate::primitives::{
    family::{Paint, Slide, disabled_motion},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;
const GRID: usize = 96;
/// Rows overlap by 2.4 row heights, so the tallest ridge rises `30 * 2.4` pixels.
const ROW: f32 = 30.;
const OVERLAP: f32 = 2.4;
const LIFT: f32 = 6.;
const AXIS: f32 = 28.;
const TIP: f32 = 14.;
#[derive(Clone, Debug)]
pub struct RidgeSeries {
    pub id: SharedString,
    pub label: SharedString,
    pub samples: Vec<f32>,
    pub color: Hsla,
}
impl RidgeSeries {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        samples: Vec<f32>,
        color: Hsla,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            samples,
            color,
        }
    }
    fn sorted(&self) -> Vec<f32> {
        let mut v: Vec<_> = self
            .samples
            .iter()
            .copied()
            .filter(|v| v.is_finite())
            .collect();
        v.sort_by(f32::total_cmp);
        v
    }
}
#[derive(Clone, Debug)]
pub struct RidgeInspect {
    pub id: SharedString,
    pub count: usize,
    pub min: f32,
    pub q1: f32,
    pub median: f32,
    pub q3: f32,
    pub max: f32,
}
fn quantile(values: &[f32], p: f32) -> f32 {
    if values.is_empty() {
        return 0.;
    }
    let at = p * (values.len() - 1) as f32;
    let i = at.floor() as usize;
    let j = (i + 1).min(values.len() - 1);
    values[i] + (values[j] - values[i]) * (at - i as f32)
}
/// The reference ticks and keyboard steps land on 1/2/5 × 10^n values.
fn nice_step(span: f32, count: usize) -> f32 {
    let raw = span / count.max(1) as f32;
    if !raw.is_finite() || raw <= 0. {
        return 1.;
    }
    let power = 10f32.powf(raw.log10().floor());
    let unit = raw / power;
    (if unit >= 5. {
        5.
    } else if unit >= 2. {
        2.
    } else {
        1.
    }) * power
}
/// Silverman's rule of thumb for the kernel bandwidth of one series.
fn silverman(sorted: &[f32]) -> f32 {
    let n = sorted.len();
    if n < 2 {
        return 1.;
    }
    let mean = sorted.iter().sum::<f32>() / n as f32;
    let sd = (sorted.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / (n - 1) as f32).sqrt();
    let iqr = (quantile(sorted, 0.75) - quantile(sorted, 0.25)) / 1.34;
    let spread = if iqr > 0. { sd.min(iqr) } else { sd };
    (0.9 * if spread > 0. { spread } else { 1. } * (n as f32).powf(-0.2)).max(1e-6)
}
/// Gaussian kernel density per series on a shared grid, normalised by the
/// tallest peak of all series so ridge heights stay comparable.
fn kde_targets(samples: &[Vec<f32>], lo: f32, hi: f32) -> Vec<Vec<f32>> {
    let widths: Vec<_> = samples.iter().map(|s| silverman(s)).collect();
    let raw: Vec<_> = samples
        .iter()
        .zip(&widths)
        .map(|(sorted, bandwidth)| {
            let norm =
                1. / (sorted.len().max(1) as f32 * bandwidth * (2. * std::f32::consts::PI).sqrt());
            (0..GRID)
                .map(|i| {
                    let x = lo + (hi - lo) * i as f32 / (GRID - 1) as f32;
                    if sorted.is_empty() {
                        return 0.;
                    }
                    sorted
                        .iter()
                        .map(|v| {
                            let u = (x - v) / bandwidth;
                            if u > -5. && u < 5. {
                                (-0.5 * u * u).exp()
                            } else {
                                0.
                            }
                        })
                        .sum::<f32>()
                        * norm
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let peak = raw
        .iter()
        .flat_map(|v| v.iter().copied())
        .fold(1e-9f32, f32::max);
    raw.into_iter()
        .map(|v| v.into_iter().map(|y| y / peak).collect())
        .collect()
}
/// Shared domain: every series' range plus 2.5 bandwidths of room, rounded out to nice steps.
fn shared_domain(samples: &[Vec<f32>]) -> (f32, f32) {
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    for sorted in samples {
        if sorted.is_empty() {
            continue;
        }
        let room = 2.5 * silverman(sorted);
        lo = lo.min(sorted[0] - room);
        hi = hi.max(sorted[sorted.len() - 1] + room);
    }
    if !lo.is_finite() || !hi.is_finite() || lo == hi {
        return (0., 1.);
    }
    let step = nice_step(hi - lo, 5);
    ((lo / step).floor() * step, (hi / step).ceil() * step)
}
fn fmt(value: f32) -> String {
    let rounded = (value * 10.).round() / 10.;
    if (rounded - rounded.round()).abs() < 0.05 {
        format!("{}", rounded.round() as i64)
    } else {
        format!("{rounded:.1}")
    }
}
fn height_at(values: &[f32], t: f32) -> f32 {
    if t <= 0. || t >= 1. {
        return 0.;
    }
    let x = t * (values.len() - 1) as f32;
    let i = x.floor() as usize;
    let j = (i + 1).min(values.len() - 1);
    values[i] + (values[j] - values[i]) * (x - i as f32)
}
struct Ridge {
    id: SharedString,
    heights: Vec<Slide>,
    row: Slide,
    lift: Slide,
    sorted: Vec<f32>,
}
pub struct Ridgeline {
    title: SharedString,
    series: Vec<RidgeSeries>,
    ridges: Vec<Ridge>,
    active: Option<usize>,
    /// Keyboard reading cursor in value units; the pointer overrides it on hover.
    cursor: Option<f32>,
    pointer: Rc<Cell<Option<(f32, f32)>>>,
    tip_hold: Option<(usize, Option<f32>)>,
    tip_fade: Slide,
    tip_x: Slide,
    tip_y: Slide,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    touching: Rc<Cell<bool>>,
    reveal: Slide,
    revealed: bool,
    initialized: bool,
    lo: Slide,
    hi: Slide,
    domain_cache: (f32, f32),
    visible: bool,
    graph_focus: Option<FocusHandle>,
    focus_subscriptions: Vec<Subscription>,
}
impl EventEmitter<RidgeInspect> for Ridgeline {}
impl Ridgeline {
    pub fn new(title: impl Into<SharedString>, series: Vec<RidgeSeries>) -> Self {
        Self {
            title: title.into(),
            series,
            ridges: vec![],
            active: None,
            cursor: None,
            pointer: Default::default(),
            tip_hold: None,
            tip_fade: Slide::new(0.),
            tip_x: Slide::new(0.),
            tip_y: Slide::new(0.),
            bounds: Default::default(),
            touching: Default::default(),
            reveal: Slide::new(0.),
            revealed: false,
            initialized: false,
            lo: Slide::new(0.),
            hi: Slide::new(1.),
            domain_cache: (0., 1.),
            visible: true,
            graph_focus: None,
            focus_subscriptions: vec![],
        }
    }
    /// Set false when the host hides this chart or its tab.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
    pub fn set_series(&mut self, series: Vec<RidgeSeries>, cx: &mut Context<Self>) {
        // A ridge that survives a data change morphs into its new shape.
        let kept = self
            .active
            .and_then(|i| self.series.get(i))
            .map(|s| s.id.clone());
        self.series = series;
        self.active = kept.and_then(|id| self.series.iter().position(|s| s.id == id));
        self.reconcile(cx);
        cx.notify();
    }
    fn reconcile(&mut self, cx: &App) {
        let samples: Vec<Vec<f32>> = self.series.iter().map(|s| s.sorted()).collect();
        let (lo, hi) = shared_domain(&samples);
        self.domain_cache = (lo, hi);
        let first = self.ridges.is_empty();
        if first {
            self.lo = Slide::new(lo);
            self.hi = Slide::new(hi);
        } else {
            self.lo.duration(lo, 0.7, cx);
            self.hi.duration(hi, 0.7, cx);
        }
        let targets = kde_targets(&samples, lo, hi);
        let mut old: Vec<Option<Ridge>> = std::mem::take(&mut self.ridges)
            .into_iter()
            .map(Some)
            .collect();
        self.ridges = self
            .series
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let mut r = old
                    .iter()
                    .position(|r| r.as_ref().is_some_and(|v| v.id == item.id))
                    .and_then(|i| old[i].take())
                    .unwrap_or_else(|| Ridge {
                        id: item.id.clone(),
                        heights: (0..GRID).map(|_| Slide::new(0.)).collect(),
                        row: Slide::new(index as f32),
                        lift: Slide::new(0.),
                        sorted: vec![],
                    });
                r.sorted = samples[index].clone();
                r.row.duration(index as f32, 0.7, cx);
                for (slide, value) in r.heights.iter_mut().zip(&targets[index]) {
                    slide.duration(*value, 0.7, cx);
                }
                r.lift.duration(0., 0.24, cx);
                r
            })
            .collect();
        self.initialized = true;
    }
    fn select(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        if self.active == index {
            return;
        }
        self.active = index;
        for (i, r) in self.ridges.iter_mut().enumerate() {
            r.lift
                .duration(if index == Some(i) { 1. } else { 0. }, 0.24, cx);
        }
        self.tip_fade
            .duration(if index.is_some() { 1. } else { 0. }, 0.24, cx);
        if let Some(item) = index.and_then(|i| self.series.get(i)) {
            let values = item.sorted();
            if !values.is_empty() {
                cx.emit(RidgeInspect {
                    id: item.id.clone(),
                    count: values.len(),
                    min: values[0],
                    q1: quantile(&values, 0.25),
                    median: quantile(&values, 0.5),
                    q3: quantile(&values, 0.75),
                    max: values[values.len() - 1],
                });
            }
        }
        cx.notify();
    }
    /// Axis step for the reading cursor, like the reference's tick step.
    fn axis_step(&self) -> f32 {
        let (lo, hi) = self.domain_cache;
        let width = f32::from(self.bounds.get().size.width).max(1.);
        let gutter = if width < 420. { 36. } else { 52. };
        let plot = (width - gutter - 4.).max(40.);
        nice_step(hi - lo, ((plot / 72.).floor() as usize).max(2))
    }
    fn clear(&mut self, cx: &mut Context<Self>) {
        self.pointer.set(None);
        self.cursor = None;
        self.select(None, cx);
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
fn request_if_active(visible: bool, active: bool, request: impl FnOnce()) {
    if visible && active {
        request();
    }
}
fn sample_slides(slides: &[Slide], now: Instant, reduced: bool, active: &mut bool) -> Vec<f32> {
    slides
        .iter()
        .map(|slide| {
            let (value, moving) = slide.sample_batch(now, reduced);
            *active |= moving;
            value
        })
        .collect()
}
impl Render for Ridgeline {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.initialized {
            self.reconcile(cx);
        }
        let graph_focus = self
            .graph_focus
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        if self.focus_subscriptions.is_empty() {
            // The reference lowers the ridge when the plot loses focus.
            self.focus_subscriptions
                .push(cx.on_focus_out(&graph_focus, w, |s, _, _, cx| s.clear(cx)));
        }
        let measured = self.bounds.get();
        let on_screen =
            self.visible && w.is_visible() && intersects_viewport(measured, w.viewport_size());
        if !self.revealed && on_screen {
            self.revealed = true;
            self.reveal.duration(1., 1.0, cx);
        }
        let now = cx.background_executor().now();
        let reduced = disabled_motion(cx);
        let (reveal, mut active_motion) = self.reveal.sample_batch(now, reduced);
        let reveal = reveal.clamp(0., 1.);
        let p = Paint::get(cx);
        let bounds = self.bounds.clone();
        let touching = self.touching.clone();
        let touch = cx.listener(|s, e: &TouchDragEvent, _, cx| {
            if s.series.is_empty() {
                return;
            }
            let b = s.bounds.get();
            let y = f32::from(e.position.y - b.origin.y);
            s.select(Some(row_band(y, s.series.len())), cx);
        });
        let count = self.series.len().max(1) as f32;
        let top = ((OVERLAP - 1.) * ROW).ceil() + LIFT + 8.;
        let height = top + count * ROW + AXIS;
        let (lo_d, moving) = self.lo.sample_batch(now, reduced);
        let (hi_d, moving_hi) = self.hi.sample_batch(now, reduced);
        active_motion |= moving || moving_hi;
        let (lo, hi) = (lo_d.min(hi_d), lo_d.max(hi_d).max(lo_d + 0.001));
        let active = self.active;
        let (lo_t, hi_t) = self.domain_cache;
        let hue = self.series.first().map_or(p.bright, |s| s.color);
        // One hue, shaded by each ridge's median from a light tint to full strength.
        let ridge_tone = move |median: f32| {
            let t = ((median - lo_t) / (hi_t - lo_t).max(0.001)).clamp(0., 1.);
            p.panel.blend(hue.opacity(0.38 + t * 0.62))
        };
        let rows: Vec<_> = self
            .ridges
            .iter_mut()
            .zip(&self.series)
            .map(|(ridge, item)| {
                let q1 = quantile(&ridge.sorted, 0.25);
                let median = quantile(&ridge.sorted, 0.5);
                let q3 = quantile(&ridge.sorted, 0.75);
                let heights = sample_slides(&ridge.heights, now, reduced, &mut active_motion);
                let (row, moving) = ridge.row.sample_batch(now, reduced);
                active_motion |= moving;
                let (lift, moving) = ridge.lift.sample_batch(now, reduced);
                active_motion |= moving;
                (
                    heights,
                    row,
                    lift,
                    item.label.clone(),
                    q1,
                    median,
                    q3,
                    ridge_tone(median),
                )
            })
            .collect();
        request_if_active(on_screen, active_motion, || w.request_animation_frame());
        let cursor = self.pointer.clone();
        // The tooltip keeps its last content while it fades out on leave.
        if let Some(i) = active {
            let value = self
                .pointer
                .get()
                .map(|(x, _)| {
                    let width = f32::from(self.bounds.get().size.width).max(1.);
                    let gutter = if width < 420. { 36. } else { 52. };
                    let plot = (width - gutter - 4.).max(40.);
                    lo + ((x - gutter) / plot).clamp(0., 1.) * (hi - lo)
                })
                .or(self.cursor);
            let next = (i, value);
            if self.tip_hold.as_ref().is_none_or(|held| *held != next) {
                self.tip_hold = Some(next);
            }
        }
        let tip_fade = self.tip_fade.sample_batch(now, reduced).0.clamp(0., 1.);
        if let Some((i, value)) = &self.tip_hold {
            let width = f32::from(self.bounds.get().size.width).max(1.);
            let gutter = if width < 420. { 36. } else { 52. };
            let plot = (width - gutter - 4.).max(40.);
            // Anchor at the pointer, else above the ridge's median peak.
            let (ax, ay) = if let Some((x, y)) = self.pointer.get() {
                (x, y)
            } else {
                let base =
                    top + (self.ridges.get(*i).map(|r| r.row.value(cx)).unwrap_or(0.) + 1.) * ROW;
                let heights: Vec<f32> = self
                    .ridges
                    .get(*i)
                    .map(|r| r.heights.iter().map(|s| s.value(cx)).collect())
                    .unwrap_or_default();
                let median = self
                    .ridges
                    .get(*i)
                    .map(|r| quantile(&r.sorted, 0.5))
                    .unwrap_or(0.);
                let t = value.unwrap_or(median);
                let x_at = gutter + ((t - lo) / (hi - lo)).clamp(0., 1.) * plot;
                let peak = if heights.is_empty() {
                    0.
                } else {
                    height_at(&heights, ((median - lo) / (hi - lo)).clamp(0., 1.))
                };
                (x_at, base - peak * ROW * OVERLAP - LIFT)
            };
            let left = if ax + TIP + 170. > width {
                (ax - TIP - 170.).max(0.)
            } else {
                ax + TIP
            };
            let top_at = if ay < TIP + 96. {
                ay + TIP
            } else {
                (ay - TIP - 96.).max(0.)
            };
            self.tip_x.duration(left, 0.26, cx);
            self.tip_y.duration(top_at, 0.26, cx);
        }
        let tip_x = self.tip_x.get(w, cx);
        let tip_y = self.tip_y.get(w, cx);
        let reading = self.cursor;
        let mut graph = div()
            .id("ridgeline")
            .role(Role::Figure)
            .tab_index(0)
            .track_focus(&graph_focus)
            .relative()
            .w_full()
            .h(px(height))
            .overflow_hidden()
            .aria_label(format!(
                "{}. {}",
                self.title,
                tr(
                    cx,
                    "Use up and down to inspect distributions",
                    "使用上下方向键查看分布"
                )
            ))
            .on_hover(cx.listener(|s, h: &bool, _, cx| {
                if !h {
                    s.clear(cx);
                }
            }))
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                if s.series.is_empty() {
                    return;
                }
                let b = s.bounds.get();
                let x = f32::from(e.position.x - b.origin.x);
                let y = f32::from(e.position.y - b.origin.y);
                s.pointer.set(Some((x, y)));
                // A ridge owns the space its curve covers, else the row band.
                let i = s.pick_row(x, y, cx);
                s.select(Some(i), cx)
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, _, cx| {
                    if s.series.is_empty() {
                        return;
                    }
                    let b = s.bounds.get();
                    let x = f32::from(e.position.x - b.origin.x);
                    let y = f32::from(e.position.y - b.origin.y);
                    s.pointer.set(Some((x, y)));
                    s.select(Some(s.pick_row(x, y, cx)), cx);
                }),
            )
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                let n = s.series.len();
                if n == 0 {
                    return;
                }
                let at = s.active.map_or(-1i32, |i| i as i32);
                let choose = |s: &mut Self, index: i32, cx: &mut Context<Self>| {
                    let idx = index.clamp(0, n as i32 - 1) as usize;
                    let median = s
                        .ridges
                        .get(idx)
                        .map(|r| quantile(&r.sorted, 0.5))
                        .unwrap_or(s.domain_cache.0);
                    s.select(Some(idx), cx);
                    // The reading cursor starts at the chosen ridge's median.
                    if s.cursor.is_none() {
                        s.cursor = Some(median);
                    }
                };
                match e.keystroke.key.as_str() {
                    "down" => choose(s, at + 1, cx),
                    "up" => choose(s, if at < 0 { 0 } else { at - 1 }, cx),
                    "home" => choose(s, 0, cx),
                    "end" => choose(s, n as i32 - 1, cx),
                    "left" | "right" => {
                        let direction = if e.keystroke.key == "right" { 1. } else { -1. };
                        if s.active.is_none() {
                            choose(s, 0, cx);
                        }
                        let (lo, hi) = s.domain_cache;
                        let step = s.axis_step();
                        let base = s
                            .cursor
                            .or_else(|| {
                                s.active
                                    .and_then(|i| s.ridges.get(i).map(|r| quantile(&r.sorted, 0.5)))
                            })
                            .unwrap_or(lo);
                        s.cursor = Some((((base / step).round() + direction) * step).clamp(lo, hi));
                        s.tip_fade.duration(1., 0.24, cx);
                        cx.notify();
                    }
                    "escape" => s.clear(cx),
                    _ => return,
                }
                cx.stop_propagation();
                w.prevent_default();
            }))
            .child(
                canvas(
                    move |b, w, _| {
                        let previous = bounds.get();
                        if previous.size != b.size
                            || intersects_viewport(previous, w.viewport_size())
                                != intersects_viewport(b, w.viewport_size())
                        {
                            w.request_animation_frame();
                        }
                        bounds.set(b)
                    },
                    move |b, _, w, cx| {
                        let width = f32::from(b.size.width);
                        let gutter = if width > 0. && width < 420. { 36. } else { 52. };
                        let plot = (width - gutter - 4.).max(40.);
                        let lift = ROW * OVERLAP;
                        let sx = |value: f32| gutter + (value - lo) / (hi - lo) * plot;
                        // Axis ticks: one-pixel rules with centred labels at the bottom.
                        let step = nice_step(hi - lo, ((plot / 72.).floor() as usize).max(2));
                        let first_k = (lo / step).ceil();
                        let last_k = (hi / step).floor();
                        for k in first_k as i64..=last_k as i64 {
                            let value = k as f32 * step;
                            let x = b.origin.x + px(sx(value));
                            w.paint_quad(fill(
                                Bounds {
                                    origin: point(x, b.origin.y + px(top - ROW * 0.5)),
                                    size: size(px(1.), px(height - AXIS + 22. - (top - ROW * 0.5))),
                                },
                                p.line,
                            ));
                            let label: SharedString = fmt(value).into();
                            let mut run = w.text_style().to_run(label.len());
                            run.color = p.secondary;
                            let text = w.text_system().shape_line(label, px(12.), &[run], None);
                            let _ = text.paint(
                                point(x, b.origin.y + px(height - 8.)),
                                px(12.),
                                TextAlign::Center,
                                None,
                                w,
                                cx,
                            );
                        }
                        let denom = 1. - (rows.len() as f32 * 0.025).min(0.5);
                        let mut order: Vec<usize> =
                            (0..rows.len()).filter(|i| Some(*i) != active).collect();
                        if let Some(i) = active.filter(|i| *i < rows.len()) {
                            order.push(i);
                        }
                        for i in order {
                            let (values, row, lift_slide, label, q1, median, q3, tone) = &rows[i];
                            let stagger =
                                ((reveal - i as f32 * 0.025) / denom.max(0.001)).clamp(0., 1.);
                            let base = top + (row + 1.) * ROW - lift_slide * LIFT;
                            let at = |j: usize, y: f32| {
                                point(
                                    b.origin.x + px(gutter + plot * j as f32 / (GRID - 1) as f32),
                                    b.origin.y + px(base - y * lift * stagger),
                                )
                            };
                            let fade = if active.is_some() && active != Some(i) {
                                0.38
                            } else {
                                1.
                            };
                            let is_active = active == Some(i);
                            let fill_strength = if is_active { 0.18 } else { 0.34 };
                            let mut path = PathBuilder::fill();
                            path.move_to(at(0, 0.));
                            for (j, &v) in values.iter().enumerate() {
                                path.line_to(at(j, v));
                            }
                            path.line_to(at(GRID - 1, 0.));
                            path.close();
                            if let Ok(path) = path.build() {
                                w.paint_path(
                                    path,
                                    p.panel.blend(tone.opacity(fill_strength)).opacity(fade),
                                );
                            }
                            let a = ((q1 - lo) / (hi - lo) * (GRID - 1) as f32)
                                .round()
                                .clamp(0., (GRID - 1) as f32)
                                as usize;
                            let z = ((q3 - lo) / (hi - lo) * (GRID - 1) as f32)
                                .round()
                                .clamp(0., (GRID - 1) as f32)
                                as usize;
                            let m = ((median - lo) / (hi - lo) * (GRID - 1) as f32)
                                .round()
                                .clamp(0., (GRID - 1) as f32)
                                as usize;
                            if is_active && z > a {
                                // The lifted copy shades its middle half.
                                let mut middle = PathBuilder::fill();
                                middle.move_to(at(a, 0.));
                                for (j, &v) in values.iter().enumerate().take(z + 1).skip(a) {
                                    middle.line_to(at(j, v));
                                }
                                middle.line_to(at(z, 0.));
                                middle.close();
                                if let Ok(path) = middle.build() {
                                    w.paint_path(path, tone.opacity(0.48));
                                }
                            }
                            {
                                // Every ridge carries its median tick.
                                let mut median_line =
                                    PathBuilder::stroke(px(if is_active { 1.5 } else { 1. }));
                                median_line.move_to(at(m, 0.));
                                median_line.line_to(at(
                                    m,
                                    height_at(values, m as f32 / (GRID - 1) as f32),
                                ));
                                let color = if is_active {
                                    p.ink
                                } else {
                                    p.panel.blend(tone.opacity(0.55))
                                };
                                if let Ok(path) = median_line.build() {
                                    w.paint_path(path, color.opacity(fade));
                                }
                            }
                            let mut line =
                                PathBuilder::stroke(px(if is_active { 2.25 } else { 1.5 }));
                            line.move_to(at(0, values[0]));
                            for (j, &v) in values.iter().enumerate().skip(1) {
                                line.line_to(at(j, v));
                            }
                            if let Ok(path) = line.build() {
                                w.paint_path(path, tone.opacity(fade));
                            }
                            let mut style = w.text_style();
                            style.font_weight = if is_active {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            };
                            let mut run = style.to_run(label.len());
                            run.color = if is_active { p.ink } else { p.secondary };
                            let text =
                                w.text_system()
                                    .shape_line(label.clone(), px(12.), &[run], None);
                            if f32::from(text.width) < gutter - 12. {
                                let _ = text.paint(
                                    point(b.origin.x, b.origin.y + px(base - 9.)),
                                    px(18.),
                                    TextAlign::Left,
                                    None,
                                    w,
                                    cx,
                                );
                            }
                        }
                        // Reading cursor crosshair, at the pointer or the keyboard cursor.
                        let reading_x = cursor.get().map(|(x, _)| x).or_else(|| reading.map(sx));
                        if active.is_some()
                            && let Some(x) = reading_x
                        {
                            w.paint_quad(fill(
                                Bounds {
                                    origin: point(
                                        b.origin.x + px(x),
                                        b.origin.y + px(top - ROW * 0.5),
                                    ),
                                    size: size(px(1.), px(height - AXIS - (top - ROW * 0.5))),
                                },
                                p.ink.opacity(0.35),
                            ));
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
                            if e.phase == TouchPhase::Ended && dx < 6. && dy < 6. {
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
        let empty = self.series.is_empty() || self.series.iter().all(|s| s.samples.is_empty());
        if empty {
            graph = graph.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(14.))
                    .text_color(p.faint)
                    .child(tr(cx, "No data yet", "暂无数据")),
            );
        }
        if tip_fade > 0.01
            && let Some((i, value)) = &self.tip_hold
            && let Some(series) = self.series.get(*i)
        {
            let values = self
                .ridges
                .get(*i)
                .map(|r| r.sorted.clone())
                .unwrap_or_default();
            let (median, q1, q3) = if values.is_empty() {
                (0., 0., 0.)
            } else {
                (
                    quantile(&values, 0.5),
                    quantile(&values, 0.25),
                    quantile(&values, 0.75),
                )
            };
            let below = value
                .filter(|_| !values.is_empty())
                .map(|v| values.iter().filter(|&&s| s <= v).count() as f32 / values.len() as f32);
            let mut tip = div()
                .absolute()
                .left(px(tip_x))
                .top(px(tip_y))
                .min_w(px(170.))
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
                .opacity(tip_fade)
                .child(
                    div()
                        .mb(px(2.))
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .text_color(p.secondary)
                        .child(series.label.clone()),
                )
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .items_baseline()
                        .gap(px(12.))
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(fmt(median))
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::NORMAL)
                                .text_color(p.faint)
                                .child(tr(cx, "Median", "中位数")),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .items_baseline()
                        .gap(px(12.))
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .text_color(p.ink)
                        .child(format!("{} {} {}", fmt(q1), tr(cx, "to", "至"), fmt(q3)))
                        .child(
                            div()
                                .text_color(p.faint)
                                .child(tr(cx, "Middle half", "中间一半")),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .items_baseline()
                        .gap(px(12.))
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .text_color(p.ink)
                        .child(format!(
                            "{} {} {}",
                            fmt(values.first().copied().unwrap_or(0.)),
                            tr(cx, "to", "至"),
                            fmt(values.last().copied().unwrap_or(0.))
                        ))
                        .child(div().text_color(p.faint).child(tr(cx, "Range", "范围"))),
                );
            if let Some(fraction) = below
                && let Some(v) = value
            {
                tip = tip.child(
                    div()
                        .mt(px(4.))
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .text_color(p.faint)
                        .child(format!(
                            "{}% {} {}",
                            (fraction * 100.).round(),
                            tr(cx, "at or below", "及以下"),
                            fmt(*v)
                        )),
                );
            }
            graph = graph.child(tip);
        }
        let detail = self
            .active
            .and_then(|i| self.series.get(i).zip(self.ridges.get(i)))
            .map(|(s, r)| {
                let values = &r.sorted;
                if values.is_empty() {
                    return s.label.to_string();
                }
                format!(
                    "{}  ·  {} {}  ·  {} {} / {}  ·  n={}",
                    s.label,
                    tr(cx, "Median", "中位数"),
                    fmt(quantile(values, 0.5)),
                    tr(cx, "Middle half", "中间一半"),
                    fmt(quantile(values, 0.25)),
                    fmt(quantile(values, 0.75)),
                    values.len()
                )
            })
            .unwrap_or_else(|| tr(cx, "Select a distribution", "选择分布查看").to_string());
        let (lo_t, hi_t) = self.domain_cache;
        column()
            .relative()
            .w(px(560.))
            .max_w_full()
            .gap_3()
            .p_4()
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(title(self.title.clone(), p))
            .child(graph)
            .when(!empty, |card| {
                // The stepped tint scale under the axis.
                card.child(
                    row()
                        .justify_end()
                        .items_center()
                        .gap(px(8.))
                        .mt(px(2.))
                        .child(hint(tr(cx, "Median", "中位数"), p))
                        .child(hint(fmt(lo_t), p))
                        .child(div().flex().gap(px(2.)).children(
                            [0., 0.25, 0.5, 0.75, 1.].into_iter().map(|t| {
                                div()
                                    .w(px(18.))
                                    .h(px(10.))
                                    .rounded(px(3.))
                                    .bg(p.panel.blend(hue.opacity(0.38 + t * 0.62)))
                            }),
                        ))
                        .child(hint(fmt(hi_t), p)),
                )
            })
            // A visually hidden table lists every series' quartiles.
            .when(!empty, |card| {
                card.child(
                    div()
                        .id("ridgeline-readings")
                        .absolute()
                        .size(px(1.))
                        .overflow_hidden()
                        .role(Role::Table)
                        .aria_label(tr(cx, "Distributions", "分布读数"))
                        .children(self.series.iter().enumerate().map(|(i, s)| {
                            let values = self
                                .ridges
                                .get(i)
                                .map(|r| r.sorted.clone())
                                .unwrap_or_default();
                            div()
                                .id(("ridgeline-reading", i))
                                .role(Role::Row)
                                .aria_label(format!(
                                    "{}: {} {}, {} {}–{}, n={}",
                                    s.label,
                                    tr(cx, "median", "中位数"),
                                    fmt(quantile(&values, 0.5)),
                                    tr(cx, "middle half", "中间一半"),
                                    fmt(quantile(&values, 0.25)),
                                    fmt(quantile(&values, 0.75)),
                                    values.len(),
                                ))
                        })),
                )
            })
            .child(hint(detail, p))
    }
}
/// Fallback row band for pointer picking.
fn row_band(y: f32, n: usize) -> usize {
    (((y - 56.) / ROW).floor().max(0.) as usize).min(n.saturating_sub(1))
}
impl Ridgeline {
    /// Pick the ridge whose curve covers the point, preferring the lower row.
    fn pick_row(&self, x: f32, y: f32, cx: &App) -> usize {
        let n = self.series.len();
        if n == 0 {
            return 0;
        }
        let width = f32::from(self.bounds.get().size.width).max(1.);
        let gutter = if width < 420. { 36. } else { 52. };
        let plot = (width - gutter - 4.).max(40.);
        let top = 56.;
        let mut best: Option<usize> = None;
        for (i, ridge) in self.ridges.iter().enumerate() {
            let base = top + (ridge.row.value(cx) + 1.) * ROW;
            let t = ((x - gutter) / plot).clamp(0., 1.);
            let curve = height_at(
                &ridge
                    .heights
                    .iter()
                    .map(|s| s.value(cx))
                    .collect::<Vec<_>>(),
                t,
            ) * ROW
                * OVERLAP;
            if y <= base + 2. && y >= base - curve - 4. && best.is_none_or(|b| i > b) {
                best = Some(i);
            }
        }
        best.unwrap_or_else(|| row_band(y, n))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        kde_targets, nice_step, request_if_active, sample_slides, shared_domain, silverman,
    };
    use crate::primitives::family::Slide;
    use gpui::TestAppContext;
    use std::time::Duration;
    #[test]
    fn batch_schedules_once_and_retarget_is_continuous() {
        let app = TestAppContext::single();
        let mut slides = app.update(|cx| {
            (0..480)
                .map(|_| {
                    let mut s = Slide::new(0.);
                    s.duration(1., 0.5, cx);
                    s
                })
                .collect::<Vec<_>>()
        });
        app.executor().advance_clock(Duration::from_millis(80));
        app.update(|cx| {
            let now = cx.background_executor().now();
            let mut moving = false;
            let before = sample_slides(&slides, now, false, &mut moving);
            let mut frames = 0;
            request_if_active(true, moving, || frames += 1);
            request_if_active(false, moving, || frames += 1);
            assert_eq!(frames, 1);
            request_if_active(true, moving, || frames += 1);
            assert_eq!(frames, 2);
            slides[0].duration(0., 0.5, cx);
            let mut reverse_moving = false;
            let after = sample_slides(&slides, now, false, &mut reverse_moving);
            assert_eq!(before[0], after[0]);
            let mut reduced_moving = false;
            sample_slides(&slides, now, true, &mut reduced_moving);
            assert!(!reduced_moving);
        });
    }
    #[test]
    fn nice_steps_land_on_1_2_5() {
        assert_eq!(nice_step(10., 5), 2.);
        assert_eq!(nice_step(100., 5), 20.);
        assert_eq!(nice_step(0.5, 5), 0.1);
    }
    #[test]
    fn silverman_shrinks_with_sample_count() {
        let small: Vec<f32> = (0..8).map(|i| i as f32 * 100. / 7.).collect();
        let large: Vec<f32> = (0..800).map(|i| i as f32 * 100. / 799.).collect();
        assert!(silverman(&large) < silverman(&small));
    }
    #[test]
    fn tallest_ridge_peaks_at_one_and_others_keep_relative_height() {
        let dense: Vec<f32> = vec![5.; 40];
        let spread: Vec<f32> = (0..40).map(|i| i as f32 * 0.5).collect();
        let targets = kde_targets(&[dense, spread], 0., 20.);
        let peak = targets[0].iter().copied().fold(0f32, f32::max);
        assert!((peak - 1.).abs() < 0.01, "global peak normalises to 1");
        let spread_peak = targets[1].iter().copied().fold(0f32, f32::max);
        assert!(
            spread_peak < 0.6,
            "a wider ridge stays visibly lower than the tallest"
        );
    }
    #[test]
    fn domain_extends_past_extremes_to_nice_bounds() {
        let samples = vec![(0..30).map(|i| 10. + i as f32).collect::<Vec<_>>()];
        let (lo, hi) = shared_domain(&samples);
        assert!(lo < 10. && hi > 39., "room on each side");
        assert_eq!(lo % 5., 0.);
        assert_eq!(hi % 5., 0.);
    }
}
