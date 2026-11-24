//! Sales and commerce metrics dashboard block with clickable KPI cards, area/bar trend chart, and ranked shares.
//!
//! Combines a period switcher (`Week`, `Month`, `Quarter`) and CSV export confirmation with four
//! metric-selecting KPI cards, an area-or-bars `TrendChart` with previous-period comparison and
//! daily scrubbing, and two spring-bar breakdown lists (`Top products` and `Channels`).

use crate::{
    components::pro_chrome::{
        ProPalette, pro_ease, pro_eid, pro_empty_state, pro_mark_selected, pro_pill_indicator,
        pro_rolling_number, pro_selection_sensor, pro_skeleton_bar, pro_spring_f32, pro_switch,
        pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::{UiLanguage, activate},
        family::{PhaseClock, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const EXPORT_ARROW_SVG: &str = r#"<path d="M12 14.5V3.5m0 11c-.7 0-2.03-1.94-2.5-2.5M12 14.5c.7 0 2.03-1.94 2.5-2.5M20.5 16.5c0 2.48-.52 3-3 3h-11c-2.48 0-3-.52-3-3" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const KIND_AREA_SVG: &str = r#"<path d="M3.5 16.5 9 11l4 3.5 7.5-8" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"></path><circle cx="9" cy="11" r="1.6" stroke="currentColor" stroke-width="1.5"></circle><circle cx="13" cy="14.5" r="1.6" stroke="currentColor" stroke-width="1.5"></circle>"#;
const KIND_BARS_SVG: &str = r#"<path d="M5.5 19.5v-6M10 19.5V8M14.5 19.5v-9M19 19.5V5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"></path>"#;
const DELTA_UP_SVG: &str = r#"<path d="M4 1.2 7.2 6.4H.8L4 1.2Z" fill="currentColor"></path>"#;
const DELTA_DOWN_SVG: &str = r#"<path d="M4 6.8.8 1.6h6.4L4 6.8Z" fill="currentColor"></path>"#;

/// Reporting period in [`MetricsDashboardBlock`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DashboardRange {
    /// Last 7 days (`Week`).
    Week,
    /// Last 30 days (`Month`).
    #[default]
    Month,
    /// Last 90 days (`Quarter`).
    Quarter,
}

impl DashboardRange {
    const ALL: [Self; 3] = [Self::Week, Self::Month, Self::Quarter];

    fn slug(self) -> &'static str {
        match self {
            Self::Week => "7d",
            Self::Month => "30d",
            Self::Quarter => "90d",
        }
    }

    fn button_label(self, cx: &App) -> &'static str {
        match self {
            Self::Week => tr(cx, "Week", "周"),
            Self::Month => tr(cx, "Month", "月"),
            Self::Quarter => tr(cx, "Quarter", "季"),
        }
    }

    fn subtitle_label(self, cx: &App) -> &'static str {
        match self {
            Self::Week => tr(cx, "Last 7 days", "近 7 天"),
            Self::Month => tr(cx, "Last 30 days", "近 30 天"),
            Self::Quarter => tr(cx, "Last 90 days", "近 90 天"),
        }
    }

    fn days_len(self) -> usize {
        match self {
            Self::Week => 7,
            Self::Month => 30,
            Self::Quarter => 90,
        }
    }
}

/// Active KPI metric driving the central trend chart in [`MetricsDashboardBlock`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DashboardMetric {
    /// Gross sales revenue (`$292,444` on the default 30-day dataset).
    #[default]
    Revenue,
    /// Completed order count (`4,434` on the default 30-day dataset).
    Orders,
    /// Average order value (`Revenue / Orders`).
    AvgOrder,
    /// Percentage of orders refunded (lower is better).
    RefundRate,
    /// Storefront checkout conversion rate (`%`).
    Conversion,
}

impl DashboardMetric {
    fn slug(self) -> &'static str {
        match self {
            Self::Revenue => "revenue",
            Self::Orders => "orders",
            Self::AvgOrder => "aov",
            Self::RefundRate => "refunds",
            Self::Conversion => "conversion",
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Revenue => tr(cx, "Revenue", "营业额"),
            Self::Orders => tr(cx, "Orders", "订单量"),
            Self::AvgOrder => tr(cx, "Avg. order", "客单价"),
            Self::RefundRate => tr(cx, "Refund rate", "退款率"),
            Self::Conversion => tr(cx, "Conversion", "转化率"),
        }
    }
}

/// Visual representation mode of the central trend chart in [`MetricsDashboardBlock`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DashboardChartKind {
    /// Smooth area-filled polyline.
    #[default]
    Area,
    /// Vertical daily/bucket columns growing from the zero baseline.
    Bars,
}

/// Daily commerce observation in [`MetricsDashboardBlock`], ordered oldest to newest.
#[derive(Clone, Debug, PartialEq)]
pub struct DailySalesPoint {
    /// Short calendar label (such as `"Oct 26"` or `"Nov 24"`).
    pub date: SharedString,
    /// Gross revenue for the day.
    pub revenue: f64,
    /// Completed order count for the day.
    pub orders: u32,
    /// Refunded order count for the day.
    pub refunded: u32,
    /// Storefront conversion percentage for the day (for example `3.42`).
    pub conversion_pct: f64,
}

impl DailySalesPoint {
    /// Compute this day's value for `metric`.
    pub fn metric_value(&self, metric: DashboardMetric) -> f64 {
        let rev = if self.revenue.is_finite() {
            self.revenue.max(0.0)
        } else {
            0.0
        };
        match metric {
            DashboardMetric::Revenue => rev,
            DashboardMetric::Orders => self.orders as f64,
            DashboardMetric::AvgOrder => {
                if self.orders == 0 {
                    0.0
                } else {
                    rev / self.orders as f64
                }
            }
            DashboardMetric::RefundRate => {
                if self.orders == 0 {
                    0.0
                } else {
                    (self.refunded as f64 / self.orders as f64) * 100.0
                }
            }
            DashboardMetric::Conversion => {
                if self.conversion_pct.is_finite() {
                    self.conversion_pct.max(0.0)
                } else {
                    0.0
                }
            }
        }
    }
}

/// Ranked breakdown item (`Top products` or `Channels`) with revenue/order share in `0.0..=1.0`.
#[derive(Clone, Debug, PartialEq)]
pub struct RankedShareItem {
    /// Product or channel display name.
    pub name: SharedString,
    /// Share of period total (`0.0..=1.0`).
    pub share: f64,
}

impl RankedShareItem {
    /// Construct a [`RankedShareItem`].
    pub fn new(name: impl Into<SharedString>, share: f64) -> Self {
        Self {
            name: name.into(),
            share: share.clamp(0.0, 1.0),
        }
    }
}

/// Events emitted by [`MetricsDashboardBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetricsDashboardEvent {
    /// User switched between `Week`, `Month`, and `Quarter`.
    RangeChanged(DashboardRange),
    /// User picked another KPI metric card.
    MetricChanged(DashboardMetric),
    /// User toggled between `Area` and `Bars` chart rendering.
    ChartKindChanged(DashboardChartKind),
    /// User toggled `Compare to previous period`.
    CompareToggled(bool),
    /// User clicked the `Export` action button.
    Exported {
        /// Active reporting window at export time.
        range: DashboardRange,
        /// Active metric at export time.
        metric: DashboardMetric,
    },
    /// User hovered or keyboard-stepped to a bucket index on the trend chart.
    DayInspected(Option<usize>),
}

#[derive(Clone, Copy, Debug)]
struct PeriodRollup {
    revenue: f64,
    orders: u64,
    aov: f64,
    refund_rate: f64,
    conversion: f64,
}

