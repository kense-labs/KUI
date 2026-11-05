//! Single and two-ended sliders with independent, non-crossing keyboard stops.
use crate::primitives::{
    direct_touch::Contact,
    family::{Paint, PhaseClock, Slide, disabled_motion, measure},
    form_chrome::*,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::VecDeque, rc::Rc};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlideValue {
    pub low: f64,
    pub high: Option<f64>,
}
pub struct Slider {
    label: SharedString,
    min: f64,
    max: f64,
    step: f64,
    detents: Vec<f64>,
    value: SlideValue,
    suffix: SharedString,
    prefix: SharedString,
    contact: Contact,
    drag: Option<bool>,
    position: [Slide; 2],
    bubble: [Slide; 2],
    labels: [RollingValue; 2],
    readout: RollingValue,
    direction: [f32; 2],
    last_thumb: usize,
    linger: PhaseClock,
    since_move: PhaseClock,
    pointer: f64,
    grab: f64,
    samples: VecDeque<(f32, f32)>,
    since_grab: PhaseClock,
    track: Rc<Cell<Bounds<Pixels>>>,
    focus: [FocusHandle; 2],
}
impl EventEmitter<SlideValue> for Slider {}
impl Slider {
    pub fn new(
        label: impl Into<SharedString>,
        range: std::ops::RangeInclusive<f64>,
        step: f64,
        value: SlideValue,
        cx: &mut Context<Self>,
    ) -> Self {
        let (min, max) = (*range.start(), *range.end());
        assert!(min < max && min.is_finite() && max.is_finite() && step > 0. && step.is_finite());
        let low = value.low.clamp(min, max);
        Self {
            label: label.into(),
            min,
            max,
            step,
            detents: Vec::new(),
            value: SlideValue {
                low,
                high: value.high.map(|h| h.clamp(low, max)),
            },
            suffix: "".into(),
            prefix: "".into(),
            contact: Default::default(),
            drag: None,
            position: [
                Slide::snappy(((low - min) / (max - min)) as f32),
                Slide::snappy(((value.high.unwrap_or(low) - min) / (max - min)) as f32),
            ],
            bubble: [Slide::snappy(0.); 2],
            labels: Default::default(),
            readout: Default::default(),
            direction: [1.; 2],
            last_thumb: 0,
            linger: Default::default(),
            since_move: Default::default(),
            pointer: low,
            grab: 0.,
            samples: VecDeque::with_capacity(6),
            since_grab: Default::default(),
            track: Default::default(),
            focus: [
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
            ],
        }
    }
    pub fn with_prefix(mut self, s: impl Into<SharedString>) -> Self {
        self.prefix = s.into();
        self
    }
    fn display(&self, v: f64) -> String {
        let d = v.to_string();
        if self.prefix.as_ref() == "$" && v.fract() == 0. {
            let mut t = String::new();
            for (i, c) in d.chars().enumerate() {
                if i > 0 && (d.len() - i).is_multiple_of(3) {
                    t.push(',')
                }
                t.push(c)
            }
            format!("{}{}{}", self.prefix, t, self.suffix)
        } else {
            format!("{}{}{}", self.prefix, d, self.suffix)
        }
    }
    pub fn with_suffix(mut self, s: impl Into<SharedString>) -> Self {
        self.suffix = s.into();
        self
    }
    pub fn value(&self) -> SlideValue {
        self.value
    }
    pub fn set_low(&mut self, n: f64, cx: &mut Context<Self>) {
        self.set(n, false, cx)
    }
    /// Optional drag detents, normalized to the existing step grid.
    /// An empty list preserves the ordinary slider behavior.
    pub fn set_detents(&mut self, values: Vec<f64>, cx: &mut Context<Self>) {
        self.detents = values
            .into_iter()
            .filter(|v| v.is_finite() && *v >= self.min && *v <= self.max)
            .map(|v| {
                (self.min + ((v - self.min) / self.step).round() * self.step)
                    .clamp(self.min, self.max)
            })
            .collect();
        self.detents.sort_by(f64::total_cmp);
        self.detents.dedup();
        cx.notify();
    }
    fn detent(&self, n: f64) -> f64 {
        let width = f64::from(f32::from(self.track.get().size.width));
        if self.detents.is_empty() || width <= 0. || n < self.min || n > self.max {
            return n;
        }
        let tolerance = (self.max - self.min) * 8. / width;
        self.detents
            .iter()
            .copied()
            .min_by(|a, b| (a - n).abs().total_cmp(&(b - n).abs()))
            .filter(|value| (value - n).abs() <= tolerance)
            .unwrap_or(n)
    }
    fn val(&self, x: Pixels) -> f64 {
        let b = self.track.get();
        let f = (x - b.left()) / b.size.width.max(px(1.));
        self.min + f64::from(f) * (self.max - self.min)
    }
    // Physical motion may strain past a stop; the value emitted to the host stays on the grid.
    fn stop(&self, n: f64, high: bool) -> f64 {
        n.clamp(
            if high { self.value.low } else { self.min },
            if high {
                self.max
            } else {
                self.value.high.unwrap_or(self.max)
            },
        )
    }
    fn place_drag(&mut self, n: f64, high: bool, cx: &mut Context<Self>) {
        let stop = self.stop(n, high);
        let span = self.max - self.min;
        let width = f64::from(f32::from(self.track.get().size.width)).max(1.);
        let outside = (n - stop) / span * width;
        let resistance = if disabled_motion(cx) {
            0.
        } else {
            outside * 0.55 / (1. + outside.abs() * 0.55 / 9.)
        };
        self.position[usize::from(high)]
            .reset(((stop - self.min) / span + resistance / width) as f32);
    }
    fn begin(&mut self, n: f64, w: &mut Window, cx: &mut Context<Self>) {
        let high = self
            .value
            .high
            .is_some_and(|v| (v - n).abs() < (self.value.low - n).abs());
        self.drag = Some(high);
        self.last_thumb = usize::from(high);
        self.since_grab.restart(cx);
        self.samples.clear();
        let span = self.max - self.min;
        let visible = self.min + f64::from(self.position[self.last_thumb].value(cx)) * span;
        let width = f64::from(f32::from(self.track.get().size.width)).max(1.);
        let distance = (n - visible) / span * width;
        self.grab = if distance.abs() <= 11. {
            // A still-returning thumb can already be outside the rail. Invert the
            // resistance when grabbing it, so the first move starts under the pointer.
            let edge = self.stop(visible, high);
            let stretch = ((visible - edge) / span * width).clamp(-8.99, 8.99);
            let uncompressed = stretch / (0.55 * (1. - stretch.abs() / 9.));
            n - edge - uncompressed / width * span
        } else {
            0.
        };
        self.pointer = self.detent(n - self.grab);
        self.place_drag(self.pointer, high, cx);
        self.samples
            .push_back((0., self.position[self.last_thumb].value(cx)));
        self.since_move.restart(cx);
        self.focus[self.last_thumb].focus(w, cx);
        self.set(self.pointer, high, cx);
        cx.notify();
    }
    fn move_thumb(&mut self, n: f64, cx: &mut Context<Self>) {
        if let Some(high) = self.drag {
            let n = self.detent(n - self.grab);
            self.place_drag(n, high, cx);
            if self.samples.len() == 6 {
                self.samples.pop_front();
            }
            self.samples.push_back((
                self.since_grab.age(cx),
                self.position[usize::from(high)].value(cx),
            ));
            self.pointer = n;
            self.since_move.restart(cx);
            self.set(n, high, cx);
            cx.notify();
        }
    }
    fn end(&mut self, cancelled: bool, cx: &mut Context<Self>) {
        if let Some(high) = self.drag.take() {
            let coarse = f64::from(f32::from(self.track.get().size.width)) * self.step
                / (self.max - self.min)
                >= 12.;
            let velocity = match (self.samples.front(), self.samples.back()) {
                (Some((t0, p0)), Some((t1, p1)))
                    if t1 - t0 > 0.008 && self.since_move.age(cx) < 0.060 =>
                {
                    (p1 - p0) / (t1 - t0)
                }
                _ => 0.,
            };
            let span = self.max - self.min;
            if !cancelled && coarse {
                let projected = self.detent(
                    self.pointer
                        + (0.099 * f64::from(velocity) * span).clamp(-self.step, self.step),
                );
                self.set(projected, high, cx);
            }
            let next = if high {
                self.value.high.unwrap()
            } else {
                self.value.low
            };
            self.position[usize::from(high)].release(
                ((next - self.min) / span) as f32,
                velocity,
                0.26,
                cx,
            );
            self.linger.restart(cx);
            cx.notify()
        }
    }
    fn set(&mut self, n: f64, high: bool, cx: &mut Context<Self>) {
        let n =
            (self.min + ((n - self.min) / self.step).round() * self.step).clamp(self.min, self.max);
        let prev = self.value;
        if high {
            self.value.high = Some(n.max(self.value.low))
        } else {
            self.value.low = n.min(self.value.high.unwrap_or(self.max))
        }
        if self.value != prev {
            let old = if high {
                prev.high.unwrap_or(prev.low)
            } else {
                prev.low
            };
            let i = usize::from(high);
            self.direction[i] = if n >= old { 1. } else { -1. };
            self.last_thumb = i;
            cx.emit(self.value);
            cx.notify()
        }
    }
}
impl Render for Slider {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let f = |n: f64| ((n - self.min) / (self.max - self.min)) as f32;
        let linger = self.linger.time(0.70, w, cx) < 0.70;
        let mut fractions = [0.; 2];
        for (i, slot) in fractions.iter_mut().enumerate() {
            let n = if i == 0 {
                self.value.low
            } else {
                self.value.high.unwrap_or(self.value.low)
            };
            if self.drag != Some(i != 0) {
                self.position[i].duration(f(n), 0.26, cx);
            }
            *slot = self.position[i].get(w, cx);
        }
        let a = fractions[0];
        let b = self.value.high.map(|_| fractions[1]);
        let origin = if b.is_some() { a.clamp(0., 1.) } else { 0. };
        let end = b.unwrap_or(a).clamp(origin, 1.);
        let track = self.track.clone();
        let mut rail = div()
            .relative()
            .size_full()
            .child(
                div()
                    .absolute()
                    .top(px(14.))
                    .w_full()
                    .h(px(5.))
                    .rounded_full()
                    .bg(p.hover),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(origin))
                    .top(px(14.))
                    .w(relative(end - origin))
                    .h(px(5.))
                    .bg(p.bright)
                    .rounded_full(),
            );
        for &detent in &self.detents {
            rail = rail.child(
                div()
                    .absolute()
                    .left(relative(f(detent)))
                    .top(px(12.))
                    .w(px(2.))
                    .h(px(9.))
                    .bg(p.faint),
            );
        }
        for (i, &anchor) in fractions
            .iter()
            .enumerate()
            .take(if b.is_some() { 2 } else { 1 })
        {
            let high = i == 1;
            let value = if high {
                self.value.high.unwrap()
            } else {
                self.value.low
            };
            let n = self.label.clone();
            let active = self.drag == Some(high);
            let show = active
                || (self.last_thumb == i && linger)
                || self.focus[i].is_focused(w) && w.last_input_was_keyboard();
            self.bubble[i].duration(show as u8 as f32, if show { 0.26 } else { 0.16 }, cx);
            let lift = self.bubble[i].get(w, cx);
            let side = 22. * (1. + if active { 0.16 * lift } else { 0. });
            let label: SharedString = self.display(value).into();
            let bubble_label = self.labels[i].label(label.clone(), self.direction[i], w, cx);
            if lift > 0. {
                let width = measure(&label, 14., w) + 16.;
                let rail_width = f32::from(self.track.get().size.width).max(width);
                let center = (anchor * rail_width).clamp(width * 0.5, rail_width - width * 0.5);
                rail = rail.child(
                    div()
                        .absolute()
                        .left(px(center - width * 0.5))
                        .top(px(-31. + (1. - lift) * 5.))
                        .h(px(26.))
                        .w(px(width))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(crate::foundation::surface_radius(cx))
                        .bg(p.bright)
                        .text_color(p.inverse)
                        .opacity(lift)
                        .child(bubble_label),
                );
            }
            let thumb = div()
                .id(("thumb", i))
                .debug_selector(move || format!("Slider thumb {i}"))
                .role(Role::Slider)
                .aria_label(n)
                .aria_value(self.display(value))
                .aria_numeric_value(value)
                .aria_min_numeric_value(if high { self.value.low } else { self.min })
                .aria_max_numeric_value(if high {
                    self.max
                } else {
                    self.value.high.unwrap_or(self.max)
                })
                .aria_numeric_value_step(self.step)
                .track_focus(&self.focus[i])
                .absolute()
                .top(px(16. - side * 0.5))
                .left(relative(anchor))
                .ml(px(-side * 0.5))
                .size(px(side))
                .rounded_full()
                .bg(p.panel)
                .border_1()
                .border_color(p.faint)
                .shadow_sm()
                .cursor_pointer()
                .focus_visible(move |s| s.border_2().border_color(p.focus))
                .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                    let current = if high {
                        s.value.high.unwrap()
                    } else {
                        s.value.low
                    };
                    let delta = if e.keystroke.modifiers.shift { 10. } else { 1. };
                    let next = match e.keystroke.key.as_str() {
                        "left" | "down" => current - s.step * delta,
                        "right" | "up" => current + s.step * delta,
                        "home" => s.min,
                        "end" => s.max,
                        _ => return,
                    };
                    s.set(next, high, cx);
                    cx.stop_propagation();
                    w.prevent_default()
                }));
            rail = rail.child(thumb)
        }
        let value = if let Some(h) = self.value.high {
            format!("{} – {}", self.display(self.value.low), self.display(h))
        } else {
            self.display(self.value.low)
        };
        section(420., cx)
            .child(
                row()
                    .justify_between()
                    .child(self.label.clone())
                    .child(
                        self.readout
                            .label(value, self.direction[self.last_thumb], w, cx),
                    ),
            )
            .child(
                div()
                    .relative()
                    .px(px(11.))
                    .h(px(34.))
                    .on_children_prepainted(move |bs, _, _| {
                        if let Some(b) = bs.first() {
                            track.set(*b)
                        }
                    })
                    .id("track")
                    .debug_selector(|| "Slider track".into())
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|s, e: &MouseDownEvent, w, cx| {
                            let n = s.val(e.position.x);
                            s.begin(n, w, cx)
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|s, _, _, cx| s.end(false, cx)),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(|s, _, _, cx| s.end(false, cx)),
                    )
                    .child(rail)
                    .when(self.drag.is_some(), |track| {
                        let moved = cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                            if e.pressed_button == Some(MouseButton::Left) {
                                s.move_thumb(s.val(e.position.x), cx);
                            } else {
                                s.end(true, cx);
                            }
                        });
                        let released = cx.listener(|s, _: &MouseUpEvent, _, cx| s.end(false, cx));
                        track.child(
                            canvas(
                                |_, _, _| (),
                                move |_, _, w, _| {
                                    w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                                        if phase.capture() {
                                            moved(e, w, cx)
                                        }
                                    });
                                    w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                                        if phase.capture() && e.button == MouseButton::Left {
                                            released(e, w, cx)
                                        }
                                    });
                                },
                            )
                            .absolute()
                            .size(px(0.)),
                        )
                    })
                    .child(self.contact.listen(cx, |s, e, w, cx| {
                        let n = s.val(e.position.x);
                        match e.phase {
                            TouchPhase::Started => s.begin(n, w, cx),
                            TouchPhase::Moved => s.move_thumb(n, cx),
                            TouchPhase::Ended => s.end(false, cx),
                            TouchPhase::Cancelled => s.end(true, cx),
                        }
                    })),
            )
    }
}

#[cfg(test)]
mod detent_tests {
    use super::{SlideValue, Slider};
    use gpui::{Bounds, TestAppContext, point, px, size};

    #[test]
    fn detents_snap_only_near_valid_normalized_stops() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|_, cx| {
            Slider::new(
                "Value",
                0.0..=100.0,
                1.,
                SlideValue {
                    low: 0.,
                    high: None,
                },
                cx,
            )
        });
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                s.track
                    .set(Bounds::new(point(px(0.), px(0.)), size(px(200.), px(20.))));
                assert_eq!(s.detent(53.), 53.);
                s.set_detents(vec![50.4, f64::NAN, -1., 200.], cx);
                assert_eq!(s.detents, vec![50.]);
                assert_eq!(s.detent(53.), 50.);
                assert_eq!(s.detent(56.), 56.);
                assert_eq!(s.detent(101.), 101.);
            })
        });
    }
}
