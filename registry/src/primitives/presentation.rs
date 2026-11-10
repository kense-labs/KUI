//! Layout and actions for status surfaces and visualizations.
use crate::primitives::{
    control::UiLanguage,
    family::{Paint, Tone, face},
};
use gpui::{prelude::*, *};

pub fn tr(cx: &App, en: &'static str, zh: &'static str) -> &'static str {
    UiLanguage::current(cx).text(en, zh)
}
pub fn column() -> Div {
    div().min_w_0().flex().flex_col()
}
pub fn row() -> Div {
    div().min_w_0().flex().items_center()
}
pub fn frame(width: f32, p: Paint) -> Div {
    column()
        .w(px(width))
        .max_w_full()
        .gap_3()
        .p_4()
        .rounded(crate::foundation::SURFACE_RADIUS)
        .bg(p.panel)
        .border_1()
        .border_color(p.line)
        .text_color(p.ink)
}
pub fn hint(text: impl Into<SharedString>, p: Paint) -> Div {
    div()
        .text_size(px(12.))
        .line_height(px(17.))
        .text_color(p.secondary)
        .child(text.into())
}
pub fn title(text: impl Into<SharedString>, p: Paint) -> Div {
    div()
        .text_size(px(15.))
        .line_height(px(20.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(p.ink)
        .child(text.into())
}
pub fn command<T: 'static>(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    cx: &Context<T>,
    f: impl Fn(&mut T, &mut Window, &mut Context<T>) + 'static,
) -> Stateful<Div> {
    toned_command(id, label, Tone::Surface, cx, f)
}
pub fn toned_command<T: 'static>(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    tone: Tone,
    cx: &Context<T>,
    f: impl Fn(&mut T, &mut Window, &mut Context<T>) + 'static,
) -> Stateful<Div> {
    let p = Paint::get(cx);
    let label = label.into();
    face(
        id,
        label.clone(),
        34.,
        crate::foundation::surface_radius(cx).into(),
        tone,
        cx,
    )
    .px_3()
    .border_1()
    .border_color(p.line)
    .when(!label.is_empty(), |el| el.child(label))
    .hover(move |s| {
        s.bg(if tone == Tone::Primary {
            p.primary_hover(0.12)
        } else {
            p.hover
        })
    })
    .active(|s| s.opacity(0.74))
    .on_click(cx.listener(move |v, _, w, cx| f(v, w, cx)))
}
pub fn dot(size: f32, color: Hsla) -> Div {
    div().flex_none().size(px(size)).rounded_full().bg(color)
}
pub const SERIES: [u32; 6] = [0x578ce2, 0x47ab8b, 0xe3aa54, 0x9983d3, 0xdc8076, 0x8c98a8];

/// Tracks the outgoing ink as well as the entering line when a label is replaced.
pub(crate) struct ChangingText {
    pub current: SharedString,
    previous: Vec<(SharedString, f32, f32)>,
    clock: crate::primitives::family::PhaseClock,
    direction: f32,
}
impl ChangingText {
    pub fn new(value: impl Into<SharedString>) -> Self {
        Self {
            current: value.into(),
            previous: Vec::new(),
            clock: Default::default(),
            direction: 1.,
        }
    }
    pub fn set(&mut self, target: impl Into<SharedString>, direction: f32, cx: &App) {
        let target = target.into();
        if self.current == target {
            return;
        }
        let age = self.clock.age(cx);
        let enter = crate::primitives::family::ease(age / 0.24);
        let exit = crate::primitives::family::standard_ease(age / 0.16);
        for (_, opacity, offset) in &mut self.previous {
            *opacity *= 1. - exit;
            *offset -= 0.3 * self.direction * exit;
        }
        self.previous.retain(|(_, a, _)| *a > 0.01);
        let old = std::mem::replace(&mut self.current, target);
        if enter > 0.01 {
            self.previous
                .push((old, enter, 0.3 * self.direction * (1. - enter)));
        }
        self.direction = direction.signum();
        self.clock.restart(cx);
    }
    pub fn render(&self, em: f32, w: &Window, cx: &App) -> Div {
        use crate::primitives::family::{ease, standard_ease};
        let age = self.clock.time(0.24, w, cx);
        let enter = ease(age / 0.24);
        let exit = standard_ease(age / 0.16);
        let mut body = div().relative().child(
            div()
                .relative()
                .top(px(em * 0.3 * self.direction * (1. - enter)))
                .opacity(enter)
                .child(self.current.clone()),
        );
        if exit < 1. {
            for (text, opacity, offset) in &self.previous {
                body = body.child(
                    div()
                        .absolute()
                        .left_0()
                        .top(px(em * (offset - 0.3 * self.direction * exit)))
                        .opacity(opacity * (1. - exit))
                        .child(text.clone()),
                );
            }
        }
        body
    }
}

