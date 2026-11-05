//! A precise measurement control with a fixed needle, edge-faded moving ruler and rolling readout.
use crate::primitives::{
    family::{Paint, PhaseClock, Slide, disabled_motion},
    nav::HorizontalScrollRegion,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::VecDeque, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RulerChanged(pub f64);
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RulerCommitted(pub f64);
/// Drag the tape beneath its fixed needle, or focus it and use arrow keys.
pub struct RulerSlider {
    label: SharedString,
    unit: SharedString,
    min: f64,
    max: f64,
    step: f64,
    value: f64,
    wheel_value: f64,
    readout: DigitReadout,
    precision: usize,
    major_every: usize,
    spacing: f64,
    position: Slide,
    focus: FocusHandle,
    disabled: bool,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    touch: Rc<Cell<u8>>,
    drag: Option<(f32, f64, f64)>,
    samples: VecDeque<(f32, f32)>,
    clock: PhaseClock,
}
impl EventEmitter<RulerChanged> for RulerSlider {}
impl EventEmitter<RulerCommitted> for RulerSlider {}
impl RulerSlider {
    pub fn new(
        label: impl Into<SharedString>,
        range: std::ops::RangeInclusive<f64>,
        step: f64,
        value: f64,
        cx: &mut Context<Self>,
    ) -> Self {
        let (min, max) = (*range.start(), *range.end());
        let (min, max) =
            if min.is_finite() && max.is_finite() && min < max && (max - min).is_finite() {
                (min, max)
            } else {
                (0., 100.)
            };
        let step = if step.is_finite() && step > 0. {
            step
        } else {
            1.
        };
        let value = if value.is_finite() {
            value.clamp(min, max)
        } else {
            min
        };
        let value = (min + ((value - min) / step).round() * step).clamp(min, max);
        Self {
            label: label.into(),
            unit: "".into(),
            min,
            max,
            step,
            value,
            wheel_value: value,
            readout: DigitReadout::default(),
            precision: decimal_places(min).max(decimal_places(step)),
            major_every: 10,
            spacing: 10.,
            position: Slide::snappy(((value - min) / (max - min)) as f32),
            focus: cx.focus_handle().tab_stop(true),
            disabled: false,
            bounds: Default::default(),
            touch: Default::default(),
            drag: None,
            samples: VecDeque::new(),
            clock: Default::default(),
        }
    }
    pub fn with_unit(mut self, unit: impl Into<SharedString>) -> Self {
        self.unit = unit.into();
        self
    }
    pub fn with_major_every(mut self, count: usize) -> Self {
        self.major_every = count.max(1);
        self
    }
    pub fn with_spacing(mut self, spacing: f64) -> Self {
        if spacing.is_finite() && spacing >= 2. {
            self.spacing = spacing;
        }
        self
    }
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    fn set_value(&mut self, value: f64, cx: &mut Context<Self>) {
        if !value.is_finite() {
            return;
        }
        let value = (self.min + ((value - self.min) / self.step).round() * self.step)
            .clamp(self.min, self.max);
        if self.value != value {
            self.value = value;
            cx.emit(RulerChanged(value));
            cx.notify();
        }
    }
    fn begin(&mut self, x: Pixels, w: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let current = self.min + f64::from(self.position.value(cx)) * (self.max - self.min);
        self.drag = Some((x.into(), current, self.value));
        self.wheel_value = self.value;
        self.focus.focus(w, cx);
        self.clock.restart(cx);
        self.samples.clear();
        self.samples.push_back((0., current as f32));
        cx.notify();
    }
    fn drag_to(&mut self, x: Pixels, cx: &mut Context<Self>) {
        if let Some((start, value, _)) = self.drag {
            let x: f32 = x.into();
            let raw =
                (value - f64::from(x - start) / self.spacing * self.step).clamp(self.min, self.max);
            self.position
                .reset(((raw - self.min) / (self.max - self.min)) as f32);
            if self.samples.len() == 6 {
                self.samples.pop_front();
            }
            self.samples.push_back((self.clock.age(cx), raw as f32));
            self.set_value(raw, cx);
            cx.notify();
        }
    }
    fn end(&mut self, x: Pixels, cancelled: bool, cx: &mut Context<Self>) {
        let Some((start, start_value, original_value)) = self.drag.take() else {
            return;
        };
        let x: f32 = x.into();
        let click = (x - start).abs() < 4.;
        let raw = if cancelled {
            original_value
        } else if click {
            let b = self.bounds.get();
            let center: f32 = (b.left() + b.size.width * 0.5).into();
            start_value + f64::from(x - center) / self.spacing * self.step
        } else {
            self.min + f64::from(self.position.value(cx)) * (self.max - self.min)
        };
        let velocity = match (self.samples.front(), self.samples.back()) {
            (Some((t0, v0)), Some((t1, v1)))
                if t1 - t0 > 0.008 && self.clock.age(cx) - t1 < 0.08 =>
            {
                (v1 - v0) / (t1 - t0)
            }
            _ => 0.,
        };
        let momentum = if disabled_motion(cx) || click || cancelled {
            0.
        } else {
            (f64::from(velocity) * 0.1).clamp(-self.step * 10., self.step * 10.)
        };
        self.set_value(raw + momentum, cx);
        self.wheel_value = self.value;
        self.position.release(
            ((self.value - self.min) / (self.max - self.min)) as f32,
            if cancelled {
                0.
            } else {
                velocity / (self.max - self.min) as f32
            },
            0.5,
            cx,
        );
        if !cancelled {
            cx.emit(RulerCommitted(self.value));
        }
        cx.notify();
    }
}
impl Render for RulerSlider {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let current = self.min + f64::from(self.position.get(w, cx)) * (self.max - self.min);
        let bounds = self.bounds.clone();
        let min = self.min;
        let max = self.max;
        let step = self.step;
        let spacing = self.spacing;
        let major = self.major_every;
        let ruler = canvas(
            move |b, _, _| bounds.set(b),
            move |b, _, w, cx| {
                if let Some(host) = cx.try_global::<HorizontalScrollRegion>() {
                    (host.0)(b, true, true);
                }
                let width: f32 = b.size.width.into();
                let fade_zone = (width * 0.22).max(16.);
                let left = (current - (f64::from(width) * 0.5 / spacing + 2.) * step).max(min);
                let right = (current + (f64::from(width) * 0.5 / spacing + 2.) * step).min(max);
                let start = ((left - min) / step).ceil().max(0.) as usize;
                let end = ((right - min) / step).floor().max(0.) as usize;
                for i in start..=end.min(start.saturating_add(2048)) {
                    let val = min + i as f64 * step;
                    let x = width * 0.5 + ((val - current) / step * spacing) as f32;
                    let edge_alpha = (x / fade_zone).min((width - x) / fade_zone).clamp(0., 1.);
                    if edge_alpha <= 0.01 {
                        continue;
                    }
                    let large = i % major == 0;
                    let y = if large {
                        28.
                    } else if i % (major / 2).max(1) == 0 {
                        20.
                    } else {
                        12.
                    };
                    let mut path = PathBuilder::stroke(px(1.));
                    path.move_to(b.origin + point(px(x), px(0.)));
                    path.line_to(b.origin + point(px(x), px(y)));
                    if let Ok(path) = path.build() {
                        let base_col = if large {
                            p.ink.opacity(0.48)
                        } else {
                            p.ink.opacity(0.20)
                        };
                        w.paint_path(path, base_col.opacity(base_col.a * edge_alpha));
                    }
                    if large {
                        let text: SharedString = format!("{val}").into();
                        let mut style = w.text_style();
                        style.color = p.faint.opacity(edge_alpha);
                        let line = w.text_system().shape_line(
                            text.clone(),
                            px(11.),
                            &[style.to_run(text.len())],
                            None,
                        );
                        let _ = line.paint(
                            b.origin + point(px(x) - line.width * 0.5, px(36.)),
                            px(14.),
                            TextAlign::Left,
                            None,
                            w,
                            cx,
                        );
                    }
                }
                w.paint_quad(
                    fill(
                        Bounds::new(
                            b.origin + point(px(width * 0.5 - 1.), px(0.)),
                            size(px(2.), px(36.)),
                        ),
                        p.bright,
                    )
                    .corner_radii(Corners {
                        top_left: px(0.),
                        top_right: px(0.),
                        bottom_left: px(2.),
                        bottom_right: px(2.),
                    }),
                );
            },
        )
        .absolute()
        .inset_0();
        let mut tape = div()
            .id("ruler-slider")
            .relative()
            .h(px(72.))
            .w_full()
            .rounded(radius)
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .overflow_hidden()
            .role(Role::Slider)
            .aria_label(self.label.clone())
            .aria_value(format!("{} {}", self.value, self.unit))
            .aria_numeric_value(self.value)
            .aria_min_numeric_value(self.min)
            .aria_max_numeric_value(self.max)
            .aria_numeric_value_step(self.step)
            .track_focus(&self.focus)
            .tab_index(if self.disabled { -1 } else { 0 })
            .focus_visible(move |s| s.border_color(p.focus))
            .child(ruler);
        if !self.disabled {
            tape = tape
                .on_scroll(cx.listener(|s, e: &ScrollEvent, w, cx| {
                    let delta = e.delta.pixel_delta(px(18.));
                    let dx = f64::from(f32::from(delta.x));
                    let dy = f64::from(f32::from(delta.y));
                    if dx.abs() <= dy.abs() || dx.abs() < 0.01 {
                        return;
                    }
                    let target = (s.wheel_value - dx / s.spacing * s.step).clamp(s.min, s.max);
                    s.wheel_value = target;
                    s.set_value(target, cx);
                    s.position
                        .duration(((s.value - s.min) / (s.max - s.min)) as f32, 0.26, cx);
                    cx.emit(RulerCommitted(s.value));
                    cx.stop_propagation();
                    w.prevent_default();
                }))
                .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                    let big = e.keystroke.modifiers.shift;
                    let delta = if big {
                        s.step * s.major_every as f64
                    } else {
                        s.step
                    };
                    let target = match e.keystroke.key.as_str() {
                        "left" | "down" => s.value - delta,
                        "right" | "up" => s.value + delta,
                        "pageup" => s.value + s.step * s.major_every as f64,
                        "pagedown" => s.value - s.step * s.major_every as f64,
                        "home" => s.min,
                        "end" => s.max,
                        _ => return,
                    };
                    s.set_value(target, cx);
                    s.wheel_value = s.value;
                    s.position
                        .duration(((s.value - s.min) / (s.max - s.min)) as f32, 0.26, cx);
                    cx.emit(RulerCommitted(s.value));
                    cx.stop_propagation();
                    w.prevent_default();
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|s, e: &MouseDownEvent, w, cx| s.begin(e.position.x, w, cx)),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|s, e: &MouseUpEvent, _, cx| s.end(e.position.x, false, cx)),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|s, e: &MouseUpEvent, _, cx| s.end(e.position.x, false, cx)),
                )
                .when(self.drag.is_some(), |tape| {
                    let moved = cx.listener(|s, e: &MouseMoveEvent, _, cx| {
                        if e.pressed_button == Some(MouseButton::Left) {
                            s.drag_to(e.position.x, cx);
                        }
                    });
                    let released =
                        cx.listener(|s, e: &MouseUpEvent, _, cx| s.end(e.position.x, false, cx));
                    tape.child(
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
                .child({
                    let touch = self.touch.clone();
                    let gesture = cx.listener(|s, e: &TouchDragEvent, w, cx| match e.phase {
                        TouchPhase::Started => s.begin(e.start_position.x, w, cx),
                        TouchPhase::Moved => s.drag_to(e.position.x, cx),
                        TouchPhase::Ended => s.end(e.position.x, false, cx),
                        TouchPhase::Cancelled => s.end(e.position.x, true, cx),
                    });
                    canvas(
                        |bounds, w, _| w.insert_hitbox(bounds, HitboxBehavior::Normal),
                        move |_, hit, w, _| {
                            w.on_mouse_event(move |e: &TouchDragEvent, phase, w, cx| {
                                if !phase.bubble() {
                                    return;
                                }
                                if e.phase == TouchPhase::Started {
                                    touch.set(u8::from(hit.is_hovered(w)));
                                    return;
                                }
                                if touch.get() == 1 && e.phase == TouchPhase::Moved {
                                    let dx = f32::from((e.position.x - e.start_position.x).abs());
                                    let dy = f32::from((e.position.y - e.start_position.y).abs());
                                    if dy > 4. && dy >= dx {
                                        touch.set(0);
                                        return;
                                    }
                                    if dx > 4. && dx > dy {
                                        touch.set(2);
                                        let mut start = e.clone();
                                        start.phase = TouchPhase::Started;
                                        gesture(&start, w, cx);
                                    }
                                }
                                if touch.get() == 2 {
                                    w.prevent_default();
                                    gesture(e, w, cx);
                                    cx.stop_propagation();
                                } else if touch.get() == 1 && e.phase == TouchPhase::Ended {
                                    let dx = f32::from((e.position.x - e.start_position.x).abs());
                                    let dy = f32::from((e.position.y - e.start_position.y).abs());
                                    if dx < 4. && dy < 4. {
                                        let mut start = e.clone();
                                        start.phase = TouchPhase::Started;
                                        gesture(&start, w, cx);
                                        gesture(e, w, cx);
                                        w.prevent_default();
                                        cx.stop_propagation();
                                    }
                                }
                                if matches!(e.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                                    touch.set(0);
                                }
                            });
                        },
                    )
                    .absolute()
                    .inset_0()
                });
        }
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(4.))
            .w_full()
            .max_w_full()
            .text_color(p.ink)
            .when(self.disabled, |e| e.opacity(0.5))
            .child(
                div()
                    .text_size(px(14.))
                    .text_color(p.secondary)
                    .child(self.label.clone()),
            )
            .child(
                div()
                    .id("ruler-readout")
                    .debug_selector(|| "ruler-readout".into())
                    .mb(px(12.))
                    .flex()
                    .items_baseline()
                    .gap(px(8.4))
                    .child(self.readout.label(
                        format!("{:.*}", self.precision, self.value).into(),
                        w,
                        cx,
                    ))
                    .child(
                        div()
                            .id("ruler-unit")
                            .debug_selector(|| "ruler-unit".into())
                            .text_size(px(16.))
                            .text_color(p.secondary)
                            .child(self.unit.clone()),
                    ),
            )
            .child(tape)
    }
}

