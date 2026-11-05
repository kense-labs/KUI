//! An activity label with a restrained moving highlight.
use crate::primitives::family::{Paint, Slide, disabled_motion, ease, standard_ease};
use gpui::{prelude::*, *};
use std::time::Duration;
use unicode_segmentation::UnicodeSegmentation;
use web_time::Instant;

const FONT: f32 = 16.;
const BAND_EM: f32 = 2.2;

fn animate(
    active: bool,
    visible: bool,
    in_view: bool,
    window_visible: bool,
    reduced: bool,
) -> bool {
    active && visible && in_view && window_visible && !reduced
}

/// Source falloff stops, in units of the 2.2em band.
fn falloff(offset: f32) -> f32 {
    const STOPS: [(f32, f32); 7] = [
        (-1., 0.),
        (-0.5, 0.5),
        (-0.2, 0.9),
        (0., 1.),
        (0.2, 0.9),
        (0.5, 0.5),
        (1., 0.),
    ];
    if offset <= STOPS[0].0 || offset >= STOPS[6].0 {
        return 0.;
    }
    for pair in STOPS.windows(2) {
        let (start, start_value) = pair[0];
        let (end, end_value) = pair[1];
        if offset >= start && offset <= end {
            let span = (end - start).max(0.0001);
            return start_value + (end_value - start_value) * (offset - start) / span;
        }
    }
    0.
}

/// Linear sweep after a 0.12s hold, then a pause of `0.32 * duration` parked at the left.
fn sweep_t(age: f32, duration: f32) -> f32 {
    let duration = duration.max(0.1);
    let time = age - 0.12;
    if time <= 0. {
        return 0.;
    }
    let cycle = duration * 1.32;
    let local = time % cycle;
    if local < duration {
        local / duration
    } else {
        0.
    }
}

fn glide_duration(duration: f32, t: f32) -> f32 {
    (0.42_f32).min(duration * (1. - t).max(0.) * 0.5)
}

