//! Daily usage metering bars with blue-to-lilac gradients, live Today bar, and rolling totals.
//!
//! Renders per-day usage slots drifting through soft blue, periwinkle, and lilac vertical gradients,
//! Dock-like `Slide::snappy` proximity highlights, X-gliding breakdown tooltips, click-to-pin inspection, and rolling totals.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_card_shadow, pro_ease, pro_eid, pro_empty_state, pro_mix_color,
        pro_rolling_number, pro_skeleton_bar, pro_spring_f32, pro_swap_label, pro_tactile, tr,
    },
    primitives::{
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_app_artwork, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const GITHUB_MINI_SVG: &str = r#"<path d="M12 .3a12 12 0 0 0-3.8 23.4c.6.1.8-.3.8-.6v-2c-3.3.7-4-1.6-4-1.6-.6-1.4-1.4-1.8-1.4-1.8-1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1.1 1.8 2.8 1.3 3.5 1 .1-.8.4-1.3.8-1.6-2.7-.3-5.5-1.3-5.5-5.9 0-1.3.5-2.4 1.2-3.2-.1-.3-.5-1.5.1-3.2 0 0 1-.3 3.3 1.2a11.5 11.5 0 0 1 6 0c2.3-1.5 3.3-1.2 3.3-1.2.6 1.7.2 2.9.1 3.2.8.8 1.2 1.9 1.2 3.2 0 4.6-2.8 5.6-5.5 5.9.4.4.8 1.1.8 2.2v3.3c0 .3.2.7.8.6A12 12 0 0 0 12 .3Z" fill="currentColor"></path>"#;
const NOTION_MINI_SVG: &str = r#"<path d="M4.5 4.2c.7.6 1 .6 2.4.5l13.2-.8c.3 0 0-.3 0-.3L17.9 2c-.4-.3-1-.7-2.1-.6L3 2.3c-.5 0-.6.3-.4.5zm.8 3.1v13.9c0 .7.4 1 1.2 1l14.5-.8c.8 0 .9-.6.9-1.2V6.4c0-.6-.2-.9-.7-.9L6 6.4c-.6 0-.7.3-.7.9zm14.3.7c.1.4 0 .8-.4.9l-.7.1v10.3c-.6.3-1.2.5-1.6.5-.8 0-1-.2-1.6-1l-4.9-7.7v7.5l1.5.3s0 .8-1.2.8l-3.3.2c-.1-.2 0-.7.3-.8l.9-.2V9.4L7.4 9.3c-.1-.4.1-1 .8-1.1l3.6-.2 5.1 7.8V8.9l-1.3-.1c-.1-.5.3-.9.7-.9z" fill="currentColor"></path>"#;
const FIGMA_TINY_SVG: &str = r##"<path d="M8 12a4 4 0 1 1 8 0 4 4 0 0 1-8 0Z" fill="#1ABCFE"></path><path d="M0 20a4 4 0 0 1 4-4h4v4a4 4 0 1 1-8 0Z" fill="#0ACF83"></path><path d="M8 0v8h4a4 4 0 1 0 0-8H8Z" fill="#FF7262"></path><path d="M0 4a4 4 0 0 0 4 4h4V0H4a4 4 0 0 0-4 4Z" fill="#F24E1E"></path><path d="M0 12a4 4 0 0 0 4 4h4V8H4a4 4 0 0 0-4 4Z" fill="#A259FF"></path>"##;

/// Tool or model share entry inside a [`MeteringDay`] breakdown tooltip.
#[derive(Clone, Debug, PartialEq)]
pub struct MeteringBreakdownPart {
    /// Part name (for example `"GitHub"`, `"Slack"`, `"Notion"`).
    pub label: SharedString,
    /// Raw usage count for this part on that day.
    pub value: f64,
    /// Optional app/tool identifier for rendering a leading icon mark.
    pub logo: Option<SharedString>,
}

impl MeteringBreakdownPart {
    /// Construct a breakdown entry with `label` and `value`.
    pub fn new(label: impl Into<SharedString>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            logo: None,
        }
    }

    /// Attach an app/brand logo identifier (such as `"github"`, `"slack"`, `"notion"`, `"figma"`).
    pub fn with_logo(mut self, logo: impl Into<SharedString>) -> Self {
        self.logo = Some(logo.into());
        self
    }
}

/// One daily metering bar slot in [`MeteringChart`].
#[derive(Clone, Debug, PartialEq)]
pub struct MeteringDay {
    /// Date label (for example `"Sep 1"` or `"Sep 21"`).
    pub label: SharedString,
    /// Daily usage count; `None` or non-finite leaves an empty slot.
    pub value: Option<f64>,
    /// Optional per-tool or per-model composition shown as whole percentages on hover.
    pub breakdown: Vec<MeteringBreakdownPart>,
}

impl MeteringDay {
    /// Construct a daily metering entry with `label` and `value`.
    pub fn new(label: impl Into<SharedString>, value: f64) -> Self {
        Self {
            label: label.into(),
            value: value.is_finite().then_some(value.max(0.0)),
            breakdown: Vec::new(),
        }
    }

    /// Construct an empty missing-data slot for `label`.
    pub fn empty(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            value: None,
            breakdown: Vec::new(),
        }
    }

    /// Attach a per-tool or per-model breakdown list for the hover tag.
    pub fn with_breakdown(mut self, breakdown: Vec<MeteringBreakdownPart>) -> Self {
        self.breakdown = breakdown;
        self
    }
}

/// Rolling summary figure displayed beneath the [`MeteringChart`] title.
#[derive(Clone, Debug, PartialEq)]
pub struct MeteringTotal {
    /// Stable identifier for the rolling counter state.
    pub id: SharedString,
    /// Caption shown to the left of the rolling figure (for example `"Total volume"`).
    pub label: SharedString,
    /// Raw figure value.
    pub value: f64,
    /// Scale divisor applied before formatting (for example `1000.0` with `"k"` suffix).
    pub divisor: f64,
    /// Decimal places rendered by [`pro_rolling_number`].
    pub decimals: usize,
    /// Prefix shown before the number (for example `"$"`).
    pub prefix: SharedString,
    /// Suffix shown after the number (for example `"k"`).
    pub suffix: SharedString,
}

impl MeteringTotal {
    /// Construct a summary counter with `id`, `label`, and `value`.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>, value: f64) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value,
            divisor: 1.0,
            decimals: 0,
            prefix: "".into(),
            suffix: "".into(),
        }
    }

    /// Configure display scaling and affixes (`divisor`, `decimals`, `prefix`, `suffix`).
    pub fn with_format(
        mut self,
        divisor: f64,
        decimals: usize,
        prefix: impl Into<SharedString>,
        suffix: impl Into<SharedString>,
    ) -> Self {
        self.divisor = if divisor.abs() > 1e-9 { divisor } else { 1.0 };
        self.decimals = decimals;
        self.prefix = prefix.into();
        self.suffix = suffix.into();
        self
    }
}