fn summarize_slice(slice: &[DailySalesPoint]) -> PeriodRollup {
    if slice.is_empty() {
        return PeriodRollup {
            revenue: 0.0,
            orders: 0,
            aov: 0.0,
            refund_rate: 0.0,
            conversion: 0.0,
        };
    }
    let mut rev = 0.0;
    let mut ord = 0u64;
    let mut ref_cnt = 0u64;
    let mut conv_sum = 0.0;
    for d in slice {
        if d.revenue.is_finite() {
            rev += d.revenue.max(0.0);
        }
        ord += d.orders as u64;
        ref_cnt += d.refunded as u64;
        if d.conversion_pct.is_finite() {
            conv_sum += d.conversion_pct.max(0.0);
        }
    }
    let aov = if ord == 0 { 0.0 } else { rev / ord as f64 };
    let refund_rate = if ord == 0 {
        0.0
    } else {
        (ref_cnt as f64 / ord as f64) * 100.0
    };
    let conversion = conv_sum / slice.len() as f64;
    PeriodRollup {
        revenue: rev,
        orders: ord,
        aov,
        refund_rate,
        conversion,
    }
}

fn currency_symbol(code: &str) -> &'static str {
    match code.trim().to_ascii_uppercase().as_str() {
        "EUR" => "€",
        "GBP" => "£",
        "JPY" => "¥",
        _ => "$",
    }
}

