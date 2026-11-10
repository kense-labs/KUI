//! A value display with place-keyed digit wheels.
use crate::primitives::family::{Paint, Slide, disabled_motion, measure};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

struct Digit {
    target: i32,
    position: Slide,
    width: Slide,
}
/// A host-controlled, formatted value. Values are not announced automatically.
pub struct AnimatedCounter {
    value: f64,
    label: SharedString,
    prefix: SharedString,
    suffix: SharedString,
    decimals: usize,
    size: f32,
    line_height: f32,
    wheel_seconds: f32,
    feather: f32,
    font_weight: FontWeight,
    digits: Vec<Digit>,
    started: bool,
    accessible: bool,
    animate_on_view: bool,
    revealed: bool,
    visible: bool,
    bounds: Rc<Cell<Bounds<Pixels>>>,
}
impl AnimatedCounter {
    pub fn new(value: f64) -> Self {
        Self {
            value: if value.is_finite() { value } else { 0. },
            label: "".into(),
            prefix: "".into(),
            suffix: "".into(),
            decimals: 0,
            size: 30.,
            line_height: 1.,
            wheel_seconds: 0.4,
            feather: 0.16,
            font_weight: FontWeight::MEDIUM,
            digits: vec![],
            started: false,
            accessible: true,
            animate_on_view: false,
            revealed: false,
            visible: true,
            bounds: Default::default(),
        }
    }
    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
    pub fn with_prefix(mut self, prefix: impl Into<SharedString>) -> Self {
        self.prefix = prefix.into();
        self
    }
    pub fn with_suffix(mut self, suffix: impl Into<SharedString>) -> Self {
        self.suffix = suffix.into();
        self
    }
    pub fn with_decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals.min(6);
        self
    }
    pub fn with_size(mut self, size: f32) -> Self {
        if size.is_finite() {
            self.size = size.clamp(10., 96.);
        }
        self
    }
    #[allow(dead_code)]
    pub(crate) fn with_wheel_duration(mut self, seconds: f32) -> Self {
        self.wheel_seconds = seconds;
        self
    }
    // Optional sibling controls can resize the wheel without restarting its digits.
    #[allow(dead_code)]
    pub(crate) fn set_wheel_size(&mut self, size: f32) {
        self.size = size;
    }
    // Optional sibling controls use this when installed alongside the counter.
    #[allow(dead_code)]
    pub(crate) fn with_wheel_style(
        mut self,
        line_height: f32,
        feather: f32,
        font_weight: FontWeight,
    ) -> Self {
        self.line_height = line_height;
        self.feather = feather;
        self.font_weight = font_weight;
        self
    }
    /// Hide a decorative duplicate from the accessibility tree.
    pub fn with_accessibility(mut self, accessible: bool) -> Self {
        self.accessible = accessible;
        self
    }
    /// Roll from zero once at least sixty percent of the counter is visible.
    pub fn with_animate_on_view(mut self, animate: bool) -> Self {
        self.animate_on_view = animate;
        self
    }
    /// Stop requesting paint frames while the embedding host hides the counter.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Update the displayed value; current digit positions survive rapid retargeting.
    pub fn set_value(&mut self, value: f64, cx: &mut Context<Self>) {
        if !value.is_finite() || value == self.value {
            return;
        }
        let upward = value > self.value;
        self.value = value;
        if self.animate_on_view && !self.revealed {
            self.started = false;
        } else {
            self.update_digits(upward, cx);
        }
        cx.notify();
    }
    fn raw(&self) -> String {
        format!("{:.*}", self.decimals, self.value.abs())
    }
    fn formatted(&self) -> String {
        let raw = self.raw();
        let (whole, fractional) = raw.split_once('.').unwrap_or((&raw, ""));
        let mut grouped = String::new();
        for (i, ch) in whole.chars().enumerate() {
            if i > 0 && (whole.len() - i) % 3 == 0 {
                grouped.push(',');
            }
            grouped.push(ch);
        }
        if !fractional.is_empty() {
            grouped.push('.');
            grouped.push_str(fractional);
        }
        format!(
            "{}{}{}{}",
            self.prefix,
            if self.value < 0. { "−" } else { "" },
            grouped,
            self.suffix
        )
    }
    fn update_digits(&mut self, up: bool, cx: &App) {
        let incoming: Vec<_> = self
            .raw()
            .bytes()
            .filter(u8::is_ascii_digit)
            .rev()
            .map(|b| i32::from(b - b'0'))
            .collect();
        let first = !self.started;
        self.started = true;
        let len = incoming.len();
        self.digits
            .resize_with(self.digits.len().max(len), || Digit {
                target: 0,
                position: Slide::new(0.),
                width: Slide::new(if first { 1. } else { 0. }),
            });
        for (i, digit) in self.digits.iter_mut().enumerate() {
            digit.width.duration(f32::from(i < len), 0.42, cx);
            if let Some(&next) = incoming.get(i) {
                if first {
                    digit.target = next;
                    digit
                        .position
                        .reset(if self.animate_on_view && !self.revealed {
                            0.
                        } else {
                            next as f32
                        });
                } else {
                    if digit.target.abs() > 10_000 {
                        let shift = (digit.target / 10) * 10;
                        digit.target -= shift;
                        digit.position.translate(-(shift as f32));
                    }
                    digit.target = wheel_target(digit.target, next, up);
                    digit
                        .position
                        .duration(digit.target as f32, self.wheel_seconds, cx);
                }
            }
        }
    }
}

