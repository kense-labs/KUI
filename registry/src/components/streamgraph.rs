//! Layered time series with smoothly changing data and inspectable points.
use crate::primitives::{
    family::{Paint, Slide, Tone, disabled_motion, face},
    presentation::{column, hint, row, title, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const GRID: usize = 161;
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
    pub values: Vec<f32>,
}
impl StreamPoint {
    pub fn new(label: impl Into<SharedString>, values: Vec<f32>) -> Self {
        Self {
            label: label.into(),
            values,
        }
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
    layers: Vec<StreamLayer>,
    points: Vec<StreamPoint>,
    hidden: Vec<SharedString>,
    samples: Vec<Vec<Slide>>,
    reveal: Slide,
    revealed: bool,
    active: Option<usize>,
    cursor: Slide,
    dragging: bool,
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
            layers,
            points,
            hidden: vec![],
            samples: vec![],
            reveal: Slide::new(0.),
            revealed: false,
            active: None,
            cursor: Slide::new(0.),
            dragging: false,
            bounds: Default::default(),
            touching: Default::default(),
            initialized: false,
            visible: true,
        }
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
        self.reconcile(cx);
        cx.notify();
    }
    fn sample(points: &[StreamPoint], layer: usize, t: f32) -> f32 {
        if points.is_empty() {
            return 0.;
        }
        let at = |i: usize| {
            points[i]
                .values
                .get(layer)
                .copied()
                .filter(|v| v.is_finite())
                .unwrap_or(0.)
                .max(0.)
        };
        if points.len() == 1 {
            return at(0);
        }
        let pos = t * (points.len() - 1) as f32;
        let i = (pos.floor() as usize).min(points.len() - 2);
        let f = pos - i as f32;
        at(i) + (at(i + 1) - at(i)) * f
    }
    fn reconcile(&mut self, cx: &App) {
        while self.samples.len() < self.layers.len() {
            self.samples
                .push((0..GRID).map(|_| Slide::new(0.)).collect());
        }
        self.samples.truncate(self.layers.len());
        for (i, layer) in self.layers.iter().enumerate() {
            let visible = !self.hidden.contains(&layer.id);
            for j in 0..GRID {
                let target = if visible {
                    Self::sample(&self.points, i, j as f32 / (GRID - 1) as f32)
                } else {
                    0.
                };
                self.samples[i][j].duration(target, 0.4, cx);
            }
        }
        self.initialized = true;
    }
    fn select(&mut self, next: Option<usize>, cx: &mut Context<Self>) {
        if self.active == next {
            return;
        }
        self.active = next;
        if let Some((i, point)) = next.and_then(|i| self.points.get(i).map(|p| (i, p))) {
            let values: Vec<_> = self
                .layers
                .iter()
                .enumerate()
                .filter(|(_, l)| !self.hidden.contains(&l.id))
                .map(|(l, layer)| {
                    (
                        layer.id.clone(),
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
impl Render for Streamgraph {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
        let labels: Vec<SharedString> = self.layers.iter().map(|l| l.label.clone()).collect();
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
        let n = self.points.len();
        let active = self.active;
        let plot_w = f32::from(self.bounds.get().size.width);
        if let Some(i) = active.filter(|_| n > 0 && plot_w > 0.) {
            let x = if n > 1 {
                plot_w * i as f32 / (n - 1) as f32
            } else {
                plot_w / 2.
            };
            self.cursor.duration(x, 0.26, cx);
        }
        let cursor_x = self.cursor.get(w, cx);
        let mut graph = div()
            .id("streamgraph")
            .role(Role::Figure)
            .tab_index(0)
            .relative()
            .w_full()
            .h(px(260.))
            .aria_label(format!(
                "{}. {}",
                self.title,
                tr(
                    cx,
                    "Use left and right to inspect points",
                    "使用左右方向键查看数据"
                )
            ))
            .on_hover(cx.listener(|s, h: &bool, _, cx| {
                if !h {
                    s.select(None, cx)
                }
            }))
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
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
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                let n = s.points.len();
                if n == 0 {
                    return;
                }
                let i = s.active.unwrap_or(0);
                let next = match e.keystroke.key.as_str() {
                    "right" | "down" => Some((i + 1).min(n - 1)),
                    "left" | "up" => Some(i.saturating_sub(1)),
                    "home" => Some(0),
                    "end" => Some(n - 1),
                    "escape" => None,
                    _ => return,
                };
                cx.stop_propagation();
                s.select(next, cx);
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
                    move |b, _, w, cx| {
                        let width = f32::from(b.size.width);
                        let height = f32::from(b.size.height);
                        let totals: Vec<f32> = (0..GRID)
                            .map(|i| layers.iter().map(|(v, _)| v[i]).sum())
                            .collect();
                        // A slope-weighted centreline reduces wiggle while keeping the stream within the plot.
                        let mut baseline = vec![0f32; GRID];
                        for i in 1..GRID {
                            let total = totals[i].max(0.001);
                            let mut below = 0.;
                            let mut weighted = 0.;
                            for (values, _) in &layers {
                                let slope = values[i] - values[i - 1];
                                weighted += (below + values[i] * 0.5) * slope;
                                below += values[i];
                            }
                            baseline[i] = baseline[i - 1] - weighted / total;
                        }
                        let min = baseline.iter().copied().fold(f32::INFINITY, f32::min);
                        let max = (0..GRID)
                            .map(|i| baseline[i] + totals[i])
                            .fold(f32::NEG_INFINITY, f32::max);
                        let span = (max - min).max(1.);
                        let centers: Vec<f32> =
                            (0..GRID).map(|i| baseline[i] + totals[i] * 0.5).collect();
                        let pt = |i: usize, y: f32| {
                            let shown = centers[i] + (y - centers[i]) * reveal;
                            point(
                                b.origin.x + px(width * i as f32 / (GRID - 1) as f32),
                                b.origin.y + px(10. + (height - 20.) * (1. - (shown - min) / span)),
                            )
                        };
                        let mut lower = baseline;
                        for (layer_index, (values, color)) in layers.iter().enumerate() {
                            let upper: Vec<f32> = (0..GRID).map(|i| lower[i] + values[i]).collect();
                            let mut path = PathBuilder::fill();
                            path.move_to(pt(0, lower[0]));
                            for (i, &y) in upper.iter().enumerate() {
                                path.line_to(pt(i, y));
                            }
                            for i in (0..GRID).rev() {
                                path.line_to(pt(i, lower[i]));
                            }
                            path.close();
                            if let Ok(path) = path.build() {
                                w.paint_path(path, color.opacity(0.82));
                            }
                            let mid = (0..GRID)
                                .max_by(|&a, &b| {
                                    (upper[a] - lower[a]).total_cmp(&(upper[b] - lower[b]))
                                })
                                .unwrap_or(GRID / 2);
                            let thickness =
                                (upper[mid] - lower[mid]) * (height - 20.) / span * reveal;
                            if width >= 360. && thickness >= 19. {
                                let label = labels[layer_index].clone();
                                let mut style = w.text_style();
                                style.font_weight = FontWeight::MEDIUM;
                                let mut run = style.to_run(label.len());
                                run.color = p.ink;
                                let shaped =
                                    w.text_system().shape_line(label, px(12.), &[run], None);
                                let center = pt(mid, (upper[mid] + lower[mid]) * 0.5);
                                if shaped.width < px(width * 0.4) {
                                    let _ = shaped.paint(
                                        point(center.x - shaped.width / 2., center.y - px(8.)),
                                        px(16.),
                                        TextAlign::Left,
                                        None,
                                        w,
                                        cx,
                                    );
                                }
                            }
                            lower = upper;
                        }
                        if active.is_some() && n > 0 {
                            let mut line = PathBuilder::stroke(px(1.));
                            line.move_to(point(b.origin.x + px(cursor_x), b.origin.y));
                            line.line_to(point(b.origin.x + px(cursor_x), b.origin.y + px(height)));
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
        if let Some(i) = active.and_then(|i| self.points.get(i).map(|_| i)) {
            let point = &self.points[i];
            let mut tip = div()
                .absolute()
                .top(px(78.))
                .left(px({
                    let left = cursor_x + 14.;
                    if plot_w > 0. && left + 156. > plot_w {
                        (cursor_x - 14. - 156.).max(0.)
                    } else {
                        left.max(0.)
                    }
                }))
                .min_w(px(156.))
                .max_w(px(256.))
                .px(px(12.))
                .py(px(10.))
                .flex()
                .flex_col()
                .gap(px(4.))
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
                        .child(point.label.clone()),
                );
            for (layer_index, layer) in self.layers.iter().enumerate() {
                if self.hidden.contains(&layer.id) {
                    continue;
                }
                let value = point
                    .values
                    .get(layer_index)
                    .copied()
                    .filter(|v| v.is_finite())
                    .unwrap_or(0.)
                    .max(0.);
                tip = tip.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .child(div().w(px(10.)).h(px(2.5)).rounded(px(2.)).bg(layer.color))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.))
                                .text_color(p.secondary)
                                .child(layer.label.clone()),
                        )
                        .child(
                            div()
                                .ml(px(8.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(format!("{value:.0}")),
                        ),
                );
            }
            graph = graph.child(tip);
        }
        let detail = self
            .active
            .and_then(|i| self.points.get(i))
            .map(|point| {
                let data = self
                    .layers
                    .iter()
                    .enumerate()
                    .filter(|(_, layer)| !self.hidden.contains(&layer.id))
                    .map(|(i, l)| {
                        format!(
                            "{} {:.0}",
                            l.label,
                            point
                                .values
                                .get(i)
                                .copied()
                                .filter(|v| v.is_finite())
                                .unwrap_or(0.)
                                .max(0.)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" · ");
                format!("{}  {}", point.label, data)
            })
            .unwrap_or_else(|| tr(cx, "Select a point to inspect", "选择数据点查看").to_string());
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
            .child(graph)
            .child(hint(detail, p))
            .child(
                row()
                    .flex_wrap()
                    .gap_2()
                    .children(self.layers.iter().enumerate().map(|(i, l)| {
                        let hidden = self.hidden.contains(&l.id);
                        let text = if hidden {
                            format!("{} · {}", l.label, tr(cx, "Off", "已隐藏"))
                        } else {
                            l.label.to_string()
                        };
                        face(
                            ("stream-layer", i),
                            text.clone(),
                            32.,
                            crate::foundation::surface_radius(cx).into(),
                            Tone::Clear,
                            cx,
                        )
                        .pl(px(8.))
                        .pr(px(10.))
                        .line_height(px(20.))
                        .text_color(if hidden { p.secondary } else { p.ink })
                        .hover(|s| s.text_color(p.ink))
                        .aria_toggled((!hidden).into())
                        .gap(px(8.))
                        .child(div().size(px(10.)).rounded(px(3.)).bg(if hidden {
                            p.line
                        } else {
                            l.color
                        }))
                        .child(text)
                        .on_click(cx.listener(move |s, _, _, cx| s.toggle(i, cx)))
                    })),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{GRID, StreamLayer, request_if_active, sample_layers};
    use crate::primitives::family::Slide;
    use gpui::{TestAppContext, rgb};
    use std::time::Duration;
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
