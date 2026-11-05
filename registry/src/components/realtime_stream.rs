//! A host-driven live series which freezes while inspected.
use crate::components::animated_counter::AnimatedCounter;
use crate::primitives::{
    family::{Paint, PhaseClock, Slide, Tone, face},
    presentation::{column, hint, row, tr},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
#[derive(Clone, Debug)]
pub struct StreamReading {
    pub label: SharedString,
    pub value: f32,
    pub anomaly: Option<SharedString>,
}
impl StreamReading {
    pub fn new(label: impl Into<SharedString>, value: f32) -> Self {
        Self {
            label: label.into(),
            value,
            anomaly: None,
        }
    }
    pub fn with_anomaly(mut self, label: impl Into<SharedString>) -> Self {
        self.anomaly = Some(label.into());
        self
    }
}
#[derive(Clone, Debug)]
pub struct LiveInspect {
    pub label: SharedString,
    pub value: f32,
    pub anomaly: Option<SharedString>,
}
pub struct RealtimeStream {
    title: SharedString,
    unit: SharedString,
    counter: Option<Entity<AnimatedCounter>>,
    readings: Vec<StreamReading>,
    frozen: Option<Vec<StreamReading>>,
    hovering: bool,
    keyboard_hold: bool,
    pinned: bool,
    selected: Option<usize>,
    capacity: usize,
    visible: bool,
    shift: Slide,
    axis_min: Slide,
    axis_max: Slide,
    axis_initialized: bool,
    marker: PhaseClock,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    touching: Rc<Cell<bool>>,
    graph_focus: Option<FocusHandle>,
    _focus_out: Option<Subscription>,
}
impl EventEmitter<LiveInspect> for RealtimeStream {}
impl RealtimeStream {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            unit: "".into(),
            counter: None,
            readings: vec![],
            frozen: None,
            hovering: false,
            keyboard_hold: false,
            pinned: false,
            selected: None,
            capacity: 60,
            visible: true,
            shift: Slide::new(0.),
            axis_min: Slide::new(0.),
            axis_max: Slide::new(1.),
            axis_initialized: false,
            marker: Default::default(),
            bounds: Default::default(),
            touching: Default::default(),
            graph_focus: None,
            _focus_out: None,
        }
    }
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity.clamp(2, 2000);
        self
    }
    pub fn with_unit(mut self, unit: impl Into<SharedString>) -> Self {
        self.unit = unit.into();
        self
    }
    /// Hosts should update this when the chart leaves or re-enters their viewport.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible == visible {
            return;
        }
        self.visible = visible;
        self.shift.reset(0.);
        if visible {
            cx.notify();
        }
    }
    /// Add a finite observation. The retained buffer stays bounded while the display is frozen.
    pub fn push(&mut self, reading: StreamReading, cx: &mut Context<Self>) {
        if !reading.value.is_finite() {
            return;
        }
        if self.visible && self.frozen.is_none() && reading.anomaly.is_some() {
            self.marker.restart(cx);
        }
        self.readings.push(reading);
        if self.readings.len() > self.capacity {
            self.readings.remove(0);
        }
        if self.visible && self.frozen.is_none() {
            self.shift.translate(1.);
            self.shift.duration(0., 0.35, cx);
        }
        if self.visible && self.frozen.is_none() {
            cx.notify();
        }
    }
    fn data(&self) -> &[StreamReading] {
        self.frozen.as_deref().unwrap_or(&self.readings)
    }
    /// Hold or resume the display; the host may continue supplying readings.
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        if self.pinned == paused {
            return;
        }
        self.pinned = paused;
        self.freeze(self.pinned || self.hovering || self.keyboard_hold, cx);
        cx.notify();
    }
    fn freeze(&mut self, freeze: bool, cx: &mut Context<Self>) {
        if freeze && self.frozen.is_none() {
            self.frozen = Some(self.readings.clone());
        } else if !freeze && self.frozen.take().is_some() {
            self.selected = None;
            self.shift.reset(0.);
        } else {
            return;
        }
        cx.notify();
    }
    fn inspect(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        if self.selected == index {
            return;
        }
        self.selected = index;
        if let Some(reading) = index.and_then(|i| self.data().get(i)) {
            cx.emit(LiveInspect {
                label: reading.label.clone(),
                value: reading.value,
                anomaly: reading.anomaly.clone(),
            });
        }
        cx.notify();
    }
}
fn axis_scale(data: &[StreamReading]) -> f64 {
    data.iter()
        .map(|v| f64::from(v.value).abs())
        .fold(1f64, f64::max)
}
fn normalized(value: f32, scale: f64) -> f32 {
    (f64::from(value) / scale) as f32
}
impl Render for RealtimeStream {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.graph_focus.is_none() {
            let focus = cx.focus_handle().tab_stop(true);
            let subscription = cx.on_focus_out(&focus, w, |s, _, _, cx| {
                s.keyboard_hold = false;
                s.freeze(s.pinned || s.hovering, cx);
            });
            self.graph_focus = Some(focus);
            self._focus_out = Some(subscription);
        }
        let graph_focus = self
            .graph_focus
            .as_ref()
            .expect("focus initialized")
            .clone();
        let p = Paint::get(cx);
        let data = self.data().to_vec();
        let n = data.len();
        let active = self.selected;
        // Normalize before subtraction: finite f32 endpoints can have an infinite f32 span.
        let scale = axis_scale(&data);
        let to_axis = move |value: f32| normalized(value, scale);
        let (target_min, target_max) = if n == 0 {
            (0., 1.)
        } else {
            (
                data.iter().fold(0f32, |a, v| a.min(to_axis(v.value))),
                data.iter()
                    .fold(f32::NEG_INFINITY, |a, v| a.max(to_axis(v.value))),
            )
        };
        let span = (target_max - target_min)
            .max(0.1 * target_max.abs())
            .max(1e-6);
        let target_min = target_min - 0.18 * span;
        let target_max = target_max + 0.18 * span;
        if !self.axis_initialized {
            self.axis_min.reset(target_min);
            self.axis_max.reset(target_max);
            self.axis_initialized = true;
        } else {
            self.axis_min.duration(target_min, 0.45, cx);
            self.axis_max.duration(target_max, 0.45, cx);
        }
        let min = if self.visible && w.is_visible() {
            self.axis_min.get(w, cx)
        } else {
            target_min
        };
        let max = if self.visible && w.is_visible() {
            self.axis_max.get(w, cx)
        } else {
            target_max
        };
        let phase = if self.visible && w.is_visible() {
            self.shift.get(w, cx).max(0.)
        } else {
            0.
        };
        let moving = self.frozen.is_none();
        let marker_age = if self.visible && w.is_visible() {
            self.marker.time(0.4, w, cx)
        } else {
            0.4
        };
        let bounds = self.bounds.clone();
        let touching = self.touching.clone();
        let graph_data = data.clone();
        let touch = cx.listener(|s, e: &TouchDragEvent, _, cx| match e.phase {
            TouchPhase::Started => s.freeze(true, cx),
            TouchPhase::Moved | TouchPhase::Ended => {
                let n = s.data().len();
                if n > 0 {
                    let b = s.bounds.get();
                    let t = (f32::from(e.position.x - b.origin.x)
                        / (f32::from(b.size.width) - 38.).max(1.))
                    .clamp(0., 1.);
                    s.inspect(Some((t * (n - 1) as f32).round() as usize), cx);
                }
                if e.phase == TouchPhase::Ended {
                    s.freeze(s.pinned || s.hovering || s.keyboard_hold, cx);
                }
            }
            TouchPhase::Cancelled => s.freeze(s.pinned || s.hovering || s.keyboard_hold, cx),
        });
        let tip_reading = active
            .filter(|&i| i < n)
            .map(|i| graph_data[i].clone());
        let mut graph = div()
            .id("realtime-stream")
            .role(Role::Figure)
            .tab_index(0)
            .track_focus(&graph_focus)
            .relative()
            .w_full()
            .h(px(200.))
            .overflow_hidden()
            .aria_label(format!(
                "{}. {}",
                self.title,
                tr(
                    cx,
                    "Hover to pause. Use left and right to inspect readings",
                    "悬停暂停，使用左右方向键查看数据"
                )
            ))
            .on_hover(cx.listener(|s, h: &bool, _, cx| {
                s.hovering = *h;
                s.freeze(s.hovering || s.pinned || s.keyboard_hold, cx)
            }))
            .on_mouse_move(cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                let n = s.data().len();
                if n == 0 {
                    return;
                }
                let b = s.bounds.get();
                let t = (f32::from(e.position.x - b.origin.x)
                    / (f32::from(b.size.width) - 38.).max(1.))
                .clamp(0., 1.);
                s.inspect(Some((t * (n - 1) as f32).round() as usize), cx);
            }))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                let key = e.keystroke.key.as_str();
                if key == "tab" {
                    s.keyboard_hold = false;
                    s.freeze(s.pinned || s.hovering, cx);
                    return;
                }
                if !matches!(key, "left" | "right" | "home" | "end" | "escape") {
                    return;
                }
                let n = s.data().len();
                if n == 0 {
                    return;
                }
                if key != "escape" {
                    s.keyboard_hold = true;
                    if s.frozen.is_none() {
                        s.freeze(true, cx);
                    }
                }
                let i = s.selected.unwrap_or(n - 1);
                let next = match key {
                    "left" => Some(i.saturating_sub(1)),
                    "right" => Some((i + 1).min(n - 1)),
                    "home" => Some(0),
                    "end" => Some(n - 1),
                    "escape" => {
                        s.pinned = false;
                        s.keyboard_hold = false;
                        s.freeze(s.hovering, cx);
                        None
                    }
                    _ => return,
                };
                w.prevent_default();
                cx.stop_propagation();
                s.inspect(next, cx);
            }))
            .child(
                canvas(
                    move |b, _, _| bounds.set(b),
                    move |b, _, w, cx| {
                        if n == 0 {
                            return;
                        }
                        let width = (f32::from(b.size.width) - 38.).max(1.);
                        let height = (f32::from(b.size.height) - 18.).max(1.);
                        let span = (max - min).max((1. / scale) as f32);
                        let plot_width = (width - 44.).max(1.);
                        let plot_height = (height - 36.).max(1.);
                        let x_for = |i: usize| {
                            if n > 1 {
                                plot_width
                                    * (i as f32 + if moving { phase } else { 0. })
                                    / (n - 1) as f32
                            } else {
                                plot_width
                            }
                        };
                        let at = |i: usize| {
                            point(
                                b.origin.x + px(x_for(i)),
                                b.origin.y
                                    + px(14.
                                        + (1. - (to_axis(graph_data[i].value) - min) / span)
                                            * plot_height),
                            )
                        };
                        for tick in 0..=2 {
                            let y = 14. + plot_height * (1. - tick as f32 / 2.);
                            let mut grid = PathBuilder::stroke(px(1.));
                            grid.move_to(point(b.origin.x, b.origin.y + px(y)));
                            grid.line_to(point(b.origin.x + px(plot_width), b.origin.y + px(y)));
                            if let Ok(path) = grid.build() {
                                w.paint_path(path, p.line);
                            }
                            let tick_value = f64::from(min + span * tick as f32 / 2.) * scale;
                            let label: SharedString = if tick_value.abs() >= 1_000_000. {
                                format!("{tick_value:.1e}").into()
                            } else {
                                format!("{tick_value:.0}").into()
                            };
                            let mut run = w.text_style().to_run(label.len());
                            run.color = p.secondary;
                            let line = w.text_system().shape_line(label, px(10.), &[run], None);
                            let _ = line.paint(
                                point(b.origin.x + px(plot_width + 8.), b.origin.y + px(y - 6.)),
                                px(14.),
                                TextAlign::Left,
                                None,
                                w,
                                cx,
                            );
                        }
                        let mut area = PathBuilder::fill();
                        area.move_to(point(at(0).x, b.origin.y + px(height - 10.)));
                        for i in 0..n {
                            area.line_to(at(i));
                        }
                        area.line_to(point(at(n - 1).x, b.origin.y + px(height - 10.)));
                        area.close();
                        if let Ok(path) = area.build() {
                            w.paint_path(path, p.bright.opacity(0.1));
                        }
                        let mut path = PathBuilder::stroke(px(2.));
                        path.move_to(at(0));
                        for i in 1..n {
                            path.line_to(at(i));
                        }
                        if let Ok(path) = path.build() {
                            w.paint_path(path, p.bright);
                        }
                        let mut last_tag_right = b.origin.x - px(8.);
                        for (i, r) in graph_data.iter().enumerate() {
                            if let Some(label) = r.anomaly.as_ref() {
                                let pt = at(i);
                                if pt.x >= b.origin.x && pt.x <= b.origin.x + px(width) {
                                    let t = (marker_age / 0.4).clamp(0., 1.);
                                    let radius = if i + 1 == n {
                                        3. * (t + 0.3 * (std::f32::consts::PI * t).sin())
                                    } else {
                                        3.
                                    };
                                    w.paint_quad(
                                        fill(
                                            Bounds {
                                                origin: point(pt.x - px(radius), pt.y - px(radius)),
                                                size: size(px(radius * 2.), px(radius * 2.)),
                                            },
                                            p.danger,
                                        )
                                        .corner_radii(px(radius)),
                                    );
                                    let label = label.clone();
                                    let mut run = w.text_style().to_run(label.len());
                                    run.color = p.danger;
                                    let text =
                                        w.text_system().shape_line(label, px(10.), &[run], None);
                                    let x = px(f32::from(pt.x - text.width / 2.)
                                        .max(f32::from(b.origin.x))
                                        .min(f32::from(b.origin.x + px(width) - text.width)));
                                    if x >= last_tag_right + px(8.) && text.width <= px(width) {
                                        let y =
                                            px(f32::from(pt.y - px(18.))
                                                .max(f32::from(b.origin.y)));
                                        let _ = text.paint(
                                            point(x, y),
                                            px(14.),
                                            TextAlign::Left,
                                            None,
                                            w,
                                            cx,
                                        );
                                        last_tag_right = x + text.width;
                                    }
                                }
                            }
                        }
                        if n > 1 {
                            for i in [0, (n - 1) / 2, n - 1] {
                                let label = graph_data[i].label.clone();
                                let mut run = w.text_style().to_run(label.len());
                                run.color = p.secondary;
                                let line = w.text_system().shape_line(label, px(10.), &[run], None);
                                let x = x_for(i).clamp(0., (width - f32::from(line.width)).max(0.));
                                let _ = line.paint(
                                    point(b.origin.x + px(x), b.origin.y + px(height + 2.)),
                                    px(14.),
                                    TextAlign::Left,
                                    None,
                                    w,
                                    cx,
                                );
                            }
                        }
                        if let Some(i) = active.filter(|&i| i < n) {
                            let pt = at(i);
                            let mut line = PathBuilder::stroke(px(1.));
                            line.move_to(point(pt.x, b.origin.y));
                            line.line_to(point(pt.x, b.origin.y + px(height)));
                            if let Ok(path) = line.build() {
                                w.paint_path(path, p.ink.opacity(0.5));
                            }
                            w.paint_quad(fill(
                                Bounds {
                                    origin: point(pt.x - px(6.), pt.y - px(6.)),
                                    size: size(px(12.), px(12.)),
                                },
                                p.panel,
                            ).corner_radii(px(6.)));
                            w.paint_quad(fill(
                                Bounds {
                                    origin: point(pt.x - px(4.), pt.y - px(4.)),
                                    size: size(px(8.), px(8.)),
                                },
                                p.bright,
                            ).corner_radii(px(4.)));
                            let _ = cx;
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
                            if e.phase == TouchPhase::Started
                                || e.phase == TouchPhase::Ended
                                || e.phase == TouchPhase::Cancelled
                            {
                                touch(e, w, cx);
                            } else if e.phase == TouchPhase::Moved && dx > 4. && dx > dy {
                                w.prevent_default();
                                touch(e, w, cx);
                                cx.stop_propagation();
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
        if let Some(point) = tip_reading {
            let width = (f32::from(self.bounds.get().size.width) - 38.).max(1.);
            let plot_width = (width - 44.).max(1.);
            let x = if n > 1 {
                plot_width
                    * (active.unwrap_or(0) as f32 + if moving { phase } else { 0. })
                    / (n - 1) as f32
            } else {
                plot_width
            };
            let span = (max - min).max((1. / scale) as f32);
            let plot_height = (200_f32 - 18. - 36.).max(1.);
            let y = 14. + (1. - (point.value - min) / span) * plot_height;
            let left = if x + 14. + 120. > plot_width {
                (x - 14. - 120.).max(0.)
            } else {
                x + 14.
            };
            let top = if y < 12. + 44. { y + 14. } else { y - 12. - 44. };
            graph = graph.child(
                div()
                    .absolute()
                    .left(px(left.max(0.)))
                    .top(px(top.max(0.)))
                    .px(px(12.))
                    .py(px(8.))
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .rounded(crate::foundation::surface_radius(cx))
                    .border_1()
                    .border_color(p.line)
                    .bg(p.panel)
                    .shadow_lg()
                    .child(
                        div()
                            .text_size(px(14.))
                            .line_height(px(18.2))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(format!("{:.1}{}", point.value, self.unit)),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(15.6))
                            .text_color(p.faint)
                            .child(point.label.clone()),
                    ),
            );
        }
        let current = self
            .selected
            .and_then(|i| data.get(i))
            .or_else(|| data.last());
        let value = current.map_or(0., |v| v.value);
        let counter = self
            .counter
            .get_or_insert_with(|| {
                cx.new(|_| {
                    AnimatedCounter::new(f64::from(
                        value.clamp(-1_000_000_000_000., 1_000_000_000_000.),
                    ))
                    .with_size(36.)
                })
            })
            .clone();
        counter.update(cx, |v, cx| {
            v.set_value(
                f64::from(value.clamp(-1_000_000_000_000., 1_000_000_000_000.)),
                cx,
            )
        });
        let text = current
            .map(|r| {
                format!(
                    "{}  {:.1}{}",
                    r.label,
                    r.value,
                    r.anomaly
                        .as_ref()
                        .map_or(String::new(), |a| format!(" · {a}"))
                )
            })
            .unwrap_or_else(|| tr(cx, "Waiting for data", "等待数据").to_string());
        column()
            .w(px(560.))
            .max_w_full()
            .gap(px(12.))
            .p(px(24.))
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .text_color(p.ink)
            .child(
                row()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(16.))
                            .line_height(px(22.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(self.title.clone()),
                    ),
            )
            .child(
                row()
                    .items_center()
                    .justify_between()
                    .child(
                        row()
                            .items_baseline()
                            .gap(px(6.))
                            .when(value.abs() <= 1_000_000_000_000., |row| row.child(counter))
                            .when(value.abs() > 1_000_000_000_000., |row| {
                                row.child(format!("{value:.2e}"))
                            })
                            .child(hint(self.unit.clone(), p)),
                    )
                    .child({
                        let label = if self.frozen.is_some() {
                            tr(cx, "Paused", "已暂停")
                        } else {
                            tr(cx, "Live", "实时")
                        };
                        face(
                            "live-status",
                            label,
                            32.,
                            crate::foundation::surface_radius(cx).into(),
                            Tone::Clear,
                            cx,
                        )
                        .pl(px(10.))
                        .pr(px(12.))
                        .gap(px(8.))
                        .border_1()
                        .border_color(p.line)
                        .text_color(p.secondary)
                        .hover(|s| s.bg(p.tray).text_color(p.ink))
                        .aria_toggled(self.pinned.into())
                        .child(if self.frozen.is_some() {
                            div().w(px(8.)).h(px(8.)).flex().justify_between().child(
                                div().w(px(3.)).h(px(8.)).bg(p.secondary),
                            ).child(div().w(px(3.)).h(px(8.)).bg(p.secondary))
                        } else {
                            div().size(px(8.)).rounded_full().bg(p.bright)
                        })
                        .child(label)
                        .on_click(cx.listener(|s, _, _, cx| s.set_paused(!s.pinned, cx)))
                        .on_key_down(cx.listener(
                            |s, e: &KeyDownEvent, w, cx| {
                                if matches!(e.keystroke.key.as_str(), "enter" | "space")
                                    && !e.is_held
                                {
                                    s.set_paused(!s.pinned, cx);
                                    w.prevent_default();
                                    cx.stop_propagation();
                                }
                            },
                        ))
                    }),
            )
            .child(graph)
            .child(hint(text, p))
    }
}

#[cfg(test)]
mod tests {
    use super::{RealtimeStream, StreamReading, axis_scale, normalized};
    use gpui::TestAppContext;
    #[test]
    fn extreme_finite_values_produce_a_finite_axis() {
        let data = vec![
            StreamReading::new("low", -f32::MAX),
            StreamReading::new("high", f32::MAX),
        ];
        let scale = axis_scale(&data);
        let low = normalized(data[0].value, scale);
        let high = normalized(data[1].value, scale);
        assert!(low.is_finite() && high.is_finite());
        assert!((high - low).is_finite());
        assert_eq!(low, -1.);
        assert_eq!(high, 1.);
    }
    #[test]
    fn unrelated_key_does_not_freeze_and_blur_releases_keyboard_hold() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, _| RealtimeStream::new("Live"));
        w.update(|w, cx| {
            view.update(cx, |s, cx| s.push(StreamReading::new("one", 1.), cx));
            let focus = view.read(cx).graph_focus.clone().expect("focus");
            focus.focus(w, cx);
        });
        w.simulate_keystrokes("a");
        assert!(w.update(|_, cx| view.read(cx).frozen.is_none()));
        w.simulate_keystrokes("left");
        assert!(w.update(|_, cx| view.read(cx).frozen.is_some()));
        w.simulate_keystrokes("tab");
        w.run_until_parked();
        assert!(w.update(|_, cx| view.read(cx).frozen.is_none()));
    }
}
