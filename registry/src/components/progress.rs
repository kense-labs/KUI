//! Determinate and indeterminate progress share a stable track.
use crate::primitives::{
    family::{Paint, PhaseClock, Slide, disabled_motion, ease, standard_ease},
    glyph::{Glyph, transformed},
    presentation::*,
};
use gpui::{prelude::*, *};
pub struct Progress {
    label: SharedString,
    value: Option<f32>,
    fill: Slide,
    completed: PhaseClock,
    start: web_time::Instant,
}
impl Progress {
    pub fn new(label: impl Into<SharedString>, value: Option<f32>, cx: &App) -> Self {
        let value = value.map(|v| v.clamp(0., 1.));
        Self {
            label: label.into(),
            value,
            fill: Slide::new(value.unwrap_or_default()),
            completed: Default::default(),
            start: cx.background_executor().now(),
        }
    }
    pub fn set_value(&mut self, value: Option<f32>, cx: &mut Context<Self>) {
        let was_complete = self.value == Some(1.);
        self.value = value.map(|v| v.clamp(0., 1.));
        if let Some(v) = self.value {
            self.fill.duration(v, 0.4, cx);
            if !was_complete && v == 1. {
                self.completed.restart(cx)
            }
        }
        cx.notify()
    }
}
impl Render for Progress {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let mut track = div()
            .relative()
            .w_full()
            .h(px(7.))
            .bg(p.line)
            .rounded(crate::foundation::surface_radius(cx))
            .overflow_hidden();
        let (left, width) = if self.value.is_some() {
            (0., self.fill.get(w, cx).clamp(0., 1.))
        } else {
            let phase = if disabled_motion(cx) {
                0.35
            } else {
                w.request_animation_frame();
                (cx.background_executor()
                    .now()
                    .saturating_duration_since(self.start)
                    .as_secs_f32()
                    / 1.35)
                    .fract()
                    * 1.4
                    - 0.4
            };
            (phase, 0.4)
        };
        let complete = self.value == Some(1.);
        let age = if complete {
            self.completed.time(0.6, w, cx)
        } else {
            0.
        };
        let success = standard_ease((age - 0.2) / 0.24);
        track = track.child(
            div()
                .absolute()
                .left(relative(left))
                .w(relative(width))
                .h_full()
                .rounded(crate::foundation::surface_radius(cx))
                .bg(p.bright.blend(p.success.opacity(success))),
        );
        column()
            .w(px(380.))
            .max_w_full()
            .gap_2()
            .id("progress-progressindicator")
            .role(Role::ProgressIndicator)
            .aria_min_numeric_value(0.)
            .aria_max_numeric_value(1.)
            .when_some(self.value, |el, value| el.aria_numeric_value(value as f64))
            .aria_label(self.label.clone())
            .child(
                row()
                    .justify_between()
                    .child(title(self.label.clone(), p))
                    .when_some(self.value, |el, _| {
                        let arrival = ease((age - 0.24) / 0.26);
                        el.child(
                            row()
                                .gap_1()
                                .when(complete, |el| {
                                    el.child(transformed(
                                        Glyph::Check,
                                        14.,
                                        p.success.opacity(arrival),
                                        0.,
                                        0.6 + 0.4 * arrival,
                                        (0., 0.),
                                        1.,
                                    ))
                                })
                                .child(
                                    hint(format!("{:.0}%", 100. * width), p)
                                        .w(px(34.))
                                        .text_right(),
                                ),
                        )
                    }),
            )
            .child(track)
    }
}
