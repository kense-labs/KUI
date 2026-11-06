//! Overlapping sampled distributions with keyboard inspection.
use crate::primitives::{
    family::{Paint, Slide, disabled_motion},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;
const GRID: usize = 96;
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
struct Ridge {
    id: SharedString,
    heights: Vec<Slide>,
    lift: Slide,
    sorted: Vec<f32>,
}
pub struct Ridgeline {
    title: SharedString,
    series: Vec<RidgeSeries>,
    ridges: Vec<Ridge>,
    active: Option<usize>,
    pointer: Rc<Cell<Option<(f32, f32)>>>,
    tip_x: Slide,
    tip_y: Slide,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    touching: Rc<Cell<bool>>,
    reveal: Slide,
    revealed: bool,
    initialized: bool,
    domain_cache: (f32, f32),
    visible: bool,
}
impl EventEmitter<RidgeInspect> for Ridgeline {}
impl Ridgeline {
    pub fn new(title: impl Into<SharedString>, series: Vec<RidgeSeries>) -> Self {
        Self {
            title: title.into(),
            series,
            ridges: vec![],
            active: None,
            pointer: Default::default(),
            tip_x: Slide::new(0.),
            tip_y: Slide::new(0.),
            bounds: Default::default(),
            touching: Default::default(),
            reveal: Slide::new(0.),
            revealed: false,
            initialized: false,
            domain_cache: (0., 1.),
            visible: true,
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
        self.series = series;
        self.active = None;
        self.reconcile(cx);
        cx.notify();
    }
    fn domain(&self) -> (f32, f32) {
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for v in self
            .series
            .iter()
            .flat_map(|s| s.samples.iter())
            .copied()
            .filter(|v| v.is_finite())
        {
            min = min.min(v);
            max = max.max(v);
        }
        if !min.is_finite() {
            return (0., 1.);
        }
        if max <= min {
            max = min + 1.;
        }
        (min, max)
    }
    fn reconcile(&mut self, cx: &App) {
        let (min, max) = self.domain();
        self.domain_cache = (min, max);
        let span = (max - min).max(0.001);
        let bandwidth = (span * 0.08).max(0.001);
        let mut old: Vec<Option<Ridge>> = std::mem::take(&mut self.ridges)
            .into_iter()
            .map(Some)
            .collect();
        self.ridges = self
            .series
            .iter()
            .map(|item| {
                let samples = item.sorted();
                let mut r = old
                    .iter()
                    .position(|r| r.as_ref().is_some_and(|v| v.id == item.id))
                    .and_then(|i| old[i].take())
                    .unwrap_or_else(|| Ridge {
                        id: item.id.clone(),
                        heights: (0..GRID).map(|_| Slide::new(0.)).collect(),
                        lift: Slide::new(0.),
                        sorted: vec![],
                    });
                let values: Vec<f32> = (0..GRID)
                    .map(|i| {
                        let x = min + span * i as f32 / (GRID - 1) as f32;
                        if samples.is_empty() {
                            return 0.;
                        }
                        samples
                            .iter()
                            .map(|v| {
                                let z = (x - v) / bandwidth;
                                (-0.5 * z * z).exp()
                            })
                            .sum::<f32>()
                            / samples.len() as f32
                    })
                    .collect();
                let peak = values.iter().fold(0.001f32, |a, &v| a.max(v));
                r.sorted = samples;
                for (slide, value) in r.heights.iter_mut().zip(values) {
                    slide.duration(value / peak, 0.7, cx);
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
            let i = ((y - 56.) / 30.).floor().max(0.) as usize;
            s.select(Some(i.min(s.series.len() - 1)), cx);
        });
        let count = self.series.len().max(1) as f32;
        let top = ((2.4f32 - 1.) * 30.).ceil() + 6. + 8.;
        let height = top + count * 30. + 28.;
        let active = self.active;
        let rows: Vec<_> = self
            .ridges
            .iter_mut()
            .zip(&self.series)
            .map(|(ridge, item)| {
                let q1 = quantile(&ridge.sorted, 0.25);
                let median = quantile(&ridge.sorted, 0.5);
                let q3 = quantile(&ridge.sorted, 0.75);
                let heights = sample_slides(&ridge.heights, now, reduced, &mut active_motion);
                let (lift, moving) = ridge.lift.sample_batch(now, reduced);
                active_motion |= moving;
                (
                    heights,
                    lift,
                    item.color,
                    item.label.clone(),
                    q1,
                    median,
                    q3,
                )
            })
            .collect();
        request_if_active(on_screen, active_motion, || w.request_animation_frame());
        let (min, max) = self.domain_cache;
        let span = (max - min).max(0.001);
        let cursor = self.pointer.clone();
        if let (Some(i), Some((x, y))) = (self.active, self.pointer.get()) {
            let width = f32::from(self.bounds.get().size.width);
            let left = if x + 14. + 170. > width {
                (x - 14. - 170.).max(0.)
            } else {
                x + 14.
            };
            let top = if y < 14. + 96. {
                y + 14.
            } else {
                (y - 14. - 96.).max(0.)
            };
            self.tip_x.duration(left, 0.26, cx);
            self.tip_y.duration(top, 0.26, cx);
            let _ = i;
        }
        let tip_x = self.tip_x.get(w, cx);
        let tip_y = self.tip_y.get(w, cx);
        let mut graph = div()
            .id("ridgeline")
            .role(Role::Figure)
            .tab_index(0)
            .relative()
            .w_full()
            .h(px(height))
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
                    s.pointer.set(None);
                    s.select(None, cx)
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
                let i = ((y - 56.) / 30.).floor().max(0.) as usize;
                s.select(Some(i.min(s.series.len() - 1)), cx)
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, e: &MouseDownEvent, _, cx| {
                    if s.series.is_empty() {
                        return;
                    }
                    let b = s.bounds.get();
                    let y = f32::from(e.position.y - b.origin.y);
                    let i = ((y - 56.) / 30.).floor().max(0.) as usize;
                    s.select(Some(i.min(s.series.len() - 1)), cx)
                }),
            )
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                let n = s.series.len();
                if n == 0 {
                    return;
                }
                let i = s.active.unwrap_or(0);
                let next = match e.keystroke.key.as_str() {
                    "down" | "right" => Some((i + 1).min(n - 1)),
                    "up" | "left" => Some(i.saturating_sub(1)),
                    "home" => Some(0),
                    "end" => Some(n - 1),
                    "escape" => None,
                    _ => return,
                };
                cx.stop_propagation();
                s.select(next, cx)
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
                        let mut order: Vec<usize> =
                            (0..rows.len()).filter(|i| Some(*i) != active).collect();
                        if let Some(i) = active.filter(|i| *i < rows.len()) {
                            order.push(i);
                        }
                        for i in order {
                            let (values, lift, color, label, q1, median, q3) = &rows[i];
                            let denom = 1. - (rows.len() as f32 * 0.025).min(0.5);
                            let stagger =
                                ((reveal - i as f32 * 0.025) / denom.max(0.001)).clamp(0., 1.);
                            let base = 56. + (i as f32 + 1.) * 30. - lift * 6.;
                            let at = |j: usize, y: f32| {
                                point(
                                    b.origin.x + px(gutter + plot * j as f32 / (GRID - 1) as f32),
                                    b.origin.y + px(base - y * 72. * stagger),
                                )
                            };
                            let mut path = PathBuilder::fill();
                            path.move_to(at(0, 0.));
                            for (j, &v) in values.iter().enumerate() {
                                path.line_to(at(j, v));
                            }
                            path.line_to(at(GRID - 1, 0.));
                            path.close();
                            let fade = if active.is_some() && active != Some(i) {
                                0.38
                            } else {
                                1.
                            };
                            if let Ok(path) = path.build() {
                                w.paint_path(
                                    path,
                                    p.panel
                                        .blend(color.opacity(if active == Some(i) {
                                            0.58
                                        } else {
                                            0.35
                                        }))
                                        .opacity(fade),
                                );
                            }
                            if active == Some(i) {
                                let a = ((q1 - min) / span * (GRID - 1) as f32)
                                    .round()
                                    .clamp(0., (GRID - 1) as f32)
                                    as usize;
                                let z = ((q3 - min) / span * (GRID - 1) as f32)
                                    .round()
                                    .clamp(0., (GRID - 1) as f32)
                                    as usize;
                                let mut middle = PathBuilder::fill();
                                middle.move_to(at(a, 0.));
                                for (j, &v) in values.iter().enumerate().take(z + 1).skip(a) {
                                    middle.line_to(at(j, v));
                                }
                                middle.line_to(at(z, 0.));
                                middle.close();
                                if let Ok(path) = middle.build() {
                                    w.paint_path(path, color.opacity(0.32));
                                }
                                let m = ((median - min) / span * (GRID - 1) as f32)
                                    .round()
                                    .clamp(0., (GRID - 1) as f32)
                                    as usize;
                                let mut median_line = PathBuilder::stroke(px(1.));
                                median_line.move_to(at(m, 0.));
                                median_line.line_to(at(m, values[m]));
                                if let Ok(path) = median_line.build() {
                                    w.paint_path(path, *color);
                                }
                            }
                            let mut line = PathBuilder::stroke(px(1.5));
                            line.move_to(at(0, values[0]));
                            for (j, &v) in values.iter().enumerate().skip(1) {
                                line.line_to(at(j, v));
                            }
                            if let Ok(path) = line.build() {
                                w.paint_path(path, color.opacity(fade));
                            }
                            let mut style = w.text_style();
                            style.font_weight = if active == Some(i) {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            };
                            let mut run = style.to_run(label.len());
                            run.color = if active == Some(i) {
                                p.ink
                            } else {
                                p.secondary
                            };
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
                        if active.is_some()
                            && let Some((x, _)) = cursor.get()
                        {
                            let mut hair = PathBuilder::stroke(px(1.));
                            hair.move_to(point(b.origin.x + px(x), b.origin.y));
                            hair.line_to(point(b.origin.x + px(x), b.origin.y + b.size.height));
                            if let Ok(path) = hair.build() {
                                w.paint_path(path, p.ink.opacity(0.35));
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
        if let Some(i) = self.active
            && let Some(series) = self.series.get(i)
        {
            let values = self
                .ridges
                .get(i)
                .map(|r| r.sorted.clone())
                .unwrap_or_default();
            let median = if values.is_empty() {
                0.
            } else {
                quantile(&values, 0.5)
            };
            let q1 = if values.is_empty() {
                0.
            } else {
                quantile(&values, 0.25)
            };
            let q3 = if values.is_empty() {
                0.
            } else {
                quantile(&values, 0.75)
            };
            graph = graph.child(
                div()
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
                    .child(
                        div()
                            .mb(px(2.))
                            .text_size(px(12.))
                            .line_height(px(20.))
                            .text_color(p.secondary)
                            .child(series.label.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .gap(px(12.))
                            .text_size(px(14.))
                            .line_height(px(19.6))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(format!("{median:.1}"))
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
                            .mt(px(4.))
                            .text_size(px(12.))
                            .line_height(px(20.))
                            .text_color(p.faint)
                            .child(format!("Q1 {q1:.1}  ·  Q3 {q3:.1}  ·  n={}", values.len())),
                    ),
            );
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
                    "{}  ·  {} {:.1}  ·  Q1 {:.1} / Q3 {:.1}  ·  n={}",
                    s.label,
                    tr(cx, "Median", "中位数"),
                    quantile(values, 0.5),
                    quantile(values, 0.25),
                    quantile(values, 0.75),
                    values.len()
                )
            })
            .unwrap_or_else(|| tr(cx, "Select a distribution", "选择分布查看").to_string());
        column()
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
            .child(
                row()
                    .justify_between()
                    .pl(px(if f32::from(measured.size.width) < 360. {
                        70.
                    } else {
                        100.
                    }))
                    .children(
                        (0..=5).map(|i| hint(format!("{:.0}", min + span * i as f32 / 5.), p)),
                    ),
            )
            .child(
                row()
                    .justify_end()
                    .items_center()
                    .gap_2()
                    .child(hint(tr(cx, "Median", "中位数"), p))
                    .children((0..5).map(|i| {
                        div().w(px(14.)).h(px(8.)).rounded(px(3.)).bg(self
                            .series
                            .first()
                            .map_or(p.bright, |s| s.color)
                            .opacity(0.25 + i as f32 * 0.16))
                    })),
            )
            .child(hint(detail, p))
    }
}

#[cfg(test)]
mod tests {
    use super::{request_if_active, sample_slides};
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
}