/// Typed events emitted by [`MeteringChart`].
#[derive(Clone, Debug, PartialEq)]
pub enum MeteringChartEvent {
    /// Emitted when pointer hover, pin toggle, or keyboard navigation changes the inspected day slot.
    HoverDayChanged(Option<usize>),
    /// Emitted when a live metering increment updates Today's bar and rolling totals.
    LiveIncremented { day_index: usize, value: f64 },
}

impl EventEmitter<MeteringChartEvent> for MeteringChart {}

/// Fold `parts` into at most `limit` largest entries plus `other_label`, returning whole percentages summing to `100`.
pub fn compute_whole_percentages(
    parts: &[MeteringBreakdownPart],
    limit: usize,
    other_label: &str,
) -> Vec<(SharedString, Option<SharedString>, u32)> {
    let mut valid: Vec<MeteringBreakdownPart> = parts
        .iter()
        .filter(|p| p.value.is_finite() && p.value > 0.0)
        .cloned()
        .collect();
    if valid.is_empty() {
        return Vec::new();
    }
    valid.sort_by(|a, b| {
        b.value
            .partial_cmp(&a.value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let limit = limit.max(1);
    let folded: Vec<(SharedString, Option<SharedString>, f64)> = if valid.len() > limit + 1 {
        let mut top: Vec<_> = valid
            .iter()
            .take(limit)
            .map(|p| (p.label.clone(), p.logo.clone(), p.value))
            .collect();
        let rest_sum: f64 = valid.iter().skip(limit).map(|p| p.value).sum();
        if rest_sum > 0.0 {
            top.push((SharedString::from(other_label.to_string()), None, rest_sum));
        }
        top
    } else {
        valid
            .into_iter()
            .map(|p| (p.label, p.logo, p.value))
            .collect()
    };

    let total: f64 = folded.iter().map(|(_, _, v)| *v).sum();
    if total <= 1e-9 {
        return Vec::new();
    }

    // Largest-remainder method so whole percentages always sum cleanly to 100%.
    let mut floors: Vec<u32> = Vec::with_capacity(folded.len());
    let mut rems: Vec<(usize, f64)> = Vec::with_capacity(folded.len());
    let mut sum_floors = 0u32;
    for (i, (_, _, v)) in folded.iter().enumerate() {
        let exact = (*v / total) * 100.0;
        let fl = exact.floor() as u32;
        floors.push(fl);
        sum_floors += fl;
        rems.push((i, exact - fl as f64));
    }
    rems.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let deficit = 100u32.saturating_sub(sum_floors) as usize;
    for &(idx, _) in rems.iter().take(deficit) {
        floors[idx] += 1;
    }

    folded
        .into_iter()
        .zip(floors)
        .map(|((lbl, logo, _), pct)| (lbl, logo, pct))
        .collect()
}

fn format_axis_short(val: f64) -> String {
    if val <= 0.0 || !val.is_finite() {
        return "0".to_string();
    }
    if val >= 1_000_000.0 {
        let m = val / 1_000_000.0;
        if m.fract().abs() < 0.05 {
            format!("{m:.0}M")
        } else {
            format!("{m:.1}M")
        }
    } else if val >= 1_000.0 {
        let k = val / 1_000.0;
        if k.fract().abs() < 0.05 {
            format!("{k:.0}k")
        } else {
            format!("{k:.1}k")
        }
    } else {
        format!("{val:.0}")
    }
}

fn nice_metering_ceiling(max_val: f64) -> f64 {
    if !max_val.is_finite() || max_val <= 0.0 {
        return 4500.0;
    }
    // Choose a ceiling cleanly divisible by 3 for the 3-interval (4-tick) dashed grid.
    let exp = 10f64.powf(max_val.log10().floor());
    let norm = max_val / exp;
    let mult = if norm <= 1.5 {
        1.5
    } else if norm <= 3.0 {
        3.0
    } else if norm <= 4.5 {
        4.5
    } else if norm <= 6.0 {
        6.0
    } else if norm <= 9.0 {
        9.0
    } else {
        12.0
    };
    mult * exp
}

fn sample_color_ramp(stops: &[Hsla], t: f32) -> Hsla {
    if stops.is_empty() {
        return rgb(0x8aa8ff).into();
    }
    if stops.len() == 1 {
        return stops[0];
    }
    let pos = t.clamp(0., 1.) * (stops.len() - 1) as f32;
    let i0 = (pos.floor() as usize).min(stops.len() - 1);
    let i1 = (i0 + 1).min(stops.len() - 1);
    pro_mix_color(stops[i0], stops[i1], pos - i0 as f32)
}

/// Proximity curve (`0.0..=1.0`) giving macOS Dock / audio-meter smooth brightness & lift across adjacent bars.
fn dock_proximity(distance_slots: f32) -> f32 {
    let t = (1.0 - distance_slots.abs() / 2.65).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Daily usage metering bar chart component (`metering-chart`).
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::metering_chart::{MeteringChart, MeteringChartEvent, MeteringDay, MeteringTotal};
///
/// let chart = cx.new(|cx| {
///     MeteringChart::new("api-calls", cx)
///         .with_title("API calls")
///         .with_live(true)
///         .with_period("Sep 1 – Sep 21")
///         .with_data(vec![
///             MeteringDay::new("Sep 1", 1870.),
///             MeteringDay::new("Sep 2", 2290.),
///             MeteringDay::new("Today", 2860.),
///         ])
///         .with_totals(vec![
///             MeteringTotal::new("vol", "Total volume", 50.39).with_format(1.0, 2, "$", "k"),
///             MeteringTotal::new("evt", "Total events", 36.0).with_format(1.0, 1, "", "k"),
///         ])
/// });
/// cx.subscribe(&chart, |_, ev: &MeteringChartEvent, _| {
///     let _ = ev;
/// }).detach();
/// # }
/// ```
pub struct MeteringChart {
    id: ElementId,
    title: SharedString,
    data: Vec<MeteringDay>,
    totals: Vec<MeteringTotal>,
    period: Option<SharedString>,
    live: bool,
    badge: Option<SharedString>,
    colors: Vec<Hsla>,
    accent: Hsla,
    height: f32,
    breakdown_limit: usize,
    other_label: SharedString,
    empty_text: Option<SharedString>,
    hovered_day: Option<usize>,
    pinned_day: Option<usize>,
    pointer_slot: Option<f32>,
    hover_slides: Vec<Slide>,
    spotlight_mix: Slide,
    tooltip_x: Slide,
    tooltip_presence: Slide,
    tooltip_ready: bool,
    loading: bool,
    empty: bool,
    plot_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    focus: FocusHandle,
    started_at: Instant,
}

impl MeteringChart {
    /// Create a [`MeteringChart`] preloaded with 21 days (`Sep 1 – Sep 21`) of live `"API calls"` metering data.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let daily_counts = [
            1870., 2290., 2860., 2410., 3120., 1640., 1490., 2980., 3340., 3680., 3420., 3890.,
            2150., 1920., 3510., 3960., 4180., 3740., 4090., 2680., 3240.,
        ];
        let mut data = Vec::with_capacity(daily_counts.len());
        for (i, &val) in daily_counts.iter().enumerate() {
            let w0 = 0.32 + 0.06 * ((i as f64 * 0.7).sin());
            let w1 = 0.27 + 0.04 * ((i as f64 * 1.1).cos());
            let w2 = 0.15 + 0.03 * ((i as f64 * 0.5).sin());
            let w3 = 0.10;
            let w4 = 0.08;
            let w5 = (1.0 - (w0 + w1 + w2 + w3 + w4)).max(0.05);
            let breakdown = vec![
                MeteringBreakdownPart::new("GitHub", val * w0).with_logo("github"),
                MeteringBreakdownPart::new("Slack", val * w1).with_logo("slack"),
                MeteringBreakdownPart::new("Notion", val * w2).with_logo("notion"),
                MeteringBreakdownPart::new("Figma", val * w3).with_logo("figma"),
                MeteringBreakdownPart::new("Stripe", val * w4).with_logo("stripe"),
                MeteringBreakdownPart::new("Drive", val * w5).with_logo("drive"),
            ];
            let label = if i + 1 == daily_counts.len() {
                "Today".to_string()
            } else {
                format!("Sep {}", i + 1)
            };
            data.push(MeteringDay::new(label, val).with_breakdown(breakdown));
        }

        let hover_slides = (0..data.len()).map(|_| Slide::snappy(0.0)).collect();
        let totals = vec![
            MeteringTotal::new("vol", "Total volume", 52.05).with_format(1.0, 2, "$", "k"),
            MeteringTotal::new("evt", "Total events", 62.46).with_format(1.0, 1, "", "k"),
        ];

        Self {
            id: id.into(),
            title: "API calls".into(),
            data,
            totals,
            period: Some("Sep 1 \u{2013} Sep 21".into()),
            live: true,
            badge: None,
            colors: vec![
                rgb(0x78b7ff).into(),
                rgb(0x9da8ff).into(),
                rgb(0xc19bff).into(),
                rgb(0xdc94ef).into(),
            ],
            accent: rgb(0x5e5cff).into(),
            height: 168.,
            breakdown_limit: 4,
            other_label: "Other".into(),
            empty_text: None,
            hovered_day: None,
            pinned_day: None,
            pointer_slot: None,
            hover_slides,
            spotlight_mix: Slide::snappy(0.0),
            tooltip_x: Slide::snappy(0.0),
            tooltip_presence: Slide::snappy(0.0),
            tooltip_ready: false,
            loading: false,
            empty: false,
            plot_bounds: Rc::new(Cell::new(None)),
            focus: cx.focus_handle().tab_stop(true),
            started_at: Instant::now(),
        }
    }

    /// Set the card heading.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Replace the daily usage bars (oldest first; when `live` is true, the last bar represents Today).
    pub fn with_data(mut self, data: Vec<MeteringDay>) -> Self {
        self.hover_slides = (0..data.len()).map(|_| Slide::snappy(0.0)).collect();
        self.data = data;
        self.hovered_day = None;
        self.pinned_day = None;
        self.pointer_slot = None;
        self
    }

    /// Replace the rolling summary totals displayed beneath the heading.
    pub fn with_totals(mut self, totals: Vec<MeteringTotal>) -> Self {
        self.totals = totals;
        self
    }

    /// Set or clear the period caption on the top right (for example `"Sep 1 – Sep 21"`).
    pub fn with_period(mut self, period: impl Into<SharedString>) -> Self {
        let s: SharedString = period.into();
        self.period = (!s.is_empty()).then_some(s);
        self
    }

    /// Enable or disable live Today styling (`"Real-time"` badge + accented growing last bar).
    pub fn with_live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }

    /// Override the badge text next to the title (defaults to `"Real-time"` when `live == true`).
    pub fn with_badge(mut self, badge: impl Into<SharedString>) -> Self {
        let s: SharedString = badge.into();
        self.badge = (!s.is_empty()).then_some(s);
        self
    }

    /// Set the horizontal color stops that historical bars drift through from left to right.
    pub fn with_colors(mut self, colors: Vec<Hsla>) -> Self {
        if !colors.is_empty() {
            self.colors = colors;
        }
        self
    }

    /// Set the accent color used for the live Today bar.
    pub fn with_accent(mut self, accent: Hsla) -> Self {
        self.accent = accent;
        self
    }

    /// Set the plot height in pixels (defaults to `168.0`).
    pub fn with_height(mut self, height: f32) -> Self {
        self.height = height.clamp(96., 320.);
        self
    }

    /// Configure how many breakdown parts display individually before folding into `other_label`.
    pub fn with_breakdown_limit(mut self, limit: usize) -> Self {
        self.breakdown_limit = limit.max(1);
        self
    }

    /// Customize the folded remainder row label (`"Other"` by default).
    pub fn with_other_label(mut self, label: impl Into<SharedString>) -> Self {
        self.other_label = label.into();
        self
    }

    /// Customize the message shown when all daily slots are empty.
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

    /// Return the currently inspected day index (hovered or pinned), if any.
    pub fn active_day(&self) -> Option<usize> {
        self.hovered_day
            .or(self.pinned_day)
            .filter(|&i| i < self.data.len())
    }

    fn toggle_pin_day(&mut self, day_idx: usize, cx: &mut Context<Self>) {
        if day_idx >= self.data.len() {
            return;
        }
        if self.pinned_day == Some(day_idx) {
            self.pinned_day = None;
        } else {
            self.pinned_day = Some(day_idx);
        }
        cx.emit(MeteringChartEvent::HoverDayChanged(self.active_day()));
        cx.notify();
    }

    /// Increment Today's live bar and update rolling totals smoothly.
    pub fn record_live_burst(&mut self, add_calls: f64, cx: &mut Context<Self>) {
        let Some(last_idx) = self.data.len().checked_sub(1) else {
            return;
        };
        let cur = self.data[last_idx].value.unwrap_or(0.0);
        let next = (cur + add_calls.max(0.0)).min(999_999.0);
        self.data[last_idx].value = Some(next);
        if let Some(part) = self.data[last_idx].breakdown.first_mut() {
            part.value += add_calls.max(0.0);
        }
        if let Some(t0) = self.totals.first_mut() {
            t0.value += add_calls * 0.00084;
        }
        if let Some(t1) = self.totals.get_mut(1) {
            t1.value += add_calls / 1000.0;
        }
        cx.emit(MeteringChartEvent::LiveIncremented {
            day_index: last_idx,
            value: next,
        });
        cx.notify();
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.data.is_empty() {
            return;
        }
        let max_i = self.data.len() - 1;
        match ev.keystroke.key.as_str() {
            "left" => {
                let next = match self.active_day() {
                    Some(i) => i.saturating_sub(1),
                    None => max_i,
                };
                self.hovered_day = Some(next);
                self.pointer_slot = Some(next as f32);
                cx.emit(MeteringChartEvent::HoverDayChanged(Some(next)));
                cx.notify();
            }
            "right" => {
                let next = match self.active_day() {
                    Some(i) => (i + 1).min(max_i),
                    None => 0,
                };
                self.hovered_day = Some(next);
                self.pointer_slot = Some(next as f32);
                cx.emit(MeteringChartEvent::HoverDayChanged(Some(next)));
                cx.notify();
            }
            "enter" | "space" => {
                if let Some(cur) = self.active_day() {
                    self.toggle_pin_day(cur, cx);
                }
            }
            "home" => {
                self.hovered_day = Some(0);
                self.pointer_slot = Some(0.0);
                cx.emit(MeteringChartEvent::HoverDayChanged(Some(0)));
                cx.notify();
            }
            "end" => {
                self.hovered_day = Some(max_i);
                self.pointer_slot = Some(max_i as f32);
                cx.emit(MeteringChartEvent::HoverDayChanged(Some(max_i)));
                cx.notify();
            }
            "escape" if self.hovered_day.is_some() || self.pinned_day.is_some() => {
                self.hovered_day = None;
                self.pinned_day = None;
                self.pointer_slot = None;
                cx.emit(MeteringChartEvent::HoverDayChanged(None));
                cx.notify();
            }
            _ => {}
        }
    }

    fn render_part_logo(&self, logo: Option<&SharedString>, fg: Hsla, cx: &App) -> AnyElement {
        if let Some(name) = logo {
            let lower = name.trim().to_ascii_lowercase();
            match lower.as_str() {
                "github" => return pro_svg("0 0 24 24", GITHUB_MINI_SVG, 12., fg),
                "notion" => return pro_svg("0 0 24 24", NOTION_MINI_SVG, 12., fg),
                "figma" => return pro_svg("0 0 16 24", FIGMA_TINY_SVG, 11., fg),
                _ => {
                    if let Some(art) = pro_app_artwork(&lower, 12., cx) {
                        return art;
                    }
                }
            }
        }
        div()
            .size(px(5.))
            .rounded_full()
            .bg(fg.opacity(0.45))
            .into_any_element()
    }
}

impl Render for MeteringChart {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let dark = p.dark;
        let card_bg: Hsla = if dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border: Hsla = if dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.065)
        };
        let grid_c: Hsla = if dark {
            hsla(0., 0., 1., 0.14)
        } else {
            hsla(0., 0., 0.46, 0.25)
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
                .max_w(px(520.))
                .rounded(px(10.))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(pro_skeleton_bar(148., 18., 5., phase, cx))
                        .child(pro_skeleton_bar(88., 14., 4., phase, cx)),
                )
                .child(pro_skeleton_bar(240., 18., 4., phase, cx))
                .child(pro_skeleton_bar(480., self.height, 6., phase, cx))
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(520.))
                .rounded(px(10.))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .child(pro_empty_state::<Self>(
                    ProIcon::Activity,
                    tr(cx, "No usage recorded", "暂无用量记录"),
                    tr(
                        cx,
                        "Metered API events and daily breakdowns appear here once traffic starts.",
                        "产生 API 请求流量后，此处将按日展示梯度柱状图与工具占比。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let reduced = disabled_motion(cx);
        let elapsed = self.started_at.elapsed().as_secs_f32();
        if (self.live || elapsed < 0.95) && !reduced {
            w.request_animation_frame();
        }

        // Subtle live breathing + growing drift on Today's bar when `live` is enabled.
        let live_growth = if self.live && !reduced {
            (elapsed * 22.0).min(420.0) as f64
        } else {
            0.0
        };
        let pulse_wave = if self.live && !reduced {
            0.5 + 0.5 * (elapsed * 2.6).sin()
        } else {
            1.0
        };
        let sheen_pos = if self.live && !reduced {
            ((elapsed / 2.4).fract() * 1.6 - 0.3).clamp(0.0, 1.0)
        } else {
            -1.0
        };

        let n_days = self.data.len();
        let mut all_days_sum = 0.0f64;
        let max_val = self
            .data
            .iter()
            .enumerate()
            .filter_map(|(i, d)| {
                d.value.map(|v| {
                    let eff = if self.live && i + 1 == n_days {
                        v + live_growth
                    } else {
                        v
                    };
                    all_days_sum += eff;
                    eff
                })
            })
            .fold(0.0f64, f64::max);
        let has_values = n_days > 0 && max_val > 0.0;
        let ceiling = nice_metering_ceiling(max_val);
        let anim_ceil =
            pro_spring_f32(pro_eid(&self.id, "ceil"), ceiling as f32, 0.36, w, cx).max(1.0) as f64;

        // Sync per-bar `Slide::snappy` proximity highlight slides (macOS Dock / audio meter feel).
        if self.hover_slides.len() < n_days {
            self.hover_slides.resize_with(n_days, || Slide::snappy(0.0));
        } else if self.hover_slides.len() > n_days {
            self.hover_slides.truncate(n_days);
        }

        let hovered_idx = self.active_day();
        let focus_slot: Option<f32> = self
            .pointer_slot
            .or_else(|| hovered_idx.map(|idx| idx as f32));
        self.spotlight_mix
            .set(if focus_slot.is_some() { 1.0 } else { 0.0 }, cx);
        let spot_mix = self.spotlight_mix.get(w, cx).clamp(0.0, 1.0);

        let mut bar_heights = Vec::with_capacity(n_days);
        let mut bar_colors = Vec::with_capacity(n_days);
        let mut bar_hovs = Vec::with_capacity(n_days);

        for (i, d) in self.data.iter().enumerate() {
            let is_live_bar = self.live && i + 1 == n_days;
            let eff_val = d
                .value
                .map(|v| if is_live_bar { v + live_growth } else { v })
                .unwrap_or(0.0);
            let target_ratio = if d.value.is_some() {
                ((eff_val / anim_ceil) as f32).clamp(0.02, 1.0)
            } else {
                0.0
            };
            let spring_r = pro_spring_f32(
                pro_eid(&self.id, format!("bar-{i}")),
                target_ratio,
                0.38,
                w,
                cx,
            );
            let wave_reveal = if reduced {
                1.0
            } else {
                let delay = (i as f32 / n_days.max(1) as f32) * 0.28;
                pro_ease(((elapsed - delay) / 0.52).clamp(0., 1.))
            };
            bar_heights.push(spring_r * wave_reveal);

            let target_hov = match focus_slot {
                Some(center) => dock_proximity(i as f32 - center),
                None => 0.0,
            };
            self.hover_slides[i].set(target_hov, cx);
            bar_hovs.push(self.hover_slides[i].get(w, cx).clamp(0.0, 1.0));

            let c = if is_live_bar {
                if dark {
                    rgb(0x7472ff).into()
                } else {
                    self.accent
                }
            } else {
                let t = if n_days <= 2 {
                    0.0
                } else {
                    i as f32 / (n_days.saturating_sub(2)).max(1) as f32
                };
                let base = sample_color_ramp(&self.colors, t);
                if dark {
                    pro_mix_color(base, rgb(0x8c7aff).into(), 0.24)
                } else {
                    base
                }
            };
            bar_colors.push(c);
        }

        let badge_text: Option<SharedString> = self
            .badge
            .clone()
            .or_else(|| self.live.then(|| tr(cx, "Real-time", "实时计量").into()));
        let badge_el: Option<AnyElement> = badge_text.map(|txt| {
            let (bbg, bfg): (Hsla, Hsla) = if dark {
                (hsla(327. / 360., 0.84, 0.60, 0.18), rgb(0xf472b6).into())
            } else {
                (hsla(327. / 360., 0.84, 0.59, 0.10), rgb(0xe0338f).into())
            };
            div()
                .h(px(20.))
                .px(px(7.))
                .rounded(px(5.))
                .bg(bbg)
                .text_color(bfg)
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .flex()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .size(px(9.))
                        .rounded_full()
                        .bg(bfg.opacity(0.22))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .size(px(4.5))
                                .rounded_full()
                                .bg(bfg.opacity(0.55 + 0.45 * pulse_wave)),
                        ),
                )
                .child(txt)
                .into_any_element()
        });

        let wash: Hsla = if dark {
            hsla(0., 0., 1., 0.06)
        } else {
            hsla(0., 0., 0., 0.04)
        };
        let burst_btn: Option<AnyElement> = self.live.then(|| {
            pro_tactile(
                pro_eid(&self.id, "burst-btn"),
                tr(cx, "Simulate live API traffic", "模拟实时 API 流量"),
                5.,
                transparent_black(),
                wash,
                p.line,
                p.line_strong,
                false,
                div()
                    .h(px(22.))
                    .px(px(7.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(11.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.text)
                    .whitespace_nowrap()
                    .child(tr(cx, "+320 calls", "+320 次调用")),
                cx,
                |this, _, cx| {
                    this.record_live_burst(320.0, cx);
                },
            )
            .flex_none()
            .into_any_element()
        });

        // Inspection status readout in top-right using `pro_swap_label` (clickable when pinned to unpin).
        let status_label: Option<SharedString> = if let Some(pi) = self.pinned_day
            && let Some(d) = self.data.get(pi)
        {
            Some(format!("{} \u{00b7} {}", tr(cx, "Pinned", "已固定"), d.label).into())
        } else if let Some(hi) = self.hovered_day
            && let Some(d) = self.data.get(hi)
        {
            Some(format!("{} \u{00b7} {}", tr(cx, "Inspecting", "查看"), d.label).into())
        } else {
            self.period.clone()
        };

        let is_pinned = self.pinned_day.is_some();
        let status_el: Option<AnyElement> = status_label.map(|lbl| {
            if is_pinned {
                pro_tactile(
                    pro_eid(&self.id, "pin-chip"),
                    tr(cx, "Unpin day inspection", "取消固定日期"),
                    5.,
                    p.accent_soft,
                    p.accent_soft,
                    p.blue.opacity(0.32),
                    p.blue.opacity(0.55),
                    true,
                    div()
                        .h(px(22.))
                        .px(px(7.))
                        .flex()
                        .items_center()
                        .child(pro_swap_label(
                            pro_eid(&self.id, "inspect-lbl"),
                            lbl,
                            11.5,
                            FontWeight::MEDIUM,
                            p.blue,
                        )),
                    cx,
                    |this, _, cx| {
                        this.pinned_day = None;
                        cx.emit(MeteringChartEvent::HoverDayChanged(this.active_day()));
                        cx.notify();
                    },
                )
                .flex_none()
                .into_any_element()
            } else {
                div()
                    .h(px(22.))
                    .flex()
                    .items_center()
                    .child(pro_swap_label(
                        pro_eid(&self.id, "inspect-lbl"),
                        lbl,
                        12.,
                        FontWeight::NORMAL,
                        p.muted,
                    ))
                    .into_any_element()
            }
        });

        // Pinned/inspected day share for dynamic rolling `Total volume` / `Total events`.
        let pinned_share: Option<(f64, SharedString)> = self.pinned_day.and_then(|pi| {
            let d = self.data.get(pi)?;
            let is_live_d = self.live && pi + 1 == n_days;
            let dv = d
                .value
                .map(|v| if is_live_d { v + live_growth } else { v })?;
            let ratio = if all_days_sum > 1e-9 {
                (dv / all_days_sum).clamp(0.0, 1.0)
            } else {
                0.0
            };
            Some((ratio, d.label.clone()))
        });

        let mut totals_row = div()
            .mt(px(6.))
            .flex()
            .flex_wrap()
            .items_baseline()
            .gap_x(px(20.))
            .gap_y(px(4.));
        for (ti, tot) in self.totals.iter().enumerate() {
            let live_bump = if self.live {
                if ti == 0 {
                    live_growth * 0.00084
                } else {
                    live_growth / 1000.0
                }
            } else {
                0.0
            };
            let full_scaled = (tot.value + live_bump) / tot.divisor;
            let (shown_val, shown_lbl) = if let Some((ratio, ref dlbl)) = pinned_share {
                (
                    full_scaled * ratio,
                    SharedString::from(format!("{} \u{00b7} {}", tot.label, dlbl)),
                )
            } else {
                (full_scaled, tot.label.clone())
            };
            totals_row = totals_row.child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(px(6.))
                    .child(pro_swap_label(
                        pro_eid(&self.id, format!("tlbl-{}", tot.id)),
                        shown_lbl,
                        13.,
                        FontWeight::NORMAL,
                        p.text,
                    ))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, format!("tot-{}", tot.id)),
                        shown_val,
                        tot.decimals,
                        tot.prefix.clone(),
                        tot.suffix.clone(),
                        15.,
                        FontWeight::MEDIUM,
                        p.ink,
                    )),
            );
        }

        let plot_h = self.height;
        let fade_bottom = if dark { 0.30f32 } else { 0.42f32 };
        let live_flag = self.live;
        let pinned_for_paint = self.pinned_day;
        let bounds_cell = self.plot_bounds.clone();
        let canvas_bar_colors = bar_colors.clone();
        let canvas_bar_hovs = bar_hovs.clone();

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

                // 4 horizontal dashed reference lines (100%, 66.7%, 33.3%, 0%).
                for step in 0..=3usize {
                    let y = f32::from(bounds.top()) + bh * (step as f32 / 3.0);
                    let mut x = f32::from(bounds.left());
                    let x_end = f32::from(bounds.right());
                    let mut dash = PathBuilder::stroke(px(1.0));
                    let mut has_seg = false;
                    while x < x_end {
                        let seg_end = (x + 3.5).min(x_end);
                        dash.move_to(point(px(x), px(y)));
                        dash.line_to(point(px(seg_end), px(y)));
                        has_seg = true;
                        x += 7.0;
                    }
                    if has_seg && let Ok(path) = dash.build() {
                        window.paint_path(path, grid_c);
                    }
                }

                if !has_values || bar_heights.is_empty() {
                    return;
                }

                let n = bar_heights.len();
                let slot_w = bw / n as f32;
                let base_bar_w = (slot_w * 0.62).clamp(2.5, 11.0);
                let bot_y = f32::from(bounds.bottom());

                for i in 0..n {
                    let ratio = bar_heights[i];
                    if ratio <= 0.002 {
                        continue;
                    }
                    let is_live = live_flag && i + 1 == n;
                    let hov = canvas_bar_hovs[i];
                    let is_pin = pinned_for_paint == Some(i);
                    let boost = if is_pin { hov.max(0.85) } else { hov };

                    let dim_floor = if dark { 0.66 } else { 0.58 };
                    let alpha_mul = (1.0 - spot_mix * dim_floor * (1.0 - boost)).clamp(0.30, 1.0);

                    let bar_w =
                        (base_bar_w * (1.0 + 0.14 * boost)).min((slot_w - 1.5).max(base_bar_w));
                    let h_px = (bh * ratio * (1.0 + 0.035 * boost)).clamp(3.0, bh);
                    let cx_x = f32::from(bounds.left()) + (i as f32 + 0.5) * slot_w;
                    let x0 = cx_x - bar_w * 0.5;
                    let y0 = bot_y - h_px;
                    let r = px(3.5f32.min(bar_w * 0.5).min(h_px * 0.5));
                    let bar_rect = Bounds::new(point(px(x0), px(y0)), size(px(bar_w), px(h_px)));

                    let c = pro_mix_color(
                        canvas_bar_colors[i],
                        rgb(0xffffff).into(),
                        (if dark { 0.22 } else { 0.14 }) * boost,
                    );
                    let bot_alpha = if is_live {
                        0.58 + 0.20 * boost
                    } else {
                        fade_bottom + 0.24 * boost
                    };
                    let mut bar_path = PathBuilder::fill();
                    let rf = f32::from(r);
                    bar_path.move_to(point(px(x0), px(bot_y)));
                    bar_path.line_to(point(px(x0), px(y0 + rf)));
                    bar_path.curve_to(point(px(x0 + rf), px(y0)), point(px(x0), px(y0)));
                    bar_path.line_to(point(px(x0 + bar_w - rf), px(y0)));
                    bar_path.curve_to(
                        point(px(x0 + bar_w), px(y0 + rf)),
                        point(px(x0 + bar_w), px(y0)),
                    );
                    bar_path.line_to(point(px(x0 + bar_w), px(bot_y)));
                    bar_path.close();

                    if let Ok(path) = bar_path.build() {
                        window.paint_path(
                            path,
                            linear_gradient(
                                180.,
                                linear_color_stop(c.opacity(alpha_mul), 0.),
                                linear_color_stop(c.opacity((bot_alpha * alpha_mul).min(1.0)), 1.),
                            ),
                        );
                    }

                    if is_pin {
                        window.paint_quad(
                            fill(
                                Bounds::new(
                                    point(px(cx_x - 2.2), px(bot_y + 2.5)),
                                    size(px(4.4), px(4.4)),
                                ),
                                c,
                            )
                            .corner_radii(px(99.)),
                        );
                    }

                    if is_live && sheen_pos >= 0.0 && h_px > 12.0 {
                        let band_h = (h_px * 0.38).max(8.0);
                        let band_y = bot_y - sheen_pos * (h_px - band_h) - band_h;
                        let sheen_rect = Bounds::new(
                            point(bar_rect.origin.x, px(band_y.max(y0))),
                            size(px(bar_w), px(band_h.min(bot_y - band_y.max(y0)))),
                        );
                        window.paint_quad(
                            fill(
                                sheen_rect,
                                hsla(
                                    0.,
                                    0.,
                                    1.,
                                    0.24 * alpha_mul
                                        * (1.0 - (sheen_pos - 0.5).abs() * 1.6).clamp(0., 1.),
                                ),
                            )
                            .corner_radii(r),
                        );
                    }
                }
            },
        )
        .size_full();

        let plot_w = self
            .plot_bounds
            .get()
            .map(|b| f32::from(b.size.width))
            .unwrap_or(440.);

        if let Some(idx) = hovered_idx
            && let Some(day) = self.data.get(idx)
            && day.value.is_some()
        {
            let parts_empty = day.breakdown.is_empty();
            let slot_cx = ((idx as f32 + 0.5) / n_days.max(1) as f32) * plot_w;
            let target_left = if parts_empty {
                let chip_w = 112.0f32;
                (slot_cx - chip_w * 0.5).clamp(0.0, (plot_w - chip_w).max(0.0))
            } else {
                let card_w = 212.0f32;
                let flip_left = slot_cx > plot_w * 0.52;
                if flip_left {
                    (slot_cx - card_w - 14.0).max(0.0)
                } else {
                    (slot_cx + 14.0).min((plot_w - card_w).max(0.0))
                }
            };
            if !self.tooltip_ready {
                self.tooltip_x.reset(target_left);
                self.tooltip_ready = true;
            } else {
                self.tooltip_x.set(target_left, cx);
            }
            self.tooltip_presence.set(1.0, cx);
        } else {
            self.tooltip_presence.set(0.0, cx);
            if self.tooltip_presence.value(cx) < 0.04 {
                self.tooltip_ready = false;
            }
        }

        let gliding_left_px = self.tooltip_x.get(w, cx);
        let tip_alpha = self.tooltip_presence.get(w, cx).clamp(0.0, 1.0);

        let tag_overlay: Option<AnyElement> = hovered_idx.and_then(|idx| {
            let day = self.data.get(idx)?;
            let is_live_day = self.live && idx + 1 == n_days;
            let val = day
                .value
                .map(|v| if is_live_day { v + live_growth } else { v })?;
            let chip_bg: Hsla = if dark {
                hsla(0., 0., 1., 0.94)
            } else {
                hsla(240. / 360., 0.06, 0.08, 0.93)
            };
            let chip_fg: Hsla = if dark {
                rgb(0x111111).into()
            } else {
                rgb(0xfcfcfc).into()
            };
            let chip_line: Hsla = if dark {
                hsla(0., 0., 0., 0.10)
            } else {
                hsla(0., 0., 1., 0.15)
            };
            let bar_tint = bar_colors.get(idx).copied().unwrap_or(self.accent);
            let day_tag: SharedString = if self.pinned_day == Some(idx) {
                format!("{} \u{00b7} {}", tr(cx, "Pinned", "已固定"), day.label).into()
            } else {
                day.label.clone()
            };

            let parts =
                compute_whole_percentages(&day.breakdown, self.breakdown_limit, &self.other_label);

            if parts.is_empty() {
                return Some(
                    div()
                        .absolute()
                        .top(px(6. + 3. * (1. - tip_alpha)))
                        .left(px(gliding_left_px))
                        .opacity(tip_alpha)
                        .px(px(8.))
                        .py(px(3.))
                        .rounded(px(6.))
                        .bg(chip_bg)
                        .shadow_md()
                        .flex()
                        .items_baseline()
                        .gap(px(6.))
                        .text_size(px(11.5))
                        .text_color(chip_fg)
                        .child(pro_rolling_number(
                            pro_eid(&self.id, "tip-val"),
                            val,
                            0,
                            "",
                            "",
                            11.5,
                            FontWeight::SEMIBOLD,
                            chip_fg,
                        ))
                        .child(pro_swap_label(
                            pro_eid(&self.id, "tip-day"),
                            day_tag,
                            11.5,
                            FontWeight::NORMAL,
                            chip_fg.opacity(0.68),
                        ))
                        .into_any_element(),
                );
            }

            let card_w = 212.0f32;
            let mut part_rows = div()
                .mt(px(6.))
                .pt(px(6.))
                .border_t_1()
                .border_color(chip_line)
                .flex()
                .flex_col()
                .gap(px(4.));

            for (pi, (lbl, logo, pct)) in parts.into_iter().enumerate() {
                let anim_pct = pro_spring_f32(
                    pro_eid(&self.id, format!("tip-bar-{pi}")),
                    (pct as f32 / 100.0).clamp(0.0, 1.0),
                    0.22,
                    w,
                    cx,
                )
                .clamp(0.0, 1.0);
                part_rows = part_rows.child(
                    div()
                        .h(px(16.))
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .text_size(px(11.))
                        .text_color(chip_fg)
                        .child(
                            div()
                                .size(px(14.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(self.render_part_logo(logo.as_ref(), chip_fg, cx)),
                        )
                        .child(div().flex_1().min_w_0().opacity(0.76).truncate().child(lbl))
                        .child(
                            div()
                                .w(px(36.))
                                .h(px(4.))
                                .flex_none()
                                .rounded(px(2.))
                                .bg(chip_line)
                                .overflow_hidden()
                                .child(
                                    div()
                                        .h_full()
                                        .w(relative(anim_pct))
                                        .rounded(px(2.))
                                        .bg(bar_tint),
                                ),
                        )
                        .child(div().w(px(28.)).flex_none().flex().justify_end().child(
                            pro_rolling_number(
                                pro_eid(&self.id, format!("tip-pct-{pi}")),
                                pct as f64,
                                0,
                                "",
                                "%",
                                11.,
                                FontWeight::MEDIUM,
                                chip_fg,
                            ),
                        )),
                );
            }

            Some(
                div()
                    .absolute()
                    .top(px(8. + 3. * (1. - tip_alpha)))
                    .left(px(gliding_left_px))
                    .opacity(tip_alpha)
                    .w(px(card_w))
                    .px(px(10.))
                    .py(px(8.))
                    .rounded(px(8.))
                    .bg(chip_bg)
                    .shadow_lg()
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .justify_between()
                            .gap(px(6.))
                            .text_size(px(11.5))
                            .text_color(chip_fg)
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "tip-val"),
                                val,
                                0,
                                "",
                                "",
                                11.5,
                                FontWeight::SEMIBOLD,
                                chip_fg,
                            ))
                            .child(pro_swap_label(
                                pro_eid(&self.id, "tip-day"),
                                day_tag,
                                11.5,
                                FontWeight::NORMAL,
                                chip_fg.opacity(0.68),
                            )),
                    )
                    .child(part_rows)
                    .into_any_element(),
            )
        });

        let empty_overlay: Option<AnyElement> = (!has_values).then(|| {
            let msg = self
                .empty_text
                .clone()
                .unwrap_or_else(|| tr(cx, "No usage yet", "暂无用量").into());
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
        let bounds_for_click = self.plot_bounds.clone();
        let plot_box = div()
            .id(pro_eid(&self.id, "plot"))
            .w_full()
            .flex_none()
            .h(px(plot_h))
            .relative()
            .cursor(if has_values {
                CursorStyle::PointingHand
            } else {
                CursorStyle::Arrow
            })
            .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, _, cx| {
                let n = this.data.len();
                if n == 0 {
                    return;
                }
                let Some(b) = bounds_for_move.get() else {
                    return;
                };
                let bw = f32::from(b.size.width).max(1.0);
                let rel_x = f32::from(ev.position.x - b.left()).clamp(0.0, bw - 0.01);
                let slot_f = ((rel_x / bw) * n as f32 - 0.5).clamp(0.0, (n - 1) as f32);
                let idx = ((rel_x / bw) * n as f32).floor() as usize;
                let idx = idx.min(n - 1);
                let slot_shifted = this
                    .pointer_slot
                    .is_none_or(|prev| (prev - slot_f).abs() > 0.03);
                this.pointer_slot = Some(slot_f);
                if this.hovered_day != Some(idx) {
                    this.hovered_day = Some(idx);
                    cx.emit(MeteringChartEvent::HoverDayChanged(Some(idx)));
                    cx.notify();
                } else if slot_shifted {
                    cx.notify();
                }
            }))
            .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                if !*inside && (this.hovered_day.is_some() || this.pointer_slot.is_some()) {
                    this.hovered_day = None;
                    this.pointer_slot = None;
                    cx.emit(MeteringChartEvent::HoverDayChanged(this.active_day()));
                    cx.notify();
                }
            }))
            .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                let n = this.data.len();
                if n == 0 {
                    return;
                }
                let target_idx = if let Some(b) = bounds_for_click.get() {
                    let bw = f32::from(b.size.width).max(1.0);
                    let rel_x = f32::from(ev.position().x - b.left()).clamp(0.0, bw - 0.01);
                    (((rel_x / bw) * n as f32).floor() as usize).min(n - 1)
                } else if let Some(hi) = this.hovered_day {
                    hi
                } else {
                    return;
                };
                this.toggle_pin_day(target_idx, cx);
            }))
            .child(plot_canvas)
            .when_some(empty_overlay, |d, el| d.child(el))
            .when_some(tag_overlay, |d, tag| d.child(tag));

        // Pick 4 clean X-axis date ticks (e.g. Sep 5, Sep 11, Sep 17, Today).
        let mut x_ticks = div()
            .w_full()
            .flex_none()
            .mt(px(8.))
            .h(px(16.))
            .flex()
            .items_center()
            .justify_between()
            .text_size(px(11.))
            .line_height(px(16.))
            .text_color(p.muted);
        if n_days >= 4 {
            let idxs = [n_days / 5, (n_days * 2) / 4, (n_days * 4) / 5, n_days - 1];
            for &ix in &idxs {
                if let Some(d) = self.data.get(ix) {
                    x_ticks = x_ticks.child(d.label.clone());
                }
            }
        } else {
            for d in &self.data {
                x_ticks = x_ticks.child(d.label.clone());
            }
        }

        let y3 = format_axis_short(ceiling);
        let y2 = format_axis_short(ceiling * (2.0 / 3.0));
        let y1 = format_axis_short(ceiling / 3.0);

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .role(Role::Group)
            .aria_label(self.title.clone())
            .w_full()
            .max_w(px(520.))
            .rounded(px(10.))
            .bg(card_bg)
            .border_1()
            .border_color(card_border)
            .shadow(pro_card_shadow(dark))
            .px(px(16.))
            .pt(px(14.))
            .pb(px(12.))
            .flex()
            .flex_col()
            .on_key_down(cx.listener(Self::handle_key_down))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(15.))
                                    .line_height(px(24.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .truncate()
                                    .child(self.title.clone()),
                            )
                            .when_some(badge_el, |d, b| d.child(b)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .when_some(status_el, |d, st| d.child(st))
                            .when_some(burst_btn, |d, btn| d.child(btn)),
                    ),
            )
            .child(totals_row)
            .child(
                div()
                    .mt(px(22.))
                    .flex()
                    .items_start()
                    .gap(px(12.))
                    .child(
                        div()
                            .w(px(28.))
                            .h(px(plot_h))
                            .flex_none()
                            .flex()
                            .flex_col()
                            .justify_between()
                            .items_end()
                            .text_size(px(11.))
                            .line_height(px(14.))
                            .text_color(p.muted)
                            .child(y3)
                            .child(y2)
                            .child(y1)
                            .child("0"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(plot_box)
                            .child(x_ticks),
                    ),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{MeteringBreakdownPart, MeteringChart, compute_whole_percentages, dock_proximity};
    use gpui::{AppContext, TestAppContext, px, size};
    use std::time::Duration;

    #[test]
    fn whole_percentages_fold_remainder_and_sum_to_hundred() {
        let parts = vec![
            MeteringBreakdownPart::new("GitHub", 362.0),
            MeteringBreakdownPart::new("Slack", 289.0),
            MeteringBreakdownPart::new("Notion", 141.0),
            MeteringBreakdownPart::new("Figma", 79.0),
            MeteringBreakdownPart::new("Stripe", 69.0),
            MeteringBreakdownPart::new("Drive", 60.0),
        ];
        let out = compute_whole_percentages(&parts, 4, "Other");
        assert_eq!(out.len(), 5);
        assert_eq!(out[4].0.as_ref(), "Other");
        let sum: u32 = out.iter().map(|(_, _, p)| *p).sum();
        assert_eq!(sum, 100);
    }

    #[gpui::test]
    fn live_burst_grows_today_bar_and_totals(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let chart = cx.new(|cx| MeteringChart::new("mtr-test", cx));
        chart.update(cx, |c, cx| {
            let before = c.data.last().and_then(|d| d.value).unwrap_or(0.0);
            c.record_live_burst(500.0, cx);
            let after = c.data.last().and_then(|d| d.value).unwrap_or(0.0);
            assert!((after - (before + 500.0)).abs() < 1e-6);
        });
    }

    #[gpui::test]
    fn dock_proximity_slides_and_click_pinning_glide_smoothly(cx: &mut TestAppContext) {
        assert!((dock_proximity(0.0) - 1.0).abs() < 1e-5);
        assert!(dock_proximity(1.0) > 0.5 && dock_proximity(1.0) < 0.9);
        assert_eq!(dock_proximity(3.0), 0.0);

        cx.update(crate::init);
        let exec = cx.executor();
        let (chart, w) = cx.add_window_view(|_, cx| MeteringChart::new("mtr-glide", cx));
        w.simulate_resize(size(px(520.), px(340.)));
        w.run_until_parked();

        w.update(|_, app| {
            chart.update(app, |c, cx| {
                c.toggle_pin_day(10, cx);
                assert_eq!(c.active_day(), Some(10));
            });
        });
        w.run_until_parked();

        exec.advance_clock(Duration::from_millis(45));
        w.update(|win, app| win.simulate_next_frame(app));
        w.run_until_parked();

        w.update(|_, app| {
            let c = chart.read(app);
            let center_h = c.hover_slides[10].value(app);
            let neighbor_h = c.hover_slides[11].value(app);
            let far_h = c.hover_slides[2].value(app);
            assert!(center_h > neighbor_h && neighbor_h > far_h);
        });
    }
}
