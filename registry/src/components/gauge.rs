//! An accessible 270-degree meter with animated values and named thresholds.
use crate::primitives::family::{Paint, Slide};
use gpui::{prelude::*, *};
use std::{cell::Cell, f32::consts::PI, rc::Rc};

/// Visual category for a gauge threshold.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GaugeTone {
    #[default]
    Accent,
    Success,
    Warning,
    Danger,
}

/// A named band beginning at a value in the gauge's range.
#[derive(Clone, Debug)]
pub struct GaugeThreshold {
    pub from: f32,
    pub label: SharedString,
    pub tone: GaugeTone,
}
impl GaugeThreshold {
    pub fn new(from: f32, label: impl Into<SharedString>, tone: GaugeTone) -> Self {
        Self {
            from,
            label: label.into(),
            tone,
        }
    }
}

/// Displays a value in a range; the host owns the value and thresholds.
pub struct Gauge {
    label: SharedString,
    detail: SharedString,
    min: f32,
    max: f32,
    value: f32,
    tone: GaugeTone,
    thresholds: Vec<GaugeThreshold>,
    displayed: Slide,
    started: bool,
    filled: bool,
    visible: bool,
    bounds: Rc<Cell<Bounds<Pixels>>>,
}
impl Gauge {
    pub fn new(label: impl Into<SharedString>, value: f32) -> Self {
        Self {
            label: label.into(),
            detail: "".into(),
            min: 0.,
            max: 100.,
            value: if value.is_finite() { value } else { 0. },
            tone: GaugeTone::Accent,
            thresholds: vec![],
            displayed: Slide::new(0.),
            started: false,
            filled: false,
            visible: true,
            bounds: Default::default(),
        }
    }
    pub fn with_range(mut self, min: f32, max: f32) -> Self {
        if min.is_finite() && max.is_finite() && max > min {
            self.min = min;
            self.max = max;
        }
        self
    }
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = detail.into();
        self
    }
    pub fn with_tone(mut self, tone: GaugeTone) -> Self {
        self.tone = tone;
        self
    }
    pub fn with_thresholds(mut self, thresholds: Vec<GaugeThreshold>) -> Self {
        self.thresholds = thresholds;
        self
    }
    pub fn value(&self) -> f32 {
        self.value
    }
    /// Set false while the host hides the meter or its tab.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }
    pub fn set_value(&mut self, value: f32, cx: &mut Context<Self>) {
        if !value.is_finite() {
            return;
        }
        self.value = value;
        if self.started {
            self.displayed.duration(self.fraction(), 0.42, cx);
        }
        cx.notify();
    }
    fn fraction(&self) -> f32 {
        ((self.value - self.min) / (self.max - self.min)).clamp(0., 1.)
    }
}

