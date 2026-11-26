//! Multi-series KPI metric card with rolling headline, legend toggles, and pointer crosshair tooltip.
//!
//! Renders a headline figure with directional delta badge above a `PathBuilder` multi-line/area chart
//! whose series morph smoothly via [`Slide`] springs when solo-highlighted or toggled in the legend and scrub continuously with a gliding crosshair.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_card_shadow, pro_ease, pro_eid, pro_empty_state, pro_mix_color,
        pro_rolling_number, pro_skeleton_bar, pro_spring_f32, pro_swap_label, pro_tactile, tr,
    },
    primitives::{
        family::{Slide, disabled_motion},
        pro_icons::ProIcon,
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

/// Visual rendering mode for [`KpiChart`] series curves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KpiChartStyle {
    /// Crisp multi-series strokes without area wash.
    #[default]
    Line,
    /// Multi-series strokes with soft vertical gradient area washes.
    Area,
}

/// One plotted channel or segment line inside [`KpiChart`].
#[derive(Clone, Debug, PartialEq)]
pub struct KpiSeries {
    /// Stable series identifier used by legend toggles.
    pub id: SharedString,
    /// Display name shown in the legend and floating tooltip.
    pub label: SharedString,
    /// Stroke and swatch color.
    pub color: Hsla,
    /// One data value per X-axis label.
    pub values: Vec<f64>,
    /// Whether this series is initially enabled in the chart.
    pub visible: bool,
}

impl KpiSeries {
    /// Construct a visible series from `id`, `label`, `color`, and `values`.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        color: Hsla,
        values: Vec<f64>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            color,
            values,
            visible: true,
        }
    }

    /// Set initial legend visibility for this series.
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Sum all finite values in `values`.
    pub fn sum(&self) -> f64 {
        self.values.iter().copied().filter(|v| v.is_finite()).sum()
    }
}

/// Typed interaction events emitted by [`KpiChart`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KpiChartEvent {
    /// Emitted when pointer hover or keyboard navigation changes the inspected point index.
    HoverIndexChanged(Option<usize>),
    /// Emitted when a series pill in the legend is toggled on or off.
    SeriesToggled { id: SharedString, visible: bool },
    /// Emitted when switching between [`KpiChartStyle::Line`] and [`KpiChartStyle::Area`].
    StyleChanged(KpiChartStyle),
}

impl EventEmitter<KpiChartEvent> for KpiChart {}

/// Format an axis or tooltip number compactly (for example `$4.7k`, `$6k`, or `$850`).
pub fn format_kpi_value(value: f64, prefix: &str, suffix: &str) -> String {
    if !value.is_finite() {
        return format!("{prefix}0{suffix}");
    }
    let neg = value < 0.0;
    let abs = value.abs();
    let sign = if neg { "\u{2212}" } else { "" };
    let body = if abs >= 1_000_000.0 {
        let m = abs / 1_000_000.0;
        if (m.fract()).abs() < 0.05 {
            format!("{m:.0}M")
        } else {
            format!("{m:.1}M")
        }
    } else if abs >= 1_000.0 {
        let k = abs / 1_000.0;
        if (k.fract()).abs() < 0.05 {
            format!("{k:.0}k")
        } else {
            format!("{k:.1}k")
        }
    } else if (abs.fract()).abs() < 0.05 {
        format!("{abs:.0}")
    } else {
        format!("{abs:.1}")
    };
    format!("{sign}{prefix}{body}{suffix}")
}

fn nice_axis_ceiling(max_val: f64) -> f64 {
    if !max_val.is_finite() || max_val <= 0.0 {
        return 100.0;
    }
    let exp = 10f64.powf(max_val.log10().floor());
    let norm = max_val / exp;
    let step = if norm <= 1.2 {
        1.2
    } else if norm <= 2.0 {
        2.0
    } else if norm <= 3.0 {
        3.0
    } else if norm <= 4.0 {
        4.0
    } else if norm <= 5.0 {
        5.0
    } else if norm <= 6.0 {
        6.0
    } else if norm <= 8.0 {
        8.0
    } else {
        10.0
    };
    step * exp
}

fn build_smooth_points(pts: &[Point<Pixels>]) -> Vec<Point<Pixels>> {
    if pts.len() <= 2 {
        return pts.to_vec();
    }
    let subs = 6usize;
    let mut out = Vec::with_capacity((pts.len() - 1) * subs + 1);
    for i in 0..(pts.len() - 1) {
        let p0 = if i == 0 { pts[0] } else { pts[i - 1] };
        let p1 = pts[i];
        let p2 = pts[i + 1];
        let p3 = if i + 2 < pts.len() {
            pts[i + 2]
        } else {
            pts[pts.len() - 1]
        };
        let x0 = f32::from(p0.x);
        let y0 = f32::from(p0.y);
        let x1 = f32::from(p1.x);
        let y1 = f32::from(p1.y);
        let x2 = f32::from(p2.x);
        let y2 = f32::from(p2.y);
        let x3 = f32::from(p3.x);
        let y3 = f32::from(p3.y);

        let min_y = y1.min(y2);
        let max_y = y1.max(y2);

        for s in 0..subs {
            let t = s as f32 / subs as f32;
            let t2 = t * t;
            let t3 = t2 * t;
            let x = 0.5
                * ((2.0 * x1)
                    + (-x0 + x2) * t
                    + (2.0 * x0 - 5.0 * x1 + 4.0 * x2 - x3) * t2
                    + (-x0 + 3.0 * x1 - 3.0 * x2 + x3) * t3);
            let y_raw = 0.5
                * ((2.0 * y1)
                    + (-y0 + y2) * t
                    + (2.0 * y0 - 5.0 * y1 + 4.0 * y2 - y3) * t2
                    + (-y0 + 3.0 * y1 - 3.0 * y2 + y3) * t3);
            out.push(point(px(x), px(y_raw.clamp(min_y, max_y))));
        }
    }
    if let Some(&last) = pts.last() {
        out.push(last);
    }
    out
}

