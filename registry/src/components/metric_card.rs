//! Animated headline, change indicator and comparison period.
use crate::primitives::{family::Paint, presentation::*};
use gpui::{prelude::*, *};
pub struct MetricCard {
    label: SharedString,
    value: RollingNumber,
    change: ChangingText,
    delta: f32,
    comparison: ChangingText,
}
impl MetricCard {
    pub fn new(
        label: impl Into<SharedString>,
        value: f32,
        delta: f32,
        comparison: impl Into<SharedString>,
    ) -> Self {
        Self {
            label: label.into(),
            value: RollingNumber::new(value),
            change: ChangingText::new(format!("{delta:+.1}%")),
            delta,
            comparison: ChangingText::new(comparison),
        }
    }
    pub fn update_metric(
        &mut self,
        value: f32,
        delta: f32,
        comparison: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.value.update(value, cx);
        let dir = if delta < self.delta { -1. } else { 1. };
        self.change.set(format!("{delta:+.1}%"), dir, cx);
        self.delta = delta;
        self.comparison.set(comparison, dir, cx);
        cx.notify()
    }
}
impl Render for MetricCard {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        frame(300., p)
            .gap_3()
            .p_5()
            .id("metric_card-article")
            .role(Role::Article)
            .child(
                row()
                    .justify_between()
                    .child(title(self.label.clone(), p))
                    .child(
                        div()
                            .px_2()
                            .rounded(crate::foundation::surface_radius(cx))
                            .text_size(px(12.))
                            .text_color(if self.delta >= 0. {
                                p.success
                            } else {
                                p.danger
                            })
                            .bg((if self.delta >= 0. {
                                p.success
                            } else {
                                p.danger
                            })
                            .opacity(0.1))
                            .child(self.change.render(12., w, cx)),
                    ),
            )
            .child(
                self.value
                    .render(38., w, cx)
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                self.comparison
                    .render(12., w, cx)
                    .text_size(px(12.))
                    .text_color(p.secondary),
            )
    }
}