pub struct TextShimmer {
    font_size: f32,
    text: SharedString,
    previous: SharedString,
    exit_at: Option<Instant>,
    active: bool,
    visible: bool,
    in_view: bool,
    window_visible: bool,
    reduced: bool,
    duration: Duration,
    elapsed: Duration,
    since: Option<Instant>,
    width: Slide,
    sized: bool,
    idle_at: Option<Instant>,
    resume_at: Option<Instant>,
    settle_from: f32,
    settle_seconds: f32,
    glide_from: f32,
    glide_seconds: f32,
    label_at: Option<Instant>,
}
impl TextShimmer {
    pub fn new(text: impl Into<SharedString>, cx: &App) -> Self {
        Self {
            font_size: FONT,
            text: text.into(),
            previous: "".into(),
            exit_at: None,
            active: true,
            visible: true,
            in_view: true,
            window_visible: true,
            reduced: false,
            duration: Duration::from_millis(1800),
            elapsed: Duration::ZERO,
            since: Some(cx.background_executor().now()),
            width: Slide::new(0.),
            sized: false,
            idle_at: None,
            resume_at: None,
            settle_from: 0.,
            settle_seconds: 0.36,
            glide_from: 0.,
            glide_seconds: 0.,
            label_at: None,
        }
    }
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration.max(Duration::from_millis(100));
        self
    }
    /// Set the text size in px; finite positive values preserve the em-based band.
    pub fn with_font_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0. {
            self.font_size = size;
        }
        self
    }
    /// Set the initial activity without playing an activation or settling transition.
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        if !active {
            self.since = None;
        }
        self
    }
    fn update_clock(&mut self, cx: &App) {
        let now = cx.background_executor().now();
        if let Some(start) = self.since.take() {
            self.elapsed += now.saturating_duration_since(start);
        }
        if animate(
            self.active,
            self.visible,
            self.in_view,
            self.window_visible,
            self.reduced,
        ) {
            self.since = Some(now);
        }
    }
    fn age_secs(&self, cx: &App) -> f32 {
        let extra = self.since.map_or(Duration::ZERO, |start| {
            cx.background_executor()
                .now()
                .saturating_duration_since(start)
        });
        (self.elapsed + extra).as_secs_f32()
    }
    fn displayed_settle(&self, cx: &App) -> f32 {
        let now = cx.background_executor().now();
        if let Some(at) = self.idle_at {
            let u = standard_ease(
                (now.saturating_duration_since(at).as_secs_f32() / self.settle_seconds.max(0.001))
                    .clamp(0., 1.),
            );
            return self.settle_from + (1. - self.settle_from) * u;
        }
        if let Some(at) = self.resume_at {
            let u = standard_ease(
                (now.saturating_duration_since(at).as_secs_f32() / 0.24).clamp(0., 1.),
            );
            return self.settle_from * (1. - u);
        }
        if self.active { 0. } else { 1. }
    }
    pub fn set_active(&mut self, active: bool, cx: &mut Context<Self>) {
        if self.active != active {
            let settle_now = self.displayed_settle(cx);
            let t_now = sweep_t(self.age_secs(cx), self.duration.as_secs_f32());
            self.active = active;
            self.update_clock(cx);
            let now = cx.background_executor().now();
            let reduced = disabled_motion(cx);
            self.settle_from = settle_now;
            if active {
                self.resume_at = Some(now);
                self.idle_at = None;
                self.glide_seconds = 0.;
            } else {
                self.glide_from = if reduced { 0. } else { t_now };
                self.glide_seconds = if reduced || !(t_now > 0. && t_now < 1.) {
                    0.
                } else {
                    glide_duration(self.duration.as_secs_f32(), t_now)
                };
                self.settle_seconds = if reduced { 0.12 } else { 0.36 };
                self.idle_at = Some(now);
                self.resume_at = None;
            }
            cx.notify();
        }
    }
    /// Report host visibility in addition to automatic viewport detection.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            self.update_clock(cx);
            cx.notify();
        }
    }
    pub fn set_text(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        let text = text.into();
        if text != self.text {
            self.previous = self.text.clone();
            let now = cx.background_executor().now();
            self.exit_at = Some(now);
            self.text = text;
            self.label_at = Some(now);
            cx.notify();
        }
    }
}
impl Render for TextShimmer {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let reduced = disabled_motion(cx);
        if self.window_visible != w.is_visible() || self.reduced != reduced {
            self.window_visible = w.is_visible();
            self.reduced = reduced;
            self.update_clock(cx);
        }
        let now = cx.background_executor().now();
        let running = animate(
            self.active,
            self.visible,
            self.in_view,
            self.window_visible,
            reduced,
        );
        let seconds = self.duration.as_secs_f32();
        let age = self.age_secs(cx);
        let settle = self.displayed_settle(cx);
        let t = if reduced {
            0.
        } else if let Some(at) = self.idle_at {
            let idle = now.saturating_duration_since(at).as_secs_f32();
            if self.glide_seconds > 0. {
                let u = ease((idle / self.glide_seconds).clamp(0., 1.));
                self.glide_from + (1. - self.glide_from) * u
            } else {
                self.glide_from
            }
        } else if self.active {
            sweep_t(age, seconds)
        } else {
            0.
        };
        let label_age = self
            .label_at
            .map(|at| now.saturating_duration_since(at).as_secs_f32());
        let (label_rise, label_fade) = match label_age {
            Some(age) if reduced => (1., standard_ease((age / 0.12).clamp(0., 1.))),
            Some(age) => {
                let u = ease((age / 0.24).clamp(0., 1.));
                (u, u)
            }
            None => (1., 1.),
        };
        let settling = self.idle_at.is_some_and(|at| {
            now.saturating_duration_since(at).as_secs_f32()
                < self.settle_seconds.max(self.glide_seconds)
        }) || self
            .resume_at
            .is_some_and(|at| now.saturating_duration_since(at).as_secs_f32() < 0.24)
            || label_age.is_some_and(|age| age < if reduced { 0.12 } else { 0.24 });
        if running || settling {
            w.request_animation_frame();
        }
        let style = w.text_style();
        let shaped = w.text_system().shape_line(
            self.text.clone(),
            px(self.font_size),
            &[style.to_run(self.text.len())],
            None,
        );
        let target: f32 = shaped.width.into();
        if !self.sized || reduced {
            self.width.reset(target);
            self.sized = true;
        } else if (self.width.value(cx) - target).abs() > 0.5 {
            self.width.duration(target, 0.42, cx);
        }
        let width = if running || settling {
            self.width.get(w, cx)
        } else {
            self.width.value(cx)
        }
        .max(target);
        let band = BAND_EM * self.font_size;
        let peak = t * target + (2. * t - 1.) * band;
        let base = p.faint.blend(p.ink.opacity(0.12));
        let rest = base.blend(p.ink.opacity(settle));
        let styles = self.text.grapheme_indices(true).map(|(start, grapheme)| {
            let end = start + grapheme.len();
            let left: f32 = shaped.x_for_index(start).into();
            let right: f32 = shaped.x_for_index(end).into();
            let offset = ((left + right) * 0.5 - peak) / band;
            let strength = falloff(offset);
            (
                start..end,
                HighlightStyle {
                    color: Some(rest.blend(p.ink.opacity(strength))),
                    ..Default::default()
                },
            )
        });
        let text = StyledText::new(self.text.clone()).with_highlights(styles);
        let viewport = w.viewport_size();
        let weak = cx.weak_entity();
        div()
            .on_children_prepainted(move |bounds, _, cx| {
                if let Some(b) = bounds.first() {
                    let margin = px(64.);
                    let visible = b.left() < viewport.width + margin
                        && b.top() < viewport.height + margin
                        && b.right() > -margin
                        && b.bottom() > -margin;
                    let _ = weak.update(cx, |s, cx| {
                        if s.in_view != visible {
                            s.in_view = visible;
                            s.update_clock(cx);
                            cx.notify();
                        }
                    });
                }
            })
            .id("text-shimmer")
            .role(Role::Label)
            .aria_label(self.text.clone())
            .w(px(width))
            .relative()
            .top(px(0.3 * self.font_size * (1. - label_rise)))
            .opacity(label_fade)
            .text_size(px(self.font_size))
            .whitespace_nowrap()
            .text_color(p.secondary)
            .child(text)
            .children(self.exit_at.and_then(|at| {
                let age = now.saturating_duration_since(at).as_secs_f32();
                if age >= 0.16 || self.previous.is_empty() {
                    return None;
                }
                let leave = if reduced {
                    1.
                } else {
                    standard_ease((age / 0.16).clamp(0., 1.))
                };
                Some(
                    div()
                        .absolute()
                        .top(px(-0.24 * self.font_size * leave))
                        .opacity(1. - leave)
                        .whitespace_nowrap()
                        .text_color(p.secondary)
                        .child(self.previous.clone()),
                )
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::{animate, falloff, glide_duration, sweep_t};
    #[test]
    fn hidden_or_reduced_motion_does_not_tick() {
        assert!(animate(true, true, true, true, false));
        assert!(!animate(true, true, false, true, false));
        assert!(!animate(true, true, true, false, false));
        assert!(!animate(true, true, true, true, true));
    }
    #[test]
    fn band_and_sweep_follow_the_source() {
        assert_eq!(falloff(0.), 1.);
        assert_eq!(falloff(0.2), 0.9);
        assert_eq!(falloff(-0.5), 0.5);
        assert_eq!(falloff(1.), 0.);
        assert!((falloff(0.35) - falloff(-0.35)).abs() < 0.0001);
        assert_eq!(sweep_t(0.12, 1.8), 0.);
        assert!((sweep_t(0.12 + 0.9, 1.8) - 0.5).abs() < 0.0001);
        assert_eq!(sweep_t(0.12 + 1.8 + 0.2, 1.8), 0.);
        assert!((glide_duration(1.8, 0.9) - 0.09).abs() < 0.0001);
        assert_eq!(glide_duration(1.8, 0.2), 0.42);
    }
}
