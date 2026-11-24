//! Top-of-dashboard KPI board with period switching, sparkline cards, comparison chart, and focus card.
//!
//! Coordinates three headline trend cards with rolling figures, a multi-segment period-over-period
//! comparison chart, and a product focus card under one shared date-range selector (`7d`, `30d`,
//! `90d`, `12m`) with interactive pointer and keyboard day scrubbing.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_btn, pro_ease, pro_eid, pro_empty_state, pro_mark_selected,
        pro_mix_color, pro_pill_indicator, pro_rolling_number, pro_selection_sensor,
        pro_skeleton_bar, pro_spring_f32, pro_tactile, pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::{UiLanguage, activate},
        family::{PhaseClock, Slide, disabled_motion},
        pro_icons::{ProIcon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

const CUBE_FOCUS_SVG: &str = r#"<path d="M8 1.9 13.3 4.8v6.4L8 14.1 2.7 11.2V4.8L8 1.9Z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"></path><path d="M2.9 4.9 8 7.7l5.1-2.8M8 7.7V14" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"></path>"#;
const SLIDERS_SETUP_SVG: &str = r#"<path d="M2 4.2h5.2M10.2 4.2H12M2 9.8h1.8M6.8 9.8H12" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"></path><circle cx="8.7" cy="4.2" r="1.5" stroke="currentColor" stroke-width="1.3"></circle><circle cx="5.3" cy="9.8" r="1.5" stroke="currentColor" stroke-width="1.3"></circle>"#;
const DELTA_UP_SVG: &str = r#"<path d="M4 1.2 7.2 6.4H.8L4 1.2Z" fill="currentColor"></path>"#;
const DELTA_DOWN_SVG: &str = r#"<path d="M4 6.8.8 1.6h6.4L4 6.8Z" fill="currentColor"></path>"#;

/// Selectable reporting window in the top bar of [`KpiBoardBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KpiRangeOption {
    /// Stable range identifier (such as `"7d"`, `"30d"`, `"90d"`, or `"12m"`).
    pub id: SharedString,
    /// Display label rendered in the period switcher.
    pub label: SharedString,
}

impl KpiRangeOption {
    /// Construct a date-range option from `id` and `label`.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

/// Segment filter option inside the overview comparison card of [`KpiBoardBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KpiSegmentOption {
    /// Stable segment identifier (such as `"all"`, `"pro"`, or `"team"`).
    pub id: SharedString,
    /// Display label rendered in the overview segment control.
    pub label: SharedString,
}

impl KpiSegmentOption {
    /// Construct an overview segment filter option.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

/// Headline metric card specification in the top row of [`KpiBoardBlock`].
#[derive(Clone, Debug, PartialEq)]
pub struct KpiStatCardSpec {
    /// Stable card identifier.
    pub id: SharedString,
    /// Metric title shown above the rolling figure.
    pub label: SharedString,
    /// Per-point date labels matching `series`.
    pub dates: Vec<SharedString>,
    /// Daily or weekly metric series (`NaN` values count as `0.0` in totals).
    pub series: Vec<f64>,
    /// Previous-period aggregate used to compute the percentage delta (`<= 0.0` hides the badge).
    pub previous_total: f64,
    /// Whether the headline figure averages finite points instead of summing them.
    pub average_mode: bool,
    /// Scale divisor applied to the headline rolling number (for example `1000.0` with `"k"` suffix).
    pub display_divisor: f64,
    /// Decimal places rendered in the rolling headline figure.
    pub decimals: usize,
    /// Prefix rendered before the rolling figure (for example `"$"`).
    pub prefix: SharedString,
    /// Suffix rendered after the rolling figure (for example `"k"` or `"%"`).
    pub suffix: SharedString,
    /// Whether negative deltas represent an improvement (such as churn or error rate).
    pub lower_is_better: bool,
}

impl KpiStatCardSpec {
    /// Compute the finite aggregate for `series` (sum by default, mean when `average_mode` is set).
    pub fn aggregate_value(&self) -> f64 {
        let mut sum = 0.0;
        let mut count = 0usize;
        for &v in &self.series {
            if v.is_finite() {
                sum += v;
                count += 1;
            }
        }
        if self.average_mode {
            if count == 0 { 0.0 } else { sum / count as f64 }
        } else {
            sum
        }
    }

    /// Compute `(pct_change, is_good)` relative to `previous_total`, returning `None` when `previous_total <= 0.0`.
    pub fn delta(&self) -> Option<(f64, bool)> {
        compute_delta(
            self.aggregate_value(),
            self.previous_total,
            self.lower_is_better,
        )
    }
}

/// Period-over-period overview chart data in [`KpiBoardBlock`].
#[derive(Clone, Debug, PartialEq)]
pub struct KpiOverviewSpec {
    /// Metric heading inside the overview card.
    pub label: SharedString,
    /// Per-point date labels along the X-axis.
    pub dates: Vec<SharedString>,
    /// Default current-period series.
    pub current: Vec<f64>,
    /// Default previous-period comparison series.
    pub previous: Vec<f64>,
    /// Optional segment-specific `(current, previous)` overrides keyed by segment id.
    pub segments: HashMap<SharedString, (Vec<f64>, Vec<f64>)>,
}

impl KpiOverviewSpec {
    /// Resolve `(current, previous)` series for `segment_id`, falling back to default series.
    pub fn series_for_segment(&self, segment_id: &str) -> (&[f64], &[f64]) {
        if let Some((cur, prev)) = self.segments.get(segment_id) {
            (cur.as_slice(), prev.as_slice())
        } else {
            (self.current.as_slice(), self.previous.as_slice())
        }
    }
}

/// Highlighted product or plan focus card on the right side of [`KpiBoardBlock`].
#[derive(Clone, Debug, PartialEq)]
pub struct KpiFocusSpec {
    /// Featured product or tier name (for example `"Pro license"`).
    pub product_name: SharedString,
    /// Billing cadence subtitle (for example `"One-time"`).
    pub product_subtitle: SharedString,
    /// Metric caption above the count (for example `"Licenses sold"`).
    pub metric_label: SharedString,
    /// Singular noun used when the count is `1` (for example `"license"`).
    pub unit_singular: SharedString,
    /// Plural noun used for counts other than `1` (for example `"licenses"`).
    pub unit_plural: SharedString,
    /// Per-point date labels matching `current`.
    pub dates: Vec<SharedString>,
    /// Current-period unit series.
    pub current: Vec<f64>,
    /// Previous-period unit series.
    pub previous: Vec<f64>,
}

impl KpiFocusSpec {
    /// Sum finite values in `current`.
    pub fn current_total(&self) -> f64 {
        sum_finite(&self.current)
    }

    /// Sum finite values in `previous`.
    pub fn previous_total(&self) -> f64 {
        sum_finite(&self.previous)
    }

    /// Return the grammatically appropriate unit suffix for `count`.
    pub fn unit_for_count(&self, count: f64) -> &str {
        if (count.round() - 1.0).abs() < 1e-6 {
            self.unit_singular.as_ref()
        } else {
            self.unit_plural.as_ref()
        }
    }
}

/// Complete dataset for one date-range key in [`KpiBoardBlock`].
#[derive(Clone, Debug, PartialEq)]
pub struct KpiBoardData {
    /// Three headline trend cards.
    pub stats: Vec<KpiStatCardSpec>,
    /// Wide period-over-period comparison chart card.
    pub overview: KpiOverviewSpec,
    /// Right-hand product focus card.
    pub focus: KpiFocusSpec,
}

/// Events emitted by [`KpiBoardBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KpiBoardEvent {
    /// User selected a new date range (`"7d"`, `"30d"`, `"90d"`, `"12m"`).
    RangeChanged(SharedString),
    /// User switched the segment filter inside the overview card.
    SegmentChanged(SharedString),
    /// User pressed the top-right `Set up` button on the focus card.
    SetupClicked,
    /// User pressed the bottom action button on the focus card.
    ActionClicked,
    /// User hovered or stepped onto a day point on one of the charts.
    DayInspected {
        /// Identifier of the inspected chart (`"overview"`, `"focus"`, or a stat id).
        target: SharedString,
        /// Inspected point index, or `None` when cleared.
        index: Option<usize>,
    },
}

fn sum_finite(vals: &[f64]) -> f64 {
    vals.iter().copied().filter(|v| v.is_finite()).sum()
}

fn compute_delta(current: f64, previous: f64, lower_is_better: bool) -> Option<(f64, bool)> {
    if !previous.is_finite() || previous.abs() <= 1e-9 || !current.is_finite() {
        return None;
    }
    let pct = ((current - previous) / previous.abs()) * 100.0;
    let is_good = if lower_is_better {
        pct <= 0.0
    } else {
        pct >= 0.0
    };
    Some((pct, is_good))
}