fn decimal_places(value: f64) -> usize {
    let text = value.to_string();
    let (mantissa, exponent) = text
        .split_once('e')
        .map_or((text.as_str(), 0), |(base, exp)| {
            (base, exp.parse::<i32>().unwrap_or(0))
        });
    let fractional = mantissa.split_once('.').map_or(0, |(_, part)| part.len());
    (fractional as i32 - exponent).clamp(0, 15) as usize
}

struct DigitColumn {
    character: char,
    position: Slide,
    presence: Slide,
}
#[derive(Default)]
struct DigitReadout {
    current: Option<SharedString>,
    // Columns keep their identity from the right. A carry adds one slot without
    // replacing the neighboring glyphs or changing their baseline.
    columns: Vec<DigitColumn>,
}
impl DigitReadout {
    fn label(&mut self, next: SharedString, window: &Window, cx: &App) -> Stateful<Div> {
        if self.current.as_ref() != Some(&next) {
            let first = self.current.is_none();
            let chars: Vec<_> = next.chars().rev().collect();
            while self.columns.len() < chars.len() {
                let character = chars[self.columns.len()];
                self.columns.push(DigitColumn {
                    character,
                    position: Slide::new(character.to_digit(10).unwrap_or(0) as f32),
                    presence: Slide::new(if first { 1. } else { 0. }),
                });
            }
            for (index, column) in self.columns.iter_mut().enumerate() {
                let Some(&character) = chars.get(index) else {
                    column.presence.duration(0., 0.4, cx);
                    continue;
                };
                column.character = character;
                column.presence.duration(1., 0.4, cx);
                if let Some(digit) = character.to_digit(10) {
                    column.position.duration(digit as f32, 0.24, cx);
                }
            }
            self.current = Some(next);
        }
        let features = FontFeatures(vec![("tnum".into(), 1)].into());
        let mut style = window.text_style();
        style.font_weight = FontWeight::MEDIUM;
        style.font_features = features.clone();
        let advance = |text: SharedString| {
            window
                .text_system()
                .shape_line(text.clone(), px(28.), &[style.to_run(text.len())], None)
                .width
        };
        let digit_width = advance("0".into());
        let mut row = div()
            .id("ruler-digits")
            .debug_selector(|| "ruler-digits".into())
            .flex()
            .items_baseline()
            .h(px(34.))
            .line_height(px(34.))
            .text_size(px(28.))
            .font_weight(FontWeight::MEDIUM)
            .font_features(features);
        for (index, column) in self.columns.iter_mut().enumerate().rev() {
            let present = column.presence.get(window, cx).clamp(0., 1.);
            if present < 0.0001 {
                continue;
            }
            let digit = column.character.is_ascii_digit();
            let width = if digit {
                digit_width
            } else {
                advance(column.character.to_string().into())
            };
            let mut cell = div()
                .id(("ruler-digit", index as u64))
                .debug_selector(move || format!("ruler-digit-{index}"))
                .relative()
                .w(width * present)
                .h(px(34.))
                .flex_none()
                .overflow_hidden()
                .opacity(present)
                // Keep an in-flow text baseline in every frame, including rest.
                .child(div().opacity(0.).child("0"));
            if digit {
                let position = column.position.get(window, cx).clamp(0., 9.);
                let first = position.floor() as u32;
                for value in first..=(first + 1).min(9) {
                    cell = cell.child(
                        div()
                            .absolute()
                            .top(px((value as f32 - position) * 34.))
                            .left_0()
                            .w(width)
                            .text_center()
                            .child(value.to_string()),
                    );
                }
            } else {
                cell = cell.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .child(column.character.to_string()),
                );
            }
            row = row.child(cell);
        }
        row
    }
}

