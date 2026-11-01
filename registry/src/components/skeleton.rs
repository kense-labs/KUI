//! Placeholder geometry stays stable while the host swaps in its content.
use crate::primitives::{
    family::{Paint, disabled_motion},
    presentation::*,
};
use gpui::{prelude::*, *};
pub struct Skeleton {
    label: SharedString,
    lines: Vec<f32>,
    avatar: bool,
    start: web_time::Instant,
}
impl Skeleton {
    pub fn new(
        label: impl Into<SharedString>,
        widths: impl IntoIterator<Item = f32>,
        cx: &App,
    ) -> Self {
        Self {
            label: label.into(),
            lines: widths.into_iter().map(|v| v.clamp(0., 1.)).collect(),
            avatar: false,
            start: cx.background_executor().now(),
        }
    }
    /// Start a fresh pulse sequence when the host reloads its content.
    pub fn restart(&mut self, cx: &mut Context<Self>) {
        self.start = cx.background_executor().now();
        cx.notify();
    }
    pub fn with_avatar(mut self) -> Self {
        self.avatar = true;
        self
    }
}
impl Render for Skeleton {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let age = cx
            .background_executor()
            .now()
            .saturating_duration_since(self.start)
            .as_secs_f32();
        let reduced = disabled_motion(cx);
        if !reduced && age < 19.8 + self.lines.len() as f32 * 0.09 {
            w.request_animation_frame();
        }
        let tint = |index: usize| {
            let t = (age - index as f32 * 0.09).max(0.);
            let amount = if reduced || t >= 19.8 {
                1.
            } else {
                let cycle = (t / 1.8).fract() * 2.;
                1. - 0.48 * in_out(if cycle < 1. { cycle } else { 2. - cycle })
            };
            p.line.opacity(amount)
        };
        row()
            .w(px(338.))
            .max_w_full()
            .gap_3()
            .id("skeleton-status")
            .role(Role::Status)
            .aria_label(self.label.clone())
            .when(self.avatar, |el| el.child(dot(40., tint(0))))
            .child(
                column()
                    .flex_1()
                    .gap_2()
                    .children(self.lines.iter().enumerate().map(|(i, &width)| {
                        div()
                            .h(px(10.))
                            .w(relative(width))
                            .rounded(crate::foundation::surface_radius(cx))
                            .bg(tint(i + 1))
                    })),
            )
    }
}