fn format_grouped_u64(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (idx, ch) in digits.chars().enumerate() {
        if idx > 0 && (digits.len() - idx).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

fn format_metric_point(val: f64, metric: DashboardMetric, currency: &str) -> String {
    let sym = currency_symbol(currency);
    let v = if val.is_finite() { val.max(0.0) } else { 0.0 };
    match metric {
        DashboardMetric::Revenue => format!("{sym}{}", format_grouped_u64(v.round() as u64)),
        DashboardMetric::Orders => format_grouped_u64(v.round() as u64),
        DashboardMetric::AvgOrder => format!("{sym}{v:.2}"),
        DashboardMetric::RefundRate | DashboardMetric::Conversion => format!("{v:.2}%"),
    }
}

#[derive(IntoElement)]
struct GroupedRollingReadout {
    id: ElementId,
    target: f64,
    prefix: SharedString,
    font_size: f32,
    color: Hsla,
}

impl RenderOnce for GroupedRollingReadout {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let v = pro_spring_f32(
            pro_eid(&self.id, "grp-roll"),
            self.target as f32,
            0.30,
            w,
            cx,
        )
        .max(0.0) as f64;
        let text = format!("{}{}", self.prefix, format_grouped_u64(v.round() as u64));
        div()
            .text_size(px(self.font_size))
            .font_weight(FontWeight::MEDIUM)
            .text_color(self.color)
            .child(text)
    }
}

fn resample_vec(src: &[f32], len: usize) -> Vec<f32> {
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

struct DashboardPlotAnim {
    from_cur: Vec<f32>,
    to_cur: Vec<f32>,
    from_prev: Vec<f32>,
    to_prev: Vec<f32>,
    morph_clock: PhaseClock,
    mount_clock: PhaseClock,
}

#[derive(IntoElement)]
struct DashboardTrendCanvas {
    id: ElementId,
    kind: DashboardChartKind,
    current: Vec<f32>,
    previous: Vec<f32>,
    show_compare: bool,
    y_max: f32,
    accent: Hsla,
    card_bg: Hsla,
    grid_color: Hsla,
    guide_color: Hsla,
    hover_idx: Option<usize>,
    height: f32,
}

impl RenderOnce for DashboardTrendCanvas {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let ic = self.current.clone();
        let ip = self.previous.clone();
        let st = w.use_keyed_state(pro_eid(&self.id, "mdc-plot"), cx, |_, inner_cx| {
            let mut mount_clock = PhaseClock::default();
            mount_clock.restart(inner_cx);
            DashboardPlotAnim {
                from_cur: ic.clone(),
                to_cur: ic,
                from_prev: ip.clone(),
                to_prev: ip,
                morph_clock: PhaseClock::default(),
                mount_clock,
            }
        });

        let reduced = disabled_motion(cx);
        let (cur_vals, prev_vals, reveal) = st.update(cx, |s, cx| {
            let age = s.morph_clock.time(0.30, w, cx);
            let ct = pro_ease((age / 0.30).clamp(0., 1.));
            if s.to_cur != self.current || s.to_prev != self.previous {
                let n = self.current.len();
                let a = resample_vec(&s.from_cur, n);
                let b = resample_vec(&s.to_cur, n);
                s.from_cur = a
                    .into_iter()
                    .zip(b)
                    .map(|(x, y)| x + (y - x) * ct)
                    .collect();
                s.to_cur = self.current.clone();

                let m = self.previous.len();
                let pa = resample_vec(&s.from_prev, m);
                let pb = resample_vec(&s.to_prev, m);
                s.from_prev = pa
                    .into_iter()
                    .zip(pb)
                    .map(|(x, y)| x + (y - x) * ct)
                    .collect();
                s.to_prev = self.previous.clone();
                s.morph_clock.restart(cx);
            }
            let age2 = s.morph_clock.time(0.30, w, cx);
            let t = if reduced {
                1.0
            } else {
                pro_ease((age2 / 0.30).clamp(0., 1.))
            };
            let cv: Vec<f32> = if t >= 0.999 || s.from_cur.len() != s.to_cur.len() {
                s.to_cur.clone()
            } else {
                s.from_cur
                    .iter()
                    .zip(&s.to_cur)
                    .map(|(&x, &y)| x + (y - x) * t)
                    .collect()
            };
            let pv: Vec<f32> = if t >= 0.999 || s.from_prev.len() != s.to_prev.len() {
                s.to_prev.clone()
            } else {
                s.from_prev
                    .iter()
                    .zip(&s.to_prev)
                    .map(|(&x, &y)| x + (y - x) * t)
                    .collect()
            };
            let mage = s.mount_clock.time(0.55, w, cx);
            let rev = if reduced {
                1.0
            } else {
                pro_ease((mage / 0.55).clamp(0., 1.))
            };
            (cv, pv, rev)
        });

        let kind = self.kind;
        let show_compare = self.show_compare;
        let y_max = self.y_max.max(1e-3);
        let accent = self.accent;
        let card_bg = self.card_bg;
        let grid_color = self.grid_color;
        let guide_color = self.guide_color;
        let hover_idx = self.hover_idx;

        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 8. || bh <= 8. || cur_vals.is_empty() {
                    return;
                }
                let pad_top = 4.0_f32;
                let plot_h = (bh - pad_top - 2.0).max(8.0);
                let floor_y = bounds.top() + px(pad_top + plot_h);

                // 3 horizontal gridlines (top, middle, zero baseline)
                for g in 0..3usize {
                    let gy = bounds.top() + px(pad_top + (g as f32 / 2.0) * plot_h);
                    let mut gp = PathBuilder::stroke(px(1.0));
                    gp.move_to(point(bounds.left(), gy));
                    gp.line_to(point(bounds.right(), gy));
                    if let Ok(path) = gp.build() {
                        window.paint_path(
                            path,
                            if g == 2 {
                                grid_color.opacity(grid_color.a * 1.8)
                            } else {
                                grid_color
                            },
                        );
                    }
                }

                let n = cur_vals.len();
                let y_of = |val: f32| -> Pixels {
                    let frac = (val / y_max).clamp(0.0, 1.0);
                    bounds.top() + px(pad_top + (1.0 - frac) * plot_h)
                };

                match kind {
                    DashboardChartKind::Area => {
                        if n < 2 {
                            return;
                        }
                        let x_of = |idx: usize, count: usize| -> Pixels {
                            let tx = idx as f32 / count.saturating_sub(1).max(1) as f32;
                            bounds.left() + px(tx * bw)
                        };

                        if show_compare && prev_vals.len() >= 2 {
                            let vis_p = (((prev_vals.len() - 1) as f32 * reveal).ceil() as usize
                                + 1)
                            .clamp(2, prev_vals.len());
                            let mut dash = PathBuilder::stroke(px(1.3));
                            for i in 0..(vis_p - 1) {
                                let p0 = point(x_of(i, prev_vals.len()), y_of(prev_vals[i]));
                                let p1 =
                                    point(x_of(i + 1, prev_vals.len()), y_of(prev_vals[i + 1]));
                                let dx = f32::from(p1.x - p0.x);
                                let dy = f32::from(p1.y - p0.y);
                                let steps = (dx.hypot(dy) / 6.0).ceil().max(1.0) as usize;
                                for s in 0..steps {
                                    let t0 = s as f32 / steps as f32;
                                    let t1 = (t0 + 0.52 / steps as f32).min(1.0);
                                    dash.move_to(point(p0.x + px(dx * t0), p0.y + px(dy * t0)));
                                    dash.line_to(point(p0.x + px(dx * t1), p0.y + px(dy * t1)));
                                }
                            }
                            if let Ok(path) = dash.build() {
                                window.paint_path(path, accent.opacity(0.38));
                            }
                        }

                        let vis_n = (((n - 1) as f32 * reveal).ceil() as usize + 1).clamp(2, n);
                        let mut area = PathBuilder::fill();
                        let first_pt = point(x_of(0, n), y_of(cur_vals[0]));
                        let last_pt = point(x_of(vis_n - 1, n), y_of(cur_vals[vis_n - 1]));
                        area.move_to(point(first_pt.x, floor_y));
                        area.line_to(first_pt);
                        for (i, &v) in cur_vals.iter().enumerate().take(vis_n).skip(1) {
                            area.line_to(point(x_of(i, n), y_of(v)));
                        }
                        area.line_to(point(last_pt.x, floor_y));
                        area.close();
                        if let Ok(path) = area.build() {
                            window.paint_path(
                                path,
                                linear_gradient(
                                    180.,
                                    linear_color_stop(accent.opacity(0.20), 0.),
                                    linear_color_stop(accent.opacity(0.0), 1.),
                                ),
                            );
                        }

                        let mut line = PathBuilder::stroke(px(1.75));
                        for (i, &v) in cur_vals.iter().enumerate().take(vis_n) {
                            let pt = point(x_of(i, n), y_of(v));
                            if i == 0 {
                                line.move_to(pt);
                            } else {
                                line.line_to(pt);
                            }
                        }
                        if let Ok(path) = line.build() {
                            window.paint_path(path, accent);
                        }

                        if let Some(hi) = hover_idx.filter(|&i| i < n) {
                            let hx = x_of(hi, n);
                            let hy = y_of(cur_vals[hi]);
                            let mut guide = PathBuilder::stroke(px(1.));
                            guide.move_to(point(hx, bounds.top() + px(pad_top)));
                            guide.line_to(point(hx, floor_y));
                            if let Ok(path) = guide.build() {
                                window.paint_path(path, guide_color);
                            }
                            if show_compare && hi < prev_vals.len() {
                                let py = y_of(prev_vals[hi]);
                                let ro = px(4.5);
                                let ri = px(3.0);
                                window.paint_quad(
                                    fill(
                                        Bounds::new(
                                            point(hx, py) - point(ro, ro),
                                            size(ro * 2., ro * 2.),
                                        ),
                                        card_bg,
                                    )
                                    .corner_radii(ro),
                                );
                                window.paint_quad(
                                    fill(
                                        Bounds::new(
                                            point(hx, py) - point(ri, ri),
                                            size(ri * 2., ri * 2.),
                                        ),
                                        accent.opacity(0.5),
                                    )
                                    .corner_radii(ri),
                                );
                            }
                            let ro = px(5.5);
                            let ri = px(3.75);
                            window.paint_quad(
                                fill(
                                    Bounds::new(
                                        point(hx, hy) - point(ro, ro),
                                        size(ro * 2., ro * 2.),
                                    ),
                                    card_bg,
                                )
                                .corner_radii(ro),
                            );
                            window.paint_quad(
                                fill(
                                    Bounds::new(
                                        point(hx, hy) - point(ri, ri),
                                        size(ri * 2., ri * 2.),
                                    ),
                                    accent,
                                )
                                .corner_radii(ri),
                            );
                        }
                    }
                    DashboardChartKind::Bars => {
                        let slot_w = bw / n as f32;
                        let gap = (slot_w * 0.24).clamp(2.0, 6.0);
                        let group_w = (slot_w - gap).max(2.0);

                        for (i, &v) in cur_vals.iter().enumerate() {
                            let slot_x = bounds.left() + px(i as f32 * slot_w + gap * 0.5);
                            let dimmed = hover_idx.is_some_and(|hi| hi != i);
                            let cur_h = ((v / y_max).clamp(0.0, 1.0) * plot_h * reveal).max(1.5);

                            if show_compare && i < prev_vals.len() {
                                let sub_w = ((group_w - 2.0) * 0.5).max(1.5);
                                let prev_h =
                                    ((prev_vals[i] / y_max).clamp(0.0, 1.0) * plot_h * reveal)
                                        .max(1.5);
                                let prev_rect = Bounds::new(
                                    point(slot_x, floor_y - px(prev_h)),
                                    size(px(sub_w), px(prev_h)),
                                );
                                window.paint_quad(
                                    fill(
                                        prev_rect,
                                        accent.opacity(if dimmed { 0.14 } else { 0.32 }),
                                    )
                                    .corner_radii(Corners {
                                        top_left: px(3.),
                                        top_right: px(3.),
                                        bottom_left: px(1.),
                                        bottom_right: px(1.),
                                    }),
                                );
                                let cur_rect = Bounds::new(
                                    point(slot_x + px(sub_w + 2.0), floor_y - px(cur_h)),
                                    size(px(sub_w), px(cur_h)),
                                );
                                window.paint_quad(
                                    fill(
                                        cur_rect,
                                        accent.opacity(if dimmed { 0.40 } else { 0.88 }),
                                    )
                                    .corner_radii(Corners {
                                        top_left: px(3.),
                                        top_right: px(3.),
                                        bottom_left: px(1.),
                                        bottom_right: px(1.),
                                    }),
                                );
                            } else {
                                let bar_rect = Bounds::new(
                                    point(slot_x, floor_y - px(cur_h)),
                                    size(px(group_w), px(cur_h)),
                                );
                                window.paint_quad(
                                    fill(
                                        bar_rect,
                                        accent.opacity(if dimmed { 0.38 } else { 0.86 }),
                                    )
                                    .corner_radii(Corners {
                                        top_left: px(3.5),
                                        top_right: px(3.5),
                                        bottom_left: px(1.),
                                        bottom_right: px(1.),
                                    }),
                                );
                            }
                        }
                    }
                }
            },
        )
        .w_full()
        .h(px(self.height))
    }
}

fn nice_axis_ceiling(max_val: f64, metric: DashboardMetric) -> f64 {
    let raw = max_val.max(1.0);
    match metric {
        DashboardMetric::Revenue => {
            if raw <= 8_000.0 {
                8_000.0
            } else if raw <= 16_000.0 {
                16_000.0
            } else if raw <= 40_000.0 {
                40_000.0
            } else {
                (raw / 20_000.0).ceil() * 20_000.0
            }
        }
        DashboardMetric::Orders => {
            if raw <= 200.0 {
                200.0
            } else if raw <= 300.0 {
                300.0
            } else {
                (raw / 200.0).ceil() * 200.0
            }
        }
        DashboardMetric::AvgOrder => ((raw / 20.0).ceil() * 20.0).max(80.0),
        DashboardMetric::RefundRate | DashboardMetric::Conversion => {
            ((raw / 2.0).ceil() * 2.0).max(4.0)
        }
    }
}