fn ring(bounds: Bounds<Pixels>, fraction: f32, color: Hsla, w: &mut Window) {
    let width: f32 = bounds.size.width.into();
    let height: f32 = bounds.size.height.into();
    let size = width.min(height / 0.9);
    if size <= 0. {
        return;
    }
    let cx = bounds.origin.x + px(width / 2.);
    let cy = bounds.origin.y + px(size / 2.);
    let radius = size * 0.42;
    let at = |a: f32| point(cx + px(a.sin() * radius), cy - px(a.cos() * radius));
    let start = -0.75 * PI;
    let span = 1.5 * PI * fraction.clamp(0., 1.);
    let cap_radius = size * 0.045;
    let mut stroke = PathBuilder::stroke(px(size * 0.09));
    stroke.move_to(at(start));
    for i in 1..=72 {
        stroke.line_to(at(start + span * i as f32 / 72.));
    }
    if let Ok(path) = stroke.build() {
        w.paint_path(path, color);
    }
    for angle in [start, start + span] {
        let center = at(angle);
        let mut cap = PathBuilder::fill();
        for i in 0..=24 {
            let a = 2. * PI * i as f32 / 24.;
            let p = point(
                center.x + px(cap_radius * a.cos()),
                center.y + px(cap_radius * a.sin()),
            );
            if i == 0 {
                cap.move_to(p)
            } else {
                cap.line_to(p)
            }
        }
        cap.close();
        if let Ok(path) = cap.build() {
            w.paint_path(path, color);
        }
    }
}
fn half_visible(bounds: Bounds<Pixels>, viewport: Size<Pixels>) -> bool {
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    if width <= 0. || height <= 0. {
        return false;
    }
    let x = f32::from(bounds.origin.x);
    let y = f32::from(bounds.origin.y);
    let visible_width = (x + width).min(f32::from(viewport.width)).max(0.) - x.max(0.);
    let visible_height = (y + height).min(f32::from(viewport.height)).max(0.) - y.max(0.);
    visible_width.max(0.) * visible_height.max(0.) >= width * height * 0.5
}
impl Render for Gauge {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let measured = self.bounds.get();
        let visible = self.visible && w.is_visible() && half_visible(measured, w.viewport_size());
        if !self.started && visible {
            self.started = true;
            self.displayed.duration(self.fraction(), 0.768, cx);
        }
        let now = cx.background_executor().now();
        let (progress, active) = self
            .displayed
            .sample_batch(now, crate::primitives::family::disabled_motion(cx));
        if visible && active {
            w.request_animation_frame();
        }
        if self.started && visible && !active {
            self.filled = true;
        }
        let progress = progress.clamp(0., 1.);
        let width = if measured.size.width > px(0.) {
            f32::from(measured.size.width)
        } else {
            176.
        };
        let bounds = self.bounds.clone();
        let current = self.min + progress * (self.max - self.min);
        let band = self
            .thresholds
            .iter()
            .filter(|b| b.from.is_finite() && b.from <= current)
            .max_by(|a, b| a.from.total_cmp(&b.from));
        let tone = band.map_or(self.tone, |b| b.tone);
        let color = match tone {
            GaugeTone::Accent => p.bright,
            GaugeTone::Success => p.success,
            GaugeTone::Warning => rgb(0xd49522).into(),
            GaugeTone::Danger => p.danger,
        };
        let status = band.map(|b| b.label.clone()).unwrap_or_default();
        let value = self.value.clamp(self.min, self.max);
        let aria = if status.is_empty() {
            format!("{}: {:.0}%", self.label, self.fraction() * 100.)
        } else {
            format!("{}: {:.0}%, {}", self.label, self.fraction() * 100., status)
        };
        let gauge = div()
            .id("gauge-meter")
            .role(Role::Meter)
            .aria_label(aria)
            .aria_min_numeric_value(self.min as f64)
            .aria_max_numeric_value(self.max as f64)
            .aria_numeric_value(value as f64)
            .relative()
            .w(px(176.))
            .max_w_full()
            .h(px(width * 0.9))
            .child(
                canvas(
                    move |b, w, _| {
                        let previous = bounds.get();
                        if previous.size != b.size
                            || half_visible(previous, w.viewport_size())
                                != half_visible(b, w.viewport_size())
                        {
                            w.request_animation_frame();
                        }
                        bounds.set(b);
                    },
                    move |b, _, w, _| {
                        ring(b, 1., color.opacity(0.13), w);
                        if progress > 0.001 {
                            ring(b, progress, color, w);
                        }
                    },
                )
                .size_full(),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .w_full()
                    .h(px(width))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .flex()
                            .items_end()
                            .text_color(p.ink)
                            .text_size(px(width * 30. / 176.))
                            .line_height(px(width * 30. / 176.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(format!("{:.0}", progress * 100.))
                            .child(
                                div()
                                    .ml(px(1.))
                                    .text_size(px(width * 15. / 176.))
                                    .line_height(px(width * 18. / 176.))
                                    .text_color(p.secondary)
                                    .font_weight(FontWeight::NORMAL)
                                    .child("%"),
                            ),
                    )
                    .when(self.filled && !status.is_empty(), |e| {
                        e.child(
                            div()
                                .min_w(px(100.))
                                .text_size(px(12.))
                                .line_height(px(17.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(color)
                                .text_center()
                                .child(status),
                        )
                    }),
            );
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(10.))
            .min_w_0()
            .child(gauge)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(3.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(self.label.clone()),
                    )
                    .when(!self.detail.is_empty(), |e| {
                        e.child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(17.))
                                .text_color(p.faint)
                                .child(self.detail.clone()),
                        )
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::half_visible;
    use gpui::{Bounds, point, px, size};
    #[test]
    fn reveal_requires_half_the_meter_inside_viewport() {
        let viewport = size(px(200.), px(200.));
        let full = Bounds {
            origin: point(px(10.), px(10.)),
            size: size(px(100.), px(100.)),
        };
        let half = Bounds {
            origin: point(px(10.), px(150.)),
            ..full
        };
        let less = Bounds {
            origin: point(px(10.), px(151.)),
            ..full
        };
        assert!(half_visible(full, viewport));
        assert!(half_visible(half, viewport));
        assert!(!half_visible(less, viewport));
    }
}