fn format_grouped_int(val: f64) -> String {
    let neg = val < -0.5;
    let n = val.abs().round() as u64;
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if neg {
        out.push('-');
    }
    for (idx, ch) in digits.chars().enumerate() {
        if idx > 0 && (digits.len() - idx).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

fn format_money_compact(val: f64) -> String {
    let v = if val.is_finite() { val } else { 0.0 };
    format!("${}", format_grouped_int(v))
}

fn format_stat_point(spec: &KpiStatCardSpec, val: f64) -> String {
    let v = if val.is_finite() { val } else { 0.0 };
    if spec.prefix.as_ref() == "$" && spec.decimals == 0 {
        format_money_compact(v)
    } else {
        format!("{}{:.*}{}", spec.prefix, spec.decimals, v, spec.suffix)
    }
}

fn resample_f32(src: &[f32], len: usize) -> Vec<f32> {
    if len == 0 {
        return Vec::new();
    }
    if src.is_empty() {
        return vec![0.; len];
    }
    if src.len() == len {
        return src.to_vec();
    }
    if src.len() == 1 || len == 1 {
        return vec![src[0]; len];
    }
    (0..len)
        .map(|i| {
            let pos = (i as f32 / (len - 1) as f32) * (src.len() - 1) as f32;
            let i0 = (pos.floor() as usize).min(src.len() - 1);
            let i1 = (i0 + 1).min(src.len() - 1);
            let frac = pos - i0 as f32;
            src[i0] + (src[i1] - src[i0]) * frac
        })
        .collect()
}

fn sample_finite(vals: &[f32], pos: f32) -> Option<f32> {
    if vals.is_empty() {
        return None;
    }
    let clamped = pos.clamp(0., (vals.len() - 1) as f32);
    let i0 = (clamped.floor() as usize).min(vals.len() - 1);
    let i1 = (i0 + 1).min(vals.len() - 1);
    let frac = clamped - i0 as f32;
    match (vals[i0].is_finite(), vals[i1].is_finite()) {
        (true, true) => Some(vals[i0] + (vals[i1] - vals[i0]) * frac),
        (true, false) => Some(vals[i0]),
        (false, true) => Some(vals[i1]),
        (false, false) => None,
    }
}

struct SeriesMorphState {
    from_cur: Vec<f32>,
    to_cur: Vec<f32>,
    from_prev: Option<Vec<f32>>,
    to_prev: Option<Vec<f32>>,
    morph_clock: PhaseClock,
    mount_clock: PhaseClock,
    scrub_pos: Slide,
    scrub_alpha: Slide,
    was_scrubbing: bool,
}

#[derive(IntoElement)]
struct KpiInteractiveSeries {
    id: ElementId,
    current: Vec<f32>,
    previous: Option<Vec<f32>>,
    accent: Hsla,
    card_bg: Hsla,
    grid_color: Hsla,
    guide_color: Hsla,
    show_grid: bool,
    fill_alpha: f32,
    height: f32,
    hover_idx: Option<usize>,
}

impl RenderOnce for KpiInteractiveSeries {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let init_c = self.current.clone();
        let init_p = self.previous.clone();
        let st = w.use_keyed_state(pro_eid(&self.id, "morph"), cx, |_, inner_cx| {
            let mut mount_clock = PhaseClock::default();
            mount_clock.restart(inner_cx);
            SeriesMorphState {
                from_cur: init_c.clone(),
                to_cur: init_c,
                from_prev: init_p.clone(),
                to_prev: init_p,
                morph_clock: PhaseClock::default(),
                mount_clock,
                scrub_pos: Slide::snappy(0.),
                scrub_alpha: Slide::snappy(0.),
                was_scrubbing: false,
            }
        });

        let reduced = disabled_motion(cx);
        let valid_hover = self
            .hover_idx
            .filter(|&idx| idx < self.current.len() && self.current[idx].is_finite());
        let (cur_vals, prev_vals, reveal, scrub_pos, scrub_alpha) = st.update(cx, |s, cx| {
            let age = s.morph_clock.time(0.30, w, cx);
            let cur_t = pro_ease((age / 0.30).clamp(0., 1.));
            if s.to_cur != self.current || s.to_prev != self.previous {
                let n = self.current.len();
                let pa = resample_f32(&s.from_cur, n);
                let pb = resample_f32(&s.to_cur, n);
                s.from_cur = pa
                    .into_iter()
                    .zip(pb)
                    .map(|(a, b)| a + (b - a) * cur_t)
                    .collect();
                s.to_cur = self.current.clone();
                s.from_prev = match (&s.from_prev, &s.to_prev, &self.previous) {
                    (Some(fa), Some(fb), Some(next_p)) => {
                        let m = next_p.len();
                        let qa = resample_f32(fa, m);
                        let qb = resample_f32(fb, m);
                        Some(
                            qa.into_iter()
                                .zip(qb)
                                .map(|(a, b)| a + (b - a) * cur_t)
                                .collect(),
                        )
                    }
                    (_, prev, _) => prev.clone(),
                };
                s.to_prev = self.previous.clone();
                s.morph_clock.restart(cx);
            }
            let age2 = s.morph_clock.time(0.30, w, cx);
            let t = if reduced {
                1.0
            } else {
                pro_ease((age2 / 0.30).clamp(0., 1.))
            };
            let cur: Vec<f32> = if t >= 0.999 || s.from_cur.len() != s.to_cur.len() {
                s.to_cur.clone()
            } else {
                s.from_cur
                    .iter()
                    .zip(&s.to_cur)
                    .map(|(&a, &b)| a + (b - a) * t)
                    .collect()
            };
            let prev = match (&s.from_prev, &s.to_prev) {
                (Some(fp), Some(tp)) if t < 0.999 && fp.len() == tp.len() => Some(
                    fp.iter()
                        .zip(tp)
                        .map(|(&a, &b)| a + (b - a) * t)
                        .collect::<Vec<_>>(),
                ),
                (_, other) => other.clone(),
            };
            let mount_age = s.mount_clock.time(0.56, w, cx);
            let rev = if reduced {
                1.0
            } else {
                pro_ease((mount_age / 0.56).clamp(0., 1.))
            };

            if let Some(hi) = valid_hover {
                if !s.was_scrubbing && s.scrub_alpha.value(cx) < 0.08 {
                    s.scrub_pos.reset(hi as f32);
                } else {
                    s.scrub_pos.set(hi as f32, cx);
                }
                s.scrub_alpha.set(1., cx);
                s.was_scrubbing = true;
            } else {
                s.scrub_alpha.set(0., cx);
                s.was_scrubbing = false;
            }
            let sp = s.scrub_pos.get(w, cx);
            let sa = s.scrub_alpha.get(w, cx).clamp(0., 1.);
            (cur, prev, rev, sp, sa)
        });

        let accent = self.accent;
        let card_bg = self.card_bg;
        let grid_color = self.grid_color;
        let guide_color = self.guide_color;
        let show_grid = self.show_grid;
        let fill_alpha = self.fill_alpha;

        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 6. || bh <= 6. || cur_vals.len() < 2 {
                    return;
                }
                let pad_y = 6.0_f32;
                let plot_h = (bh - 2. * pad_y).max(6.);
                let mut min_v = f32::INFINITY;
                let mut max_v = f32::NEG_INFINITY;
                for &v in cur_vals.iter().chain(prev_vals.iter().flatten()) {
                    if v.is_finite() {
                        min_v = min_v.min(v);
                        max_v = max_v.max(v);
                    }
                }
                if !min_v.is_finite() || !max_v.is_finite() {
                    min_v = 0.;
                    max_v = 1.;
                }
                if show_grid {
                    min_v = min_v.min(0.0);
                }
                let span = (max_v - min_v).max(1e-3);

                let map_pt = |idx: usize, count: usize, val: f32| -> Point<Pixels> {
                    let tx = idx as f32 / count.saturating_sub(1).max(1) as f32;
                    let ty = 1. - ((val - min_v) / span).clamp(0., 1.);
                    point(
                        bounds.left() + px(tx * bw),
                        bounds.top() + px(pad_y + ty * plot_h),
                    )
                };
                let map_pos = |pos: f32, count: usize, val: f32| -> Point<Pixels> {
                    let tx = (pos / count.saturating_sub(1).max(1) as f32).clamp(0., 1.);
                    let ty = 1. - ((val - min_v) / span).clamp(0., 1.);
                    point(
                        bounds.left() + px(tx * bw),
                        bounds.top() + px(pad_y + ty * plot_h),
                    )
                };

                if show_grid {
                    for g in 0..3usize {
                        let gy = bounds.top() + px(pad_y + (g as f32 / 2.) * plot_h);
                        let mut gp = PathBuilder::stroke(px(1.));
                        gp.move_to(point(bounds.left(), gy));
                        gp.line_to(point(bounds.right(), gy));
                        if let Ok(path) = gp.build() {
                            window.paint_path(path, grid_color);
                        }
                    }
                }

                // Dashed comparison line ("Last period")
                if let Some(prev) = &prev_vals
                    && prev.len() >= 2
                {
                    let vis_p = (((prev.len() - 1) as f32 * reveal).ceil() as usize + 1)
                        .clamp(2, prev.len());
                    let mut dash_pb = PathBuilder::stroke(px(1.35));
                    for i in 0..(vis_p - 1) {
                        if !prev[i].is_finite() || !prev[i + 1].is_finite() {
                            continue;
                        }
                        let p0 = map_pt(i, prev.len(), prev[i]);
                        let p1 = map_pt(i + 1, prev.len(), prev[i + 1]);
                        let dx = f32::from(p1.x - p0.x);
                        let dy = f32::from(p1.y - p0.y);
                        let seg_len = dx.hypot(dy);
                        let steps = (seg_len / 6.0).ceil().max(1.0) as usize;
                        for s in 0..steps {
                            let t0 = s as f32 / steps as f32;
                            let t1 = (t0 + 0.52 / steps as f32).min(1.0);
                            dash_pb.move_to(point(p0.x + px(dx * t0), p0.y + px(dy * t0)));
                            dash_pb.line_to(point(p0.x + px(dx * t1), p0.y + px(dy * t1)));
                        }
                    }
                    if let Ok(path) = dash_pb.build() {
                        window.paint_path(path, accent.opacity(0.38));
                    }
                }

                let vis_n = (((cur_vals.len() - 1) as f32 * reveal).ceil() as usize + 1)
                    .clamp(2, cur_vals.len());

                // Area gradient fill under finite segments
                if fill_alpha > 0.001 {
                    let mut area = PathBuilder::fill();
                    let mut started = false;
                    let mut last_valid_pt = point(bounds.left(), bounds.bottom());
                    for (i, &v) in cur_vals.iter().enumerate().take(vis_n) {
                        if !v.is_finite() {
                            continue;
                        }
                        let pt = map_pt(i, cur_vals.len(), v);
                        if !started {
                            area.move_to(point(pt.x, bounds.bottom()));
                            area.line_to(pt);
                            started = true;
                        } else {
                            area.line_to(pt);
                        }
                        last_valid_pt = pt;
                    }
                    if started {
                        area.line_to(point(last_valid_pt.x, bounds.bottom()));
                        area.close();
                        if let Ok(path) = area.build() {
                            window.paint_path(
                                path,
                                linear_gradient(
                                    180.,
                                    linear_color_stop(accent.opacity(fill_alpha), 0.),
                                    linear_color_stop(accent.opacity(0.0), 1.),
                                ),
                            );
                        }
                    }
                }

                // Solid current-period line (gaps skipped cleanly on NaN)
                let mut line = PathBuilder::stroke(px(1.75));
                let mut pen_down = false;
                for (i, &v) in cur_vals.iter().enumerate().take(vis_n) {
                    if !v.is_finite() {
                        pen_down = false;
                        continue;
                    }
                    let pt = map_pt(i, cur_vals.len(), v);
                    if !pen_down {
                        line.move_to(pt);
                        pen_down = true;
                    } else {
                        line.line_to(pt);
                    }
                }
                if let Ok(path) = line.build() {
                    window.paint_path(path, accent);
                }

                // Spring-gliding hover scrubber guide & ringed markers
                if scrub_alpha > 0.01
                    && let Some(cur_y) = sample_finite(&cur_vals, scrub_pos)
                {
                    let cur_pt = map_pos(scrub_pos, cur_vals.len(), cur_y);
                    let mut guide = PathBuilder::stroke(px(1.));
                    guide.move_to(point(cur_pt.x, bounds.top()));
                    guide.line_to(point(cur_pt.x, bounds.bottom()));
                    if let Ok(path) = guide.build() {
                        window.paint_path(path, guide_color.opacity(guide_color.a * scrub_alpha));
                    }

                    let scale = 0.78 + 0.22 * scrub_alpha;
                    if let Some(prev) = &prev_vals
                        && prev.len() >= 2
                    {
                        let prev_pos = if cur_vals.len() > 1 {
                            (scrub_pos / (cur_vals.len() - 1) as f32) * (prev.len() - 1) as f32
                        } else {
                            0.
                        };
                        if let Some(prev_y) = sample_finite(prev, prev_pos) {
                            let prev_pt = map_pos(prev_pos, prev.len(), prev_y);
                            let r_out = px(4.5 * scale);
                            let r_in = px(3.0 * scale);
                            window.paint_quad(
                                fill(
                                    Bounds::new(
                                        prev_pt - point(r_out, r_out),
                                        size(r_out * 2., r_out * 2.),
                                    ),
                                    card_bg.opacity(scrub_alpha),
                                )
                                .corner_radii(r_out),
                            );
                            window.paint_quad(
                                fill(
                                    Bounds::new(
                                        prev_pt - point(r_in, r_in),
                                        size(r_in * 2., r_in * 2.),
                                    ),
                                    accent.opacity(0.55 * scrub_alpha),
                                )
                                .corner_radii(r_in),
                            );
                        }
                    }

                    let r_out = px(if show_grid { 5.5 } else { 4.5 } * scale);
                    let r_in = px(if show_grid { 3.75 } else { 3.0 } * scale);
                    window.paint_quad(
                        fill(
                            Bounds::new(cur_pt - point(r_out, r_out), size(r_out * 2., r_out * 2.)),
                            card_bg.opacity(scrub_alpha),
                        )
                        .corner_radii(r_out),
                    );
                    window.paint_quad(
                        fill(
                            Bounds::new(cur_pt - point(r_in, r_in), size(r_in * 2., r_in * 2.)),
                            accent.opacity(scrub_alpha),
                        )
                        .corner_radii(r_in),
                    );
                }
            },
        )
        .w_full()
        .h(px(self.height))
    }
}