fn wheel_target(previous: i32, digit: i32, upward: bool) -> i32 {
    let delta = digit - previous.rem_euclid(10);
    previous
        + if upward {
            delta.rem_euclid(10)
        } else {
            -(-delta).rem_euclid(10)
        }
}
impl Render for AnimatedCounter {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.started {
            self.update_digits(true, cx);
        }
        let viewport = w.viewport_size();
        let portion = visible_portion(self.bounds.get(), viewport);
        let visible = self.visible && w.is_visible() && portion > 0.;
        if self.animate_on_view && !self.revealed && visible && portion >= 0.6 {
            self.revealed = true;
            let columns = self.digits.len();
            for (index, digit) in self.digits.iter_mut().enumerate() {
                let delay = ((columns - 1 - index) as f32 * 0.035).min(0.25);
                digit.position.delayed(digit.target as f32, 0.48, delay, cx);
            }
        }
        let now = cx.background_executor().now();
        let reduced = disabled_motion(cx);
        let mut moving = false;
        let p = Paint::get(cx);
        let h = self.size * self.line_height;
        let feather = self.size * self.feather;
        let em = self.size;
        let mut content = div()
            .flex()
            .items_center()
            .text_size(px(em))
            .line_height(px(h))
            .font_weight(self.font_weight)
            .text_color(p.ink);
        if !self.prefix.is_empty() {
            content = content.child(self.prefix.clone());
        }
        if self.value < 0. {
            content = content.child("−");
        }
        let digit_width = measure(&"0".into(), em, w);
        for (i, digit) in self.digits.iter().enumerate().rev() {
            let (alpha, active) = digit.width.sample_batch(now, reduced);
            moving |= active;
            let alpha = alpha.clamp(0., 1.);
            if alpha < 0.001 {
                continue;
            }
            let (pos, active) = digit.position.sample_batch(now, reduced);
            moving |= active;
            let first = pos.floor() as i32;
            let y = pos - first as f32;
            let col = div()
                .relative()
                .h(px(h + feather * 2.))
                .mt(px(-feather))
                .mb(px(-feather))
                .w(px(digit_width * alpha))
                .overflow_hidden()
                .opacity(alpha)
                .child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, _, w, cx| {
                            w.with_content_mask(Some(ContentMask { bounds }), |w| {
                                for (d, offset) in [(first, -y), (first + 1, 1. - y)] {
                                    let opacity = (1. - offset.abs()).clamp(0., 1.);
                                    if opacity < 0.001 {
                                        continue;
                                    }
                                    let digit: SharedString = d.rem_euclid(10).to_string().into();
                                    let mut run = w.text_style().to_run(digit.len());
                                    run.color = p.ink.opacity(opacity);
                                    let line =
                                        w.text_system().shape_line(digit, px(em), &[run], None);
                                    let _ = line.paint(
                                        point(
                                            bounds.origin.x,
                                            bounds.origin.y + px(feather + offset * h),
                                        ),
                                        px(h),
                                        TextAlign::Left,
                                        None,
                                        w,
                                        cx,
                                    );
                                }
                            });
                        },
                    )
                    .size_full(),
                );
            content = content.child(col);
            if i == self.decimals && self.decimals > 0 {
                content = content.child(
                    div()
                        .w(px(measure(&".".into(), em, w) * alpha))
                        .opacity(alpha)
                        .overflow_x_hidden()
                        .child("."),
                );
            } else if i > self.decimals && (i - self.decimals).is_multiple_of(3) {
                content = content.child(
                    div()
                        .w(px(measure(&",".into(), em, w) * alpha))
                        .opacity(alpha)
                        .overflow_x_hidden()
                        .child(","),
                );
            }
        }
        if !self.suffix.is_empty() {
            content = content.child(self.suffix.clone());
        }
        let description = if self.label.is_empty() {
            self.formatted()
        } else {
            format!("{}: {}", self.label, self.formatted())
        };
        if moving && visible {
            w.request_animation_frame();
        }
        let bounds = self.bounds.clone();
        div()
            .id("animated-counter")
            .relative()
            .when(self.accessible, |element| {
                element.role(Role::Label).aria_label(description)
            })
            .flex()
            .flex_col()
            .child(
                canvas(
                    move |b, w, _| {
                        let old = bounds.replace(b);
                        let viewport = w.viewport_size();
                        if old.size != b.size
                            || (visible_portion(old, viewport) >= 0.6)
                                != (visible_portion(b, viewport) >= 0.6)
                        {
                            w.request_animation_frame();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0(),
            )
            .when(!self.label.is_empty(), |e| {
                e.child(
                    div()
                        .mb(px(8.))
                        .text_size(px(12.))
                        .line_height(px(16.8))
                        .text_color(p.faint)
                        .child(self.label.clone()),
                )
            })
            .child(content)
    }
}

fn visible_portion(bounds: Bounds<Pixels>, viewport: Size<Pixels>) -> f32 {
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    if width <= 0. || height <= 0. {
        return 0.;
    }
    let visible_width =
        f32::from(bounds.right().min(viewport.width) - bounds.left().max(px(0.))).max(0.);
    let visible_height =
        f32::from(bounds.bottom().min(viewport.height) - bounds.top().max(px(0.))).max(0.);
    (visible_width * visible_height / (width * height)).clamp(0., 1.)
}

#[cfg(test)]
mod tests {
    use super::{AnimatedCounter, visible_portion, wheel_target};
    use gpui::{Bounds, point, px, size};
    #[test]
    fn formatting_preserves_values_beyond_a_trillion() {
        let counter = AnimatedCounter::new(2_123_456_789_012.25)
            .with_prefix("$")
            .with_decimals(2);
        assert_eq!(counter.formatted(), "$2,123,456,789,012.25");
        assert_eq!(counter.value(), 2_123_456_789_012.25);
    }
    #[test]
    fn reveal_threshold_uses_the_actual_visible_area() {
        let viewport = size(px(100.), px(100.));
        let bounds = Bounds::new(point(px(0.), px(40.)), size(px(100.), px(100.)));
        assert!((visible_portion(bounds, viewport) - 0.6).abs() < 0.0001);
        let offscreen = Bounds::new(point(px(110.), px(0.)), size(px(100.), px(100.)));
        assert_eq!(visible_portion(offscreen, viewport), 0.);
    }

    #[test]
    fn wheels_follow_the_whole_numbers_direction_and_wrap_continuously() {
        assert_eq!(wheel_target(9, 0, true), 10);
        assert_eq!(wheel_target(10, 9, false), 9);
        // -5 -> -4 increases the whole value, so its digit rolls upward too.
        assert_eq!(wheel_target(5, 4, true), 14);
        assert_eq!(wheel_target(14, 5, false), 5);
    }
}