fn sample_polyline_y(pts: &[Point<Pixels>], target_x: Pixels) -> Option<Pixels> {
    let first = *pts.first()?;
    let last = *pts.last()?;
    let tx = f32::from(target_x);
    if tx <= f32::from(first.x) {
        return Some(first.y);
    }
    if tx >= f32::from(last.x) {
        return Some(last.y);
    }
    for seg in pts.windows(2) {
        let x0 = f32::from(seg[0].x);
        let x1 = f32::from(seg[1].x);
        if tx >= x0 && tx <= x1 {
            let span = (x1 - x0).max(1e-4);
            let u = ((tx - x0) / span).clamp(0.0, 1.0);
            let y0 = f32::from(seg[0].y);
            let y1 = f32::from(seg[1].y);
            return Some(px(y0 + (y1 - y0) * u));
        }
    }
    Some(last.y)
}

/// Multi-series KPI metric card (`kpi-chart`).
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::{prelude::*, rgb};
/// use kui::components::kpi_chart::{KpiChart, KpiChartEvent, KpiSeries};
///
/// let chart = cx.new(|cx| {
///     KpiChart::new("revenue-kpi", cx)
///         .with_title("Revenue")
///         .with_labels(["Mar 3", "Mar 4", "Mar 5", "Mar 6", "Mar 7"])
///         .with_series(vec![
///             KpiSeries::new("organic", "Organic", rgb(0x4fb0fe).into(), vec![4200., 4700., 3900., 4600., 5400.]),
///             KpiSeries::new("paid", "Paid ads", rgb(0xee3e9e).into(), vec![3600., 3300., 4400., 4800., 4000.]),
///         ])
///         .with_delta(Some(7_900.0))
///         .with_footer("Top channels")
/// });
/// cx.subscribe(&chart, |_, ev: &KpiChartEvent, _| {
///     let _ = ev;
/// }).detach();
/// # }
/// ```
pub struct KpiChart {
    id: ElementId,
    title: SharedString,
    headline_override: Option<f64>,
    delta: Option<f64>,
    prefix: SharedString,
    suffix: SharedString,
    headline_divisor: f64,
    headline_decimals: usize,
    headline_unit_suffix: SharedString,
    labels: Vec<SharedString>,
    tooltip_labels: Vec<SharedString>,
    series: Vec<KpiSeries>,
    footer: Option<SharedString>,
    default_index: Option<usize>,
    hovered_index: Option<usize>,
    hovered_series: Option<SharedString>,
    solo_series: Option<SharedString>,
    series_opacities: Vec<Slide>,
    scrub_x: Slide,
    scrub_presence: Slide,
    scrub_ready: bool,
    area_mix: Slide,
    style: KpiChartStyle,
    empty_text: Option<SharedString>,
    loading: bool,
    empty: bool,
    plot_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    focus: FocusHandle,
    started_at: Instant,
}