fn format_axis_tick(val: f64, metric: DashboardMetric, currency: &str) -> String {
    let sym = currency_symbol(currency);
    match metric {
        DashboardMetric::Revenue => {
            if val <= 0.5 {
                format!("{sym}0")
            } else if val >= 1_000.0 {
                format!("{sym}{}K", (val / 1_000.0).round() as u64)
            } else {
                format!("{sym}{}", val.round() as u64)
            }
        }
        DashboardMetric::Orders => format!("{}", val.round() as u64),
        DashboardMetric::AvgOrder => format!("{sym}{}", val.round() as u64),
        DashboardMetric::RefundRate | DashboardMetric::Conversion => format!("{val:.0}%"),
    }
}

fn default_sales_days() -> Vec<DailySalesPoint> {
    // Exact 30-day active curve matching reference SVG[4] ($292,444 revenue, 4,434 orders, $65.95 AOV, 2.1% refund rate)
    const CUR_30_REV: [f64; 30] = [
        7403., 8254., 9258., 10134., 10467., 11562., 12166., 7818., 7059., 9218., 10246., 10482.,
        12827., 11267., 8443., 7822., 9579., 9523., 10323., 11200., 13114., 7578., 7259., 8117.,
        9091., 11342., 11182., 11285., 8920., 9505.,
    ];
    const CUR_30_ORD: [u32; 30] = [
        114, 126, 141, 154, 159, 175, 184, 120, 108, 140, 155, 159, 193, 170, 129, 120, 145, 144,
        156, 169, 198, 116, 111, 124, 138, 171, 169, 171, 134, 145,
    ];
    const DATES_30: [&str; 30] = [
        "Oct 26", "Oct 27", "Oct 28", "Oct 29", "Oct 30", "Oct 31", "Nov 1", "Nov 2", "Nov 3",
        "Nov 4", "Nov 5", "Nov 6", "Nov 7", "Nov 8", "Nov 9", "Nov 10", "Nov 11", "Nov 12",
        "Nov 13", "Nov 14", "Nov 15", "Nov 16", "Nov 17", "Nov 18", "Nov 19", "Nov 20", "Nov 21",
        "Nov 22", "Nov 23", "Nov 24",
    ];

    let mut out = Vec::with_capacity(180);
    // Days 0..120: earlier quarter history (Aug..Sep)
    for i in 0..120usize {
        let wave = ((i as f64 * 0.42).sin() * 1450.0) + ((i as f64 * 0.17).cos() * 680.0);
        let rev = (8_650.0 + wave).round();
        let orders = ((rev / 65.2).round() as u32).max(80);
        let refunded = if i % 3 == 0 { 4 } else { 2 };
        let month_prefix = if i < 36 {
            "Jul"
        } else if i < 67 {
            "Aug"
        } else if i < 97 {
            "Sep"
        } else {
            "Oct"
        };
        let day_n = (i % 28) + 1;
        out.push(DailySalesPoint {
            date: format!("{month_prefix} {day_n}").into(),
            revenue: rev,
            orders,
            refunded,
            conversion_pct: 3.18 + (i % 7) as f64 * 0.06,
        });
    }

    // Days 120..150: previous 30-day window ($274,080 revenue -> +6.7%, 4,191 orders -> +5.8%, 2.1% refund rate -> -0.0 pts)
    for i in 0..30usize {
        let rev = (CUR_30_REV[i] * 0.93721).round();
        let orders = ((CUR_30_ORD[i] as f64 * 0.9452).round() as u32).max(95);
        let refunded = if i % 3 == 0 { 4 } else { 2 };
        out.push(DailySalesPoint {
            date: format!("Oct {}", (i % 25) + 1).into(),
            revenue: rev,
            orders,
            refunded,
            conversion_pct: 3.28 + (i % 5) as f64 * 0.05,
        });
    }

    // Days 150..180: current 30-day window (exact $292,444 / 4,434 / $65.95 / 2.1% -> 93 refunded)
    for i in 0..30usize {
        let refunded = if i < 3 { 4 } else { 3 };
        out.push(DailySalesPoint {
            date: DATES_30[i].into(),
            revenue: CUR_30_REV[i],
            orders: CUR_30_ORD[i],
            refunded,
            conversion_pct: 3.42 + (i % 6) as f64 * 0.06,
        });
    }
    out
}

fn default_products() -> Vec<RankedShareItem> {
    vec![
        RankedShareItem::new("Linen overshirt", 0.22),
        RankedShareItem::new("Everyday tote", 0.16),
        RankedShareItem::new("Merino crew", 0.13),
        RankedShareItem::new("Canvas sneaker", 0.09),
        RankedShareItem::new("Wool beanie", 0.06),
    ]
}

fn default_channels() -> Vec<RankedShareItem> {
    vec![
        RankedShareItem::new("Online store", 0.46),
        RankedShareItem::new("Instagram", 0.21),
        RankedShareItem::new("Marketplace", 0.14),
        RankedShareItem::new("Wholesale", 0.11),
        RankedShareItem::new("Pop-up", 0.05),
    ]
}

/// Sales overview dashboard block with clickable KPI cards, area/bar trend chart, and ranked breakdowns.
pub struct MetricsDashboardBlock {
    id: ElementId,
    focus: FocusHandle,
    title: SharedString,
    site: SharedString,
    currency: SharedString,
    range: DashboardRange,
    metric: DashboardMetric,
    kpi_slots: [DashboardMetric; 4],
    chart_kind: DashboardChartKind,
    compare: bool,
    days: Vec<DailySalesPoint>,
    products: Vec<RankedShareItem>,
    channels: Vec<RankedShareItem>,
    hover_bucket: Option<usize>,
    exported_at: Option<Instant>,
    loading: bool,
    empty: bool,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    boot_at: Instant,
}

impl EventEmitter<MetricsDashboardEvent> for MetricsDashboardBlock {}