#[cfg(test)]
mod tests {
    use super::{RulerCommitted, RulerSlider};
    use gpui::{TestAppContext, VisualTestContext, px};
    use std::{cell::Cell, rc::Rc};

    fn tick(w: &mut VisualTestContext, ms: u64) {
        w.executor()
            .advance_clock(std::time::Duration::from_millis(ms));
        w.run_until_parked();
        w.update(|win, cx| {
            win.simulate_next_frame(cx);
            win.refresh();
        });
        w.run_until_parked();
    }

    #[test]
    fn numeric_readouts_keep_their_slots_and_baseline_during_fast_updates() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) = app.add_window_view(|_, cx| {
            RulerSlider::new("Weight", 35.0..=140.0, 0.5, 68.5, cx).with_unit("kg")
        });
        tick(w, 0);
        let unit = w.debug_bounds("ruler-unit").unwrap();
        let digits = w.debug_bounds("ruler-digits").unwrap();
        let slot = w.debug_bounds("ruler-digit-0").unwrap();
        for value in [69., 69.5, 70., 70.5, 71., 70.5, 70., 69.5] {
            control.update(w, |s, cx| s.set_value(value, cx));
            tick(w, 16);
            for elapsed in [0, 16, 49] {
                tick(w, elapsed);
                assert_eq!(w.debug_bounds("ruler-unit").unwrap(), unit);
                assert_eq!(w.debug_bounds("ruler-digits").unwrap(), digits);
                assert_eq!(w.debug_bounds("ruler-digit-0").unwrap(), slot);
            }
        }
        tick(w, 1000);
        assert_eq!(w.debug_bounds("ruler-unit").unwrap(), unit);
        assert_eq!(w.debug_bounds("ruler-digits").unwrap(), digits);
        control.update(w, |s, cx| s.set_value(99.5, cx));
        tick(w, 1000);
        let before = w.debug_bounds("ruler-digits").unwrap();
        control.update(w, |s, cx| s.set_value(100., cx));
        tick(w, 0);
        let start = w.debug_bounds("ruler-digits").unwrap();
        assert_eq!(start.size.width, before.size.width, "carry begins in place");
        tick(w, 50);
        let during = w.debug_bounds("ruler-digits").unwrap();
        tick(w, 1000);
        let settled = w.debug_bounds("ruler-digits").unwrap();
        assert!(before.size.width < during.size.width);
        assert!(during.size.width < settled.size.width);
        assert_eq!(settled.top(), before.top());
        assert_eq!(w.debug_bounds("ruler-unit").unwrap().top(), unit.top());
    }

    #[test]
    fn cancelled_drag_restores_original_value_without_commit() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) =
            app.add_window_view(|_, cx| RulerSlider::new("Height", 0.0..=100.0, 1., 50., cx));
        let commits = Rc::new(Cell::new(0));
        let events = commits.clone();
        let _sub = w.update(|_, cx| {
            cx.subscribe(&control, move |_, _: &RulerCommitted, _| {
                events.set(events.get() + 1)
            })
        });
        w.update(|w, cx| {
            control.update(cx, |s, cx| {
                s.begin(px(100.), w, cx);
                s.drag_to(px(40.), cx);
                assert_ne!(s.value(), 50.);
                s.end(px(40.), true, cx);
                assert_eq!(s.value(), 50.);
                assert!(s.drag.is_none());
            })
        });
        assert_eq!(commits.get(), 0);
    }
}
