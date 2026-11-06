//! A single-value slider with elastic overscroll, optional side icons, clickable marks and a trailing value bubble.
use crate::{
    components::slider::{SlideValue, Slider},
    primitives::{
        family::{Paint, Tone, face},
        glyph::{Glyph, icon},
    },
};
use gpui::{prelude::*, *};

/// The value selected by a drag, tap or keyboard step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElasticChanged(pub f64);
/// A single-ended elastic slider built on the shared KUI slider behavior.
pub struct ElasticSlider {
    slider: Entity<Slider>,
    range: std::ops::RangeInclusive<f64>,
    icons: Option<(Glyph, Glyph)>,
    marks: Vec<(f64, SharedString)>,
    _subscription: Subscription,
}
impl EventEmitter<ElasticChanged> for ElasticSlider {}
impl ElasticSlider {
    pub fn new(
        label: impl Into<SharedString>,
        range: std::ops::RangeInclusive<f64>,
        step: f64,
        value: f64,
        suffix: impl Into<SharedString>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let label = label.into();
        let suffix = suffix.into();
        let slider_range = range.clone();
        let slider = cx.new(|cx| {
            Slider::new(
                label,
                slider_range,
                step,
                SlideValue {
                    low: value,
                    high: None,
                },
                cx,
            )
            .with_suffix(suffix)
        });
        let subscription =
            cx.subscribe_in(&slider, w, |_: &mut Self, _, value: &SlideValue, _, cx| {
                cx.emit(ElasticChanged(value.low))
            });
        Self {
            slider,
            range,
            icons: None,
            marks: Vec::new(),
            _subscription: subscription,
        }
    }
    pub fn with_icons(mut self, start: Glyph, end: Glyph) -> Self {
        self.icons = Some((start, end));
        self
    }
    pub fn with_marks(mut self, marks: Vec<(f64, SharedString)>, cx: &mut Context<Self>) -> Self {
        let detents = marks.iter().map(|(v, _)| *v).collect();
        self.slider
            .update(cx, |slider, cx| slider.set_detents(detents, cx));
        self.marks = marks;
        self
    }
    pub fn value(&self, cx: &App) -> f64 {
        self.slider.read(cx).value().low
    }
    /// Add optional drag detents. Values are normalized to the configured step grid.
    pub fn set_detents(&mut self, values: Vec<f64>, cx: &mut Context<Self>) {
        self.slider
            .update(cx, |slider, cx| slider.set_detents(values, cx));
    }
    pub fn set_value(&mut self, value: f64, cx: &mut Context<Self>) {
        self.slider.update(cx, |s, cx| s.set_low(value, cx));
    }
}
impl Render for ElasticSlider {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let current = self.value(cx);
        let min = *self.range.start();
        let max = *self.range.end();
        let span = (max - min).max(f64::EPSILON);
        let mut row = div().w_full().flex().items_center().gap(px(12.));
        if let Some((start_g, _)) = self.icons {
            row = row.child(
                div()
                    .pt(px(22.))
                    .flex_none()
                    .child(icon(start_g, 18., p.secondary)),
            );
        }
        row = row.child(div().flex_1().min_w_0().child(self.slider.clone()));
        if let Some((_, end_g)) = self.icons {
            row = row.child(
                div()
                    .pt(px(22.))
                    .flex_none()
                    .child(icon(end_g, 18., p.secondary)),
            );
        }
        let mut root = div().flex().flex_col().w_full().max_w_full().child(row);
        if !self.marks.is_empty() {
            let pl = if self.icons.is_some() { 42. } else { 12. };
            let pr = if self.icons.is_some() { 42. } else { 12. };
            let mut marks_row = div().relative().w_full().h(px(20.)).pl(px(pl)).pr(px(pr));
            let mut inner = div().relative().size_full();
            for (idx, (val, label)) in self.marks.iter().cloned().enumerate() {
                let frac = ((val - min) / span).clamp(0., 1.) as f32;
                let active = (current - val).abs() < 0.02;
                inner = inner.child(
                    div().absolute().left(relative(frac)).ml(px(-18.)).child(
                        face(
                            ("elastic-mark", idx),
                            label.clone(),
                            22.,
                            radius.into(),
                            Tone::Clear,
                            cx,
                        )
                        .px(px(6.))
                        .text_size(px(12.))
                        .text_color(if active { p.ink } else { p.faint })
                        .hover(move |s| s.text_color(p.ink))
                        .child(label)
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.set_value(val, cx);
                            cx.emit(ElasticChanged(val));
                        })),
                    ),
                );
            }
            marks_row = marks_row.child(inner);
            root = root.child(marks_row);
        }
        root
    }
}