struct ScrubOverlayState {
    last_idx: usize,
    x_ratio: Slide,
    alpha: Slide,
    was_open: bool,
    host_w: Rc<Cell<f32>>,
}

#[derive(IntoElement)]
struct ScrubReadoutOverlay {
    id: ElementId,
    active_idx: Option<usize>,
    count: usize,
    pill_width: f32,
    fallback_w: f32,
    top: f32,
    render_pill: Box<dyn FnOnce(usize) -> AnyElement>,
}

impl RenderOnce for ScrubReadoutOverlay {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let fallback_w = self.fallback_w;
        let st = w.use_keyed_state(pro_eid(&self.id, "scrub-pop"), cx, |_, _| {
            ScrubOverlayState {
                last_idx: 0,
                x_ratio: Slide::snappy(0.5),
                alpha: Slide::snappy(0.),
                was_open: false,
                host_w: Rc::new(Cell::new(fallback_w)),
            }
        });

        let count = self.count;
        let (shown_idx, rx, amt, host_cell) = st.update(cx, |s, cx| {
            if let Some(di) = self.active_idx.filter(|&d| d < count) {
                let ratio = if count > 1 {
                    di as f32 / (count - 1) as f32
                } else {
                    0.5
                };
                if !s.was_open && s.alpha.value(cx) < 0.08 {
                    s.x_ratio.reset(ratio);
                } else {
                    s.x_ratio.set(ratio, cx);
                }
                s.last_idx = di;
                s.alpha.set(1., cx);
                s.was_open = true;
            } else {
                s.alpha.set(0., cx);
                s.was_open = false;
            }
            let rx = s.x_ratio.get(w, cx).clamp(0., 1.);
            let amt = s.alpha.get(w, cx).clamp(0., 1.);
            (
                s.last_idx.min(count.saturating_sub(1)),
                rx,
                amt,
                s.host_w.clone(),
            )
        });

        let cw = host_cell.get().max(self.pill_width + 8.);
        let measure_cell = host_cell.clone();
        let sensor = canvas(
            move |b, window, _| {
                let nw = f32::from(b.size.width);
                if nw > 12. && (measure_cell.get() - nw).abs() > 1.0 {
                    measure_cell.set(nw);
                    window.request_animation_frame();
                }
            },
            |_, _, _, _| (),
        )
        .w_full()
        .h(px(0.));

        if amt <= 0.01 || count == 0 {
            return div()
                .absolute()
                .left_0()
                .right_0()
                .top_0()
                .child(sensor)
                .into_any_element();
        }

        let left_px =
            (rx * cw - self.pill_width * 0.5).clamp(4., (cw - self.pill_width - 4.).max(4.));
        let pill = (self.render_pill)(shown_idx);
        div()
            .absolute()
            .left_0()
            .right_0()
            .top_0()
            .child(sensor)
            .child(
                div()
                    .absolute()
                    .left(px(left_px))
                    .top(px(self.top + 4. * (1. - amt)))
                    .opacity(amt)
                    .child(pill),
            )
            .into_any_element()
    }
}

fn default_ranges() -> Vec<KpiRangeOption> {
    vec![
        KpiRangeOption::new("7d", "7d"),
        KpiRangeOption::new("30d", "30d"),
        KpiRangeOption::new("90d", "90d"),
        KpiRangeOption::new("12m", "12m"),
    ]
}

fn default_segments() -> Vec<KpiSegmentOption> {
    vec![
        KpiSegmentOption::new("all", "All plans"),
        KpiSegmentOption::new("pro", "Pro"),
        KpiSegmentOption::new("team", "Team"),
    ]
}

fn build_dates(prefix: &str, start: usize, count: usize, step: usize) -> Vec<SharedString> {
    (0..count)
        .map(|i| SharedString::from(format!("{prefix} {}", start + i * step)))
        .collect()
}

