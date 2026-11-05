//! A two-thumb numeric range paired with a histogram of the available values.
#[allow(dead_code)]
#[path = "histogram_range_track.rs"]
mod histogram_range_track;
use self::histogram_range_track::{SlideValue, Slider};
use crate::primitives::{
    control::UiLanguage, direct_touch::Contact, family::Paint, form_chrome::RollingValue,
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

/// Selected bounds, reported as they change.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HistogramRangeChanged {
    pub low: f64,
    pub high: f64,
}

/// Counts finite values into equally spaced bins; the maximum belongs to the last bin.
/// Invalid bounds, zero bins, or more than 4096 bins produce an empty vector.
pub fn histogram(data: &[f64], min: f64, max: f64, bins: usize) -> Vec<usize> {
    if !min.is_finite()
        || !max.is_finite()
        || min >= max
        || !(max - min).is_finite()
        || !(1..=4096).contains(&bins)
    {
        return Vec::new();
    }
    let mut counts = vec![0; bins];
    for &value in data {
        if value.is_finite() && value >= min && value <= max {
            let index = (((value - min) / (max - min)) * bins as f64) as usize;
            counts[index.min(bins - 1)] += 1;
        }
    }
    counts
}

/// A distribution-aware filter. Drag either handle or focus it and use the arrow keys.
pub struct RangeHistogramSlider {
    slider: Entity<Slider>,
    bounds: std::ops::RangeInclusive<f64>,
    data: Vec<f64>,
    bins: Vec<usize>,
    chart_bounds: Rc<Cell<Bounds<Pixels>>>,
    contact: Contact,
    tally: RollingValue,
    prev_count: usize,
    _subscription: Subscription,
}
impl EventEmitter<HistogramRangeChanged> for RangeHistogramSlider {}
impl RangeHistogramSlider {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        label: impl Into<SharedString>,
        data: Vec<f64>,
        bounds: std::ops::RangeInclusive<f64>,
        step: f64,
        value: (f64, f64),
        bin_count: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let bounds = if bounds.start().is_finite()
            && bounds.end().is_finite()
            && bounds.start() < bounds.end()
            && (bounds.end() - bounds.start()).is_finite()
        {
            bounds
        } else {
            0.0..=1.0
        };
        let step = if step.is_finite() && step > 0. {
            step
        } else {
            1.
        };
        let low = if value.0.is_finite() {
            value.0.clamp(*bounds.start(), *bounds.end())
        } else {
            *bounds.start()
        };
        let high = if value.1.is_finite() {
            value.1.clamp(low, *bounds.end())
        } else {
            *bounds.end()
        };
        let bins = histogram(&data, *bounds.start(), *bounds.end(), bin_count);
        let initial_count = data
            .iter()
            .filter(|&&v| v.is_finite() && v >= low && v <= high)
            .count();
        let slider = cx.new(|cx| {
            Slider::new(
                label,
                bounds.clone(),
                step,
                SlideValue {
                    low,
                    high: Some(high),
                },
                cx,
            )
            .with_bin_count(if bins.is_empty() { 0 } else { bin_count })
            .with_hide_header(true)
        });
        let subscription = cx.subscribe_in(
            &slider,
            window,
            |_: &mut Self, _, value: &SlideValue, _, cx| {
                if let Some(high) = value.high {
                    cx.emit(HistogramRangeChanged {
                        low: value.low,
                        high,
                    });
                }
                cx.notify();
            },
        );
        Self {
            slider,
            bounds,
            data,
            bins,
            chart_bounds: Default::default(),
            contact: Default::default(),
            tally: Default::default(),
            prev_count: initial_count,
            _subscription: subscription,
        }
    }
    fn choose_chart_position(&mut self, x: Pixels, w: &mut Window, cx: &mut Context<Self>) {
        let b = self.chart_bounds.get();
        let fraction = f64::from((x - b.left()) / b.size.width.max(px(1.))).clamp(0., 1.);
        self.slider
            .update(cx, |s, cx| s.choose_fraction(fraction, w, cx));
    }
    pub fn value(&self, cx: &App) -> (f64, f64) {
        let value = self.slider.read(cx).value();
        (value.low, value.high.unwrap_or(value.low))
    }
    pub fn count_in_range(&self, cx: &App) -> usize {
        let (low, high) = self.value(cx);
        self.data
            .iter()
            .filter(|&&v| v.is_finite() && v >= low && v <= high)
            .count()
    }
}
impl Render for RangeHistogramSlider {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
        let (low, high) = self.value(cx);
        let label = self.slider.read(cx).label_text();
        let range_str = self.slider.read(cx).formatted_range();
        let drag_guide = self.slider.read(cx).active_drag_fraction(cx);
        let header = div()
            .w_full()
            .flex()
            .items_baseline()
            .justify_between()
            .gap(px(16.))
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(label),
            )
            .child(
                div()
                    .id("histogram-readout")
                    .debug_selector(|| "histogram-readout".into())
                    .flex_none()
                    .whitespace_nowrap()
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .font_features(FontFeatures(vec![("tnum".into(), 1)].into()))
                    .text_color(p.secondary)
                    .child(range_str),
            );
        let min = *self.bounds.start();
        let span = *self.bounds.end() - min;
        let largest = *self.bins.iter().max().unwrap_or(&0) as f32;
        let bounds = self.chart_bounds.clone();
        let mut bars = div()
            .id("histogram-chart")
            .relative()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|s, event: &MouseDownEvent, w, cx| {
                    s.choose_chart_position(event.position.x, w, cx);
                }),
            )
            .child(self.contact.listen(cx, |s, event, w, cx| {
                if event.phase == TouchPhase::Started {
                    s.choose_chart_position(event.position.x, w, cx);
                }
            }))
            .w_full()
            .h(px(76.))
            .px(px(11.))
            .flex()
            .items_end()
            .gap(px(2.));
        bars = bars.child(
            canvas(move |b, _, _| bounds.set(b), |_, _, _, _| ())
                .absolute()
                .top(px(0.))
                .bottom(px(0.))
                .left(px(11.))
                .right(px(11.)),
        );
        let count = self.bins.len();
        for (i, &value) in self.bins.iter().enumerate() {
            let bin_start = min + i as f64 * span / count as f64;
            let bin_width = span / count as f64;
            let selected_start = ((low - bin_start) / bin_width).clamp(0., 1.) as f32;
            let selected_end = ((high - bin_start) / bin_width).clamp(0., 1.) as f32;
            let height = if value == 0 || largest == 0. {
                0.
            } else {
                (value as f32 / largest * 76.).max(76. * 0.04)
            };
            bars = bars.child(
                div()
                    .relative()
                    .overflow_hidden()
                    .flex_1()
                    .min_w_0()
                    .h(px(height))
                    .rounded_tl(px(3.))
                    .rounded_tr(px(3.))
                    .rounded_bl(px(1.))
                    .rounded_br(px(1.))
                    .bg(p.ink.opacity(0.11))
                    .child(
                        div()
                            .absolute()
                            .top(px(0.))
                            .bottom(px(0.))
                            .left(relative(selected_start))
                            .w(relative((selected_end - selected_start).max(0.)))
                            .bg(p.bright),
                    ),
            );
        }
        if let Some(guide_frac) = drag_guide {
            bars = bars.child(
                div()
                    .absolute()
                    .top(px(0.))
                    .bottom(px(-14.))
                    .left(px(11.))
                    .right(px(11.))
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(relative(guide_frac))
                            .w(px(1.))
                            .bg(p.ink.opacity(0.4)),
                    ),
            );
        }
        let in_range = self.count_in_range(cx);
        let count_dir = if in_range >= self.prev_count { 1. } else { -1. };
        self.prev_count = in_range;
        let footer = div()
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .text_size(px(13.))
            .text_color(p.secondary)
            .child(lang.text(
                "Stays in Lisbon in this range",
                "里斯本符合该价格区间的房源",
            ))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(self.tally.label(in_range.to_string(), count_dir, w, cx)),
                    )
                    .child(format!("{} {}", lang.text("of", "/"), self.data.len())),
            );
        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .w_full()
            .max_w_full()
            .child(header)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(bars)
                    .child(div().mt(px(-14.)).child(self.slider.clone())),
            )
            .child(footer)
    }
}