/// Place-keyed digit wheels. A carry does not disturb a digit that kept its value.
pub(crate) struct RollingNumber {
    wheels: Vec<(
        i32,
        crate::primitives::family::Slide,
        crate::primitives::family::Slide,
    )>,
    number: f32,
    started: bool,
    initial_roll: bool,
}
impl RollingNumber {
    pub fn new(value: f32) -> Self {
        Self {
            wheels: vec![],
            number: value,
            started: false,
            initial_roll: true,
        }
    }
    pub(crate) fn with_initial_roll(mut self, initial_roll: bool) -> Self {
        self.initial_roll = initial_roll;
        self
    }
    pub fn update(&mut self, value: f32, cx: &App) {
        use crate::primitives::family::Slide;
        let first = !self.started;
        if !first && value == self.number {
            return;
        }
        let down = !first && value < self.number;
        self.number = value;
        self.started = true;
        let digits: Vec<_> = format!("{:.0}", value.abs())
            .bytes()
            .rev()
            .filter(u8::is_ascii_digit)
            .map(|v| i32::from(v - b'0'))
            .collect();
        let len = digits.len();
        self.wheels.resize_with(self.wheels.len().max(len), || {
            (0, Slide::new(0.), Slide::new(if first { 1. } else { 0. }))
        });
        for (i, (target, wheel, width)) in self.wheels.iter_mut().enumerate() {
            width.duration(f32::from(i < len), 0.42, cx);
            if let Some(&digit) = digits.get(i) {
                let delta = digit - target.rem_euclid(10);
                *target += if down {
                    -(-delta).rem_euclid(10)
                } else {
                    delta.rem_euclid(10)
                };
                if first && !self.initial_roll {
                    wheel.reset(*target as f32);
                } else if first {
                    wheel.delayed(
                        *target as f32,
                        0.48,
                        ((len - 1 - i) as f32 * 0.035).min(0.25),
                        cx,
                    )
                } else {
                    wheel.duration(*target as f32, 0.4, cx)
                }
            }
        }
    }
    pub fn render(&mut self, em: f32, w: &Window, cx: &App) -> Div {
        self.update(self.number, cx);
        let h = em * 1.2;
        let mut digits = row().text_size(px(em)).line_height(px(h));
        if self.number.is_sign_negative() {
            digits = digits.child("−")
        }
        for (i, (_, wheel, width)) in self.wheels.iter().enumerate().rev() {
            let alpha = width.get(w, cx).clamp(0., 1.);
            if alpha < 0.001 {
                continue;
            }
            let pos = wheel.get(w, cx);
            let first = pos.floor() as i32;
            let y = pos - first as f32;
            let mut col = div()
                .h(px(h))
                .w(px(
                    crate::primitives::family::measure(&"0".into(), em, w) * alpha
                ))
                .relative()
                .overflow_hidden()
                .opacity(alpha);
            for (digit, offset) in [(first, -y), (first + 1, 1. - y)] {
                col = col.child(
                    div()
                        .absolute()
                        .top(px(offset * h))
                        .left_0()
                        .child(digit.rem_euclid(10).to_string()),
                );
            }
            digits = digits.child(col);
            if i > 0 && i % 3 == 0 {
                digits = digits.child(
                    div()
                        .opacity(alpha)
                        .w(px(
                            crate::primitives::family::measure(&",".into(), em, w) * alpha
                        ))
                        .child(","),
                );
            }
        }
        digits
    }
}

/// Symmetric acceleration and deceleration for a path drawn across the viewport.
pub(crate) fn in_out(x: f32) -> f32 {
    if x <= 0. || x >= 1. {
        return x.clamp(0., 1.);
    }
    let (mut a, mut b) = (0., 1.);
    for _ in 0..14 {
        let t = (a + b) * 0.5;
        let s = 1. - t;
        if 3. * s * s * t * 0.65 + 3. * s * t * t * 0.35 + t * t * t < x {
            a = t
        } else {
            b = t
        }
    }
    let t = (a + b) * 0.5;
    t * t * (3. - 2. * t)
}
