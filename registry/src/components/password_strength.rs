//! A new-password field with progressive feedback on a configurable policy.
use crate::{
    components::password_field::PasswordField,
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{Paint, PhaseClock, Slide, ease},
        field::FieldEvent,
        glyph::{Glyph, transformed},
    },
};
use gpui::{prelude::*, *};
use std::rc::Rc;
pub struct PasswordRule {
    pub label: SharedString,
    test: Rc<dyn Fn(&str) -> bool>,
    remaining: Option<usize>,
}
impl PasswordRule {
    pub fn new(label: impl Into<SharedString>, test: impl Fn(&str) -> bool + 'static) -> Self {
        Self {
            label: label.into(),
            test: Rc::new(test),
            remaining: None,
        }
    }
    pub fn length(label: impl Into<SharedString>, minimum: usize) -> Self {
        let mut r = Self::new(label, move |s| s.chars().count() >= minimum);
        r.remaining = Some(minimum);
        r
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Strength {
    pub level: u8,
    pub too_short: bool,
    pub met: Vec<bool>,
}
#[derive(Clone)]
pub enum StrengthEvent {
    Changed(SharedString, Strength),
    Submitted,
    Visibility(bool),
}
pub struct PasswordStrength {
    field: Entity<PasswordField>,
    rules: Vec<PasswordRule>,
    short: usize,
    score: Strength,
    segments: [Slide; 4],
    ticks: Vec<Slide>,
    word: PhaseClock,
    shake: PhaseClock,
    error: SharedString,
    _sub: Subscription,
}
impl EventEmitter<StrengthEvent> for PasswordStrength {}
impl PasswordStrength {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let l = UiLanguage::current(cx);
        let rules = vec![
            PasswordRule::length(l.text("At least 12 characters", "至少 12 个字符"), 12),
            PasswordRule::new(
                l.text("Upper and lowercase letters", "同时包含大写和小写字母"),
                |s| s.chars().any(char::is_uppercase) && s.chars().any(char::is_lowercase),
            ),
            PasswordRule::new(
                l.text("At least one number", "至少包含一个数字"),
                |s| s.chars().any(char::is_numeric),
            ),
            PasswordRule::new(
                l.text("At least one symbol", "至少包含一个符号"),
                |s| {
                    s.chars()
                        .any(|c| !c.is_alphanumeric() && !c.is_whitespace())
                },
            ),
        ];
        let field = cx.new(|cx| PasswordField::new(id, label, w, cx));
        let sub = cx.subscribe(&field, |s, _, e: &FieldEvent, cx| {
            match e {
                FieldEvent::Changed(v) => {
                    s.rescore(v, cx);
                    cx.emit(StrengthEvent::Changed(v.clone(), s.score.clone()))
                }
                FieldEvent::Submitted => cx.emit(StrengthEvent::Submitted),
                FieldEvent::Visibility(v) => cx.emit(StrengthEvent::Visibility(*v)),
                _ => {}
            }
            cx.notify()
        });
        Self {
            field,
            rules,
            short: 8,
            score: Strength {
                level: 0,
                too_short: false,
                met: vec![false; 4],
            },
            segments: [Slide::snappy(0.); 4],
            ticks: vec![Slide::snappy(0.); 4],
            word: Default::default(),
            shake: Default::default(),
            error: "".into(),
            _sub: sub,
        }
    }
    pub fn with_policy(
        mut self,
        rules: Vec<PasswordRule>,
        short_length: usize,
        cx: &mut Context<Self>,
    ) -> Self {
        self.rules = rules;
        self.short = short_length;
        self.ticks = vec![Slide::snappy(0.); self.rules.len()];
        let value = self.value(cx).to_owned();
        self.rescore(&value, cx);
        self
    }
    pub fn value<'a>(&'a self, cx: &'a App) -> &'a str {
        self.field.read(cx).value(cx)
    }
    pub fn strength(&self) -> &Strength {
        &self.score
    }
    pub fn set_value(&mut self, v: impl Into<String>, w: &Window, cx: &mut Context<Self>) {
        self.field.update(cx, |s, cx| s.set_value(v, w, cx))
    }
    pub fn set_error(&mut self, v: impl Into<SharedString>, cx: &mut Context<Self>) {
        let v = v.into();
        if v != self.error {
            self.error = v.clone();
            if !v.is_empty() {
                self.shake.restart(cx)
            }
            self.field.update(cx, |s, cx| s.set_error(v, cx));
            cx.notify()
        }
    }
    fn rescore(&mut self, v: &str, cx: &App) {
        let met: Vec<_> = self.rules.iter().map(|r| (r.test)(v)).collect();
        let short = !v.is_empty() && v.chars().count() < self.short;
        let count = met.iter().filter(|v| **v).count();
        let level = if v.is_empty() {
            0
        } else if short {
            1
        } else {
            (4. * count as f32 / met.len().max(1) as f32)
                .round()
                .clamp(1., 4.) as u8
        };
        if self.score.level != level || self.score.too_short != short {
            self.word.restart(cx)
        }
        for (i, t) in self.ticks.iter_mut().enumerate() {
            t.set(f32::from(met[i]), cx)
        }
        for (i, t) in self.segments.iter_mut().enumerate() {
            t.duration(f32::from(i < level as usize), 0.18 + i as f32 * 0.035, cx)
        }
        self.score = Strength {
            level,
            met,
            too_short: short,
        }
    }
}
impl Render for PasswordStrength {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let colors = &Theme::global(cx).tokens.colors;
        let l = UiLanguage::current(cx);
        let level = self.score.level;
        let tone = match level {
            0 => p.faint,
            1 => colors.danger,
            2 => colors.warning,
            3 => colors.success,
            _ => colors.success,
        };
        let word = if self.score.too_short {
            l.text("Too short", "太短")
        } else {
            match level {
                1 => l.text("Weak", "弱"),
                2 => l.text("Fair", "一般"),
                3 => l.text("Good", "良好"),
                4 => l.text("Strong", "强"),
                _ => "",
            }
        };
        let age = self.word.time(0.19, w, cx);
        let shake = self.shake.time(0.28, w, cx);
        let mut meter = div()
            .id("strength-meter")
            .role(Role::Meter)
            .aria_label(l.text("Password strength", "密码强度"))
            .aria_value(format!("{level}/4 {word}"))
            .flex_1()
            .flex()
            .items_center()
            .gap(px(4.));
        for t in &self.segments {
            let v = t.get(w, cx).clamp(0., 1.);
            meter = meter.child(
                div()
                    .flex_1()
                    .h(px(4.))
                    .rounded_full()
                    .bg(p.line)
                    .child(div().w(relative(v)).h_full().rounded_full().bg(tone)),
            )
        }
        let mut rules = div().flex().flex_col().gap(px(5.));
        let len = self.value(cx).chars().count();
        for (i, r) in self.rules.iter().enumerate() {
            let met = self.score.met[i];
            let amount = self.ticks[i].get(w, cx).clamp(0., 1.);
            let rem = r.remaining.unwrap_or(0).saturating_sub(len);
            rules = rules.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .text_color(if met { p.ink } else { p.secondary })
                    .child(
                        div()
                            .size(px(16.))
                            .rounded_full()
                            .border_1()
                            .bg(p.success.opacity(0.16 * amount))
                            .border_color(if met {
                                transparent_black()
                            } else {
                                p.faint.opacity(0.55)
                            })
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(transformed(
                                Glyph::Check,
                                10.,
                                p.success,
                                0.,
                                1.,
                                (0., 0.),
                                amount,
                            )),
                    )
                    .child(div().flex_1().child(r.label.clone()))
                    .when(rem > 0 && len > 0, |e| {
                        e.child(div().text_size(px(12.)).text_color(p.faint).child(
                            if l == UiLanguage::English {
                                format!("{rem} more")
                            } else {
                                format!("还差 {rem} 个")
                            },
                        ))
                    }),
            )
        }
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(15.))
            .child(
                div()
                    .relative()
                    .left(px(if shake < 0.28 {
                        (shake * 105.).sin() * 4. * (1. - shake / 0.28)
                    } else {
                        0.
                    }))
                    .child(self.field.clone()),
            )
            .child(
                div().flex().items_center().gap(px(10.)).child(meter).child(
                    div()
                        .w(px(72.))
                        .h(px(21.))
                        .relative()
                        .top(px(3. * (1. - ease(age / 0.19))))
                        .opacity(0.5 + 0.5 * ease(age / 0.19))
                        .text_right()
                        .text_size(px(13.))
                        .text_color(tone)
                        .child(word),
                ),
            )
            .child(rules)
    }
}