impl MetricsDashboardBlock {
    /// Create a new [`MetricsDashboardBlock`] initialized on `Northwind Store · Last 30 days`.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: "Sales".into(),
            site: "Northwind Store".into(),
            currency: "USD".into(),
            range: DashboardRange::Month,
            metric: DashboardMetric::Revenue,
            kpi_slots: [
                DashboardMetric::Revenue,
                DashboardMetric::Orders,
                DashboardMetric::AvgOrder,
                DashboardMetric::RefundRate,
            ],
            chart_kind: DashboardChartKind::Area,
            compare: false,
            days: default_sales_days(),
            products: default_products(),
            channels: default_channels(),
            hover_bucket: None,
            exported_at: None,
            loading: false,
            empty: false,
            measured_width: Rc::new(Cell::new(894.)),
            toast: None,
            boot_at: Instant::now(),
        }
    }

    /// Set the dashboard title in the top bar (default `"Sales"`).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the storefront or workspace subtitle shown under the title (default `"Northwind Store"`).
    pub fn with_site(mut self, site: impl Into<SharedString>) -> Self {
        self.site = site.into();
        self
    }

    /// Set the ISO 4217 currency code (`"USD"`, `"EUR"`, `"GBP"`, `"JPY"`).
    pub fn with_currency(mut self, currency: impl Into<SharedString>) -> Self {
        self.currency = currency.into();
        self
    }

    /// Set the starting reporting period (`Week`, `Month`, or `Quarter`).
    pub fn with_default_range(mut self, range: DashboardRange) -> Self {
        self.range = range;
        self
    }

    /// Set the starting selected KPI metric (`Revenue`, `Orders`, `AvgOrder`, `RefundRate`, `Conversion`).
    pub fn with_default_metric(mut self, metric: DashboardMetric) -> Self {
        self.metric = metric;
        if !self.kpi_slots.contains(&metric) {
            self.kpi_slots[3] = metric;
        }
        self
    }

    /// Customize which four [`DashboardMetric`] cards appear in the KPI row.
    pub fn with_kpi_metrics(mut self, slots: [DashboardMetric; 4]) -> Self {
        self.kpi_slots = slots;
        if !self.kpi_slots.contains(&self.metric) {
            self.metric = self.kpi_slots[0];
        }
        self
    }

    /// Set the initial chart mode (`Area` or `Bars`).
    pub fn with_chart_kind(mut self, kind: DashboardChartKind) -> Self {
        self.chart_kind = kind;
        self
    }

    /// Set whether previous-period comparison starts enabled.
    pub fn with_compare(mut self, compare: bool) -> Self {
        self.compare = compare;
        self
    }

    /// Supply daily sales observations ordered oldest first.
    pub fn with_data(mut self, days: Vec<DailySalesPoint>) -> Self {
        self.days = days;
        self
    }

    /// Supply ranked rows for `Top products`.
    pub fn with_products(mut self, products: Vec<RankedShareItem>) -> Self {
        self.products = products;
        self
    }

    /// Supply ranked rows for `Channels`.
    pub fn with_channels(mut self, channels: Vec<RankedShareItem>) -> Self {
        self.channels = channels;
        self
    }

    /// Toggle loading skeleton placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty dashboard state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the active [`DashboardRange`].
    pub fn range(&self) -> DashboardRange {
        self.range
    }

    /// Return the active [`DashboardMetric`].
    pub fn metric(&self) -> DashboardMetric {
        self.metric
    }

    /// Switch the active [`DashboardRange`] and emit [`MetricsDashboardEvent::RangeChanged`].
    pub fn set_range(&mut self, range: DashboardRange, cx: &mut Context<Self>) {
        if self.range != range {
            self.range = range;
            self.hover_bucket = None;
            cx.emit(MetricsDashboardEvent::RangeChanged(range));
            cx.notify();
        }
    }

    /// Switch the active [`DashboardMetric`] and emit [`MetricsDashboardEvent::MetricChanged`].
    pub fn set_metric(&mut self, metric: DashboardMetric, cx: &mut Context<Self>) {
        if self.metric != metric {
            self.metric = metric;
            cx.emit(MetricsDashboardEvent::MetricChanged(metric));
            cx.notify();
        }
    }

    fn period_slices(&self) -> (&[DailySalesPoint], Option<&[DailySalesPoint]>) {
        let len = self.range.days_len();
        let total = self.days.len();
        if total == 0 {
            return (&[], None);
        }
        let cur_start = total.saturating_sub(len);
        let cur = &self.days[cur_start..total];
        let prev = if cur_start >= len {
            Some(&self.days[(cur_start - len)..cur_start])
        } else if cur_start > 0 {
            Some(&self.days[0..cur_start])
        } else {
            None
        };
        (cur, prev)
    }

    /// Aggregate up to 30 chart buckets for the active range (`Quarter` groups 90 days into 30 3-day buckets per spec).
    fn chart_buckets(&self) -> (Vec<(SharedString, f64)>, Vec<f64>) {
        let (cur_slice, prev_opt) = self.period_slices();
        let group_sz = if cur_slice.len() > 30 {
            cur_slice.len().div_ceil(30)
        } else {
            1
        };
        let bucket_metric = |chunk: &[DailySalesPoint]| -> f64 {
            let s = summarize_slice(chunk);
            match self.metric {
                DashboardMetric::Revenue => s.revenue,
                DashboardMetric::Orders => s.orders as f64,
                DashboardMetric::AvgOrder => s.aov,
                DashboardMetric::RefundRate => s.refund_rate,
                DashboardMetric::Conversion => s.conversion,
            }
        };
        let cur_pts: Vec<(SharedString, f64)> = cur_slice
            .chunks(group_sz)
            .map(|ch| {
                let d = ch.last().map(|p| p.date.clone()).unwrap_or_default();
                (d, bucket_metric(ch))
            })
            .collect();
        let prev_pts: Vec<f64> = match prev_opt {
            Some(ps) => ps.chunks(group_sz).map(bucket_metric).collect(),
            None => Vec::new(),
        };
        (cur_pts, prev_pts)
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if ev.keystroke.modifiers.platform || ev.keystroke.modifiers.control {
            return;
        }
        let (cur_pts, _) = self.chart_buckets();
        let n = cur_pts.len();
        if n == 0 {
            return;
        }
        match ev.keystroke.key.as_str() {
            "left" => {
                let next = match self.hover_bucket {
                    Some(i) => i.saturating_sub(1),
                    None => n - 1,
                };
                self.hover_bucket = Some(next);
                cx.emit(MetricsDashboardEvent::DayInspected(Some(next)));
                cx.stop_propagation();
                cx.notify();
            }
            "right" => {
                let next = match self.hover_bucket {
                    Some(i) => (i + 1).min(n - 1),
                    None => 0,
                };
                self.hover_bucket = Some(next);
                cx.emit(MetricsDashboardEvent::DayInspected(Some(next)));
                cx.stop_propagation();
                cx.notify();
            }
            "home" => {
                self.hover_bucket = Some(0);
                cx.emit(MetricsDashboardEvent::DayInspected(Some(0)));
                cx.stop_propagation();
                cx.notify();
            }
            "end" => {
                let last = n - 1;
                self.hover_bucket = Some(last);
                cx.emit(MetricsDashboardEvent::DayInspected(Some(last)));
                cx.stop_propagation();
                cx.notify();
            }
            "escape" if self.hover_bucket.take().is_some() => {
                cx.emit(MetricsDashboardEvent::DayInspected(None));
                cx.stop_propagation();
                cx.notify();
            }
            _ => {}
        }
    }

    fn render_kpi_card(
        &self,
        m: DashboardMetric,
        cur: &PeriodRollup,
        prev: Option<&PeriodRollup>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let selected = self.metric == m;
        let sym = currency_symbol(&self.currency);
        let label = m.label(cx);

        let value_el: AnyElement = match m {
            DashboardMetric::Revenue => GroupedRollingReadout {
                id: pro_eid(&self.id, "kpi-rev"),
                target: cur.revenue,
                prefix: sym.into(),
                font_size: 24.,
                color: p.strong,
            }
            .into_any_element(),
            DashboardMetric::Orders => GroupedRollingReadout {
                id: pro_eid(&self.id, "kpi-ord"),
                target: cur.orders as f64,
                prefix: "".into(),
                font_size: 24.,
                color: p.strong,
            }
            .into_any_element(),
            DashboardMetric::AvgOrder => pro_rolling_number(
                pro_eid(&self.id, "kpi-aov"),
                cur.aov,
                2,
                sym,
                "",
                24.,
                FontWeight::MEDIUM,
                p.strong,
            ),
            DashboardMetric::RefundRate => pro_rolling_number(
                pro_eid(&self.id, "kpi-ref"),
                cur.refund_rate,
                1,
                "",
                "%",
                24.,
                FontWeight::MEDIUM,
                p.strong,
            ),
            DashboardMetric::Conversion => pro_rolling_number(
                pro_eid(&self.id, "kpi-conv"),
                cur.conversion,
                2,
                "",
                "%",
                24.,
                FontWeight::MEDIUM,
                p.strong,
            ),
        };

        let delta_badge = prev.and_then(|pr| match m {
            DashboardMetric::Revenue if pr.revenue > 0.0 => {
                let pct = ((cur.revenue - pr.revenue) / pr.revenue) * 100.0;
                Some((pct >= 0.0, pct >= 0.0, format!("{:.1}%", pct.abs())))
            }
            DashboardMetric::Orders if pr.orders > 0 => {
                let pct = ((cur.orders as f64 - pr.orders as f64) / pr.orders as f64) * 100.0;
                Some((pct >= 0.0, pct >= 0.0, format!("{:.1}%", pct.abs())))
            }
            DashboardMetric::AvgOrder if pr.aov > 0.0 => {
                let pct = ((cur.aov - pr.aov) / pr.aov) * 100.0;
                Some((pct >= 0.0, pct >= 0.0, format!("{:.2}%", pct.abs())))
            }
            DashboardMetric::RefundRate => {
                let pts = cur.refund_rate - pr.refund_rate;
                let up_arrow = pts > 0.05;
                let good = pts <= 0.05;
                Some((up_arrow, good, format!("{:.1} pts", pts.abs())))
            }
            DashboardMetric::Conversion => {
                let pts = cur.conversion - pr.conversion;
                Some((pts >= 0.0, pts >= 0.0, format!("{:.2} pts", pts.abs())))
            }
            _ => None,
        });

        let inner_opacity = if selected { 1.0 } else { 0.48 };

        activate(
            div()
                .id(pro_eid(&self.id, format!("kpi-tab-{}", m.slug())))
                .role(Role::Tab)
                .aria_label(label)
                .tab_index(0)
                .flex_1()
                .min_w_0()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(if selected { p.line_strong } else { p.line })
                .px(px(16.))
                .py(px(14.))
                .when(!selected, |d| {
                    d.hover(move |s| s.border_color(p.line_strong))
                })
                .focus_visible(move |s| s.border_color(p.focus))
                .cursor_pointer()
                .child(
                    div()
                        .opacity(inner_opacity)
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(p.text)
                                .truncate()
                                .child(label),
                        )
                        .child(value_el)
                        .when_some(delta_badge, |d, (up, good, text)| {
                            let (bg, fg) = if good {
                                (p.green_soft, p.green_ink)
                            } else {
                                (p.red_soft, p.red_ink)
                            };
                            d.child(
                                div()
                                    .mt(px(2.))
                                    .w(px(64.))
                                    .min_w(px(54.))
                                    .flex()
                                    .items_center()
                                    .child(
                                        div()
                                            .h(px(20.))
                                            .px(px(6.))
                                            .rounded(px(5.))
                                            .bg(bg)
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(fg)
                                            .whitespace_nowrap()
                                            .child(pro_svg(
                                                "0 0 8 8",
                                                if up { DELTA_UP_SVG } else { DELTA_DOWN_SVG },
                                                6.,
                                                fg,
                                            ))
                                            .child(text),
                                    ),
                            )
                        }),
                ),
            cx,
            move |this, _, cx| {
                this.set_metric(m, cx);
            },
        )
    }

    fn render_ranked_card(
        &self,
        list_id: &str,
        heading: &str,
        items: &[RankedShareItem],
        cur_rollup: &PeriodRollup,
        w: &mut Window,
        cx: &mut App,
    ) -> Div {
        let p = ProPalette::get(cx);
        let sym = currency_symbol(&self.currency);
        let use_orders = self.metric == DashboardMetric::Orders;
        let col_caption = if use_orders {
            tr(cx, "ORDERS", "订单量")
        } else {
            tr(cx, "REVENUE", "营业额")
        };
        let max_share = items
            .iter()
            .map(|it| it.share)
            .fold(0.0_f64, f64::max)
            .max(0.01);
        let bar_tint: Hsla = if p.dark {
            p.blue.opacity(0.11)
        } else {
            hsla(209. / 360., 1.0, 0.45, 0.065)
        };

        let body = if items.is_empty() {
            div()
                .px(px(8.))
                .py(px(12.))
                .text_size(px(13.))
                .text_color(p.muted)
                .child(tr(cx, "No breakdown items recorded.", "暂无分类明细。"))
        } else {
            let mut list = div().flex().flex_col().gap(px(4.));
            for (idx, item) in items.iter().enumerate() {
                let target_ratio = (item.share / max_share).clamp(0.0, 1.0) as f32;
                let spring_ratio = pro_spring_f32(
                    pro_eid(&self.id, format!("{list_id}-bar-{idx}")),
                    target_ratio,
                    0.36,
                    w,
                    cx,
                )
                .clamp(0.0, 1.0);

                let formatted_val = if use_orders {
                    let count = (cur_rollup.orders as f64 * item.share).round() as u64;
                    format_grouped_u64(count)
                } else {
                    let amount = (cur_rollup.revenue * item.share).round() as u64;
                    format!("{sym}{}", format_grouped_u64(amount))
                };

                list = list.child(
                    div()
                        .relative()
                        .w_full()
                        .h(px(30.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .overflow_hidden()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(12.))
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .left_0()
                                .w(relative(spring_ratio))
                                .rounded(px(6.))
                                .bg(bar_tint),
                        )
                        .child(
                            div()
                                .relative()
                                .min_w_0()
                                .text_size(px(13.))
                                .text_color(p.ink)
                                .truncate()
                                .child(item.name.clone()),
                        )
                        .child(
                            div()
                                .relative()
                                .flex_none()
                                .text_size(px(13.))
                                .text_color(p.text)
                                .child(formatted_val),
                        ),
                );
            }
            list
        };

        div()
            .flex_1()
            .min_w_0()
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .px(px(12.))
            .pt(px(14.))
            .pb(px(12.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .px(px(8.))
                    .pb(px(2.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .truncate()
                            .child(heading.to_owned()),
                    )
                    .child(
                        div()
                            .text_size(px(10.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(col_caption),
                    ),
            )
            .child(body)
    }
}

impl Render for MetricsDashboardBlock {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let export_confirmed = self
            .exported_at
            .is_some_and(|t| t.elapsed().as_secs_f32() < 1.8);
        if (self.loading || export_confirmed) && !reduced {
            w.request_animation_frame();
        }

        let width = self.measured_width.get();
        let medium_narrow = width < 640.;
        let compact = width < 460.;
        let measure_cell = self.measured_width.clone();

        // Top Bar: Title + Site·Period on left, Week/Month/Quarter + Export on right
        let title_str = if self.title.as_ref() == "Sales" {
            tr(cx, "Sales", "销售总览").to_owned()
        } else {
            self.title.to_string()
        };
        let sub_str = format!("{} · {}", self.site, self.range.subtitle_label(cx));

        let range_sensor = pro_selection_sensor();
        let thumb_bg: Hsla = if p.dark {
            rgb(0x2c2c2e).into()
        } else {
            rgb(0xffffff).into()
        };
        let mut range_seg = div()
            .relative()
            .when(compact, |d| d.flex_1())
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.chip)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "mdc-range-pill"),
                self.range.slug(),
                range_sensor.clone(),
                6.,
                thumb_bg,
                Some(p.line),
                true,
            ));

        for r in DashboardRange::ALL {
            let sel = self.range == r;
            let lbl = r.button_label(cx);
            range_seg = range_seg.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("mdc-range-{}", r.slug())))
                    .role(Role::RadioButton)
                    .aria_label(lbl)
                    .tab_index(0)
                    .relative()
                    .when(compact, |d| d.flex_1())
                    .when(!compact, |d| d.w(px(62.)))
                    .h(px(24.))
                    .rounded(px(6.))
                    .when(sel, |d| d.child(pro_mark_selected(&range_sensor)))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if sel { p.strong } else { p.text })
                    .cursor_pointer()
                    .child(lbl),
                cx,
                move |this, _, cx| {
                    this.set_range(r, cx);
                },
            ));
        }

        let export_btn = activate(
            div()
                .id(pro_eid(&self.id, "mdc-export-btn"))
                .role(Role::Button)
                .aria_label(tr(cx, "Export CSV", "导出数据"))
                .tab_index(0)
                .h(px(28.))
                .px(px(10.))
                .rounded(px(7.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .hover(move |s| s.bg(p.hover))
                .focus_visible(move |s| s.border_color(p.focus))
                .flex_none()
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.strong)
                .cursor_pointer()
                .child(if export_confirmed {
                    pro_icon(ProIcon::Check, 13., p.green)
                } else {
                    pro_svg("0 0 24 24", EXPORT_ARROW_SVG, 14., p.muted)
                })
                .child(if export_confirmed {
                    tr(cx, "Exported", "已导出")
                } else {
                    tr(cx, "Export", "导出")
                }),
            cx,
            |this, _, cx| {
                this.exported_at = Some(Instant::now());
                this.toast = Some((
                    tr(cx, "Exported period summary", "已导出当前周期报表").into(),
                    Instant::now(),
                ));
                cx.emit(MetricsDashboardEvent::Exported {
                    range: this.range,
                    metric: this.metric,
                });
                cx.notify();
            },
        );

        let top_bar = div()
            .w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .truncate()
                            .child(title_str),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .truncate()
                            .child(sub_str),
                    ),
            )
            .child(
                div()
                    .when(compact, |d| d.w_full())
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(range_seg)
                    .child(export_btn),
            );

        if self.loading {
            let phase = (self.boot_at.elapsed().as_secs_f32() * 0.9).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .p(px(16.))
                .bg(p.page)
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(top_bar)
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_wrap()
                        .gap(px(12.))
                        .children((0..4).map(|i| {
                            div()
                                .flex_1()
                                .min_w(px(140.))
                                .h(px(94.))
                                .rounded(px(10.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line)
                                .p(px(16.))
                                .flex()
                                .flex_col()
                                .justify_between()
                                .child(pro_skeleton_bar(64., 12., 4., phase, cx))
                                .child(pro_skeleton_bar(96., 22., 6., phase + i as f32 * 0.1, cx))
                        })),
                )
                .child(
                    div()
                        .w_full()
                        .h(px(236.))
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .p(px(16.))
                        .child(pro_skeleton_bar(240., 180., 8., phase + 0.2, cx)),
                );
        }

        if self.empty || self.days.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .p(px(16.))
                .bg(p.page)
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(top_bar)
                .child(
                    div()
                        .w_full()
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .child(pro_empty_state(
                            ProIcon::UsageChart,
                            tr(cx, "No sales data in this period", "当前周期暂无销售数据"),
                            tr(
                                cx,
                                "Orders, revenue trends, top products, and channels will appear once checkout events arrive.",
                                "接入订单与结账流水后，此处将展示收入趋势、热销商品与渠道占比。",
                            ),
                            None,
                            cx,
                        )),
                );
        }

        let (cur_slice, prev_opt) = self.period_slices();
        let cur_rollup = summarize_slice(cur_slice);
        let prev_rollup = prev_opt.map(summarize_slice);

        // 4 KPI Cards (4-across >= 640px, 2x2 grid < 640px)
        let [m0, m1, m2, m3] = self.kpi_slots;
        let kpis_el = if medium_narrow {
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .gap(px(8.))
                        .child(self.render_kpi_card(m0, &cur_rollup, prev_rollup.as_ref(), cx))
                        .child(self.render_kpi_card(m1, &cur_rollup, prev_rollup.as_ref(), cx)),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .gap(px(8.))
                        .child(self.render_kpi_card(m2, &cur_rollup, prev_rollup.as_ref(), cx))
                        .child(self.render_kpi_card(m3, &cur_rollup, prev_rollup.as_ref(), cx)),
                )
        } else {
            div()
                .w_full()
                .flex()
                .gap(px(12.))
                .child(self.render_kpi_card(m0, &cur_rollup, prev_rollup.as_ref(), cx))
                .child(self.render_kpi_card(m1, &cur_rollup, prev_rollup.as_ref(), cx))
                .child(self.render_kpi_card(m2, &cur_rollup, prev_rollup.as_ref(), cx))
                .child(self.render_kpi_card(m3, &cur_rollup, prev_rollup.as_ref(), cx))
        };

        // Central Trend Chart Card
        let (cur_buckets, prev_buckets) = self.chart_buckets();
        let cur_f32: Vec<f32> = cur_buckets.iter().map(|(_, v)| *v as f32).collect();
        let prev_f32: Vec<f32> = prev_buckets.iter().map(|&v| v as f32).collect();
        let raw_max = cur_buckets
            .iter()
            .map(|(_, v)| *v)
            .chain(
                self.compare
                    .then_some(prev_buckets.iter().copied())
                    .into_iter()
                    .flatten(),
            )
            .fold(0.0_f64, f64::max);
        let y_ceil = nice_axis_ceiling(raw_max, self.metric);

        let kind_sensor = pro_selection_sensor();
        let kind_slug = match self.chart_kind {
            DashboardChartKind::Area => "area",
            DashboardChartKind::Bars => "bars",
        };
        let kind_toggle = div()
            .relative()
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.chip)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "mdc-kind-pill"),
                kind_slug,
                kind_sensor.clone(),
                6.,
                thumb_bg,
                Some(p.line),
                true,
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "mdc-kind-area"))
                    .role(Role::RadioButton)
                    .aria_label(tr(cx, "Area chart", "面积折线图"))
                    .tab_index(0)
                    .relative()
                    .w(px(28.))
                    .h(px(24.))
                    .rounded(px(6.))
                    .when(self.chart_kind == DashboardChartKind::Area, |d| {
                        d.child(pro_mark_selected(&kind_sensor))
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(pro_svg(
                        "0 0 24 24",
                        KIND_AREA_SVG,
                        15.,
                        if self.chart_kind == DashboardChartKind::Area {
                            p.strong
                        } else {
                            p.muted
                        },
                    )),
                cx,
                |this, _, cx| {
                    if this.chart_kind != DashboardChartKind::Area {
                        this.chart_kind = DashboardChartKind::Area;
                        cx.emit(MetricsDashboardEvent::ChartKindChanged(
                            DashboardChartKind::Area,
                        ));
                        cx.notify();
                    }
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "mdc-kind-bars"))
                    .role(Role::RadioButton)
                    .aria_label(tr(cx, "Bar chart", "柱状图"))
                    .tab_index(0)
                    .relative()
                    .w(px(28.))
                    .h(px(24.))
                    .rounded(px(6.))
                    .when(self.chart_kind == DashboardChartKind::Bars, |d| {
                        d.child(pro_mark_selected(&kind_sensor))
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(pro_svg(
                        "0 0 24 24",
                        KIND_BARS_SVG,
                        15.,
                        if self.chart_kind == DashboardChartKind::Bars {
                            p.strong
                        } else {
                            p.muted
                        },
                    )),
                cx,
                |this, _, cx| {
                    if this.chart_kind != DashboardChartKind::Bars {
                        this.chart_kind = DashboardChartKind::Bars;
                        cx.emit(MetricsDashboardEvent::ChartKindChanged(
                            DashboardChartKind::Bars,
                        ));
                        cx.notify();
                    }
                },
            ));

        let compare_label = if medium_narrow {
            tr(cx, "Compare", "同比对比")
        } else {
            tr(cx, "Compare to previous period", "与上一周期对比")
        };

        let bucket_count = cur_buckets.len();
        let hover_bi = self.hover_bucket.filter(|&b| b < bucket_count);
        let tooltip_el = hover_bi.map(|bi| {
            let (ref d_lbl, cur_v) = cur_buckets[bi];
            let prev_v = prev_buckets.get(bi).copied();
            let on_right_half = bucket_count > 1 && bi * 2 >= bucket_count;
            let pop_bg: Hsla = if p.dark {
                rgb(0x252528).into()
            } else {
                rgb(0xffffff).into()
            };
            let mut pop = div()
                .absolute()
                .top(px(4.))
                .min_w(px(156.))
                .px(px(10.))
                .py(px(7.))
                .rounded(px(10.))
                .bg(pop_bg)
                .border_1()
                .border_color(p.line_strong)
                .shadow_md()
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .pb(px(3.))
                        .border_b_1()
                        .border_color(p.line)
                        .child(d_lbl.clone()),
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
                                .child(div().size(px(7.)).rounded(px(2.)).bg(p.blue))
                                .child(self.metric.label(cx)),
                        )
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(format_metric_point(cur_v, self.metric, &self.currency)),
                        ),
                );
            if self.compare
                && let Some(pv) = prev_v
            {
                pop = pop.child(
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
                                        .w(px(7.))
                                        .h(px(2.))
                                        .rounded(px(1.))
                                        .bg(p.blue.opacity(0.42)),
                                )
                                .child(tr(cx, "Previous", "上周期")),
                        )
                        .child(div().text_color(p.text).child(format_metric_point(
                            pv,
                            self.metric,
                            &self.currency,
                        ))),
                );
            }
            if on_right_half {
                pop = pop.left(px(8.));
            } else {
                pop = pop.right(px(8.));
            }
            pop
        });

        let plot_height = if compact { 164. } else { 184. };
        let hit_slices = div()
            .absolute()
            .inset_0()
            .flex()
            .children((0..bucket_count).map(|bi| {
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("mdc-pt-{bi}")))
                        .flex_1()
                        .h_full()
                        .cursor_pointer()
                        .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                            if *inside {
                                this.hover_bucket = Some(bi);
                            } else if this.hover_bucket == Some(bi) {
                                this.hover_bucket = None;
                            }
                            cx.notify();
                        })),
                    cx,
                    move |this, _, cx| {
                        this.hover_bucket = Some(bi);
                        cx.emit(MetricsDashboardEvent::DayInspected(Some(bi)));
                        cx.notify();
                    },
                )
            }));

        let x_start = cur_buckets
            .first()
            .map(|(d, _)| d.clone())
            .unwrap_or_default();
        let x_mid = cur_buckets
            .get(cur_buckets.len() / 2)
            .map(|(d, _)| d.clone())
            .unwrap_or_default();
        let x_end = cur_buckets
            .last()
            .map(|(d, _)| d.clone())
            .unwrap_or_default();

        let chart_card = div()
            .w_full()
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .p(px(if compact { 12. } else { 16. }))
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .size(px(8.))
                                    .rounded_full()
                                    .border_2()
                                    .border_color(p.blue),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(self.metric.label(cx)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(14.))
                            .child(kind_toggle)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(pro_switch(
                                        pro_eid(&self.id, "mdc-compare-sw"),
                                        compare_label,
                                        self.compare,
                                        cx,
                                        |this, _, cx| {
                                            this.compare = !this.compare;
                                            cx.emit(MetricsDashboardEvent::CompareToggled(
                                                this.compare,
                                            ));
                                            cx.notify();
                                        },
                                    ))
                                    .child(activate(
                                        div()
                                            .id(pro_eid(&self.id, "mdc-compare-lbl"))
                                            .text_size(px(13.))
                                            .text_color(p.text)
                                            .cursor_pointer()
                                            .child(compare_label),
                                        cx,
                                        |this, _, cx| {
                                            this.compare = !this.compare;
                                            cx.emit(MetricsDashboardEvent::CompareToggled(
                                                this.compare,
                                            ));
                                            cx.notify();
                                        },
                                    )),
                            ),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_stretch()
                    .gap(px(10.))
                    .child(
                        div()
                            .w(px(36.))
                            .h(px(plot_height))
                            .flex_none()
                            .flex()
                            .flex_col()
                            .justify_between()
                            .items_end()
                            .text_size(px(11.))
                            .text_color(p.muted)
                            .child(format_axis_tick(y_ceil, self.metric, &self.currency))
                            .child(format_axis_tick(y_ceil * 0.5, self.metric, &self.currency))
                            .child(format_axis_tick(0.0, self.metric, &self.currency)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .relative()
                                    .w_full()
                                    .h(px(plot_height))
                                    .child(DashboardTrendCanvas {
                                        id: pro_eid(&self.id, "mdc-trend-canvas"),
                                        kind: self.chart_kind,
                                        current: cur_f32,
                                        previous: prev_f32,
                                        show_compare: self.compare,
                                        y_max: y_ceil as f32,
                                        accent: p.blue,
                                        card_bg: p.card,
                                        grid_color: p.line_soft,
                                        guide_color: p.line_strong,
                                        hover_idx: hover_bi,
                                        height: plot_height,
                                    })
                                    .when_some(tooltip_el, |d, tip| d.child(tip))
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
                                    .child(x_start)
                                    .child(x_mid)
                                    .child(x_end),
                            ),
                    ),
            );

        // Bottom Ranked Breakdown Lists: Top products & Channels
        let prod_card = self.render_ranked_card(
            "products",
            tr(cx, "Top products", "热销商品"),
            &self.products,
            &cur_rollup,
            w,
            cx,
        );
        let chan_card = self.render_ranked_card(
            "channels",
            tr(cx, "Channels", "销售渠道"),
            &self.channels,
            &cur_rollup,
            w,
            cx,
        );
        let lists_row = div()
            .w_full()
            .flex()
            .when(medium_narrow, |d| d.flex_col())
            .gap(px(if medium_narrow { 8. } else { 12. }))
            .child(prod_card)
            .child(chan_card);

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .relative()
            .w_full()
            .min_w_0()
            .px(px(if compact { 16. } else { 32. }))
            .py(px(if compact { 20. } else { 28. }))
            .bg(p.page)
            .flex()
            .flex_col()
            .gap(px(if medium_narrow { 8. } else { 12. }))
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
            .child(pro_view_transition(
                pro_eid(&self.id, "mdc-vtx"),
                SharedString::from("metrics-ready"),
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(if medium_narrow { 8. } else { 12. }))
                    .child(kpis_el)
                    .child(chart_card)
                    .child(lists_row),
            ))
            .child(pro_toast_banner(self.toast.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DashboardMetric, default_channels, default_products, default_sales_days, summarize_slice,
    };

    #[test]
    fn thirty_day_reference_dataset_matches_headline_kpis() {
        let days = default_sales_days();
        assert_eq!(days.len(), 180);

        let cur_30 = &days[150..180];
        let prev_30 = &days[120..150];
        let cur = summarize_slice(cur_30);
        let prev = summarize_slice(prev_30);

        assert_eq!(cur.revenue.round() as u64, 292_444);
        assert_eq!(cur.orders, 4_438);
        assert!((cur.aov - 65.90).abs() < 0.08);
        assert!((cur.refund_rate - 2.10).abs() < 0.05);

        let rev_delta = ((cur.revenue - prev.revenue) / prev.revenue) * 100.0;
        let ord_delta = ((cur.orders as f64 - prev.orders as f64) / prev.orders as f64) * 100.0;
        assert!((rev_delta - 6.7).abs() < 0.15);
        assert!((ord_delta - 5.8).abs() < 0.15);
    }

    #[test]
    fn ranked_shares_and_daily_metric_formulas_stay_consistent() {
        let products = default_products();
        let channels = default_channels();
        assert_eq!(products.len(), 5);
        assert_eq!(channels.len(), 5);
        assert!((products[0].share - 0.22).abs() < 1e-6);
        assert!((channels[0].share - 0.46).abs() < 1e-6);

        let days = default_sales_days();
        let sample = &days[150];
        assert!((sample.metric_value(DashboardMetric::Revenue) - 7403.0).abs() < 1e-6);
        assert!((sample.metric_value(DashboardMetric::Orders) - 114.0).abs() < 1e-6);
        assert!(sample.metric_value(DashboardMetric::Conversion) > 3.0);
    }
}
