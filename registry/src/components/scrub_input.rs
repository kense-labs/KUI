//! Compact numeric input with a draggable label, progress fill wash and editable arithmetic expressions.
use crate::primitives::{
    editor::{EditEvent, Editor, InputMode},
    family::{Paint, Slide, disabled_motion},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrubChanged(pub f64);
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrubCommitted(pub f64);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScrubUnitChanged(pub SharedString);
/// Drag the label horizontally, or edit a number or arithmetic expression.
pub struct ScrubInput {
    label: SharedString,
    unit: SharedString,
    units: Vec<SharedString>,
    min: f64,
    max: f64,
    step: f64,
    value: f64,
    default_value: f64,
    editor: Entity<Editor>,
    _subscription: Subscription,
    drag: Option<(f32, f64)>,
    drag_factor: f64,
    strain: Slide,
    wrap: bool,
    touch: Rc<Cell<u8>>,
    disabled: bool,
}
impl EventEmitter<ScrubChanged> for ScrubInput {}
impl EventEmitter<ScrubCommitted> for ScrubInput {}
impl EventEmitter<ScrubUnitChanged> for ScrubInput {}
impl ScrubInput {
    pub fn new(
        label: impl Into<SharedString>,
        range: std::ops::RangeInclusive<f64>,
        step: f64,
        value: f64,
        unit: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (min, max) = (*range.start(), *range.end());
        let (min, max) =
            if min.is_finite() && max.is_finite() && min <= max && (max - min).is_finite() {
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
        let label = label.into();
        let unit = unit.into();
        let editor = cx.new(|cx| {
            let mut editor = Editor::new(label.clone(), InputMode::Single, window, cx);
            editor.set_value(value.to_string(), window, cx);
            editor
        });
        let subscription = cx.subscribe_in(
            &editor,
            window,
            |s: &mut Self, editor, event: &EditEvent, w, cx| match event {
                EditEvent::Submitted | EditEvent::Blurred => {
                    if s.disabled || editor.read(cx).is_composing() {
                        return;
                    }
                    let raw = editor.read(cx).value();
                    let text = raw.trim();
                    let mut matching_unit = None;
                    let mut expression = text;
                    for unit in &s.units {
                        if !unit.is_empty()
                            && let Some(prefix) = text.strip_suffix(unit.as_ref())
                        {
                            expression = prefix.trim();
                            matching_unit = Some(unit.clone());
                            break;
                        }
                        if unit.as_ref() == "°"
                            && let Some(prefix) = text.strip_suffix("deg")
                        {
                            expression = prefix.trim();
                            matching_unit = Some(unit.clone());
                            break;
                        }
                    }
                    let parsed = parse_expression(&expression.replace(',', "."));
                    if parsed.is_some()
                        && let Some(unit) = matching_unit
                        && unit != s.unit
                    {
                        s.unit = unit.clone();
                        cx.emit(ScrubUnitChanged(unit));
                    }
                    let value = parsed.unwrap_or(s.value);
                    s.set(value, w, cx);
                    cx.emit(ScrubCommitted(s.value));
                }
                EditEvent::Escape => {
                    s.write_editor(w, cx);
                }
                EditEvent::Navigate(down) => {
                    s.set(s.value + if *down { -s.step } else { s.step }, w, cx);
                    cx.emit(ScrubCommitted(s.value));
                }
                _ => {}
            },
        );
        Self {
            label,
            units: vec![unit.clone()],
            unit,
            min,
            max,
            step,
            value,
            default_value: value,
            editor,
            _subscription: subscription,
            drag: None,
            drag_factor: 1.,
            strain: Slide::snappy(0.),
            wrap: false,
            touch: Default::default(),
            disabled: false,
        }
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn unit(&self) -> &str {
        self.unit.as_ref()
    }
    pub fn set_value(&mut self, value: f64, w: &mut Window, cx: &mut Context<Self>) {
        self.set(value, w, cx);
    }
    /// Wrap around the bounds, useful for angles.
    pub fn with_wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }
    /// Set accepted units. Cycling changes the unit label, not the numeric value.
    pub fn with_units(mut self, units: Vec<SharedString>) -> Self {
        if !units.is_empty() {
            if !units.contains(&self.unit) {
                self.unit = units[0].clone();
            }
            self.units = units;
        }
        self
    }
    fn cycle_unit(&mut self, cx: &mut Context<Self>) {
        if self.disabled || self.units.len() < 2 {
            return;
        }
        let current = self
            .units
            .iter()
            .position(|unit| unit == &self.unit)
            .unwrap_or(0);
        self.unit = self.units[(current + 1) % self.units.len()].clone();
        cx.emit(ScrubUnitChanged(self.unit.clone()));
        cx.notify();
    }
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        if disabled {
            self.finish(true, cx);
            self.touch.set(0);
        }
        self.disabled = disabled;
        self.editor.update(cx, |e, _| e.disabled = disabled);
        cx.notify()
    }
    fn write_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |e, cx| e.set_value(self.value.to_string(), w, cx));
        cx.notify();
    }
    fn set(&mut self, value: f64, w: &mut Window, cx: &mut Context<Self>) {
        self.set_at_step(value, self.step, w, cx);
    }
    fn set_at_step(&mut self, value: f64, step: f64, w: &mut Window, cx: &mut Context<Self>) {
        let value = if value.is_finite() {
            let value = if self.wrap && self.max > self.min {
                self.min + (value - self.min).rem_euclid(self.max - self.min)
            } else {
                value.clamp(self.min, self.max)
            };
            let snapped =
                (self.min + ((value - self.min) / step).round() * step).clamp(self.min, self.max);
            let snapped = if self.wrap && self.max > self.min && snapped >= self.max {
                self.min
            } else {
                snapped
            };
            if snapped.abs() < 1e100 {
                (snapped * 1e8).round() / 1e8
            } else {
                snapped
            }
        } else {
            self.value
        };
        if value != self.value {
            self.value = value;
            cx.emit(ScrubChanged(value));
        }
        self.write_editor(w, cx);
    }
    fn begin(&mut self, x: Pixels, cx: &mut Context<Self>) {
        if !self.disabled {
            self.drag = Some((x.into(), self.value));
            self.drag_factor = 1.;
            cx.notify()
        }
    }
    fn drag_to(&mut self, x: Pixels, factor: f64, w: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        if let Some((previous, value)) = self.drag {
            let x: f32 = x.into();
            let raw = value + f64::from(x - previous) * self.step * factor / 2.;
            self.drag = Some((x, raw));
            self.drag_factor = factor;
            if !self.wrap && !disabled_motion(cx) {
                let overflow = (raw - raw.clamp(self.min, self.max)) / self.step;
                self.strain
                    .reset((overflow.signum() * 12. * (1. - (-overflow.abs() / 10.).exp())) as f32);
            }
            self.set_at_step(raw, self.step * factor, w, cx)
        }
    }
    fn finish(&mut self, cancelled: bool, cx: &mut Context<Self>) {
        if self.drag.take().is_some() {
            self.drag_factor = 1.;
            self.strain.duration(0., 0.34, cx);
            if !cancelled {
                cx.emit(ScrubCommitted(self.value));
            }
            cx.notify()
        }
    }
}
impl Render for ScrubInput {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let strain = self.strain.get(w, cx);
        let editor_focus = self.editor.read(cx).focus.clone();
        let focused = editor_focus.is_focused(w);
        let scrubbing = self.drag.is_some();
        let fill_fraction = if self.max > self.min {
            ((self.value - self.min) / (self.max - self.min)).clamp(0., 1.) as f32
        } else {
            0.
        };
        let modified_step = cx.listener(|s, event: &KeyDownEvent, w, cx| {
            if s.disabled || s.editor.read(cx).is_composing() {
                return;
            }
            let factor = if event.keystroke.modifiers.shift {
                10.
            } else if event.keystroke.modifiers.alt {
                0.1
            } else {
                1.
            };
            let delta = s.step
                * factor
                * if event.keystroke.key == "down" {
                    -1.
                } else {
                    1.
                };
            s.set_at_step(s.value + delta, s.step * factor, w, cx);
            cx.emit(ScrubCommitted(s.value));
            cx.stop_propagation();
            w.prevent_default();
        });
        let label = self.label.clone();
        let touch_state = self.touch.clone();
        let touch = cx.listener(|s, e: &TouchDragEvent, w, cx| match e.phase {
            TouchPhase::Started => s.begin(e.start_position.x, cx),
            TouchPhase::Moved => s.drag_to(e.position.x, 1., w, cx),
            TouchPhase::Ended => s.finish(false, cx),
            TouchPhase::Cancelled => s.finish(true, cx),
        });
        let mut handle = div()
            .id("scrub-handle")
            .relative()
            .h_full()
            .min_w(px(30.))
            .pl(px(10.))
            .pr(px(4.))
            .flex()
            .items_center()
            .justify_center()
            .font_weight(FontWeight::MEDIUM)
            .text_color(if scrubbing { p.ink } else { p.secondary })
            .cursor_ew_resize()
            .child(label);
        if !self.disabled {
            handle = handle
                .hover(move |s| s.text_color(p.ink))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|s, e: &MouseDownEvent, _, cx| s.begin(e.position.x, cx)),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|s, _, _, cx| s.finish(false, cx)),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|s, _, _, cx| s.finish(false, cx)),
                )
                .on_click(cx.listener(|s, e: &ClickEvent, w, cx| {
                    if e.click_count() >= 2 {
                        s.set(s.default_value, w, cx);
                        cx.emit(ScrubCommitted(s.value));
                    } else {
                        let focus = s.editor.read(cx).focus.clone();
                        focus.focus(w, cx);
                    }
                }))
                .child(
                    canvas(
                        |b, w, _| w.insert_hitbox(b, HitboxBehavior::Normal),
                        move |_, hit, w, _| {
                            w.on_mouse_event(move |e: &TouchDragEvent, phase, w, cx| {
                                if !phase.bubble() {
                                    return;
                                }
                                if e.phase == TouchPhase::Started {
                                    touch_state.set(u8::from(hit.is_hovered(w)));
                                    return;
                                }
                                if touch_state.get() == 1 && e.phase == TouchPhase::Moved {
                                    let dx = f32::from((e.position.x - e.start_position.x).abs());
                                    let dy = f32::from((e.position.y - e.start_position.y).abs());
                                    if dy > 4. && dy >= dx {
                                        touch_state.set(0);
                                        return;
                                    }
                                    if dx > 4. && dx > dy {
                                        touch_state.set(2);
                                        let mut start = e.clone();
                                        start.phase = TouchPhase::Started;
                                        touch(&start, w, cx);
                                    }
                                }
                                if touch_state.get() == 2 {
                                    w.prevent_default();
                                    touch(e, w, cx);
                                    cx.stop_propagation();
                                }
                                if matches!(e.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                                    touch_state.set(0);
                                }
                            });
                        },
                    )
                    .absolute()
                    .inset_0(),
                )
                .when(self.drag.is_some(), |handle| {
                    let moved = cx.listener(|s, e: &MouseMoveEvent, w, cx| {
                        if e.pressed_button == Some(MouseButton::Left) {
                            let factor = if e.modifiers.shift {
                                10.
                            } else if e.modifiers.alt {
                                0.1
                            } else {
                                1.
                            };
                            s.drag_to(e.position.x, factor, w, cx)
                        }
                    });
                    let released = cx.listener(|s, _: &MouseUpEvent, _, cx| s.finish(false, cx));
                    handle.child(
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
                                        released(e, w, cx);
                                    }
                                });
                            },
                        )
                        .absolute()
                        .size(px(0.)),
                    )
                });
        }
        let mult_badge = if scrubbing && (self.drag_factor - 1.).abs() > 0.01 {
            Some(if self.drag_factor > 1. {
                "10×"
            } else {
                "0.1×"
            })
        } else {
            None
        };
        div()
            .relative()
            .left(px(strain))
            .flex()
            .items_center()
            .h(px(36.))
            .w_full()
            .rounded(radius)
            .border_1()
            .border_color(if focused || scrubbing {
                p.faint
            } else {
                p.line.opacity(0.)
            })
            .bg(if focused { p.panel } else { p.tray })
            .text_size(px(14.))
            .text_color(p.ink)
            .when(self.disabled, |e| e.opacity(0.5))
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .rounded(radius)
                    .overflow_hidden()
                    .child(
                        div()
                            .h_full()
                            .w(relative(fill_fraction))
                            .bg(p.ink.opacity(if scrubbing { 0.10 } else { 0.06 })),
                    ),
            )
            .when_some(mult_badge, |d, badge| {
                d.child(
                    div()
                        .absolute()
                        .bottom(px(42.))
                        .left(px(6.))
                        .px(px(7.))
                        .py(px(2.))
                        .rounded_full()
                        .bg(p.ink)
                        .text_color(p.panel)
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .child(badge),
                )
            })
            .capture_key_down(move |event: &KeyDownEvent, window, cx| {
                if editor_focus.is_focused(window)
                    && matches!(event.keystroke.key.as_str(), "up" | "down")
                    && !event.keystroke.modifiers.platform
                    && !event.keystroke.modifiers.control
                {
                    modified_step(event, window, cx);
                }
            })
            .child(handle)
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .child(self.editor.clone()),
            )
            .child(if self.units.len() > 1 && !self.disabled {
                div()
                    .id("scrub-unit")
                    .relative()
                    .mr(px(10.))
                    .min_w(px(18.))
                    .px(px(4.))
                    .h(px(22.))
                    .rounded(radius)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                    .role(Role::Button)
                    .aria_label(format!("{}: {}", tr_unit(cx), self.unit))
                    .tab_index(0)
                    .cursor_pointer()
                    .child(self.unit.clone())
                    .on_click(cx.listener(|s, _, _, cx| s.cycle_unit(cx)))
                    .into_any_element()
            } else {
                div()
                    .relative()
                    .px(px(10.))
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(self.unit.clone())
                    .into_any_element()
            })
    }
}
fn tr_unit(cx: &App) -> &'static str {
    crate::primitives::control::UiLanguage::current(cx).text("Change unit", "切换单位")
}
/// Parse finite arithmetic values, without evaluation of code or external units.
pub fn parse_expression(text: &str) -> Option<f64> {
    if text.len() > 128 {
        return None;
    }
    struct Parser<'a> {
        text: &'a [u8],
        i: usize,
        depth: usize,
    }
    impl Parser<'_> {
        fn ws(&mut self) {
            while self.text.get(self.i).is_some_and(u8::is_ascii_whitespace) {
                self.i += 1;
            }
        }
        fn eat(&mut self, c: u8) -> bool {
            self.ws();
            if self.text.get(self.i) == Some(&c) {
                self.i += 1;
                true
            } else {
                false
            }
        }
        fn number(&mut self) -> Option<f64> {
            self.ws();
            let start = self.i;
            let mut digits = 0;
            while let Some(&c) = self.text.get(self.i) {
                if c.is_ascii_digit() {
                    digits += 1;
                    self.i += 1
                } else if c == b'.' {
                    self.i += 1
                } else {
                    break;
                }
            }
            if digits == 0 {
                return None;
            }
            std::str::from_utf8(&self.text[start..self.i])
                .ok()?
                .parse()
                .ok()
        }
        fn atom(&mut self) -> Option<f64> {
            if self.eat(b'+') {
                return self.atom();
            }
            if self.eat(b'-') {
                return Some(-self.atom()?);
            }
            if self.eat(b'(') {
                self.depth += 1;
                if self.depth > 32 {
                    return None;
                }
                let n = self.expr()?;
                if !self.eat(b')') {
                    return None;
                }
                self.depth -= 1;
                Some(n)
            } else {
                self.number()
            }
        }
        fn term(&mut self) -> Option<f64> {
            let mut n = self.atom()?;
            loop {
                if self.eat(b'*') {
                    n *= self.atom()?
                } else if self.eat(b'/') {
                    let x = self.atom()?;
                    if x == 0. {
                        return None;
                    }
                    n /= x
                } else {
                    return Some(n);
                }
            }
        }
        fn expr(&mut self) -> Option<f64> {
            let mut n = self.term()?;
            loop {
                if self.eat(b'+') {
                    n += self.term()?
                } else if self.eat(b'-') {
                    n -= self.term()?
                } else {
                    return Some(n);
                }
            }
        }
    }
    let mut p = Parser {
        text: text.as_bytes(),
        i: 0,
        depth: 0,
    };
    let n = p.expr()?;
    p.ws();
    (p.i == p.text.len() && n.is_finite()).then_some(n)
}
#[cfg(test)]
mod tests {
    use super::{ScrubCommitted, ScrubInput, parse_expression};
    use gpui::{EntityInputHandler, TestAppContext, px};
    use std::{cell::Cell, rc::Rc};
    #[test]
    fn arithmetic_is_bounded_and_respects_precedence() {
        assert_eq!(parse_expression("(2 + 3) * -4"), Some(-20.));
        assert_eq!(parse_expression("2+3*4"), Some(14.));
        assert_eq!(parse_expression("1/0"), None);
        assert_eq!(parse_expression("2px"), None);
        assert_eq!(parse_expression("1..2"), None);
    }
    #[test]
    fn keyboard_step_obeys_composition_and_disabled_state() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) =
            app.add_window_view(|w, cx| ScrubInput::new("Value", 0.0..=100.0, 1., 10., "", w, cx));
        w.update(|w, cx| {
            let editor = control.read(cx).editor.clone();
            editor.update(cx, |e, cx| e.focus(w, cx));
        });
        w.update(|w, cx| {
            w.simulate_next_frame(cx);
            w.refresh();
        });
        w.simulate_keystrokes("up");
        assert_eq!(w.update(|_, cx| control.read(cx).value()), 11.);
        w.update(|w, cx| {
            let editor = control.read(cx).editor.clone();
            editor.update(cx, |e, cx| {
                e.replace_and_mark_text_in_range(None, "ni", Some(2..2), w, cx)
            })
        });
        w.simulate_keystrokes("shift-up");
        assert_eq!(w.update(|_, cx| control.read(cx).value()), 11.);
        w.update(|_, cx| control.update(cx, |s, cx| s.set_disabled(true, cx)));
        w.simulate_keystrokes("up");
        assert_eq!(w.update(|_, cx| control.read(cx).value()), 11.);
    }

    #[test]
    fn cancelled_or_disabled_drag_never_commits() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) =
            app.add_window_view(|w, cx| ScrubInput::new("Value", 0.0..=100.0, 1., 10., "", w, cx));
        let count = Rc::new(Cell::new(0));
        let count_events = count.clone();
        let _sub = w.update(|_, cx| {
            cx.subscribe(&control, move |_, _: &ScrubCommitted, _| {
                count_events.set(count_events.get() + 1)
            })
        });
        w.update(|w, cx| {
            control.update(cx, |s, cx| {
                s.begin(px(0.), cx);
                s.drag_to(px(20.), 1., w, cx);
                s.finish(true, cx);
                assert!(s.drag.is_none());
                s.begin(px(0.), cx);
                s.set_disabled(true, cx);
                assert!(s.drag.is_none());
            })
        });
        assert_eq!(count.get(), 0);
    }
}