fn default_board_data() -> HashMap<SharedString, KpiBoardData> {
    let mut map = HashMap::new();

    // 7d dataset (7 daily points)
    let dates_7d: Vec<SharedString> = vec![
        "Nov 18".into(),
        "Nov 19".into(),
        "Nov 20".into(),
        "Nov 21".into(),
        "Nov 22".into(),
        "Nov 23".into(),
        "Nov 24".into(),
    ];
    let ov_cur_7d = vec![4120., 4480., 4390., 4860., 5120., 5340., 5690.];
    let ov_prev_7d = vec![3620., 3780., 3910., 4020., 4180., 4290., 4410.];
    let mut seg_7d = HashMap::new();
    seg_7d.insert(
        SharedString::from("pro"),
        (
            vec![2380., 2610., 2540., 2840., 2990., 3120., 3340.],
            vec![2040., 2120., 2210., 2290., 2380., 2440., 2510.],
        ),
    );
    seg_7d.insert(
        SharedString::from("team"),
        (
            vec![1740., 1870., 1850., 2020., 2130., 2220., 2350.],
            vec![1580., 1660., 1700., 1730., 1800., 1850., 1900.],
        ),
    );
    map.insert(
        SharedString::from("7d"),
        KpiBoardData {
            stats: vec![
                KpiStatCardSpec {
                    id: "mrr".into(),
                    label: "Net revenue".into(),
                    dates: dates_7d.clone(),
                    series: vec![1620., 1780., 1710., 1890., 2040., 2160., 2310.],
                    previous_total: 11_640.,
                    average_mode: false,
                    display_divisor: 1000.0,
                    decimals: 2,
                    prefix: "$".into(),
                    suffix: "k".into(),
                    lower_is_better: false,
                },
                KpiStatCardSpec {
                    id: "signups".into(),
                    label: "New customers".into(),
                    dates: dates_7d.clone(),
                    series: vec![42., 46., 44., 51., 55., 58., 62.],
                    previous_total: 318.,
                    average_mode: false,
                    display_divisor: 1.0,
                    decimals: 0,
                    prefix: "".into(),
                    suffix: "".into(),
                    lower_is_better: false,
                },
                KpiStatCardSpec {
                    id: "churn".into(),
                    label: "Churn rate".into(),
                    dates: dates_7d.clone(),
                    series: vec![1.92, 1.88, 1.85, 1.80, 1.78, 1.74, 1.72],
                    previous_total: 2.04,
                    average_mode: true,
                    display_divisor: 1.0,
                    decimals: 2,
                    prefix: "".into(),
                    suffix: "%".into(),
                    lower_is_better: true,
                },
            ],
            overview: KpiOverviewSpec {
                label: "Gross volume".into(),
                dates: dates_7d.clone(),
                current: ov_cur_7d,
                previous: ov_prev_7d,
                segments: seg_7d,
            },
            focus: KpiFocusSpec {
                product_name: "Pro license".into(),
                product_subtitle: "One-time".into(),
                metric_label: "Licenses sold".into(),
                unit_singular: "license".into(),
                unit_plural: "licenses".into(),
                dates: dates_7d,
                current: vec![11., 13., 12., 15., 16., 18., 19.],
                previous: vec![9., 10., 11., 12., 13., 14., 15.],
            },
        },
    );

    // 30d dataset (15 bi-daily points Oct 26 .. Nov 24)
    let dates_30d: Vec<SharedString> = vec![
        "Oct 26".into(),
        "Oct 28".into(),
        "Oct 30".into(),
        "Nov 1".into(),
        "Nov 3".into(),
        "Nov 5".into(),
        "Nov 7".into(),
        "Nov 10".into(),
        "Nov 12".into(),
        "Nov 14".into(),
        "Nov 16".into(),
        "Nov 18".into(),
        "Nov 20".into(),
        "Nov 22".into(),
        "Nov 24".into(),
    ];
    let ov_cur_30d: Vec<f64> = vec![
        7820., 8190., 8040., 8640., 8920., 9180., 9640., 9420., 9980., 10340., 10820., 10690.,
        11280., 11890., 12460.,
    ];
    let ov_prev_30d: Vec<f64> = vec![
        7120., 7040., 7280., 7390., 7520., 7680., 7810., 7940., 8120., 8060., 8290., 8410., 8620.,
        8780., 8960.,
    ];
    let mut seg_30d = HashMap::new();
    seg_30d.insert(
        SharedString::from("pro"),
        (
            ov_cur_30d.iter().map(|v| (v * 0.58).round()).collect(),
            ov_prev_30d.iter().map(|v| (v * 0.56).round()).collect(),
        ),
    );
    seg_30d.insert(
        SharedString::from("team"),
        (
            ov_cur_30d.iter().map(|v| (v * 0.42).round()).collect(),
            ov_prev_30d.iter().map(|v| (v * 0.44).round()).collect(),
        ),
    );
    map.insert(
        SharedString::from("30d"),
        KpiBoardData {
            stats: vec![
                KpiStatCardSpec {
                    id: "mrr".into(),
                    label: "Net revenue".into(),
                    dates: dates_30d.clone(),
                    series: vec![
                        2640., 2890., 2760., 2980., 3120., 3050., 3340., 3280., 3510., 3690.,
                        3840., 3790., 4020., 4210., 4420.,
                    ],
                    previous_total: 43_520.,
                    average_mode: false,
                    display_divisor: 1000.0,
                    decimals: 1,
                    prefix: "$".into(),
                    suffix: "k".into(),
                    lower_is_better: false,
                },
                KpiStatCardSpec {
                    id: "signups".into(),
                    label: "New customers".into(),
                    dates: dates_30d.clone(),
                    series: vec![
                        68., 72., 70., 76., 81., 79., 86., 84., 91., 94., 98., 96., 102., 108.,
                        114.,
                    ],
                    previous_total: 1_186.,
                    average_mode: false,
                    display_divisor: 1.0,
                    decimals: 0,
                    prefix: "".into(),
                    suffix: "".into(),
                    lower_is_better: false,
                },
                KpiStatCardSpec {
                    id: "churn".into(),
                    label: "Churn rate".into(),
                    dates: dates_30d.clone(),
                    series: vec![
                        2.12, 2.08, 2.05, 1.98, 1.96, 1.94, 1.90, 1.88, 1.86, 1.84, 1.81, 1.79,
                        1.77, 1.75, 1.72,
                    ],
                    previous_total: 2.18,
                    average_mode: true,
                    display_divisor: 1.0,
                    decimals: 2,
                    prefix: "".into(),
                    suffix: "%".into(),
                    lower_is_better: true,
                },
            ],
            overview: KpiOverviewSpec {
                label: "Gross volume".into(),
                dates: dates_30d.clone(),
                current: ov_cur_30d,
                previous: ov_prev_30d,
                segments: seg_30d,
            },
            focus: KpiFocusSpec {
                product_name: "Pro license".into(),
                product_subtitle: "One-time".into(),
                metric_label: "Licenses sold".into(),
                unit_singular: "license".into(),
                unit_plural: "licenses".into(),
                dates: dates_30d,
                current: vec![
                    16., 15., 18., 19., 21., 20., 23., 24., 26., 25., 28., 29., 31., 33., 35.,
                ],
                previous: vec![
                    14., 15., 15., 16., 17., 18., 19., 19., 21., 22., 22., 24., 25., 26., 28.,
                ],
            },
        },
    );

    // 90d dataset (13 weekly buckets)
    let dates_90d = build_dates("Wk", 1, 13, 1);
    let ov_cur_90d: Vec<f64> = vec![
        26400., 27800., 28900., 29400., 31200., 32600., 33900., 35100., 36800., 38200., 39600.,
        41400., 43800.,
    ];
    let ov_prev_90d: Vec<f64> = vec![
        22800., 23400., 24100., 24900., 25600., 26200., 27100., 27800., 28600., 29400., 30200.,
        31100., 32000.,
    ];
    let mut seg_90d = HashMap::new();
    seg_90d.insert(
        SharedString::from("pro"),
        (
            ov_cur_90d.iter().map(|v| (v * 0.59).round()).collect(),
            ov_prev_90d.iter().map(|v| (v * 0.57).round()).collect(),
        ),
    );
    seg_90d.insert(
        SharedString::from("team"),
        (
            ov_cur_90d.iter().map(|v| (v * 0.41).round()).collect(),
            ov_prev_90d.iter().map(|v| (v * 0.43).round()).collect(),
        ),
    );
    map.insert(
        SharedString::from("90d"),
        KpiBoardData {
            stats: vec![
                KpiStatCardSpec {
                    id: "mrr".into(),
                    label: "Net revenue".into(),
                    dates: dates_90d.clone(),
                    series: vec![
                        9800., 10200., 10600., 11100., 11500., 11900., 12400., 12900., 13400.,
                        14100., 14600., 15200., 15900.,
                    ],
                    previous_total: 136_400.,
                    average_mode: false,
                    display_divisor: 1000.0,
                    decimals: 1,
                    prefix: "$".into(),
                    suffix: "k".into(),
                    lower_is_better: false,
                },
                KpiStatCardSpec {
                    id: "signups".into(),
                    label: "New customers".into(),
                    dates: dates_90d.clone(),
                    series: vec![
                        240., 252., 264., 278., 290., 302., 318., 326., 342., 356., 372., 388.,
                        406.,
                    ],
                    previous_total: 3_520.,
                    average_mode: false,
                    display_divisor: 1.0,
                    decimals: 0,
                    prefix: "".into(),
                    suffix: "".into(),
                    lower_is_better: false,
                },
                KpiStatCardSpec {
                    id: "churn".into(),
                    label: "Churn rate".into(),
                    dates: dates_90d.clone(),
                    series: vec![
                        2.30, 2.24, 2.18, 2.14, 2.08, 2.04, 1.98, 1.95, 1.91, 1.88, 1.84, 1.80,
                        1.76,
                    ],
                    previous_total: 2.34,
                    average_mode: true,
                    display_divisor: 1.0,
                    decimals: 2,
                    prefix: "".into(),
                    suffix: "%".into(),
                    lower_is_better: true,
                },
            ],
            overview: KpiOverviewSpec {
                label: "Gross volume".into(),
                dates: dates_90d.clone(),
                current: ov_cur_90d,
                previous: ov_prev_90d,
                segments: seg_90d,
            },
            focus: KpiFocusSpec {
                product_name: "Pro license".into(),
                product_subtitle: "One-time".into(),
                metric_label: "Licenses sold".into(),
                unit_singular: "license".into(),
                unit_plural: "licenses".into(),
                dates: dates_90d,
                current: vec![
                    64., 68., 72., 75., 79., 83., 88., 92., 96., 101., 106., 112., 118.,
                ],
                previous: vec![
                    56., 58., 61., 64., 66., 70., 73., 76., 79., 82., 86., 89., 94.,
                ],
            },
        },
    );

    // 12m dataset (12 monthly points Dec .. Nov)
    let dates_12m: Vec<SharedString> = vec![
        "Dec".into(),
        "Jan".into(),
        "Feb".into(),
        "Mar".into(),
        "Apr".into(),
        "May".into(),
        "Jun".into(),
        "Jul".into(),
        "Aug".into(),
        "Sep".into(),
        "Oct".into(),
        "Nov".into(),
    ];
    let ov_cur_12m: Vec<f64> = vec![
        94_000., 98_500., 104_200., 111_800., 118_400., 124_900., 131_600., 138_200., 146_500.,
        154_800., 163_900., 174_200.,
    ];
    let ov_prev_12m: Vec<f64> = vec![
        76_000., 79_200., 82_400., 86_100., 89_800., 94_200., 98_600., 103_400., 108_900.,
        114_200., 119_800., 126_400.,
    ];
    let mut seg_12m = HashMap::new();
    seg_12m.insert(
        SharedString::from("pro"),
        (
            ov_cur_12m.iter().map(|v| (v * 0.60).round()).collect(),
            ov_prev_12m.iter().map(|v| (v * 0.57).round()).collect(),
        ),
    );
    seg_12m.insert(
        SharedString::from("team"),
        (
            ov_cur_12m.iter().map(|v| (v * 0.40).round()).collect(),
            ov_prev_12m.iter().map(|v| (v * 0.43).round()).collect(),
        ),
    );
    map.insert(
        SharedString::from("12m"),
        KpiBoardData {
            stats: vec![
                KpiStatCardSpec {
                    id: "mrr".into(),
                    label: "Net revenue".into(),
                    dates: dates_12m.clone(),
                    series: vec![
                        34_200., 36_100., 38_400., 40_900., 43_200., 45_800., 48_600., 51_400.,
                        54_200., 57_600., 61_200., 65_400.,
                    ],
                    previous_total: 468_000.,
                    average_mode: false,
                    display_divisor: 1000.0,
                    decimals: 1,
                    prefix: "$".into(),
                    suffix: "k".into(),
                    lower_is_better: false,
                },
                KpiStatCardSpec {
                    id: "signups".into(),
                    label: "New customers".into(),
                    dates: dates_12m.clone(),
                    series: vec![
                        890., 940., 980., 1040., 1110., 1180., 1240., 1310., 1390., 1460., 1540.,
                        1640.,
                    ],
                    previous_total: 11_980.,
                    average_mode: false,
                    display_divisor: 1.0,
                    decimals: 0,
                    prefix: "".into(),
                    suffix: "".into(),
                    lower_is_better: false,
                },
                KpiStatCardSpec {
                    id: "churn".into(),
                    label: "Churn rate".into(),
                    dates: dates_12m.clone(),
                    series: vec![
                        2.48, 2.41, 2.36, 2.29, 2.22, 2.16, 2.10, 2.04, 1.98, 1.92, 1.86, 1.78,
                    ],
                    previous_total: 2.52,
                    average_mode: true,
                    display_divisor: 1.0,
                    decimals: 2,
                    prefix: "".into(),
                    suffix: "%".into(),
                    lower_is_better: true,
                },
            ],
            overview: KpiOverviewSpec {
                label: "Gross volume".into(),
                dates: dates_12m.clone(),
                current: ov_cur_12m,
                previous: ov_prev_12m,
                segments: seg_12m,
            },
            focus: KpiFocusSpec {
                product_name: "Pro license".into(),
                product_subtitle: "One-time".into(),
                metric_label: "Licenses sold".into(),
                unit_singular: "license".into(),
                unit_plural: "licenses".into(),
                dates: dates_12m,
                current: vec![
                    220., 234., 248., 262., 279., 296., 312., 328., 346., 368., 392., 418.,
                ],
                previous: vec![
                    185., 192., 204., 215., 228., 239., 251., 264., 278., 292., 308., 324.,
                ],
            },
        },
    );

    map
}