impl KpiChart {
    /// Create a [`KpiChart`] preloaded with the 5-channel `"Revenue"` (`$96.4k`, `+$7.9k`) dataset.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let labels = vec![
            "Mar 3".into(),
            "Mar 4".into(),
            "Mar 5".into(),
            "Mar 6".into(),
            "Mar 7".into(),
            "Mar 8".into(),
            "Mar 9".into(),
        ];
        let tooltip_labels = vec![
            "Mon, Mar 3".into(),
            "Tue, Mar 4".into(),
            "Wed, Mar 5".into(),
            "Thu, Mar 6".into(),
            "Fri, Mar 7".into(),
            "Sat, Mar 8".into(),
            "Sun, Mar 9".into(),
        ];
        // Canonical 7-day 5-channel revenue showcase series (+$1.9k baseline offset -> $96.4k).
        let series = vec![
            KpiSeries::new(
                "organic",
                "Organic",
                rgb(0x4fb0fe).into(),
                vec![4200., 4700., 3900., 4600., 5400., 5100., 5900.],
            ),
            KpiSeries::new(
                "paid",
                "Paid ads",
                rgb(0xee3e9e).into(),
                vec![3600., 3300., 4400., 4800., 4000., 4500., 4200.],
            ),
            KpiSeries::new(
                "referral",
                "Referral",
                rgb(0xa651f0).into(),
                vec![1800., 2300., 2000., 1500., 2400., 2200., 2700.],
            ),
            KpiSeries::new(
                "direct",
                "Direct",
                rgb(0x13b3a0).into(),
                vec![1200., 1100., 1700., 2100., 1600., 1300., 1500.],
            ),
            KpiSeries::new(
                "partners",
                "Partners",
                rgb(0xa3a3a3).into(),
                vec![800., 1100., 1000., 1100., 1300., 1700., 1400.],
            ),
        ];
        let series_opacities = series
            .iter()
            .map(|s| Slide::snappy(if s.visible { 1.0 } else { 0.0 }))
            .collect();

        Self {
            id: id.into(),
            title: "Revenue".into(),
            headline_override: Some(96_400.0),
            delta: Some(7_900.0),
            prefix: "$".into(),
            suffix: "".into(),
            headline_divisor: 1_000.0,
            headline_decimals: 1,
            headline_unit_suffix: "k".into(),
            labels,
            tooltip_labels,
            series,
            footer: Some("Top 5 channels".into()),
            default_index: None,
            hovered_index: None,
            hovered_series: None,
            solo_series: None,
            series_opacities,
            scrub_x: Slide::snappy(0.0),
            scrub_presence: Slide::snappy(0.0),
            scrub_ready: false,
            area_mix: Slide::new(0.0),
            style: KpiChartStyle::Line,
            empty_text: None,
            loading: false,
            empty: false,
            plot_bounds: Rc::new(Cell::new(None)),
            focus: cx.focus_handle().tab_stop(true),
            started_at: Instant::now(),
        }
    }

    /// Set the metric heading above the headline number.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Set an explicit baseline total (when all series are visible); toggling series scales proportionally.
    pub fn with_headline(mut self, total: f64) -> Self {
        self.headline_override = Some(total);
        self
    }

    /// Set or clear the period-over-period change badge (`Some(7900.0)` -> `+ $7.9k`).
    pub fn with_delta(mut self, delta: Option<f64>) -> Self {
        self.delta = delta;
        self
    }

    /// Configure value prefix/suffix and headline divisor (`1000.0`, `1`, `"k"` -> `$96.4k`).
    pub fn with_format(
        mut self,
        prefix: impl Into<SharedString>,
        suffix: impl Into<SharedString>,
        headline_divisor: f64,
        headline_decimals: usize,
        headline_unit_suffix: impl Into<SharedString>,
    ) -> Self {
        self.prefix = prefix.into();
        self.suffix = suffix.into();
        self.headline_divisor = if headline_divisor.abs() > 1e-9 {
            headline_divisor
        } else {
            1.0
        };
        self.headline_decimals = headline_decimals;
        self.headline_unit_suffix = headline_unit_suffix.into();
        self
    }

    /// Set X-axis point labels (first, middle, and last render along the axis).
    pub fn with_labels(
        mut self,
        labels: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.labels = labels.into_iter().map(Into::into).collect();
        self
    }

    /// Set longer date headings used inside the floating tooltip.
    pub fn with_tooltip_labels(
        mut self,
        labels: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.tooltip_labels = labels.into_iter().map(Into::into).collect();
        self
    }

    /// Replace the plotted [`KpiSeries`] collection.
    pub fn with_series(mut self, series: Vec<KpiSeries>) -> Self {
        self.headline_override = None;
        self.series_opacities = series
            .iter()
            .map(|s| Slide::snappy(if s.visible { 1.0 } else { 0.0 }))
            .collect();
        self.hovered_series = None;
        self.solo_series = None;
        self.series = series;
        self
    }

    /// Set the caption text inside the card footer.
    pub fn with_footer(mut self, footer: impl Into<SharedString>) -> Self {
        let s: SharedString = footer.into();
        self.footer = (!s.is_empty()).then_some(s);
        self
    }

    /// Keep a specific point index highlighted while idle.
    pub fn with_default_index(mut self, index: Option<usize>) -> Self {
        self.default_index = index;
        self
    }

    /// Choose [`KpiChartStyle::Line`] or [`KpiChartStyle::Area`].
    pub fn with_style(mut self, style: KpiChartStyle) -> Self {
        self.style = style;
        self.area_mix.reset(match style {
            KpiChartStyle::Line => 0.0,
            KpiChartStyle::Area => 1.0,
        });
        self
    }

    /// Customize the message displayed inside the plot when all values are empty or zero.
    pub fn with_empty_text(mut self, text: impl Into<SharedString>) -> Self {
        self.empty_text = Some(text.into());
        self
    }

    /// Render skeleton placeholders while data loads.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Render the empty-card state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the currently inspected point index (hovered or `default_index`).
    pub fn active_index(&self) -> Option<usize> {
        let count = self.point_count();
        if count == 0 {
            return None;
        }
        self.hovered_index
            .or(self.default_index)
            .map(|i| i.min(count - 1))
    }

    /// Toggle visibility of a series by `id`, keeping at least one series visible.
    pub fn toggle_series(&mut self, id: &str, cx: &mut Context<Self>) {
        let vis_count = self.series.iter().filter(|s| s.visible).count();
        if let Some(s) = self.series.iter_mut().find(|s| s.id.as_ref() == id) {
            if s.visible && vis_count <= 1 {
                return;
            }
            s.visible = !s.visible;
            let vis = s.visible;
            let sid = s.id.clone();
            if !vis && self.solo_series.as_deref() == Some(id) {
                self.solo_series = None;
            }
            cx.emit(KpiChartEvent::SeriesToggled {
                id: sid,
                visible: vis,
            });
            cx.notify();
        }
    }

    fn toggle_solo_series(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(s) = self.series.iter().find(|s| s.id.as_ref() == id)
            && !s.visible
        {
            self.toggle_series(id, cx);
        }
        if self.solo_series.as_deref() == Some(id) {
            self.solo_series = None;
        } else {
            self.solo_series = Some(SharedString::from(id.to_string()));
        }
        cx.notify();
    }

    fn spotlight_series_id(&self) -> Option<&SharedString> {
        self.hovered_series
            .as_ref()
            .filter(|hid| self.series.iter().any(|s| s.visible && &s.id == *hid))
            .or_else(|| {
                self.solo_series
                    .as_ref()
                    .filter(|sid| self.series.iter().any(|s| s.visible && &s.id == *sid))
            })
    }

    /// Sum finite values across all currently visible series.
    pub fn visible_total(&self) -> f64 {
        let raw_all: f64 = self.series.iter().map(KpiSeries::sum).sum();
        let raw_vis: f64 = self
            .series
            .iter()
            .filter(|s| s.visible)
            .map(KpiSeries::sum)
            .sum();
        if let Some(override_all) = self.headline_override
            && raw_all > 1e-9
        {
            if self.series.iter().all(|s| s.visible) {
                override_all
            } else {
                override_all * (raw_vis / raw_all)
            }
        } else {
            raw_vis
        }
    }

    fn bucket_or_period_total(&self, active_idx: Option<usize>) -> f64 {
        let spot = self.spotlight_series_id();
        if let Some(idx) = active_idx {
            let mut sum = 0.0;
            for s in self.series.iter().filter(|s| s.visible) {
                if spot.is_some_and(|sp| sp != &s.id) {
                    continue;
                }
                if let Some(&v) = s.values.get(idx)
                    && v.is_finite()
                {
                    sum += v;
                }
            }
            return sum;
        }
        if let Some(sp) = spot
            && let Some(s) = self.series.iter().find(|s| s.visible && &s.id == sp)
        {
            let raw_all: f64 = self.series.iter().map(KpiSeries::sum).sum();
            let s_sum = s.sum();
            if let Some(override_all) = self.headline_override
                && raw_all > 1e-9
            {
                return override_all * (s_sum / raw_all);
            }
            return s_sum;
        }
        self.visible_total()
    }

    fn visible_delta(&self) -> Option<f64> {
        let base_delta = self.delta?;
        let raw_all: f64 = self.series.iter().map(KpiSeries::sum).sum();
        let spot = self.spotlight_series_id();
        let raw_vis: f64 = self
            .series
            .iter()
            .filter(|s| s.visible && spot.is_none_or(|sp| sp == &s.id))
            .map(KpiSeries::sum)
            .sum();
        if raw_all > 1e-9 && (!self.series.iter().all(|s| s.visible) || spot.is_some()) {
            Some(base_delta * (raw_vis / raw_all))
        } else {
            Some(base_delta)
        }
    }

    fn point_count(&self) -> usize {
        let from_series = self
            .series
            .iter()
            .map(|s| s.values.len())
            .max()
            .unwrap_or(0);
        if self.labels.is_empty() {
            from_series
        } else {
            self.labels.len().min(from_series.max(self.labels.len()))
        }
    }

    fn set_scrub_index(&mut self, next: Option<usize>, cx: &mut Context<Self>) {
        if self.hovered_index == next {
            return;
        }
        let n = self.point_count();
        if let Some(idx) = next
            && n > 0
        {
            let frac = idx.min(n - 1) as f32 / (n.saturating_sub(1)).max(1) as f32;
            if !self.scrub_ready {
                self.scrub_x.reset(frac);
                self.scrub_ready = true;
            } else {
                self.scrub_x.set(frac, cx);
            }
            self.scrub_presence.set(1.0, cx);
        } else {
            self.scrub_presence.set(0.0, cx);
            if self.scrub_presence.value(cx) < 0.04 {
                self.scrub_ready = false;
            }
        }
        self.hovered_index = next;
        cx.emit(KpiChartEvent::HoverIndexChanged(next));
        cx.notify();
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let n = self.point_count();
        if n == 0 {
            return;
        }
        match ev.keystroke.key.as_str() {
            "left" => {
                let next = match self.active_index() {
                    Some(i) => i.saturating_sub(1),
                    None => n - 1,
                };
                self.set_scrub_index(Some(next), cx);
            }
            "right" => {
                let next = match self.active_index() {
                    Some(i) => (i + 1).min(n - 1),
                    None => 0,
                };
                self.set_scrub_index(Some(next), cx);
            }
            "escape" if self.hovered_index.is_some() || self.solo_series.is_some() => {
                self.solo_series = None;
                self.set_scrub_index(None, cx);
            }
            _ => {}
        }
    }
}

