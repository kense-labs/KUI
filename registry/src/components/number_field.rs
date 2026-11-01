//! Bounded numeric draft with two steppers, direct entry and keyboard increments.
use crate::primitives::{
    editor::{EditEvent, Editor, InputMode},
    family::{Paint, PhaseClock},
    form_chrome::*,
    glyph::Glyph,
};
use gpui::{prelude::*, *};
pub struct NumberField {
    label: SharedString,
    min: f64,
    max: f64,
    step: f64,
    value: f64,
    suffix: SharedString,
    help: SharedString,
    edit: Entity<Editor>,
    rolling: RollingValue,
    direction: f32,
    change: PhaseClock,
    hold: Option<(bool, PhaseClock, f32, u32)>,
    limit: PhaseClock,
    _sub: Subscription,
}
impl EventEmitter<f64> for NumberField {}
impl NumberField {
    pub fn new(
        label: impl Into<SharedString>,
        value: f64,
        range: std::ops::RangeInclusive<f64>,
        step: f64,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let label = label.into();
        let (min, max) = (*range.start(), *range.end());
        assert!(min.is_finite() && max.is_finite() && min <= max && step > 0. && step.is_finite());
        let value = value.clamp(min, max);
        let edit = cx.new(|cx| {
            let mut e = Editor::new(label.clone(), InputMode::Single, w, cx);
            e.set_value(value.to_string(), w, cx);
            e
        });
        let sub = cx.subscribe_in(&edit, w, |s: &mut Self, e, ev: &EditEvent, w, cx| {
            if matches!(ev, EditEvent::Submitted | EditEvent::Blurred) {
                let n = e.read(cx).value().parse::<f64>().unwrap_or(s.value);
                s.set(n, w, cx)
            }
        });
        Self {
            label,
            min,
            max,
            step,
            value,
            suffix: "".into(),
            help: "".into(),
            edit,
            rolling: Default::default(),
            direction: 1.,
            change: Default::default(),
            hold: None,
            limit: Default::default(),
            _sub: sub,
        }
    }
    pub fn with_suffix(mut self, s: impl Into<SharedString>) -> Self {
        self.suffix = s.into();
        self
    }
    pub fn with_description(mut self, s: impl Into<SharedString>) -> Self {
        self.help = s.into();
        self
    }
    pub fn set_description(&mut self, t: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.help = t.into();
        cx.notify()
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    fn set(&mut self, n: f64, w: &mut Window, cx: &mut Context<Self>) {
        if n < self.min || n > self.max {
            self.limit.restart(cx);
            self.direction = if n > self.max { 1. } else { -1. };
        }
        let n = if n.is_finite() {
            ((self.min + ((n - self.min) / self.step).round() * self.step)
                .clamp(self.min, self.max)
                * 1e8)
                .round()
                / 1e8
        } else {
            self.value
        };
        if self.value != n {
            self.direction = if n > self.value { 1. } else { -1. };
            self.change.restart(cx);
            self.value = n;
            cx.emit(n)
        }
        self.edit
            .update(cx, |e, cx| e.set_value(self.value.to_string(), w, cx));
        cx.notify()
    }
}
impl Render for NumberField {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        if let Some((plus, clock, next, count)) = self.hold {
            let now = clock.age(cx);
            if now >= next {
                self.set(
                    self.value + if plus { self.step } else { -self.step },
                    w,
                    cx,
                );
                self.hold = Some((
                    plus,
                    clock,
                    now + (0.150 * 0.86_f32.powi((count + 1) as i32)).max(0.040),
                    count + 1,
                ));
            }
            w.request_animation_frame();
        }
        let bump = self.limit.time(0.32, w, cx) / 0.32;
        let moving =
            self.change.time(0.24, w, cx) < 0.24 && !self.edit.read(cx).focus.is_focused(w);
        let mut strip = row()
            .gap(px(4.))
            .justify_between()
            .rounded(crate::foundation::surface_radius(cx))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .p(px(4.));
        for i in 0..3 {
            if i == 1 {
                strip = strip.child(
                    row()
                        .justify_center()
                        .child({
                            let label =
                                self.rolling
                                    .label(self.value.to_string(), self.direction, w, cx);
                            div()
                                .relative()
                                .top(px(-self.direction
                                    * (bump * std::f32::consts::PI).sin()
                                    * 2.5))
                                .w(px(34.))
                                .h(px(28.))
                                .child(
                                    div()
                                        .size_full()
                                        .opacity(if moving { 0. } else { 1. })
                                        .child(self.edit.clone()),
                                )
                                .when(moving, |d| {
                                    d.child(div().absolute().left_0().top(px(4.)).child(label))
                                })
                        })
                        .child(self.suffix.clone()),
                );
                continue;
            }
            let plus = i == 2;
            let enabled = if plus {
                self.value < self.max
            } else {
                self.value > self.min
            };
            let e = glyph(
                ("step", i as usize),
                if plus {
                    tr(cx, "Increase", "增加")
                } else {
                    tr(cx, "Decrease", "减少")
                },
                if plus { Glyph::Plus } else { Glyph::Minus },
                cx,
            );
            strip = strip.child(
                e.when(!enabled, |d| d.opacity(0.45))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |s, _, w, cx| {
                            s.set(s.value + if plus { s.step } else { -s.step }, w, cx);
                            let mut clock = PhaseClock::default();
                            clock.restart(cx);
                            s.hold = Some((plus, clock, 0.4, 0));
                            cx.notify();
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|s, _, _, cx| {
                            s.hold = None;
                            cx.notify()
                        }),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(|s, _, _, cx| {
                            s.hold = None;
                            cx.notify()
                        }),
                    )
                    .on_click(cx.listener(move |s, e: &ClickEvent, w, cx| {
                        if matches!(e, ClickEvent::Keyboard(_)) {
                            s.set(s.value + if plus { s.step } else { -s.step }, w, cx)
                        }
                    })),
            )
        }
        section(240., cx)
            .id("number")
            .capture_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                let n = match e.keystroke.key.as_str() {
                    "up" => s.value + s.step,
                    "down" => s.value - s.step,
                    "home" => s.min,
                    "end" => s.max,
                    _ => return,
                };
                s.set(n, w, cx);
                cx.stop_propagation();
                w.prevent_default()
            }))
            .child(self.label.clone())
            .child(strip)
            .when(!self.help.is_empty(), |v| {
                v.child(hint(self.help.clone(), cx))
            })
    }
}