/// Top-of-dashboard KPI board block combining headline trend cards, a period comparison chart, and a focus card.
pub struct KpiBoardBlock {
    id: ElementId,
    focus: FocusHandle,
    title: SharedString,
    ranges: Vec<KpiRangeOption>,
    active_range: SharedString,
    segments: Vec<KpiSegmentOption>,
    active_segment: SharedString,
    compare_label: SharedString,
    current_label: SharedString,
    previous_label: SharedString,
    setup_label: Option<SharedString>,
    action_label: Option<SharedString>,
    data: HashMap<SharedString, KpiBoardData>,
    selected_stat: Option<usize>,
    hover_stat: Option<(usize, usize)>,
    hover_overview: Option<usize>,
    hover_focus: Option<usize>,
    loading: bool,
    empty: bool,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    boot_at: Instant,
}

impl EventEmitter<KpiBoardEvent> for KpiBoardBlock {}

impl KpiBoardBlock {
    /// Create a new [`KpiBoardBlock`] initialized on the `"30d"` reporting window.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: "Dashboard".into(),
            ranges: default_ranges(),
            active_range: "30d".into(),
            segments: default_segments(),
            active_segment: "all".into(),
            compare_label: "vs. last period".into(),
            current_label: "This period".into(),
            previous_label: "Last period".into(),
            setup_label: Some("Set up".into()),
            action_label: Some("See full analytics".into()),
            data: default_board_data(),
            selected_stat: None,
            hover_stat: None,
            hover_overview: None,
            hover_focus: None,
            loading: false,
            empty: false,
            measured_width: Rc::new(Cell::new(894.)),
            toast: None,
            boot_at: Instant::now(),
        }
    }

    /// Override the heading rendered at the top-left of the board.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Configure the available date-range options.
    pub fn with_ranges(mut self, ranges: Vec<KpiRangeOption>) -> Self {
        if let Some(first) = ranges.first()
            && !ranges.iter().any(|r| r.id == self.active_range)
        {
            self.active_range = first.id.clone();
        }
        self.ranges = ranges;
        self
    }

    /// Set the active date-range id (`"7d"`, `"30d"`, `"90d"`, `"12m"`).
    pub fn with_range(mut self, range_id: impl Into<SharedString>) -> Self {
        self.active_range = range_id.into();
        self
    }

    /// Alias matching `defaultRange` in the reference specification.
    pub fn with_default_range(self, range_id: impl Into<SharedString>) -> Self {
        self.with_range(range_id)
    }

    /// Configure segment options inside the overview card (`vec![]` hides the segment switch).
    pub fn with_segments(mut self, segments: Vec<KpiSegmentOption>) -> Self {
        if let Some(first) = segments.first()
            && !segments.iter().any(|s| s.id == self.active_segment)
        {
            self.active_segment = first.id.clone();
        }
        self.segments = segments;
        self
    }

    /// Set the active overview segment id (`"all"`, `"pro"`, `"team"`).
    pub fn with_segment(mut self, segment_id: impl Into<SharedString>) -> Self {
        self.active_segment = segment_id.into();
        self
    }

    /// Alias matching `defaultSegment` in the reference specification.
    pub fn with_default_segment(self, segment_id: impl Into<SharedString>) -> Self {
        self.with_segment(segment_id)
    }

    /// Supply custom datasets keyed by range id.
    pub fn with_data(mut self, data: HashMap<SharedString, KpiBoardData>) -> Self {
        self.data = data;
        self
    }

    /// Override the comparison caption shown next to percentage deltas.
    pub fn with_compare_label(mut self, label: impl Into<SharedString>) -> Self {
        self.compare_label = label.into();
        self
    }

    /// Override the current-period series label in tooltips and the focus legend.
    pub fn with_current_label(mut self, label: impl Into<SharedString>) -> Self {
        self.current_label = label.into();
        self
    }

    /// Override the previous-period series label in tooltips and the focus legend.
    pub fn with_previous_label(mut self, label: impl Into<SharedString>) -> Self {
        self.previous_label = label.into();
        self
    }

    /// Configure or hide (`None`) the `Set up` button in the focus card header.
    pub fn with_setup_label(mut self, label: Option<SharedString>) -> Self {
        self.setup_label = label;
        self
    }

    /// Configure or hide (`None`) the bottom action button in the focus card.
    pub fn with_action_label(mut self, label: Option<SharedString>) -> Self {
        self.action_label = label;
        self
    }

    /// Toggle shimmer skeleton placeholders across all board cards.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle the empty-board state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the active date-range id.
    pub fn range(&self) -> &str {
        self.active_range.as_ref()
    }

    /// Return the active overview segment id.
    pub fn segment(&self) -> &str {
        self.active_segment.as_ref()
    }

    /// Programmatically switch the active date range and emit [`KpiBoardEvent::RangeChanged`].
    pub fn set_range(&mut self, range_id: impl Into<SharedString>, cx: &mut Context<Self>) {
        let next = range_id.into();
        if self.active_range != next {
            self.active_range = next.clone();
            self.hover_stat = None;
            self.hover_overview = None;
            self.hover_focus = None;
            cx.emit(KpiBoardEvent::RangeChanged(next));
            cx.notify();
        }
    }

    /// Programmatically switch the active overview segment and emit [`KpiBoardEvent::SegmentChanged`].
    pub fn set_segment(&mut self, segment_id: impl Into<SharedString>, cx: &mut Context<Self>) {
        let next = segment_id.into();
        if self.active_segment != next {
            self.active_segment = next.clone();
            cx.emit(KpiBoardEvent::SegmentChanged(next));
            cx.notify();
        }
    }

    fn current_board(&self) -> Option<&KpiBoardData> {
        self.data
            .get(&self.active_range)
            .or_else(|| self.ranges.first().and_then(|r| self.data.get(&r.id)))
            .or_else(|| self.data.values().next())
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if ev.keystroke.modifiers.platform || ev.keystroke.modifiers.control {
            return;
        }
        let Some(board) = self.current_board() else {
            return;
        };
        let len = board.overview.current.len();
        if len == 0 {
            return;
        }
        match ev.keystroke.key.as_str() {
            "left" => {
                let next = match self.hover_overview {
                    Some(i) => i.saturating_sub(1),
                    None => len - 1,
                };
                self.hover_overview = Some(next);
                cx.emit(KpiBoardEvent::DayInspected {
                    target: "overview".into(),
                    index: Some(next),
                });
                cx.stop_propagation();
                cx.notify();
            }
            "right" => {
                let next = match self.hover_overview {
                    Some(i) => (i + 1).min(len - 1),
                    None => 0,
                };
                self.hover_overview = Some(next);
                cx.emit(KpiBoardEvent::DayInspected {
                    target: "overview".into(),
                    index: Some(next),
                });
                cx.stop_propagation();
                cx.notify();
            }
            "home" => {
                self.hover_overview = Some(0);
                cx.emit(KpiBoardEvent::DayInspected {
                    target: "overview".into(),
                    index: Some(0),
                });
                cx.stop_propagation();
                cx.notify();
            }
            "end" => {
                let last = len - 1;
                self.hover_overview = Some(last);
                cx.emit(KpiBoardEvent::DayInspected {
                    target: "overview".into(),
                    index: Some(last),
                });
                cx.stop_propagation();
                cx.notify();
            }
            "escape"
                if self.hover_overview.take().is_some()
                    || self.hover_stat.take().is_some()
                    || self.hover_focus.take().is_some() =>
            {
                cx.emit(KpiBoardEvent::DayInspected {
                    target: "overview".into(),
                    index: None,
                });
                cx.stop_propagation();
                cx.notify();
            }
            _ => {}
        }
    }

    fn render_delta_badge(&self, pct: f64, is_good: bool, cx: &App) -> Div {
        let p = ProPalette::get(cx);
        let (bg, fg) = if is_good {
            (p.green_soft, p.green_ink)
        } else {
            (p.red_soft, p.red_ink)
        };
        let up = pct >= 0.0;
        let glyph = if up { DELTA_UP_SVG } else { DELTA_DOWN_SVG };
        div()
            .h(px(20.))
            .px(px(6.))
            .rounded(px(5.))
            .bg(bg)
            .flex_none()
            .flex()
            .items_center()
            .gap(px(4.))
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(fg)
            .child(pro_svg("0 0 8 8", glyph, 6.5, fg))
            .child(format!("{:.1}%", pct.abs()))
    }

    fn translate_text<'a>(&self, raw: &'a str, cx: &App) -> &'a str {
        match raw {
            "Dashboard" => tr(cx, "Dashboard", "数据总览"),
            "Net revenue" => tr(cx, "Net revenue", "净收入"),
            "New customers" => tr(cx, "New customers", "新增客户"),
            "Churn rate" => tr(cx, "Churn rate", "流失率"),
            "Gross volume" => tr(cx, "Gross volume", "总交易额"),
            "All plans" => tr(cx, "All plans", "全部方案"),
            "Pro" => tr(cx, "Pro", "专业版"),
            "Team" => tr(cx, "Team", "团队版"),
            "Pro license" => tr(cx, "Pro license", "专业版授权"),
            "One-time" => tr(cx, "One-time", "买断授权"),
            "Licenses sold" => tr(cx, "Licenses sold", "已售授权数"),
            "vs. last period" => tr(cx, "vs. last period", "较上周期"),
            "This period" => tr(cx, "This period", "本周期"),
            "Last period" => tr(cx, "Last period", "上周期"),
            "Set up" => tr(cx, "Set up", "配置"),
            "See full analytics" => tr(cx, "See full analytics", "查看完整分析"),
            other => other,
        }
    }

    fn render_stat_card(
        &self,
        card_idx: usize,
        spec: &KpiStatCardSpec,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let agg = spec.aggregate_value();
        let divisor = if spec.display_divisor.abs() > 1e-9 {
            spec.display_divisor
        } else {
            1.0
        };
        let shown_val = agg / divisor;
        let delta_opt = spec.delta();
        let compare_lbl = self.translate_text(&self.compare_label, cx).to_owned();
        let card_label = self.translate_text(&spec.label, cx).to_owned();

        let hover_day = self
            .hover_stat
            .and_then(|(ci, di)| (ci == card_idx && di < spec.series.len()).then_some(di));
        let pts_f32: Vec<f32> = spec.series.iter().map(|&v| v as f32).collect();
        let count = spec.series.len();
        let stat_id = spec.id.clone();

        let is_selected = self.selected_stat == Some(card_idx);
        let dimmed = self.selected_stat.is_some_and(|sel| sel != card_idx);
        let card_alpha = pro_spring_f32(
            pro_eid(&self.id, format!("stat-alpha-{}", spec.id)),
            if dimmed { 0.72 } else { 1.0 },
            0.22,
            w,
            cx,
        );
        let sel_mix = pro_spring_f32(
            pro_eid(&self.id, format!("stat-sel-{}", spec.id)),
            if is_selected { 1.0 } else { 0.0 },
            0.22,
            w,
            cx,
        );
        let card_border = pro_mix_color(p.line, p.blue.opacity(0.65), sel_mix);

        let spec_clone = spec.clone();
        let floating_overlay = ScrubReadoutOverlay {
            id: pro_eid(&self.id, format!("stat-tag-{}", spec.id)),
            active_idx: hover_day,
            count,
            pill_width: 108.,
            fallback_w: 240.,
            top: 2.,
            render_pill: Box::new(move |di| {
                let val_str = format_stat_point(
                    &spec_clone,
                    spec_clone.series.get(di).copied().unwrap_or(0.0),
                );
                let date_str = spec_clone
                    .dates
                    .get(di)
                    .cloned()
                    .unwrap_or_else(|| SharedString::from(format!("Day {}", di + 1)));
                let chip_bg: Hsla = if p.dark {
                    rgb(0xf4f4f5).into()
                } else {
                    rgb(0x18181b).into()
                };
                let chip_fg: Hsla = if p.dark {
                    rgb(0x111113).into()
                } else {
                    rgb(0xfafafa).into()
                };
                div()
                    .px(px(7.))
                    .py(px(2.))
                    .rounded(px(5.))
                    .bg(chip_bg)
                    .text_color(chip_fg)
                    .text_size(px(11.))
                    .shadow_sm()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .whitespace_nowrap()
                    .child(div().font_weight(FontWeight::MEDIUM).child(val_str))
                    .child(div().opacity(0.68).child(date_str))
                    .into_any_element()
            }),
        };

        let hit_slices = div()
            .absolute()
            .inset_0()
            .flex()
            .children((0..count).map(|di| {
                let target_id = stat_id.clone();
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("stat-{card_idx}-pt-{di}")))
                        .flex_1()
                        .h_full()
                        .cursor_pointer()
                        .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                            if *inside {
                                this.hover_stat = Some((card_idx, di));
                            } else if this.hover_stat == Some((card_idx, di)) {
                                this.hover_stat = None;
                            }
                            cx.notify();
                        })),
                    cx,
                    move |this, _, cx| {
                        this.hover_stat = Some((card_idx, di));
                        cx.emit(KpiBoardEvent::DayInspected {
                            target: target_id.clone(),
                            index: Some(di),
                        });
                        cx.notify();
                    },
                )
            }));

        let header_content = div()
            .w_full()
            .p(px(6.))
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(p.text)
                    .truncate()
                    .child(card_label.clone()),
            )
            .child(pro_rolling_number(
                pro_eid(&self.id, format!("stat-num-{}", spec.id)),
                shown_val,
                spec.decimals,
                spec.prefix.clone(),
                spec.suffix.clone(),
                24.,
                FontWeight::MEDIUM,
                p.strong,
            ))
            .child(
                div()
                    .mt(px(2.))
                    .min_h(px(20.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .when_some(delta_opt, |d, (pct, good)| {
                        d.child(self.render_delta_badge(pct, good, cx)).child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .truncate()
                                .child(compare_lbl),
                        )
                    }),
            );

        div()
            .flex_1()
            .min_w_0()
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(card_border)
            .opacity(card_alpha)
            .when(sel_mix > 0.1, |d| d.shadow_sm())
            .p(px(9.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                pro_tactile(
                    pro_eid(&self.id, format!("stat-card-{}", spec.id)),
                    card_label,
                    8.,
                    transparent_black(),
                    p.hover,
                    transparent_black(),
                    transparent_black(),
                    is_selected,
                    header_content,
                    cx,
                    move |this, _, cx| {
                        this.selected_stat = if this.selected_stat == Some(card_idx) {
                            None
                        } else {
                            Some(card_idx)
                        };
                        cx.notify();
                    },
                )
                .w_full(),
            )
            .child(
                div().px(px(6.)).pb(px(6.)).child(
                    div()
                        .relative()
                        .w_full()
                        .h(px(46.))
                        .child(KpiInteractiveSeries {
                            id: pro_eid(&self.id, format!("stat-spark-{}", spec.id)),
                            current: pts_f32,
                            previous: None,
                            accent: p.blue,
                            card_bg: p.card,
                            grid_color: p.line_soft,
                            guide_color: p.line_strong,
                            show_grid: false,
                            fill_alpha: 0.16,
                            height: 46.,
                            hover_idx: hover_day,
                        })
                        .child(floating_overlay)
                        .child(hit_slices),
                ),
            )
    }

    fn render_overview_card(&self, overview: &KpiOverviewSpec, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let (cur_slice, prev_slice) = overview.series_for_segment(&self.active_segment);
        let cur_total = sum_finite(cur_slice);
        let prev_total = sum_finite(prev_slice);
        let delta_opt = compute_delta(cur_total, prev_total, false);

        let ov_title = self.translate_text(&overview.label, cx).to_owned();
        let compare_lbl = self.translate_text(&self.compare_label, cx).to_owned();
        let cur_lbl = self.translate_text(&self.current_label, cx).to_owned();
        let prev_lbl = self.translate_text(&self.previous_label, cx).to_owned();

        // Segment pill control inside overview header
        let seg_control = if !self.segments.is_empty() {
            let sensor = pro_selection_sensor();
            let thumb_bg: Hsla = if p.dark {
                rgb(0x2a2a2e).into()
            } else {
                rgb(0xffffff).into()
            };
            let mut row = div()
                .relative()
                .p(px(2.))
                .rounded(px(8.))
                .bg(p.chip)
                .flex()
                .items_center()
                .gap(px(2.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "ov-seg-pill"),
                    self.active_segment.clone(),
                    sensor.clone(),
                    6.,
                    thumb_bg,
                    Some(p.line),
                    true,
                ));
            for seg in &self.segments {
                let selected = seg.id == self.active_segment;
                let seg_id = seg.id.clone();
                let seg_text = self.translate_text(&seg.label, cx).to_owned();
                let seg_hover = if selected {
                    transparent_black()
                } else {
                    p.hover
                };
                row = row.child(
                    pro_tactile(
                        pro_eid(&self.id, format!("ov-seg-{seg_id}")),
                        seg_text.clone(),
                        6.,
                        transparent_black(),
                        seg_hover,
                        transparent_black(),
                        transparent_black(),
                        selected,
                        div()
                            .h(px(24.))
                            .px(px(9.))
                            .flex()
                            .items_center()
                            .whitespace_nowrap()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if selected { p.strong } else { p.muted })
                            .child(seg_text),
                        cx,
                        move |this, _, cx| {
                            this.set_segment(seg_id.clone(), cx);
                        },
                    )
                    .role(Role::Tab)
                    .relative()
                    .flex_none()
                    .when(selected, |d| d.child(pro_mark_selected(&sensor))),
                );
            }
            Some(row)
        } else {
            None
        };

        let cur_f32: Vec<f32> = cur_slice.iter().map(|&v| v as f32).collect();
        let prev_f32: Vec<f32> = prev_slice.iter().map(|&v| v as f32).collect();
        let count = cur_slice.len();
        let hover_di = self.hover_overview.filter(|&d| d < count);

        let ov_dates = overview.dates.clone();
        let cur_vec = cur_slice.to_vec();
        let prev_vec = prev_slice.to_vec();
        let cur_tip_lbl = cur_lbl.clone();
        let prev_tip_lbl = prev_lbl.clone();
        let tooltip_overlay = ScrubReadoutOverlay {
            id: pro_eid(&self.id, "ov-tip"),
            active_idx: hover_di,
            count,
            pill_width: 174.,
            fallback_w: 500.,
            top: 6.,
            render_pill: Box::new(move |di| {
                let date_str = ov_dates
                    .get(di)
                    .cloned()
                    .unwrap_or_else(|| SharedString::from(format!("Day {}", di + 1)));
                let cur_v = cur_vec.get(di).copied().unwrap_or(0.0);
                let prev_v = prev_vec.get(di).copied().unwrap_or(0.0);
                let pop_bg: Hsla = if p.dark {
                    rgb(0x252528).into()
                } else {
                    rgb(0xffffff).into()
                };
                div()
                    .min_w(px(164.))
                    .px(px(10.))
                    .py(px(8.))
                    .rounded(px(10.))
                    .bg(pop_bg)
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_md()
                    .flex()
                    .flex_col()
                    .gap(px(5.))
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .pb(px(4.))
                            .border_b_1()
                            .border_color(p.line)
                            .child(date_str),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(12.))
                            .text_size(px(12.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_color(p.text)
                                    .child(div().size(px(8.)).rounded(px(2.)).bg(p.blue))
                                    .child(cur_tip_lbl),
                            )
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(format_money_compact(cur_v)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(12.))
                            .text_size(px(12.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_color(p.muted)
                                    .child(
                                        div()
                                            .w(px(8.))
                                            .h(px(2.))
                                            .rounded(px(1.))
                                            .bg(p.blue.opacity(0.42)),
                                    )
                                    .child(prev_tip_lbl),
                            )
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.text)
                                    .child(format_money_compact(prev_v)),
                            ),
                    )
                    .into_any_element()
            }),
        };

        let hit_slices = div()
            .absolute()
            .inset_0()
            .flex()
            .children((0..count).map(|di| {
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("ov-pt-{di}")))
                        .flex_1()
                        .h_full()
                        .cursor_pointer()
                        .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                            if *inside {
                                this.hover_overview = Some(di);
                            } else if this.hover_overview == Some(di) {
                                this.hover_overview = None;
                            }
                            cx.notify();
                        })),
                    cx,
                    move |this, _, cx| {
                        this.hover_overview = Some(di);
                        cx.emit(KpiBoardEvent::DayInspected {
                            target: "overview".into(),
                            index: Some(di),
                        });
                        cx.notify();
                    },
                )
            }));

        let first_date = overview.dates.first().cloned().unwrap_or_default();
        let mid_date = overview
            .dates
            .get(overview.dates.len() / 2)
            .cloned()
            .unwrap_or_default();
        let last_date = overview.dates.last().cloned().unwrap_or_default();

        div()
            .w_full()
            .min_w_0()
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .p(px(16.))
            .flex()
            .flex_col()
            .justify_between()
            .gap(px(14.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(div().text_size(px(13.)).text_color(p.text).child(ov_title))
                            .when_some(seg_control, |d, sc| d.child(sc)),
                    )
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "ov-total-num"),
                        cur_total / 1000.0,
                        1,
                        "$",
                        "k",
                        24.,
                        FontWeight::MEDIUM,
                        p.strong,
                    ))
                    .child(
                        div()
                            .mt(px(2.))
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(8.))
                            .when_some(delta_opt, |d, (pct, good)| {
                                d.child(self.render_delta_badge(pct, good, cx))
                            })
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(compare_lbl),
                            )
                            .child(
                                div()
                                    .ml(px(6.))
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(5.))
                                            .child(
                                                div()
                                                    .w(px(10.))
                                                    .h(px(2.))
                                                    .rounded_full()
                                                    .bg(p.blue),
                                            )
                                            .child(cur_lbl),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(5.))
                                            .child(
                                                div()
                                                    .w(px(10.))
                                                    .h(px(2.))
                                                    .rounded_full()
                                                    .bg(p.blue.opacity(0.38)),
                                            )
                                            .child(prev_lbl),
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .relative()
                            .w_full()
                            .h(px(152.))
                            .child(KpiInteractiveSeries {
                                id: pro_eid(&self.id, "ov-chart"),
                                current: cur_f32,
                                previous: Some(prev_f32),
                                accent: p.blue,
                                card_bg: p.card,
                                grid_color: p.line_soft,
                                guide_color: p.line_strong,
                                show_grid: true,
                                fill_alpha: 0.18,
                                height: 152.,
                                hover_idx: hover_di,
                            })
                            .child(tooltip_overlay)
                            .child(hit_slices),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(11.))
                            .text_color(p.muted)
                            .child(first_date)
                            .child(mid_date)
                            .child(last_date),
                    ),
            )
    }

    fn render_focus_card(&self, focus: &KpiFocusSpec, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let cur_total = focus.current_total();
        let prev_total = focus.previous_total();
        let delta_opt = compute_delta(cur_total, prev_total, false);

        let prod_name = self.translate_text(&focus.product_name, cx).to_owned();
        let prod_sub = self.translate_text(&focus.product_subtitle, cx).to_owned();
        let metric_lbl = self.translate_text(&focus.metric_label, cx).to_owned();
        let compare_lbl = self.translate_text(&self.compare_label, cx).to_owned();
        let cur_lbl = self.translate_text(&self.current_label, cx).to_owned();
        let prev_lbl = self.translate_text(&self.previous_label, cx).to_owned();

        let count = focus.current.len();
        let hover_di = self.hover_focus.filter(|&d| d < count);

        let (legend_prev, legend_cur) = if let Some(di) = hover_di {
            (
                focus.previous.get(di).copied().unwrap_or(0.0),
                focus.current.get(di).copied().unwrap_or(0.0),
            )
        } else {
            (prev_total, cur_total)
        };

        let unit_suffix = format!(" {}", focus.unit_for_count(cur_total));
        let cur_f32: Vec<f32> = focus.current.iter().map(|&v| v as f32).collect();

        let focus_clone = focus.clone();
        let focus_overlay = ScrubReadoutOverlay {
            id: pro_eid(&self.id, "focus-tag"),
            active_idx: hover_di,
            count,
            pill_width: 112.,
            fallback_w: 248.,
            top: 2.,
            render_pill: Box::new(move |di| {
                let cv = focus_clone.current.get(di).copied().unwrap_or(0.0);
                let val_str = format!(
                    "{} {}",
                    format_grouped_int(cv),
                    focus_clone.unit_for_count(cv)
                );
                let date_str = focus_clone
                    .dates
                    .get(di)
                    .cloned()
                    .unwrap_or_else(|| SharedString::from(format!("Day {}", di + 1)));
                let chip_bg: Hsla = if p.dark {
                    rgb(0xf4f4f5).into()
                } else {
                    rgb(0x18181b).into()
                };
                let chip_fg: Hsla = if p.dark {
                    rgb(0x111113).into()
                } else {
                    rgb(0xfafafa).into()
                };
                div()
                    .px(px(7.))
                    .py(px(2.))
                    .rounded(px(5.))
                    .bg(chip_bg)
                    .text_color(chip_fg)
                    .text_size(px(11.))
                    .shadow_sm()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .whitespace_nowrap()
                    .child(div().font_weight(FontWeight::MEDIUM).child(val_str))
                    .child(div().opacity(0.68).child(date_str))
                    .into_any_element()
            }),
        };

        let hit_slices = div()
            .absolute()
            .inset_0()
            .flex()
            .children((0..count).map(|di| {
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("focus-pt-{di}")))
                        .flex_1()
                        .h_full()
                        .cursor_pointer()
                        .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                            if *inside {
                                this.hover_focus = Some(di);
                            } else if this.hover_focus == Some(di) {
                                this.hover_focus = None;
                            }
                            cx.notify();
                        })),
                    cx,
                    move |this, _, cx| {
                        this.hover_focus = Some(di);
                        cx.emit(KpiBoardEvent::DayInspected {
                            target: "focus".into(),
                            index: Some(di),
                        });
                        cx.notify();
                    },
                )
            }));

        let setup_btn = self.setup_label.as_ref().map(|lbl| {
            let text = self.translate_text(lbl, cx).to_owned();
            pro_tactile(
                pro_eid(&self.id, "focus-setup"),
                text.clone(),
                6.,
                p.card,
                p.hover,
                p.line_strong,
                p.line_strong,
                false,
                div()
                    .h(px(28.))
                    .px(px(9.))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .whitespace_nowrap()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(pro_svg("0 0 14 14", SLIDERS_SETUP_SVG, 12., p.muted))
                    .child(text),
                cx,
                |this, _, cx| {
                    this.toast = Some((
                        tr(cx, "Opened Pro license setup", "已打开专业版授权设置").into(),
                        Instant::now(),
                    ));
                    cx.emit(KpiBoardEvent::SetupClicked);
                    cx.notify();
                },
            )
            .flex_none()
        });

        let action_btn = self.action_label.as_ref().map(|lbl| {
            let text = self.translate_text(lbl, cx).to_owned();
            pro_btn(
                pro_eid(&self.id, "focus-action"),
                text,
                None,
                ProBtnStyle::Secondary,
                false,
                cx,
                |this, _, cx| {
                    this.toast = Some((
                        tr(cx, "Opening full license analytics", "正在打开完整授权分析").into(),
                        Instant::now(),
                    ));
                    cx.emit(KpiBoardEvent::ActionClicked);
                    cx.notify();
                },
            )
            .w_full()
            .justify_center()
        });

        div()
            .w_full()
            .h_full()
            .min_w_0()
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .p(px(16.))
            .flex()
            .flex_col()
            .justify_between()
            .gap(px(14.))
            .child(
                div()
                    .flex()
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
                                    .size(px(32.))
                                    .flex_none()
                                    .rounded(px(8.))
                                    .bg(p.subtle)
                                    .border_1()
                                    .border_color(p.line_strong)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(pro_svg("0 0 16 16", CUBE_FOCUS_SVG, 15., p.muted)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .truncate()
                                            .child(prod_name),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(prod_sub),
                                    ),
                            ),
                    )
                    .when_some(setup_btn, |d, b| d.child(b)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(metric_lbl),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(8.))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "focus-num"),
                                cur_total,
                                0,
                                "",
                                unit_suffix,
                                20.,
                                FontWeight::MEDIUM,
                                p.strong,
                            ))
                            .when_some(delta_opt, |d, (pct, good)| {
                                d.child(self.render_delta_badge(pct, good, cx)).child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(p.muted)
                                        .child(compare_lbl),
                                )
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .relative()
                            .w_full()
                            .h(px(58.))
                            .child(KpiInteractiveSeries {
                                id: pro_eid(&self.id, "focus-spark"),
                                current: cur_f32,
                                previous: None,
                                accent: p.blue,
                                card_bg: p.card,
                                grid_color: p.line_soft,
                                guide_color: p.line_strong,
                                show_grid: false,
                                fill_alpha: 0.16,
                                height: 58.,
                                hover_idx: hover_di,
                            })
                            .child(focus_overlay)
                            .child(hit_slices),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(12.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(prev_lbl)
                                    .child(pro_rolling_number(
                                        pro_eid(&self.id, "focus-leg-prev"),
                                        legend_prev,
                                        0,
                                        "",
                                        "",
                                        12.,
                                        FontWeight::MEDIUM,
                                        p.text,
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(cur_lbl)
                                    .child(pro_rolling_number(
                                        pro_eid(&self.id, "focus-leg-cur"),
                                        legend_cur,
                                        0,
                                        "",
                                        "",
                                        12.,
                                        FontWeight::MEDIUM,
                                        p.strong,
                                    )),
                            ),
                    ),
            )
            .when_some(action_btn, |d, b| d.child(b))
    }

    fn render_loading_skeleton(&self, stack_stats: bool, stack_lower: bool, cx: &App) -> Div {
        let p = ProPalette::get(cx);
        let phase = (self.boot_at.elapsed().as_secs_f32() * 0.9).fract();
        let skel_card = |h: f32| {
            div()
                .flex_1()
                .min_w_0()
                .h(px(h))
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .p(px(16.))
                .flex()
                .flex_col()
                .justify_between()
                .child(pro_skeleton_bar(84., 12., 4., phase, cx))
                .child(pro_skeleton_bar(112., 24., 6., phase + 0.15, cx))
                .child(pro_skeleton_bar(140., 40., 6., phase + 0.30, cx))
        };
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .when(stack_stats, |d| d.flex_col())
                    .gap(px(12.))
                    .child(skel_card(148.))
                    .child(skel_card(148.))
                    .child(skel_card(148.)),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .when(stack_lower, |d| d.flex_col())
                    .gap(px(12.))
                    .child(div().flex_1().child(skel_card(248.)))
                    .child(
                        div()
                            .when(!stack_lower, |d| d.w(px(284.)).flex_none())
                            .when(stack_lower, |d| d.w_full())
                            .child(skel_card(248.)),
                    ),
            )
    }
}