impl Render for KpiChart {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let dark = p.dark;
        let card_bg: Hsla = if dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let pop_bg: Hsla = if dark {
            rgb(0x252527).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border: Hsla = if dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.065)
        };
        let line: Hsla = if dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let guide_c: Hsla = if dark {
            hsla(0., 0., 1., 0.22)
        } else {
            hsla(0., 0., 0., 0.14)
        };

        if self.loading {
            let phase = if disabled_motion(cx) {
                0.25
            } else {
                w.request_animation_frame();
                (self.started_at.elapsed().as_secs_f32() / 1.4).fract()
            };
            return div()
                .id(pro_eid(&self.id, "loading"))
                .w_full()
                .max_w(px(460.))
                .rounded(px(10.))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(pro_skeleton_bar(96., 15., 4., phase, cx))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(108., 28., 6., phase, cx))
                        .child(pro_skeleton_bar(56., 20., 5., phase, cx)),
                )
                .child(pro_skeleton_bar(420., 128., 6., phase, cx))
                .child(pro_skeleton_bar(180., 14., 4., phase, cx))
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(460.))
                .rounded(px(10.))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .child(pro_empty_state::<Self>(
                    ProIcon::UsageChart,
                    tr(cx, "No KPI series", "暂无指标序列"),
                    tr(
                        cx,
                        "Provide at least one metric series to plot daily performance.",
                        "请传入至少一条时间序列以绘制趋势曲线。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let reduced = disabled_motion(cx);
        let draw_reveal = if reduced {
            1.0
        } else {
            let age = self.started_at.elapsed().as_secs_f32();
            if age < 0.72 {
                w.request_animation_frame();
                pro_ease((age / 0.72).clamp(0., 1.))
            } else {
                1.0
            }
        };

        let n_pts = self.point_count();
        let raw_max = self
            .series
            .iter()
            .filter(|s| s.visible)
            .flat_map(|s| s.values.iter().copied())
            .filter(|v| v.is_finite())
            .fold(0.0f64, f64::max);
        let plot_has_data = n_pts >= 2 && raw_max > 0.0;
        let target_ceil = nice_axis_ceiling(raw_max);
        let animated_ceil =
            pro_spring_f32(pro_eid(&self.id, "y-ceil"), target_ceil as f32, 0.34, w, cx).max(1.0)
                as f64;

        // Sync per-series Slide::snappy opacity springs (1.0 solo/normal vs 0.22 dimmed vs 0.0 hidden).
        if self.series_opacities.len() < self.series.len() {
            for s in &self.series[self.series_opacities.len()..] {
                self.series_opacities
                    .push(Slide::snappy(if s.visible { 1.0 } else { 0.0 }));
            }
        } else if self.series_opacities.len() > self.series.len() {
            self.series_opacities.truncate(self.series.len());
        }

        let spotlight_id = self.spotlight_series_id().cloned();
        let mut series_alphas = Vec::with_capacity(self.series.len());
        for (si, s) in self.series.iter().enumerate() {
            let target_alpha = if !s.visible {
                0.0
            } else if let Some(ref spot) = spotlight_id {
                if &s.id == spot { 1.0 } else { 0.22 }
            } else {
                1.0
            };
            self.series_opacities[si].set(target_alpha, cx);
            let a = self.series_opacities[si].get(w, cx).clamp(0.0, 1.0);
            series_alphas.push(a);
        }

        self.area_mix.duration(
            match self.style {
                KpiChartStyle::Line => 0.0,
                KpiChartStyle::Area => 1.0,
            },
            0.26,
            cx,
        );
        let area_alpha_mix = self.area_mix.get(w, cx).clamp(0.0, 1.0);

        let active_idx = self.active_index();
        if let Some(idx) = active_idx {
            let target_frac = idx as f32 / (n_pts.saturating_sub(1)).max(1) as f32;
            if !self.scrub_ready {
                self.scrub_x.reset(target_frac);
                self.scrub_ready = true;
            } else {
                self.scrub_x.set(target_frac, cx);
            }
            self.scrub_presence.set(1.0, cx);
        } else {
            self.scrub_presence.set(0.0, cx);
        }

        let guide_frac = self.scrub_x.get(w, cx).clamp(0.0, 1.0);
        let scrub_alpha = self.scrub_presence.get(w, cx).clamp(0.0, 1.0);

        // Dynamic rolling KPI headline: shows scrubbed bucket's total on scrub, returning to period headline on leave.
        let shown_total = self.bucket_or_period_total(active_idx.filter(|_| plot_has_data));
        let headline_scaled = shown_total / self.headline_divisor;
        let headline_suffix = format!("{}{}", self.headline_unit_suffix, self.suffix);

        let delta_badge: Option<AnyElement> =
            if let Some(idx) = active_idx.filter(|_| plot_has_data) {
                if idx > 0 {
                    let prev_tot = self.bucket_or_period_total(Some(idx - 1));
                    let diff = shown_total - prev_tot;
                    let up = diff >= 0.0;
                    let (dbg, dfg): (Hsla, Hsla) = if up {
                        if dark {
                            (hsla(145. / 360., 0.46, 0.65, 0.14), rgb(0x5fc88a).into())
                        } else {
                            (hsla(145. / 360., 0.49, 0.58, 0.10), rgb(0x1e8a4f).into())
                        }
                    } else if dark {
                        (hsla(4. / 360., 1.0, 0.69, 0.14), rgb(0xff6b5f).into())
                    } else {
                        (hsla(4. / 360., 0.86, 0.58, 0.09), rgb(0xf04438).into())
                    };
                    let sign = if up { "+ " } else { "\u{2212} " };
                    let formatted = format!(
                        "{sign}{}",
                        format_kpi_value(diff.abs(), &self.prefix, &self.suffix)
                    );
                    Some(
                        div()
                            .h(px(20.))
                            .px(px(6.))
                            .rounded(px(5.))
                            .bg(dbg)
                            .text_color(dfg)
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .flex()
                            .items_center()
                            .child(formatted)
                            .into_any_element(),
                    )
                } else {
                    None
                }
            } else {
                self.visible_delta().map(|d| {
                    let up = d >= 0.0;
                    let (dbg, dfg): (Hsla, Hsla) = if up {
                        if dark {
                            (hsla(145. / 360., 0.46, 0.65, 0.14), rgb(0x5fc88a).into())
                        } else {
                            (hsla(145. / 360., 0.49, 0.58, 0.10), rgb(0x1e8a4f).into())
                        }
                    } else if dark {
                        (hsla(4. / 360., 1.0, 0.69, 0.14), rgb(0xff6b5f).into())
                    } else {
                        (hsla(4. / 360., 0.86, 0.58, 0.09), rgb(0xf04438).into())
                    };
                    let sign = if up { "+ " } else { "\u{2212} " };
                    let formatted = format!(
                        "{sign}{}",
                        format_kpi_value(d.abs(), &self.prefix, &self.suffix)
                    );
                    div()
                        .h(px(20.))
                        .px(px(6.))
                        .rounded(px(5.))
                        .bg(dbg)
                        .text_color(dfg)
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .child(formatted)
                        .into_any_element()
                })
            };

        let context_caption: SharedString = if let Some(idx) = active_idx.filter(|_| plot_has_data)
        {
            self.tooltip_labels
                .get(idx)
                .or_else(|| self.labels.get(idx))
                .cloned()
                .unwrap_or_default()
        } else if let Some(ref spot) = spotlight_id
            && let Some(s) = self.series.iter().find(|s| &s.id == spot)
        {
            s.label.clone()
        } else {
            SharedString::default()
        };

        let style_mode = self.style;
        let wash: Hsla = if dark {
            hsla(0., 0., 1., 0.06)
        } else {
            hsla(0., 0., 0., 0.04)
        };
        let style_lbl = match style_mode {
            KpiChartStyle::Line => tr(cx, "Line", "折线"),
            KpiChartStyle::Area => tr(cx, "Area", "面积"),
        };
        let style_toggle = pro_tactile(
            pro_eid(&self.id, "style-toggle"),
            tr(cx, "Toggle line or area fill", "切换折线或面积填充"),
            6.,
            transparent_black(),
            wash,
            line,
            p.line_strong,
            style_mode == KpiChartStyle::Area,
            div()
                .h(px(24.))
                .px(px(8.))
                .flex()
                .items_center()
                .justify_center()
                .child(pro_swap_label(
                    pro_eid(&self.id, "style-lbl"),
                    style_lbl,
                    11.5,
                    FontWeight::MEDIUM,
                    p.text,
                )),
            cx,
            |this, _, cx| {
                this.style = match this.style {
                    KpiChartStyle::Line => KpiChartStyle::Area,
                    KpiChartStyle::Area => KpiChartStyle::Line,
                };
                cx.emit(KpiChartEvent::StyleChanged(this.style));
                cx.notify();
            },
        )
        .flex_none();

        let top_tick = format_kpi_value(target_ceil, &self.prefix, &self.suffix);
        let mid_tick = format_kpi_value(target_ceil * 0.5, &self.prefix, &self.suffix);
        let bot_tick = format!("{}0{}", self.prefix, self.suffix);

        let series_paint = self.series.clone();
        let alphas_paint = series_alphas.clone();
        let bounds_cell = self.plot_bounds.clone();
        let spot_for_paint = spotlight_id.clone();

        let plot_canvas = canvas(
            {
                let bounds_cell = bounds_cell.clone();
                move |bounds, _, _| {
                    bounds_cell.set(Some(bounds));
                }
            },
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 4.0 || bh <= 4.0 {
                    return;
                }
                for g in [0.0f32, 0.5, 1.0] {
                    let y = bounds.top() + px(bh * g);
                    window.paint_quad(fill(
                        Bounds::new(point(bounds.left(), y), size(bounds.size.width, px(1.))),
                        line,
                    ));
                }
                if !plot_has_data {
                    return;
                }

                let map_pt = |idx: usize, count: usize, val: f64| -> Point<Pixels> {
                    let tx = idx as f32 / (count.saturating_sub(1)).max(1) as f32;
                    let ty = (1.0 - (val / animated_ceil).clamp(0.0, 1.0)) as f32;
                    point(
                        bounds.left() + px(tx * bw),
                        bounds.top() + px(2.0 + ty * (bh - 4.0)),
                    )
                };

                let mut smooth_curves: Vec<Option<Vec<Point<Pixels>>>> =
                    Vec::with_capacity(series_paint.len());

                for (si, s) in series_paint.iter().enumerate() {
                    let alpha = alphas_paint[si];
                    if alpha <= 0.005 || s.values.len() < 2 {
                        smooth_curves.push(None);
                        continue;
                    }
                    let raw_pts: Vec<Point<Pixels>> = s
                        .values
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| {
                            let fv = if v.is_finite() { v } else { 0.0 };
                            map_pt(i, s.values.len(), fv)
                        })
                        .collect();
                    let smooth = build_smooth_points(&raw_pts);
                    let vis_len = (((smooth.len() - 1) as f32 * draw_reveal).ceil() as usize + 1)
                        .clamp(2, smooth.len());

                    if area_alpha_mix > 0.005 {
                        let mut area = PathBuilder::fill();
                        let first = smooth[0];
                        let last = smooth[vis_len - 1];
                        area.move_to(point(first.x, bounds.bottom()));
                        area.line_to(first);
                        for &pt in smooth.iter().take(vis_len).skip(1) {
                            area.line_to(pt);
                        }
                        area.line_to(point(last.x, bounds.bottom()));
                        area.close();
                        if let Ok(path) = area.build() {
                            window.paint_path(
                                path,
                                linear_gradient(
                                    180.,
                                    linear_color_stop(
                                        s.color.opacity(0.18 * alpha * area_alpha_mix),
                                        0.,
                                    ),
                                    linear_color_stop(s.color.opacity(0.0), 1.),
                                ),
                            );
                        }
                    }

                    let is_spot = spot_for_paint.as_ref() == Some(&s.id);
                    let stroke_w = if is_spot { 2.15 } else { 1.65 };
                    let mut stroke_aa = PathBuilder::stroke(px(stroke_w + 1.5));
                    let mut stroke = PathBuilder::stroke(px(stroke_w));
                    for (pi, &pt) in smooth.iter().take(vis_len).enumerate() {
                        if pi == 0 {
                            stroke_aa.move_to(pt);
                            stroke.move_to(pt);
                        } else {
                            stroke_aa.line_to(pt);
                            stroke.line_to(pt);
                        }
                    }
                    if let Ok(path_aa) = stroke_aa.build() {
                        window.paint_path(path_aa, s.color.opacity(alpha * 0.22));
                    }
                    if let Ok(path) = stroke.build() {
                        window.paint_path(path, s.color.opacity(alpha));
                    }
                    smooth_curves.push(Some(smooth));
                }

                if scrub_alpha > 0.01 {
                    let gx = bounds.left() + px(guide_frac.clamp(0., 1.) * bw);
                    window.paint_quad(fill(
                        Bounds::new(point(gx, bounds.top() - px(4.)), size(px(1.), px(bh + 4.))),
                        guide_c.opacity(guide_c.a * scrub_alpha),
                    ));
                    for (si, s) in series_paint.iter().enumerate() {
                        let alpha = alphas_paint[si] * scrub_alpha;
                        if alpha <= 0.05 {
                            continue;
                        }
                        let Some(ref curve) = smooth_curves[si] else {
                            continue;
                        };
                        let Some(gy) = sample_polyline_y(curve, gx) else {
                            continue;
                        };
                        let dot_pt = point(gx, gy);
                        let is_spot = spot_for_paint.as_ref() == Some(&s.id);
                        let (halo_r, inner_r) = if is_spot { (6.2, 4.25) } else { (5.5, 3.75) };
                        window.paint_quad(
                            fill(
                                Bounds::new(
                                    point(dot_pt.x - px(halo_r), dot_pt.y - px(halo_r)),
                                    size(px(halo_r * 2.), px(halo_r * 2.)),
                                ),
                                card_bg.opacity(scrub_alpha),
                            )
                            .corner_radii(px(99.)),
                        );
                        window.paint_quad(
                            fill(
                                Bounds::new(
                                    point(dot_pt.x - px(inner_r), dot_pt.y - px(inner_r)),
                                    size(px(inner_r * 2.), px(inner_r * 2.)),
                                ),
                                s.color.opacity(alpha),
                            )
                            .corner_radii(px(99.)),
                        );
                    }
                }
            },
        )
        .size_full();

        let tooltip_overlay: Option<AnyElement> = active_idx.filter(|_| plot_has_data).map(|idx| {
            let head_lbl = self
                .tooltip_labels
                .get(idx)
                .or_else(|| self.labels.get(idx))
                .cloned()
                .unwrap_or_else(|| format!("Day {}", idx + 1).into());
            let flip_left = guide_frac > 0.5;

            let mut rows = div()
                .mt(px(4.))
                .pt(px(4.))
                .border_t_1()
                .border_color(line)
                .flex()
                .flex_col()
                .gap(px(1.));
            for (si, s) in self.series.iter().enumerate() {
                if !s.visible {
                    continue;
                }
                let val = s.values.get(idx).copied().unwrap_or(0.0);
                let row_alpha = series_alphas[si].clamp(0.32, 1.0);
                rows = rows.child(
                    div()
                        .h(px(20.))
                        .opacity(row_alpha)
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(12.))
                        .child(div().size(px(8.)).rounded(px(2.)).bg(s.color))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_color(p.text)
                                .truncate()
                                .child(s.label.clone()),
                        )
                        .child(
                            div()
                                .pl(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(format_kpi_value(val, &self.prefix, &self.suffix)),
                        ),
                );
            }

            let tip = div()
                .w(px(166.))
                .px(px(10.))
                .py(px(7.))
                .rounded(px(10.))
                .bg(pop_bg)
                .border_1()
                .border_color(p.line_strong)
                .shadow_lg()
                .child(pro_swap_label(
                    pro_eid(&self.id, "tip-date"),
                    head_lbl,
                    13.,
                    FontWeight::MEDIUM,
                    p.strong,
                ))
                .child(rows);

            let plot_w = self
                .plot_bounds
                .get()
                .map(|b| f32::from(b.size.width))
                .unwrap_or(360.);
            let x_px = guide_frac * plot_w;
            let tip_left = if flip_left {
                (x_px - 166. - 12.).max(0.)
            } else {
                (x_px + 12.).min((plot_w - 166.).max(0.))
            };

            div()
                .absolute()
                .top(px(-8. + 3. * (1. - scrub_alpha)))
                .left(px(tip_left))
                .opacity(scrub_alpha)
                .child(tip)
                .into_any_element()
        });

        let empty_plot_label: Option<AnyElement> = (!plot_has_data).then(|| {
            let msg = self
                .empty_text
                .clone()
                .unwrap_or_else(|| tr(cx, "No data yet", "暂无数据").into());
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .px(px(10.))
                        .py(px(2.))
                        .rounded(px(4.))
                        .bg(card_bg)
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(msg),
                )
                .into_any_element()
        });

        let bounds_for_move = self.plot_bounds.clone();
        let plot_box = div()
            .id(pro_eid(&self.id, "plot"))
            .w_full()
            .flex_none()
            .h(px(128.))
            .relative()
            .cursor(if plot_has_data {
                CursorStyle::Crosshair
            } else {
                CursorStyle::Arrow
            })
            .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, _, cx| {
                let n = this.point_count();
                if n < 2 {
                    return;
                }
                let Some(b) = bounds_for_move.get() else {
                    return;
                };
                let bw = f32::from(b.size.width).max(1.0);
                let rel_x = f32::from(ev.position.x - b.left()).clamp(0.0, bw);
                let idx = ((rel_x / bw) * (n - 1) as f32).round() as usize;
                let idx = idx.min(n - 1);
                this.set_scrub_index(Some(idx), cx);
            }))
            .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                if !*inside && this.hovered_index.is_some() {
                    this.set_scrub_index(None, cx);
                }
            }))
            .child(plot_canvas)
            .when_some(empty_plot_label, |d, el| d.child(el))
            .when_some(tooltip_overlay, |d, tip| d.child(tip));

        let first_lbl = self.labels.first().cloned().unwrap_or_default();
        let mid_lbl = if self.labels.len() >= 3 {
            self.labels.get(self.labels.len() / 2).cloned()
        } else {
            None
        };
        let last_lbl = if self.labels.len() >= 2 {
            self.labels.last().cloned()
        } else {
            None
        };

        let mut legend_wrap = div().flex().flex_wrap().items_center().gap(px(5.));
        for (si, s) in self.series.iter().enumerate() {
            let sid = s.id.clone();
            let sid_hov = s.id.clone();
            let sid_click = s.id.clone();
            let vis = s.visible;
            let sw_c = s.color;
            let is_solo = self.solo_series.as_ref() == Some(&sid);
            let is_spot = spotlight_id.as_ref() == Some(&sid);
            let pill_alpha = if !vis {
                0.38
            } else {
                (0.45 + 0.55 * series_alphas[si]).clamp(0.45, 1.0)
            };
            let rest_bg = if is_solo {
                pro_mix_color(transparent_black(), sw_c, if dark { 0.18 } else { 0.11 })
            } else {
                transparent_black()
            };
            let hov_bg = pro_mix_color(wash, sw_c, if dark { 0.22 } else { 0.14 });
            let rest_bdr = if is_solo {
                sw_c.opacity(0.42)
            } else {
                transparent_black()
            };
            let hov_bdr = sw_c.opacity(0.32);

            let pill_content = div()
                .h(px(23.))
                .px(px(7.))
                .flex()
                .items_center()
                .gap(px(5.))
                .whitespace_nowrap()
                .child(div().size(px(7.)).flex_none().rounded(px(2.)).bg(sw_c))
                .child(
                    div()
                        .text_size(px(11.5))
                        .font_weight(if is_spot {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::MEDIUM
                        })
                        .text_color(if is_spot {
                            p.strong
                        } else if vis {
                            p.text
                        } else {
                            p.muted
                        })
                        .child(s.label.clone()),
                );

            legend_wrap = legend_wrap.child(
                pro_tactile(
                    pro_eid(&self.id, format!("leg-{sid}")),
                    s.label.clone(),
                    6.,
                    rest_bg,
                    hov_bg,
                    rest_bdr,
                    hov_bdr,
                    is_solo,
                    pill_content,
                    cx,
                    move |this, _, cx| {
                        this.toggle_solo_series(&sid_click, cx);
                    },
                )
                .flex_none()
                .opacity(pill_alpha)
                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                    if *inside {
                        this.hovered_series = Some(sid_hov.clone());
                    } else if this.hovered_series.as_ref() == Some(&sid_hov) {
                        this.hovered_series = None;
                    }
                    cx.notify();
                })),
            );
        }

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .role(Role::Group)
            .aria_label(self.title.clone())
            .w_full()
            .max_w(px(460.))
            .rounded(px(10.))
            .bg(card_bg)
            .border_1()
            .border_color(card_border)
            .shadow(pro_card_shadow(dark))
            .on_key_down(cx.listener(Self::handle_key_down))
            .child(
                div()
                    .px(px(16.))
                    .pt(px(14.))
                    .pb(px(12.))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(15.))
                                    .line_height(px(24.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .truncate()
                                    .child(self.title.clone()),
                            )
                            .child(style_toggle),
                    )
                    .child(
                        div()
                            .mt(px(6.))
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(8.))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "headline"),
                                headline_scaled,
                                self.headline_decimals,
                                self.prefix.clone(),
                                headline_suffix,
                                24.,
                                FontWeight::MEDIUM,
                                p.strong,
                            ))
                            .when_some(delta_badge, |d, b| d.child(b))
                            .when(!context_caption.is_empty(), |d| {
                                d.child(pro_swap_label(
                                    pro_eid(&self.id, "ctx-cap"),
                                    context_caption,
                                    12.,
                                    FontWeight::MEDIUM,
                                    p.muted,
                                ))
                            }),
                    )
                    .child(
                        div()
                            .mt(px(18.))
                            .flex()
                            .items_start()
                            .gap(px(10.))
                            .child(
                                div()
                                    .w(px(32.))
                                    .h(px(128.))
                                    .flex_none()
                                    .flex()
                                    .flex_col()
                                    .justify_between()
                                    .items_end()
                                    .text_size(px(11.))
                                    .line_height(px(14.))
                                    .text_color(p.muted)
                                    .child(top_tick)
                                    .child(mid_tick)
                                    .child(bot_tick),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(8.))
                                    .child(plot_box)
                                    .child(
                                        div()
                                            .w_full()
                                            .flex_none()
                                            .h(px(16.))
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .text_size(px(11.))
                                            .line_height(px(16.))
                                            .text_color(p.muted)
                                            .child(first_lbl)
                                            .when_some(mid_lbl, |d, m| d.child(m))
                                            .when_some(last_lbl, |d, l| d.child(l)),
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .px(px(16.))
                    .py(px(10.))
                    .border_t_1()
                    .border_color(line)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .when_some(self.footer.clone(), |d, ft| {
                        d.child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(ft),
                        )
                    })
                    .child(legend_wrap),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{KpiChart, format_kpi_value};
    use gpui::{AppContext, TestAppContext, px, size};
    use std::time::Duration;

    #[test]
    fn compact_kpi_formatter_formats_thousands_and_negatives() {
        assert_eq!(format_kpi_value(6000.0, "$", ""), "$6k");
        assert_eq!(format_kpi_value(7900.0, "$", ""), "$7.9k");
        assert_eq!(format_kpi_value(-2400.0, "$", ""), "\u{2212}$2.4k");
    }

    #[gpui::test]
    fn toggling_series_updates_visible_total_and_keeps_at_least_one(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let chart = cx.new(|cx| KpiChart::new("kpi-test", cx));
        chart.update(cx, |c, cx| {
            let full = c.visible_total();
            assert!((full - 96_400.0).abs() < 1.0);
            c.toggle_series("organic", cx);
            assert!(c.visible_total() < full);

            c.toggle_series("paid", cx);
            c.toggle_series("referral", cx);
            c.toggle_series("direct", cx);
            c.toggle_series("partners", cx);
            // Last remaining series must stay visible.
            assert!(c.visible_total() > 0.0);
        });
    }

    #[gpui::test]
    fn scrub_x_glides_between_buckets_and_solo_series_dims_others(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let exec = cx.executor();
        let (chart, w) = cx.add_window_view(|_, cx| KpiChart::new("kpi-scrub", cx));
        w.simulate_resize(size(px(500.), px(400.)));
        w.run_until_parked();

        // Enter scrub at bucket 1, then scrub to bucket 5 -> scrub_x glides smoothly.
        w.update(|_, app| {
            chart.update(app, |c, cx| {
                c.set_scrub_index(Some(1), cx);
                assert!((c.bucket_or_period_total(Some(1)) - 12_500.0).abs() < 1.0);
                c.set_scrub_index(Some(5), cx);
                c.toggle_solo_series("organic", cx);
            });
        });
        w.run_until_parked();

        exec.advance_clock(Duration::from_millis(45));
        w.update(|win, app| win.simulate_next_frame(app));
        w.run_until_parked();

        w.update(|_, app| {
            let c = chart.read(app);
            let gx = c.scrub_x.value(app);
            assert!(
                gx > 1.0 / 6.0 && gx < 5.0 / 6.0,
                "expected gliding scrub_x, got {gx}"
            );
            let dimmed = c.series_opacities[1].value(app);
            assert!(
                dimmed < 0.95 && dimmed > 0.22,
                "expected snappy dim toward 0.22, got {dimmed}"
            );
        });
    }
}