#[cfg(test)]
mod tests {
    use super::{RangeHistogramSlider, histogram};
    use gpui::{Modifiers, TestAppContext, px, size};

    #[test]
    fn numeric_readouts_keep_range_header_geometry_after_updates_and_settling() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, w) = app.add_window_view(|w, cx| {
            RangeHistogramSlider::new(
                "Price",
                vec![80., 120., 315.],
                0.0..=500.0,
                5.,
                (80., 260.),
                32,
                w,
                cx,
            )
        });
        for width in [620., 320.] {
            w.simulate_resize(size(px(width), px(400.)));
            w.update(|win, cx| win.simulate_next_frame(cx));
            w.run_until_parked();
            let thumb = w.debug_bounds("Slider thumb 1").unwrap();
            w.simulate_click(thumb.center(), Modifiers::default());
            w.run_until_parked();
            let original = w.debug_bounds("histogram-readout").unwrap();
            for low in [85., 95., 15., 80.] {
                control.update(w, |s, cx| {
                    s.slider.update(cx, |slider, cx| {
                        slider.set_low(low, cx);
                    });
                    cx.notify();
                });
                w.simulate_keystrokes("right right right");
                for elapsed in [16, 65, 300] {
                    w.executor()
                        .advance_clock(std::time::Duration::from_millis(elapsed));
                    w.run_until_parked();
                    w.update(|win, cx| {
                        win.simulate_next_frame(cx);
                        win.refresh();
                    });
                    w.run_until_parked();
                    assert_eq!(w.debug_bounds("histogram-readout").unwrap(), original);
                }
            }
        }
    }
    #[test]
    fn bins_include_the_maximum_and_ignore_nonfinite_or_outside_values() {
        assert_eq!(
            histogram(&[-1., 0., 2., 5., 10., 11., f64::NAN], 0., 10., 2),
            vec![2, 2]
        );
    }
    #[test]
    fn invalid_inputs_return_no_bins() {
        assert!(histogram(&[1.], 0., 10., 0).is_empty());
        assert!(histogram(&[1.], 2., 2., 5).is_empty());
        assert!(histogram(&[1.], 3., 2., 5).is_empty());
        assert!(histogram(&[1.], f64::NAN, 2., 5).is_empty());
        assert!(histogram(&[1.], -f64::MAX, f64::MAX, 5).is_empty());
        assert!(histogram(&[1.], 0., 10., 4097).is_empty());
    }
}