impl Render for KpiBoardBlock {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        if self.loading && !disabled_motion(cx) {
            w.request_animation_frame();
        }
        let width = self.measured_width.get();
        let stack_lower = width < 820.;
        let stack_stats = width < 600.;
        let measure_cell = self.measured_width.clone();

        // Top bar: Left heading + Right period range selector with sliding pill indicator
        let heading_text = self.translate_text(&self.title, cx).to_owned();
        let range_sensor = pro_selection_sensor();
        let thumb_bg: Hsla = if p.dark {
            rgb(0x2a2a2e).into()
        } else {
            rgb(0xffffff).into()
        };
        let mut range_bar = div()
            .relative()
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.chip)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "range-pill"),
                self.active_range.clone(),
                range_sensor.clone(),
                6.,
                thumb_bg,
                Some(p.line),
                true,
            ));

        for opt in &self.ranges {
            let selected = opt.id == self.active_range;
            let rid = opt.id.clone();
            let rlabel = opt.label.clone();
            let opt_hover = if selected {
                transparent_black()
            } else {
                p.hover
            };
            range_bar = range_bar.child(
                pro_tactile(
                    pro_eid(&self.id, format!("range-opt-{rid}")),
                    rlabel.clone(),
                    6.,
                    transparent_black(),
                    opt_hover,
                    transparent_black(),
                    transparent_black(),
                    selected,
                    div()
                        .h(px(24.))
                        .px(px(10.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .whitespace_nowrap()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if selected { p.strong } else { p.text })
                        .child(rlabel),
                    cx,
                    move |this, _, cx| {
                        this.set_range(rid.clone(), cx);
                    },
                )
                .role(Role::Tab)
                .relative()
                .flex_none()
                .when(selected, |d| d.child(pro_mark_selected(&range_sensor))),
            );
        }

        let top_bar = div()
            .w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(heading_text),
            )
            .child(range_bar);

        let body: AnyElement = if self.loading {
            self.render_loading_skeleton(stack_stats, stack_lower, cx)
                .into_any_element()
        } else if self.empty || self.current_board().is_none() {
            div()
                .w_full()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::UsageChart,
                    tr(cx, "No KPI metrics yet", "暂无关键指标数据"),
                    tr(
                        cx,
                        "Connect your billing or telemetry source to populate headline trends and period comparisons.",
                        "连接账单或遥测数据源后，此处将展示核心趋势与周期对比。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element()
        } else if let Some(board) = self.current_board().cloned() {
            let mut stats_row = div()
                .w_full()
                .flex()
                .when(stack_stats, |d| d.flex_col())
                .gap(px(if stack_stats { 8. } else { 12. }));
            for (idx, stat) in board.stats.iter().enumerate() {
                stats_row = stats_row.child(self.render_stat_card(idx, stat, w, cx));
            }

            let ov_card = self.render_overview_card(&board.overview, cx);
            let focus_card = self.render_focus_card(&board.focus, cx);

            let lower_row = div()
                .w_full()
                .flex()
                .when(stack_lower, |d| d.flex_col())
                .items_stretch()
                .gap(px(if stack_stats { 8. } else { 12. }))
                .child(div().flex_1().min_w_0().child(ov_card))
                .child(
                    div()
                        .when(!stack_lower, |d| d.w(px(286.)).flex_none())
                        .when(stack_lower, |d| d.w_full())
                        .child(focus_card),
                );

            pro_view_transition(
                pro_eid(&self.id, "board-vtx"),
                SharedString::from("ready"),
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(if stack_stats { 8. } else { 12. }))
                    .child(stats_row)
                    .child(lower_row),
            )
        } else {
            div().into_any_element()
        };

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .relative()
            .w_full()
            .min_w_0()
            .px(px(if stack_stats { 16. } else { 32. }))
            .py(px(if stack_stats { 20. } else { 28. }))
            .bg(p.page)
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                canvas(
                    move |b, window, _| {
                        let nw = f32::from(b.size.width);
                        if nw > 10. && (measure_cell.get() - nw).abs() > 1.0 {
                            measure_cell.set(nw);
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .w_full()
                .h(px(0.)),
            )
            .child(top_bar)
            .child(body)
            .child(pro_toast_banner(self.toast.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        KpiFocusSpec, KpiStatCardSpec, compute_delta, default_board_data, default_ranges,
        default_segments, format_grouped_int,
    };

    #[test]
    fn default_ranges_and_segments_populate_complete_board_series() {
        let ranges = default_ranges();
        let segments = default_segments();
        let data = default_board_data();

        assert_eq!(ranges.len(), 4);
        assert_eq!(segments.len(), 3);
        for opt in &ranges {
            let board = data
                .get(&opt.id)
                .expect("dataset exists for every default range");
            assert_eq!(board.stats.len(), 3);
            assert!(board.overview.current.len() >= 7);
            assert_eq!(board.overview.current.len(), board.overview.previous.len());
            assert_eq!(board.focus.current.len(), board.focus.previous.len());
            let (pro_cur, pro_prev) = board.overview.series_for_segment("pro");
            assert_eq!(pro_cur.len(), pro_prev.len());
        }
    }

    #[test]
    fn stat_aggregation_handles_nan_lower_is_better_and_zero_previous() {
        let churn = KpiStatCardSpec {
            id: "churn".into(),
            label: "Churn rate".into(),
            dates: vec!["D1".into(), "D2".into(), "D3".into()],
            series: vec![2.0, f64::NAN, 1.6],
            previous_total: 2.2,
            average_mode: true,
            display_divisor: 1.0,
            decimals: 2,
            prefix: "".into(),
            suffix: "%".into(),
            lower_is_better: true,
        };
        assert!((churn.aggregate_value() - 1.8).abs() < 1e-6);
        let (pct, is_good) = churn.delta().expect("non-zero previous produces delta");
        assert!(pct < 0.0);
        assert!(is_good, "falling churn must be marked good");

        assert!(compute_delta(120.0, 0.0, false).is_none());
    }

    #[test]
    fn focus_singular_and_plural_units_follow_count() {
        let focus = KpiFocusSpec {
            product_name: "Pro license".into(),
            product_subtitle: "One-time".into(),
            metric_label: "Licenses sold".into(),
            unit_singular: "license".into(),
            unit_plural: "licenses".into(),
            dates: vec!["Nov 1".into()],
            current: vec![1.0],
            previous: vec![0.0],
        };
        assert_eq!(focus.unit_for_count(1.0), "license");
        assert_eq!(focus.unit_for_count(0.0), "licenses");
        assert_eq!(focus.unit_for_count(342.0), "licenses");
        assert_eq!(format_grouped_int(292_444.0), "292,444");
    }
}
