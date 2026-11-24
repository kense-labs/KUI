//! Versatile time-series trend chart (`Line`, `Area`, `Bar`, `Sparkline`) with period-over-period overlays and synchronized scrubbing.
//!
//! Renders either a borderless standalone [`TrendChart`] (via [`TrendChart::standalone`] or builder methods)
//! or the synchronized 4-chart `"AI spend"` showcase (`Line` / `Area` / `Bar`, `7D` / `30D`, plus three linked sparklines) on [`TrendChart::new`].

use crate::{
    components::pro_chrome::{
        ProPalette, pro_ease, pro_eid, pro_empty_state, pro_mark_selected, pro_pill_indicator,
        pro_rolling_number, pro_selection_sensor, pro_skeleton_bar, pro_spring_f32, pro_swap_label,
        tr,
    },
    primitives::{
        family::{PhaseClock, Slide, disabled_motion},
        pro_icons::ProIcon,
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

/// Visual geometry drawn from the same [`TrendSeries`] data in [`TrendChart`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrendVariant {
    /// Multi-series strokes (dashed for `previous: true`).
    Line,
    /// Multi-series strokes with soft gradient area fill on current series.
    #[default]
    Area,
    /// Grouped vertical daily bars (`0.32` opacity for `previous: true`).
    Bar,
    /// Compact `40px` axis-free sparkline with mini hover tag.
    Sparkline,
}

impl TrendVariant {
    fn key(self) -> &'static str {
        match self {
            Self::Line => "line",
            Self::Area => "area",
            Self::Bar => "bar",
            Self::Sparkline => "sparkline",
        }
    }
}

/// Axis visibility configuration for [`TrendChart`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrendAxisMode {
    /// Left Y-axis ticks, horizontal gridlines, and bottom X-axis date ticks.
    #[default]
    Both,
    /// Bottom X-axis date ticks only.
    XOnly,
    /// No axes (uses compact floating mini-tag instead of full multi-series popover).
    None,
}

/// Single current-period or previous-period series plotted by [`TrendChart`].
#[derive(Clone, Debug, PartialEq)]
pub struct TrendSeries {
    /// Stable series identifier.
    pub id: SharedString,
    /// Legend and tooltip label (for example `"This period"` or `"Last period"`).
    pub label: SharedString,
    /// Point values aligned with `labels`; `None` leaves a clean line/area gap.
    pub values: Vec<Option<f64>>,
    /// Optional stroke/bar color override.
    pub color: Option<Hsla>,
    /// Marks a prior-period comparison series (drawn dashed in `Line`/`Area`, faint in `Bar`).
    pub previous: bool,
}

impl TrendSeries {
    /// Construct a current-period series from finite `f64` values.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        values: Vec<f64>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            values: values
                .into_iter()
                .map(|v| v.is_finite().then_some(v))
                .collect(),
            color: None,
            previous: false,
        }
    }

    /// Construct a series with explicit `Option<f64>` gaps.
    pub fn with_gaps(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        values: Vec<Option<f64>>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            values: values
                .into_iter()
                .map(|opt| opt.filter(|v| v.is_finite()))
                .collect(),
            color: None,
            previous: false,
        }
    }

    /// Override the series stroke or bar color.
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    /// Mark this series as the prior-period baseline (`dashed` stroke / `faint` bar).
    pub fn with_previous(mut self, previous: bool) -> Self {
        self.previous = previous;
        self
    }
}

/// Typed interaction events emitted by [`TrendChart`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrendChartEvent {
    /// Emitted when pointer scrubbing or keyboard navigation changes the inspected point index.
    IndexChanged(Option<usize>),
    /// Emitted in showcase mode when switching `Line`, `Area`, or `Bar`.
    VariantChanged(TrendVariant),
    /// Emitted in showcase mode when switching `7D` or `30D`.
    RangeChanged(SharedString),
}

impl EventEmitter<TrendChartEvent> for TrendChart {}

/// Format a trend point or axis value compactly with prefix/suffix.
pub fn format_trend_number(val: f64, prefix: &str, suffix: &str, decimals: usize) -> String {
    if !val.is_finite() {
        return format!("{prefix}0{suffix}");
    }
    let neg = val < 0.0;
    let abs = val.abs();
    let sign = if neg { "\u{2212}" } else { "" };
    if suffix.is_empty() && abs >= 1_000_000.0 {
        let m = abs / 1_000_000.0;
        let s = if m >= 100.0 {
            format!("{m:.0}M")
        } else {
            format!("{m:.1}M")
        };
        return format!("{sign}{prefix}{s}");
    }
    if suffix.is_empty() && abs >= 1_000.0 && decimals == 0 {
        let k = abs / 1_000.0;
        let s = if (k.fract()).abs() < 0.05 {
            format!("{k:.0}k")
        } else {
            format!("{k:.1}k")
        };
        return format!("{sign}{prefix}{s}");
    }
    format!("{sign}{prefix}{abs:.*}{suffix}", decimals)
}

fn compute_axis_domain(series: &[TrendSeries], zero_anchor: bool) -> (f64, f64, bool) {
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for s in series {
        for &v in s.values.iter().flatten() {
            if v.is_finite() {
                min_v = min_v.min(v);
                max_v = max_v.max(v);
            }
        }
    }
    if !min_v.is_finite() || !max_v.is_finite() || (min_v.abs() < 1e-9 && max_v.abs() < 1e-9) {
        return (0.0, 100.0, false);
    }
    if zero_anchor {
        let lo = min_v.min(0.0);
        let hi_raw = max_v.max(0.0);
        let exp = 10f64.powf(hi_raw.abs().max(1.0).log10().floor());
        let norm = hi_raw / exp;
        let step = if norm <= 1.2 {
            1.2
        } else if norm <= 2.0 {
            2.0
        } else if norm <= 2.5 {
            2.5
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
        (lo, (step * exp).max(lo + 1e-3), true)
    } else {
        let span = (max_v - min_v).max(max_v.abs() * 0.08).max(1e-3);
        let pad = span * 0.14;
        (min_v - pad, max_v + pad, true)
    }
}

fn resample_opt_series(src: &[Option<f32>], len: usize) -> Vec<Option<f32>> {
    if src.is_empty() || len == 0 {
        return vec![None; len];
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
            match (src[i0], src[i1]) {
                (Some(a), Some(b)) => Some(a + (b - a) * frac),
                (Some(a), None) if frac < 0.5 => Some(a),
                (None, Some(b)) if frac >= 0.5 => Some(b),
                _ => None,
            }
        })
        .collect()
}

fn sample_opt_series_at(vals: &[Option<f32>], frac: f32) -> Option<f32> {
    if vals.is_empty() {
        return None;
    }
    if vals.len() == 1 {
        return vals[0];
    }
    let pos = frac.clamp(0.0, 1.0) * (vals.len() - 1) as f32;
    let i0 = (pos.floor() as usize).min(vals.len() - 1);
    let i1 = (i0 + 1).min(vals.len() - 1);
    let u = pos - i0 as f32;
    match (vals[i0], vals[i1]) {
        (Some(a), Some(b)) => Some(a + (b - a) * u),
        (Some(a), None) if u < 0.5 => Some(a),
        (None, Some(b)) if u >= 0.5 => Some(b),
        _ => None,
    }
}

fn paint_dashed_polyline(
    pts: &[Point<Pixels>],
    stroke_w: f32,
    dash_px: f32,
    gap_px: f32,
    color: Hsla,
    window: &mut Window,
) {
    if pts.len() < 2 {
        return;
    }
    let mut builder = PathBuilder::stroke(px(stroke_w));
    let mut has_any = false;
    let mut drawing = true;
    let mut rem = dash_px;

    for seg in pts.windows(2) {
        let mut x0 = f32::from(seg[0].x);
        let mut y0 = f32::from(seg[0].y);
        let x1 = f32::from(seg[1].x);
        let y1 = f32::from(seg[1].y);
        let dx = x1 - x0;
        let dy = y1 - y0;
        let mut dist = (dx * dx + dy * dy).sqrt();
        if dist <= 0.05 {
            continue;
        }
        let ux = dx / dist;
        let uy = dy / dist;
        while dist > 0.05 {
            let step = dist.min(rem);
            let nx = x0 + ux * step;
            let ny = y0 + uy * step;
            if drawing {
                builder.move_to(point(px(x0), px(y0)));
                builder.line_to(point(px(nx), px(ny)));
                has_any = true;
            }
            x0 = nx;
            y0 = ny;
            dist -= step;
            rem -= step;
            if rem <= 0.05 {
                drawing = !drawing;
                rem = if drawing { dash_px } else { gap_px };
            }
        }
    }
    if has_any && let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}

struct TrendMorphState {
    from_series: Vec<Vec<Option<f32>>>,
    to_series: Vec<Vec<Option<f32>>>,
    clock: PhaseClock,
    area_mix: Slide,
    bar_mix: Slide,
    scrub_x: Slide,
    scrub_presence: Slide,
    scrub_ready: bool,
    bar_hover_slides: Vec<Slide>,
    bar_spot_mix: Slide,
}

struct TrendTipGlideState {
    x: Slide,
    presence: Slide,
    ready: bool,
}

#[derive(Clone)]
struct TrendPlotSpec {
    id: ElementId,
    aria_label: SharedString,
    variant: TrendVariant,
    axis: TrendAxisMode,
    zero: bool,
    tooltip: bool,
    height: f32,
    labels: Vec<SharedString>,
    tooltip_labels: Vec<SharedString>,
    series: Vec<TrendSeries>,
    prefix: SharedString,
    suffix: SharedString,
    decimals: usize,
    active_idx: Option<usize>,
    empty_text: SharedString,
    bounds_sink: Rc<Cell<Option<Bounds<Pixels>>>>,
}

#[derive(IntoElement)]
struct TrendPlotCanvas {
    spec: TrendPlotSpec,
    card_bg: Hsla,
    started_at: Instant,
}

impl RenderOnce for TrendPlotCanvas {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let dark = p.dark;
        let reduced = disabled_motion(cx);
        let elapsed = self.started_at.elapsed().as_secs_f32();
        let mount_reveal = if reduced {
            1.0
        } else if elapsed < 0.70 {
            w.request_animation_frame();
            pro_ease((elapsed / 0.70).clamp(0., 1.))
        } else {
            1.0
        };

        let target_vecs: Vec<Vec<Option<f32>>> = self
            .spec
            .series
            .iter()
            .map(|s| s.values.iter().map(|opt| opt.map(|v| v as f32)).collect())
            .collect();
        let n_pts = target_vecs.iter().map(Vec::len).max().unwrap_or(0);
        let init_v = target_vecs.clone();
        let init_area = if self.spec.variant == TrendVariant::Area {
            1.0
        } else {
            0.0
        };
        let init_bar = if self.spec.variant == TrendVariant::Bar {
            1.0
        } else {
            0.0
        };

        let st = w.use_keyed_state(pro_eid(&self.spec.id, "trc-morph"), cx, |_, _| {
            TrendMorphState {
                from_series: init_v.clone(),
                to_series: init_v,
                clock: PhaseClock::default(),
                area_mix: Slide::new(init_area),
                bar_mix: Slide::new(init_bar),
                scrub_x: Slide::snappy(0.0),
                scrub_presence: Slide::snappy(0.0),
                scrub_ready: false,
                bar_hover_slides: (0..n_pts).map(|_| Slide::snappy(0.0)).collect(),
                bar_spot_mix: Slide::snappy(0.0),
            }
        });

        let target_area = if self.spec.variant == TrendVariant::Area {
            1.0
        } else {
            0.0
        };
        let target_bar = if self.spec.variant == TrendVariant::Bar {
            1.0
        } else {
            0.0
        };
        let active_idx = self.spec.active_idx;

        let (morphed, area_t, bar_t, guide_frac, scrub_alpha, bar_hovs, bar_spot) =
            st.update(cx, |s, cx| {
                let age = s.clock.time(0.32, w, cx);
                let cur_t = pro_ease((age / 0.32).clamp(0., 1.));
                if s.to_series != target_vecs {
                    let mut next_from = Vec::with_capacity(target_vecs.len());
                    for (si, tgt) in target_vecs.iter().enumerate() {
                        let tlen = tgt.len();
                        let fa = s
                            .from_series
                            .get(si)
                            .map(|v| resample_opt_series(v, tlen))
                            .unwrap_or_else(|| tgt.clone());
                        let fb = s
                            .to_series
                            .get(si)
                            .map(|v| resample_opt_series(v, tlen))
                            .unwrap_or_else(|| tgt.clone());
                        let blended = fa
                            .into_iter()
                            .zip(fb)
                            .map(|(a, b)| match (a, b) {
                                (Some(va), Some(vb)) => Some(va + (vb - va) * cur_t),
                                (_, other) => other,
                            })
                            .collect();
                        next_from.push(blended);
                    }
                    s.from_series = next_from;
                    s.to_series = target_vecs.clone();
                    s.clock.restart(cx);
                }
                let age2 = s.clock.time(0.32, w, cx);
                let t = if reduced {
                    1.0
                } else {
                    pro_ease((age2 / 0.32).clamp(0., 1.))
                };
                let morphed_vals: Vec<Vec<Option<f32>>> =
                    if t >= 0.999 || s.from_series.len() != s.to_series.len() {
                        s.to_series.clone()
                    } else {
                        s.from_series
                            .iter()
                            .zip(&s.to_series)
                            .map(|(fa, tb)| {
                                fa.iter()
                                    .zip(tb)
                                    .map(|(&a, &b)| match (a, b) {
                                        (Some(va), Some(vb)) => Some(va + (vb - va) * t),
                                        (_, other) => other,
                                    })
                                    .collect()
                            })
                            .collect()
                    };

                s.area_mix.duration(target_area, 0.28, cx);
                s.bar_mix.duration(target_bar, 0.32, cx);
                let a_mix = s.area_mix.get(w, cx).clamp(0.0, 1.0);
                let b_mix = s.bar_mix.get(w, cx).clamp(0.0, 1.0);

                if let Some(idx) = active_idx {
                    let target_gx = if self.spec.variant == TrendVariant::Bar {
                        (idx as f32 + 0.5) / n_pts.max(1) as f32
                    } else {
                        idx as f32 / (n_pts.saturating_sub(1)).max(1) as f32
                    };
                    if !s.scrub_ready {
                        s.scrub_x.reset(target_gx);
                        s.scrub_ready = true;
                    } else {
                        s.scrub_x.set(target_gx, cx);
                    }
                    s.scrub_presence.set(1.0, cx);
                } else {
                    s.scrub_presence.set(0.0, cx);
                    if s.scrub_presence.value(cx) < 0.04 {
                        s.scrub_ready = false;
                    }
                }
                let g_frac = s.scrub_x.get(w, cx).clamp(0.0, 1.0);
                let g_alpha = s.scrub_presence.get(w, cx).clamp(0.0, 1.0);

                if s.bar_hover_slides.len() < n_pts {
                    s.bar_hover_slides.resize_with(n_pts, || Slide::snappy(0.0));
                } else if s.bar_hover_slides.len() > n_pts {
                    s.bar_hover_slides.truncate(n_pts);
                }
                s.bar_spot_mix
                    .set(if active_idx.is_some() { 1.0 } else { 0.0 }, cx);
                let b_spot = s.bar_spot_mix.get(w, cx).clamp(0.0, 1.0);
                let mut hovs = Vec::with_capacity(n_pts);
                for (di, sl) in s.bar_hover_slides.iter_mut().enumerate().take(n_pts) {
                    let th = match active_idx {
                        Some(ai) if ai == di => 1.0,
                        Some(ai) if ai.abs_diff(di) == 1 => 0.35,
                        _ => 0.0,
                    };
                    sl.set(th, cx);
                    hovs.push(sl.get(w, cx).clamp(0.0, 1.0));
                }

                (morphed_vals, a_mix, b_mix, g_frac, g_alpha, hovs, b_spot)
            });

        let zero_anchor = self.spec.zero || self.spec.variant == TrendVariant::Bar || bar_t > 0.02;
        let (dom_min, dom_max, has_data) = compute_axis_domain(&self.spec.series, zero_anchor);
        let anim_min = pro_spring_f32(pro_eid(&self.spec.id, "dmin"), dom_min as f32, 0.32, w, cx);
        let anim_max = pro_spring_f32(pro_eid(&self.spec.id, "dmax"), dom_max as f32, 0.32, w, cx);

        let grid_c: Hsla = if dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let guide_c: Hsla = if dark {
            hsla(0., 0., 1., 0.20)
        } else {
            hsla(0., 0., 0., 0.14)
        };
        let prev_fallback: Hsla = if dark {
            hsla(0., 0., 1., 0.36)
        } else {
            hsla(0., 0., 0., 0.30)
        };

        let variant = self.spec.variant;
        let show_grid = self.spec.axis != TrendAxisMode::None && variant != TrendVariant::Sparkline;
        let series_meta = self.spec.series.clone();
        let card_bg = self.card_bg;
        let sink = self.spec.bounds_sink.clone();

        canvas(
            move |bounds, _, _| {
                sink.set(Some(bounds));
            },
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 4.0 || bh <= 4.0 {
                    return;
                }

                if show_grid {
                    for g in [0.0f32, 0.5, 1.0] {
                        let y = bounds.top() + px(bh * g);
                        window.paint_quad(fill(
                            Bounds::new(point(bounds.left(), y), size(bounds.size.width, px(1.))),
                            grid_c,
                        ));
                    }
                }
                if !has_data || morphed.is_empty() || n_pts == 0 {
                    return;
                }

                let span = (anim_max - anim_min).max(1e-3);
                let pad_y = if variant == TrendVariant::Sparkline {
                    3.5f32
                } else {
                    2.5f32
                };
                let map_y = |v: f32| -> Pixels {
                    let ty = 1.0 - ((v - anim_min) / span).clamp(0.0, 1.0);
                    bounds.top() + px(pad_y + ty * (bh - 2.0 * pad_y))
                };

                // 1. Paint spring-morphing bars whenever `bar_t > 0.004`.
                if bar_t > 0.004 {
                    let slot_w = bw / n_pts as f32;
                    let group_w = (slot_w * 0.68).clamp(3.0, 28.0);
                    let s_cnt = morphed.len().max(1);
                    let inner_gap = if s_cnt > 1 { 2.0f32 } else { 0.0 };
                    let bar_w =
                        ((group_w - inner_gap * (s_cnt - 1) as f32) / s_cnt as f32).max(1.5);
                    let zero_y = f32::from(map_y(0.0f32.clamp(anim_min, anim_max)));

                    for (si, s_vals) in morphed.iter().enumerate() {
                        let meta = &series_meta[si];
                        let base_c = meta.color.unwrap_or(p.blue);
                        for (di, &opt_v) in s_vals.iter().enumerate() {
                            let Some(v) = opt_v else {
                                continue;
                            };
                            let y_val = f32::from(map_y(v));
                            let raw_h = (zero_y - y_val).abs() * mount_reveal * bar_t;
                            if raw_h <= 0.6 {
                                continue;
                            }
                            let hov_boost = bar_hovs.get(di).copied().unwrap_or(0.0);
                            let h_px = (raw_h * (1.0 + 0.025 * hov_boost)).max(1.5);
                            let y0 = if v >= 0.0 { zero_y - h_px } else { zero_y };
                            let slot_cx = f32::from(bounds.left()) + (di as f32 + 0.5) * slot_w;
                            let x0 = slot_cx - group_w * 0.5 + si as f32 * (bar_w + inner_gap);

                            let rest_a = if meta.previous { 0.32 } else { 0.92 };
                            let dim_a = if meta.previous { 0.14 } else { 0.36 };
                            let eff_a =
                                (rest_a - bar_spot * (rest_a - dim_a) * (1.0 - hov_boost)) * bar_t;
                            let r = px(3.0f32.min(bar_w * 0.5).min(h_px * 0.5));
                            window.paint_quad(
                                fill(
                                    Bounds::new(point(px(x0), px(y0)), size(px(bar_w), px(h_px))),
                                    base_c.opacity(eff_a.clamp(0.0, 1.0)),
                                )
                                .corner_radii(r),
                            );
                        }
                    }
                }

                // 2. Paint smooth curves & area fills whenever `line_t = (1.0 - bar_t) > 0.004`.
                let line_t = (1.0 - bar_t).clamp(0.0, 1.0);
                if line_t > 0.004 {
                    let map_x = |i: usize, len: usize| -> Pixels {
                        let line_tx = i as f32 / (len.saturating_sub(1)).max(1) as f32;
                        let bar_tx = (i as f32 + 0.5) / len.max(1) as f32;
                        let tx = line_tx + (bar_tx - line_tx) * bar_t;
                        bounds.left() + px(tx * bw)
                    };

                    let mut order: Vec<usize> = (0..morphed.len()).collect();
                    order.sort_by_key(|&si| !series_meta[si].previous);

                    for si in order {
                        let s_vals = &morphed[si];
                        let meta = &series_meta[si];
                        let color = meta.color.unwrap_or(if meta.previous {
                            prev_fallback
                        } else {
                            p.blue
                        });
                        let max_vis_i = (((s_vals.len().saturating_sub(1)) as f32 * mount_reveal)
                            .ceil() as usize)
                            .min(s_vals.len().saturating_sub(1));

                        let mut run: Vec<Point<Pixels>> = Vec::new();
                        let flush_run = |seg: &[Point<Pixels>], window: &mut Window| {
                            if seg.len() < 2 {
                                return;
                            }
                            if area_t > 0.005 && !meta.previous {
                                let mut area = PathBuilder::fill();
                                let first = seg[0];
                                let last = seg[seg.len() - 1];
                                area.move_to(point(first.x, bounds.bottom()));
                                area.line_to(first);
                                for &pt in seg.iter().skip(1) {
                                    area.line_to(pt);
                                }
                                area.line_to(point(last.x, bounds.bottom()));
                                area.close();
                                if let Ok(path) = area.build() {
                                    let top_a = (if dark { 0.22 } else { 0.16 }) * area_t * line_t;
                                    window.paint_path(
                                        path,
                                        linear_gradient(
                                            180.,
                                            linear_color_stop(color.opacity(top_a), 0.),
                                            linear_color_stop(color.opacity(0.0), 1.),
                                        ),
                                    );
                                }
                            }
                            if meta.previous {
                                paint_dashed_polyline(
                                    seg,
                                    1.3,
                                    3.2,
                                    3.2,
                                    color.opacity(color.a * line_t),
                                    window,
                                );
                            } else {
                                let mut line = PathBuilder::stroke(px(1.65));
                                for (pi, &pt) in seg.iter().enumerate() {
                                    if pi == 0 {
                                        line.move_to(pt);
                                    } else {
                                        line.line_to(pt);
                                    }
                                }
                                if let Ok(path) = line.build() {
                                    window.paint_path(path, color.opacity(color.a * line_t));
                                }
                            }
                        };

                        for (i, &opt_v) in s_vals.iter().enumerate().take(max_vis_i + 1) {
                            if let Some(v) = opt_v {
                                run.push(point(map_x(i, s_vals.len()), map_y(v)));
                            } else {
                                flush_run(&run, window);
                                run.clear();
                            }
                        }
                        flush_run(&run, window);
                    }
                }

                // 3. Paint `Slide::snappy` gliding crosshair & curve-riding active point dots.
                if scrub_alpha > 0.01 && line_t > 0.05 {
                    let eff_sc = scrub_alpha * line_t;
                    let gx = bounds.left() + px(guide_frac.clamp(0., 1.) * bw);
                    if variant != TrendVariant::Sparkline {
                        window.paint_quad(fill(
                            Bounds::new(
                                point(gx, bounds.top() - px(4.)),
                                size(px(1.), px(bh + 4.)),
                            ),
                            guide_c.opacity(guide_c.a * eff_sc),
                        ));
                    }
                    for (si, s_vals) in morphed.iter().enumerate() {
                        let Some(v) = sample_opt_series_at(s_vals, guide_frac) else {
                            continue;
                        };
                        let meta = &series_meta[si];
                        let color = meta.color.unwrap_or(if meta.previous {
                            prev_fallback
                        } else {
                            p.blue
                        });
                        let py = map_y(v);
                        let (halo_r, dot_r) = if variant == TrendVariant::Sparkline || meta.previous
                        {
                            (4.5, 3.0)
                        } else {
                            (5.5, 3.8)
                        };
                        window.paint_quad(
                            fill(
                                Bounds::new(
                                    point(gx - px(halo_r), py - px(halo_r)),
                                    size(px(halo_r * 2.), px(halo_r * 2.)),
                                ),
                                card_bg.opacity(eff_sc),
                            )
                            .corner_radii(px(99.)),
                        );
                        window.paint_quad(
                            fill(
                                Bounds::new(
                                    point(gx - px(dot_r), py - px(dot_r)),
                                    size(px(dot_r * 2.), px(dot_r * 2.)),
                                ),
                                color.opacity(color.a * eff_sc),
                            )
                            .corner_radii(px(99.)),
                        );
                    }
                }
            },
        )
        .w_full()
        .flex_none()
        .h(px(self.spec.height))
    }
}

/// Versatile standalone time-series chart and synchronized multi-chart showcase (`trend-chart`).
pub struct TrendChart {
    id: ElementId,
    showcase_mode: bool,
    showcase_range: SharedString,
    variant: TrendVariant,
    axis: Option<TrendAxisMode>,
    zero: bool,
    tooltip: bool,
    height: Option<f32>,
    aria_label: SharedString,
    labels: Vec<SharedString>,
    tooltip_labels: Vec<SharedString>,
    series: Vec<TrendSeries>,
    prefix: SharedString,
    suffix: SharedString,
    decimals: usize,
    controlled_index: Option<Option<usize>>,
    default_index: Option<usize>,
    hovered_index: Option<usize>,
    empty_text: Option<SharedString>,
    loading: bool,
    empty: bool,
    plot_bounds: [Rc<Cell<Option<Bounds<Pixels>>>>; 4],
    focus: FocusHandle,
    started_at: Instant,
}

impl TrendChart {
    /// Create a [`TrendChart`] displaying the synchronized `"AI spend"` multi-chart showcase (`Line` / `Area` / `Bar`, `7D` / `30D`, plus three linked sparklines);
    /// calling [`TrendChart::standalone`] or `.with_series(...)` switches into borderless standalone chart mode.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let (labels, tip_labels, spend_cur, spend_prev, _, _, _) = showcase_dataset("7d");
        Self {
            id: id.into(),
            showcase_mode: true,
            showcase_range: "7d".into(),
            variant: TrendVariant::Area,
            axis: None,
            zero: true,
            tooltip: true,
            height: None,
            aria_label: "Trend chart".into(),
            labels,
            tooltip_labels: tip_labels,
            series: vec![
                TrendSeries::new("cur", "This period", spend_cur),
                TrendSeries::new("prev", "Last period", spend_prev).with_previous(true),
            ],
            prefix: "$".into(),
            suffix: "".into(),
            decimals: 0,
            controlled_index: None,
            default_index: None,
            hovered_index: None,
            empty_text: None,
            loading: false,
            empty: false,
            plot_bounds: [
                Rc::new(Cell::new(None)),
                Rc::new(Cell::new(None)),
                Rc::new(Cell::new(None)),
                Rc::new(Cell::new(None)),
            ],
            focus: cx.focus_handle().tab_stop(true),
            started_at: Instant::now(),
        }
    }

    /// Create a borderless standalone [`TrendChart`] ready to embed inside any custom card or layout.
    pub fn standalone(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        let mut this = Self::new(id, cx);
        this.showcase_mode = false;
        this.variant = TrendVariant::Line;
        this
    }

    /// Switch into standalone chart mode and select `Line`, `Area`, `Bar`, or `Sparkline`.
    pub fn with_variant(mut self, variant: TrendVariant) -> Self {
        self.showcase_mode = false;
        self.variant = variant;
        self
    }

    /// Keep showcase mode active while setting the initial primary chart variant.
    pub fn with_showcase_variant(mut self, variant: TrendVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Switch into standalone chart mode and supply X-axis point labels.
    pub fn with_labels(
        mut self,
        labels: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.showcase_mode = false;
        self.labels = labels.into_iter().map(Into::into).collect();
        self
    }

    /// Supply longer tooltip header labels (falling back to `labels`).
    pub fn with_tooltip_labels(
        mut self,
        labels: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        self.tooltip_labels = labels.into_iter().map(Into::into).collect();
        self
    }

    /// Switch into standalone chart mode and replace the plotted [`TrendSeries`].
    pub fn with_series(mut self, series: Vec<TrendSeries>) -> Self {
        self.showcase_mode = false;
        self.series = series;
        self
    }

    /// Configure value prefix, suffix, and decimal precision (`"$", "", 0` -> `$435`).
    pub fn with_format(
        mut self,
        prefix: impl Into<SharedString>,
        suffix: impl Into<SharedString>,
        decimals: usize,
    ) -> Self {
        self.prefix = prefix.into();
        self.suffix = suffix.into();
        self.decimals = decimals;
        self
    }

    /// Override the plot height in pixels (`152px` default, `40px` for `Sparkline`).
    pub fn with_height(mut self, height: f32) -> Self {
        self.height = Some(height.clamp(28., 420.));
        self
    }

    /// Configure axis visibility (`Both`, `XOnly`, or `None`).
    pub fn with_axis(mut self, axis: TrendAxisMode) -> Self {
        self.axis = Some(axis);
        self
    }

    /// Enable or disable floating tooltips/tags while keeping hover dot markers active.
    pub fn with_tooltip(mut self, tooltip: bool) -> Self {
        self.tooltip = tooltip;
        self
    }

    /// Anchor the Y-axis at zero (`true`) or zoom into `[min..max]` (`false`; `Bar` always anchors at zero).
    pub fn with_zero(mut self, zero: bool) -> Self {
        self.zero = zero;
        self
    }

    /// Control the highlighted point index externally so multiple standalone charts scrub in lockstep.
    pub fn with_controlled_index(mut self, index: Option<usize>) -> Self {
        self.controlled_index = Some(index);
        self
    }

    /// Update the externally controlled point index on a live entity.
    pub fn set_controlled_index(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        self.controlled_index = Some(index);
        cx.notify();
    }

    /// Keep a resting point index highlighted when nobody hovers.
    pub fn with_default_index(mut self, index: Option<usize>) -> Self {
        self.default_index = index;
        self
    }

    /// Set the screen-reader chart label.
    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = label.into();
        self
    }

    /// Customize the empty-plot caption.
    pub fn with_empty_text(mut self, text: impl Into<SharedString>) -> Self {
        self.empty_text = Some(text.into());
        self
    }

    /// Render skeleton placeholders while data loads.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Render the empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the active point index currently highlighted across the chart(s).
    pub fn active_index(&self) -> Option<usize> {
        let len = self.point_count();
        if len == 0 {
            return None;
        }
        let raw = match self.controlled_index {
            Some(ci) => ci,
            None => self.hovered_index.or(self.default_index),
        };
        raw.map(|i| i.min(len - 1))
    }

    fn point_count(&self) -> usize {
        if self.showcase_mode {
            let (lbls, _, _, _, _, _, _) = showcase_dataset(&self.showcase_range);
            lbls.len()
        } else {
            self.series
                .iter()
                .map(|s| s.values.len())
                .max()
                .unwrap_or(self.labels.len())
        }
    }

    fn set_hover_idx(&mut self, next: Option<usize>, cx: &mut Context<Self>) {
        if self.hovered_index != next {
            self.hovered_index = next;
            cx.emit(TrendChartEvent::IndexChanged(next));
            cx.notify();
        }
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let n = self.point_count();
        if n == 0 {
            return;
        }
        let max_i = n - 1;
        match ev.keystroke.key.as_str() {
            "left" => {
                let next = match self.active_index() {
                    Some(i) => i.saturating_sub(1),
                    None => max_i,
                };
                self.set_hover_idx(Some(next), cx);
            }
            "right" => {
                let next = match self.active_index() {
                    Some(i) => (i + 1).min(max_i),
                    None => 0,
                };
                self.set_hover_idx(Some(next), cx);
            }
            "home" => self.set_hover_idx(Some(0), cx),
            "end" => self.set_hover_idx(Some(max_i), cx),
            "escape" => self.set_hover_idx(None, cx),
            _ => {}
        }
    }

    fn render_plot_widget(
        &self,
        spec: TrendPlotSpec,
        card_bg: Hsla,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let dark = p.dark;
        let pop_bg: Hsla = if dark {
            rgb(0x252527).into()
        } else {
            rgb(0xffffff).into()
        };
        let line: Hsla = if dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let prev_fallback: Hsla = if dark {
            hsla(0., 0., 1., 0.36)
        } else {
            hsla(0., 0., 0., 0.30)
        };

        let zero_anchor = spec.zero || spec.variant == TrendVariant::Bar;
        let (dom_min, dom_max, has_data) = compute_axis_domain(&spec.series, zero_anchor);
        let n_pts = spec
            .series
            .iter()
            .map(|s| s.values.len())
            .max()
            .unwrap_or(0);

        let compact_tag_mode =
            spec.variant == TrendVariant::Sparkline || spec.axis == TrendAxisMode::None;

        let bw = spec
            .bounds_sink
            .get()
            .map(|b| f32::from(b.size.width))
            .unwrap_or(if compact_tag_mode { 160. } else { 440. });

        let tip_glide_st = w.use_keyed_state(pro_eid(&spec.id, "tip-glide"), cx, |_, _| {
            TrendTipGlideState {
                x: Slide::snappy(0.0),
                presence: Slide::snappy(0.0),
                ready: false,
            }
        });
        let (gliding_gx, tip_alpha) = tip_glide_st.update(cx, |gs, cx| {
            if let Some(idx) = spec
                .active_idx
                .filter(|_| spec.tooltip && has_data && n_pts > 0)
            {
                let frac = if spec.variant == TrendVariant::Bar {
                    (idx as f32 + 0.5) / n_pts.max(1) as f32
                } else {
                    idx as f32 / (n_pts.saturating_sub(1)).max(1) as f32
                };
                let target_px = frac * bw;
                if !gs.ready {
                    gs.x.reset(target_px);
                    gs.ready = true;
                } else {
                    gs.x.set(target_px, cx);
                }
                gs.presence.set(1.0, cx);
            } else {
                gs.presence.set(0.0, cx);
                if gs.presence.value(cx) < 0.04 {
                    gs.ready = false;
                }
            }
            (
                gs.x.get(w, cx).clamp(0.0, bw.max(1.0)),
                gs.presence.get(w, cx).clamp(0.0, 1.0),
            )
        });

        let tip_overlay: Option<AnyElement> = spec
            .active_idx
            .filter(|_| spec.tooltip && has_data && n_pts > 0)
            .and_then(|idx| {
                if compact_tag_mode {
                    let s0 = spec.series.first()?;
                    let val = (*s0.values.get(idx)?)?;
                    let date_lbl = spec.labels.get(idx).cloned().unwrap_or_default();
                    let chip_bg: Hsla = if dark {
                        hsla(0., 0., 1., 0.93)
                    } else {
                        hsla(240. / 360., 0.06, 0.09, 0.90)
                    };
                    let chip_fg: Hsla = if dark {
                        rgb(0x111111).into()
                    } else {
                        rgb(0xfcfcfc).into()
                    };
                    let tag_w = 88.0f32;
                    let left_px = (gliding_gx - tag_w * 0.5).clamp(0.0, (bw - tag_w).max(0.0));
                    return Some(
                        div()
                            .absolute()
                            .top(px(-20. + 2. * (1. - tip_alpha)))
                            .left(px(left_px))
                            .opacity(tip_alpha)
                            .px(px(6.))
                            .py(px(2.))
                            .rounded(px(5.))
                            .bg(chip_bg)
                            .shadow_sm()
                            .flex()
                            .items_baseline()
                            .gap(px(5.))
                            .text_size(px(11.))
                            .text_color(chip_fg)
                            .child(div().font_weight(FontWeight::MEDIUM).child(
                                format_trend_number(val, &spec.prefix, &spec.suffix, spec.decimals),
                            ))
                            .child(div().opacity(0.68).child(date_lbl))
                            .into_any_element(),
                    );
                }

                let head_date = spec
                    .tooltip_labels
                    .get(idx)
                    .or_else(|| spec.labels.get(idx))
                    .cloned()
                    .unwrap_or_default();
                let mut rows = div()
                    .mt(px(4.))
                    .pt(px(4.))
                    .border_t_1()
                    .border_color(line)
                    .flex()
                    .flex_col()
                    .gap(px(1.));

                for s in &spec.series {
                    let Some(Some(v)) = s.values.get(idx).copied() else {
                        continue;
                    };
                    let sw_c = s
                        .color
                        .unwrap_or(if s.previous { prev_fallback } else { p.blue });
                    let swatch_el: AnyElement = if s.previous && spec.variant != TrendVariant::Bar {
                        div()
                            .w(px(8.))
                            .h(px(2.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(div().w(px(3.)).h(px(1.5)).bg(sw_c))
                            .child(div().w(px(3.)).h(px(1.5)).bg(sw_c))
                            .into_any_element()
                    } else {
                        div()
                            .size(px(8.))
                            .flex_none()
                            .rounded(px(2.))
                            .bg(sw_c.opacity(if s.previous { 0.45 } else { 1.0 }))
                            .into_any_element()
                    };

                    rows = rows.child(
                        div()
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(12.))
                            .child(swatch_el)
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
                                    .child(format_trend_number(
                                        v,
                                        &spec.prefix,
                                        &spec.suffix,
                                        spec.decimals,
                                    )),
                            ),
                    );
                }

                let tip_w = 162.0f32;
                let flip_left = gliding_gx > bw * 0.5;
                let left_px = if flip_left {
                    (gliding_gx - tip_w - 14.0).max(0.0)
                } else {
                    (gliding_gx + 14.0).min((bw - tip_w).max(0.0))
                };

                Some(
                    div()
                        .absolute()
                        .top(px(-8. + 3. * (1. - tip_alpha)))
                        .left(px(left_px))
                        .opacity(tip_alpha)
                        .w(px(tip_w))
                        .px(px(10.))
                        .py(px(7.))
                        .rounded(px(10.))
                        .bg(pop_bg)
                        .border_1()
                        .border_color(p.line_strong)
                        .shadow_lg()
                        .child(pro_swap_label(
                            pro_eid(&spec.id, "tip-head"),
                            head_date,
                            13.,
                            FontWeight::MEDIUM,
                            p.strong,
                        ))
                        .child(rows)
                        .into_any_element(),
                )
            });

        let empty_lbl: Option<AnyElement> = (!has_data).then(|| {
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
                        .child(spec.empty_text.clone()),
                )
                .into_any_element()
        });

        let sink_move = spec.bounds_sink.clone();
        let is_bar_mode = spec.variant == TrendVariant::Bar;
        let plot_id = pro_eid(&spec.id, "plot");
        let aria_lbl = spec.aria_label.clone();
        let plot_h = spec.height;
        let axis_mode = spec.axis;
        let labels_vec = spec.labels.clone();
        let pre = spec.prefix.clone();
        let suf = spec.suffix.clone();
        let dec = spec.decimals;

        let interactive_plot = div()
            .id(plot_id)
            .role(Role::Group)
            .aria_label(aria_lbl)
            .w_full()
            .flex_none()
            .h(px(plot_h))
            .relative()
            .cursor(if has_data {
                CursorStyle::Crosshair
            } else {
                CursorStyle::Arrow
            })
            .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, _, cx| {
                if n_pts == 0 {
                    return;
                }
                let Some(b) = sink_move.get() else {
                    return;
                };
                let bw = f32::from(b.size.width).max(1.0);
                let rx = f32::from(ev.position.x - b.left()).clamp(0.0, bw - 0.01);
                let idx = if is_bar_mode {
                    ((rx / bw) * n_pts as f32).floor() as usize
                } else if n_pts <= 1 {
                    0
                } else {
                    ((rx / bw) * (n_pts - 1) as f32).round() as usize
                }
                .min(n_pts - 1);
                this.set_hover_idx(Some(idx), cx);
            }))
            .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                if !*inside && this.hovered_index.is_some() {
                    this.set_hover_idx(None, cx);
                }
            }))
            .child(TrendPlotCanvas {
                spec,
                card_bg,
                started_at: self.started_at,
            })
            .when_some(empty_lbl, |d, el| d.child(el))
            .when_some(tip_overlay, |d, tip| d.child(tip));

        if axis_mode == TrendAxisMode::None {
            return interactive_plot.into_any_element();
        }

        let first_x = labels_vec.first().cloned().unwrap_or_default();
        let mid_x = (labels_vec.len() >= 3)
            .then(|| labels_vec.get(labels_vec.len() / 2).cloned())
            .flatten();
        let last_x = (labels_vec.len() >= 2)
            .then(|| labels_vec.last().cloned())
            .flatten();

        let x_axis = div()
            .w_full()
            .flex_none()
            .h(px(16.))
            .flex()
            .items_center()
            .justify_between()
            .text_size(px(11.))
            .line_height(px(16.))
            .text_color(p.muted)
            .child(first_x)
            .when_some(mid_x, |d, m| d.child(m))
            .when_some(last_x, |d, l| d.child(l));

        if axis_mode == TrendAxisMode::XOnly {
            return div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(interactive_plot)
                .child(x_axis)
                .into_any_element();
        }

        let y_top = format_trend_number(dom_max, &pre, &suf, dec);
        let y_mid = format_trend_number((dom_min + dom_max) * 0.5, &pre, &suf, dec);
        let y_bot = format_trend_number(dom_min, &pre, &suf, dec);

        div()
            .w_full()
            .flex()
            .items_start()
            .gap(px(10.))
            .child(
                div()
                    .w(px(34.))
                    .h(px(plot_h))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .justify_between()
                    .items_end()
                    .text_size(px(11.))
                    .line_height(px(14.))
                    .text_color(p.muted)
                    .child(y_top)
                    .child(y_mid)
                    .child(y_bot),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(interactive_plot)
                    .child(x_axis),
            )
            .into_any_element()
    }
}

#[allow(clippy::type_complexity)]
fn showcase_dataset(
    range: &str,
) -> (
    Vec<SharedString>,
    Vec<SharedString>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
) {
    if range == "30d" {
        let labels: Vec<SharedString> = (1..=14)
            .map(|i| SharedString::from(format!("Feb {}", i * 2)))
            .collect();
        let tip_labels: Vec<SharedString> = (1..=14)
            .map(|i| SharedString::from(format!("Feb {}, 2026", i * 2)))
            .collect();
        let spend_cur = vec![
            690., 740., 680., 810., 845., 790., 880., 835., 910., 940., 895., 975., 1020., 1060.,
        ];
        let spend_prev = vec![
            580., 610., 595., 640., 675., 660., 705., 690., 735., 760., 745., 790., 815., 840.,
        ];
        let tokens = vec![
            64., 70., 66., 74., 78., 72., 81., 77., 84., 88., 82., 91., 95., 99.,
        ];
        let reqs = vec![
            36., 39., 37., 42., 44., 41., 46., 43., 48., 50., 47., 52., 54., 56.,
        ];
        let cpr = vec![
            19.1, 18.9, 18.4, 19.3, 19.2, 19.0, 18.8, 19.4, 18.7, 18.6, 18.9, 18.5, 18.4, 18.3,
        ];
        (labels, tip_labels, spend_cur, spend_prev, tokens, reqs, cpr)
    } else {
        let labels: Vec<SharedString> = [
            "Mar 3", "Mar 4", "Mar 5", "Mar 6", "Mar 7", "Mar 8", "Mar 9",
        ]
        .into_iter()
        .map(Into::into)
        .collect();
        let tip_labels: Vec<SharedString> = [
            "Tue, Mar 3",
            "Wed, Mar 4",
            "Thu, Mar 5",
            "Fri, Mar 6",
            "Sat, Mar 7",
            "Sun, Mar 8",
            "Mon, Mar 9",
        ]
        .into_iter()
        .map(Into::into)
        .collect();
        let spend_cur = vec![435., 412., 365., 428., 392., 354., 475.];
        let spend_prev = vec![378., 356., 342., 368., 349., 331., 394.];
        let tokens = vec![40.3, 38.1, 33.9, 39.4, 36.2, 32.8, 44.0];
        let reqs = vec![20.2, 21.4, 18.6, 22.1, 19.8, 18.9, 23.6];
        let cpr = vec![21.58, 19.25, 19.62, 19.36, 19.79, 18.73, 20.12];
        (labels, tip_labels, spend_cur, spend_prev, tokens, reqs, cpr)
    }
}

impl Render for TrendChart {
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

        if self.loading {
            let phase = if disabled_motion(cx) {
                0.25
            } else {
                w.request_animation_frame();
                (self.started_at.elapsed().as_secs_f32() / 1.4).fract()
            };
            let h = self
                .height
                .unwrap_or(if self.variant == TrendVariant::Sparkline {
                    40.
                } else {
                    152.
                });
            return div()
                .id(pro_eid(&self.id, "loading"))
                .w_full()
                .max_w(px(if self.showcase_mode { 580. } else { 9999. }))
                .child(pro_skeleton_bar(540., h, 8., phase, cx))
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(580.))
                .rounded(px(10.))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .child(pro_empty_state::<Self>(
                    ProIcon::UsageChart,
                    tr(cx, "No series data", "暂无趋势序列"),
                    tr(
                        cx,
                        "Pass at least one TrendSeries to plot lines, areas, bars, or sparklines.",
                        "请传入至少一条 TrendSeries 以绘制折线、面积、柱条或迷你图。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let active_idx = self.active_index();
        let empty_str = self
            .empty_text
            .clone()
            .unwrap_or_else(|| tr(cx, "No data yet", "暂无数据").into());

        if !self.showcase_mode {
            let eff_h = self
                .height
                .unwrap_or(if self.variant == TrendVariant::Sparkline {
                    40.
                } else {
                    152.
                });
            let eff_axis = self
                .axis
                .unwrap_or(if self.variant == TrendVariant::Sparkline {
                    TrendAxisMode::None
                } else {
                    TrendAxisMode::Both
                });
            let spec = TrendPlotSpec {
                id: pro_eid(&self.id, "single"),
                aria_label: self.aria_label.clone(),
                variant: self.variant,
                axis: eff_axis,
                zero: self.zero,
                tooltip: self.tooltip,
                height: eff_h,
                labels: self.labels.clone(),
                tooltip_labels: self.tooltip_labels.clone(),
                series: self.series.clone(),
                prefix: self.prefix.clone(),
                suffix: self.suffix.clone(),
                decimals: self.decimals,
                active_idx,
                empty_text: empty_str,
                bounds_sink: self.plot_bounds[0].clone(),
            };
            return div()
                .id(self.id.clone())
                .track_focus(&self.focus)
                .w_full()
                .on_key_down(cx.listener(Self::handle_key_down))
                .child(self.render_plot_widget(spec, card_bg, w, cx))
                .into_any_element();
        }

        let (labels, tip_labels, spend_cur, spend_prev, tokens, reqs, cpr) =
            showcase_dataset(&self.showcase_range);
        let is_30d = self.showcase_range.as_ref() == "30d";

        let (spend_headline, spend_dec, spend_delta, spend_caption): (
            f64,
            usize,
            f64,
            SharedString,
        ) = if let Some(idx) = active_idx
            && let (Some(&sc), Some(&sp)) = (spend_cur.get(idx), spend_prev.get(idx))
        {
            let dlt = if sp > 1e-9 {
                ((sc - sp) / sp) * 100.0
            } else {
                0.0
            };
            let date_str = tip_labels
                .get(idx)
                .or_else(|| labels.get(idx))
                .cloned()
                .unwrap_or_default();
            (
                sc / 1_000.0,
                2,
                dlt,
                format!("{} \u{00b7} {date_str}", tr(cx, "AI spend", "AI 支出")).into(),
            )
        } else {
            (
                if is_30d { 11.4 } else { 2.7 },
                1,
                if is_30d { 21.8 } else { 16.5 },
                tr(cx, "AI spend", "AI 支出").into(),
            )
        };

        let (tok_headline, tok_dec, tok_suffix) = if let Some(idx) = active_idx
            && let Some(&tv) = tokens.get(idx)
        {
            (tv, 1usize, "M")
        } else if is_30d {
            (1.08, 2usize, "B")
        } else {
            (257.7, 1usize, "M")
        };
        let req_headline = active_idx
            .and_then(|idx| reqs.get(idx).copied())
            .unwrap_or(if is_30d { 612.4 } else { 144.6 });
        let cpr_headline = active_idx
            .and_then(|idx| cpr.get(idx).copied())
            .unwrap_or(if is_30d { 18.65 } else { 18.60 });

        let pill_fill: Hsla = if dark {
            rgb(0x28282c).into()
        } else {
            rgb(0xffffff).into()
        };
        let pill_border: Hsla = if dark {
            hsla(0., 0., 1., 0.11)
        } else {
            hsla(0., 0., 0., 0.08)
        };
        let track_bg: Hsla = if dark {
            hsla(0., 0., 1., 0.05)
        } else {
            hsla(0., 0., 0., 0.04)
        };
        let seg_hover: Hsla = if dark {
            hsla(0., 0., 1., 0.04)
        } else {
            hsla(0., 0., 0., 0.025)
        };

        let var_sensor = pro_selection_sensor();
        let cur_var = if self.variant == TrendVariant::Sparkline {
            TrendVariant::Area
        } else {
            self.variant
        };
        let mut var_seg = div()
            .relative()
            .p(px(2.))
            .rounded(px(7.))
            .bg(track_bg)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "vseg"),
                cur_var.key(),
                var_sensor.clone(),
                5.5,
                pill_fill,
                Some(pill_border),
                !dark,
            ));

        for (v_opt, v_en, v_zh) in [
            (TrendVariant::Line, "Line", "折线"),
            (TrendVariant::Area, "Area", "面积"),
            (TrendVariant::Bar, "Bar", "柱状"),
        ] {
            let selected = cur_var == v_opt;
            let lbl = tr(cx, v_en, v_zh);
            var_seg = var_seg.child(
                div()
                    .id(pro_eid(&self.id, format!("vbtn-{}", v_opt.key())))
                    .role(Role::RadioButton)
                    .aria_label(lbl)
                    .tab_index(0)
                    .h(px(22.))
                    .px(px(8.))
                    .relative()
                    .rounded(px(5.5))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if selected { p.ink } else { p.text })
                    .cursor_pointer()
                    .when(!selected, move |d| {
                        d.hover(move |s| s.bg(seg_hover).text_color(p.ink))
                    })
                    .when(selected, |d| d.child(pro_mark_selected(&var_sensor)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.variant != v_opt {
                            this.variant = v_opt;
                            cx.emit(TrendChartEvent::VariantChanged(v_opt));
                            cx.notify();
                        }
                    }))
                    .child(lbl),
            );
        }

        let rng_sensor = pro_selection_sensor();
        let mut rng_seg = div()
            .relative()
            .p(px(2.))
            .rounded(px(7.))
            .bg(track_bg)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "rseg"),
                self.showcase_range.clone(),
                rng_sensor.clone(),
                5.5,
                pill_fill,
                Some(pill_border),
                !dark,
            ));

        for (rid, rlbl) in [("7d", "7D"), ("30d", "30D")] {
            let selected = self.showcase_range.as_ref() == rid;
            rng_seg = rng_seg.child(
                div()
                    .id(pro_eid(&self.id, format!("rbtn-{rid}")))
                    .role(Role::RadioButton)
                    .aria_label(rlbl)
                    .tab_index(0)
                    .h(px(22.))
                    .px(px(8.))
                    .relative()
                    .rounded(px(5.5))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if selected { p.ink } else { p.text })
                    .cursor_pointer()
                    .when(!selected, move |d| {
                        d.hover(move |s| s.bg(seg_hover).text_color(p.ink))
                    })
                    .when(selected, |d| d.child(pro_mark_selected(&rng_sensor)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.showcase_range.as_ref() != rid {
                            this.showcase_range = rid.into();
                            this.hovered_index = None;
                            cx.emit(TrendChartEvent::RangeChanged(rid.into()));
                            cx.notify();
                        }
                    }))
                    .child(rlbl),
            );
        }

        let main_spec = TrendPlotSpec {
            id: pro_eid(&self.id, "main"),
            aria_label: tr(cx, "AI spend per day", "每日 AI 花费").into(),
            variant: cur_var,
            axis: TrendAxisMode::Both,
            zero: true,
            tooltip: true,
            height: 152.,
            labels: labels.clone(),
            tooltip_labels: tip_labels.clone(),
            series: vec![
                TrendSeries::new("this", tr(cx, "This period", "本周期"), spend_cur),
                TrendSeries::new("last", tr(cx, "Last period", "上周期"), spend_prev)
                    .with_previous(true),
            ],
            prefix: "$".into(),
            suffix: "".into(),
            decimals: 0,
            active_idx,
            empty_text: empty_str.clone(),
            bounds_sink: self.plot_bounds[0].clone(),
        };
        let main_plot = self.render_plot_widget(main_spec, card_bg, w, cx);

        let delta_up = spend_delta >= 0.0;
        let (dbg, dfg): (Hsla, Hsla) = if delta_up {
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
        let delta_sign = if delta_up { "+ " } else { "\u{2212} " };

        let main_card = div()
            .w_full()
            .rounded(px(10.))
            .bg(card_bg)
            .border_1()
            .border_color(card_border)
            .when(!dark, |d| d.shadow_xs())
            .px(px(16.))
            .pt(px(14.))
            .pb(px(12.))
            .flex()
            .flex_col()
            .gap(px(18.))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_start()
                    .justify_between()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(pro_swap_label(
                                pro_eid(&self.id, "sp-cap"),
                                spend_caption,
                                13.,
                                FontWeight::MEDIUM,
                                p.text,
                            ))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(pro_rolling_number(
                                        pro_eid(&self.id, "sp-tot"),
                                        spend_headline,
                                        spend_dec,
                                        "$",
                                        "k",
                                        24.,
                                        FontWeight::MEDIUM,
                                        p.strong,
                                    ))
                                    .child(
                                        div()
                                            .h(px(20.))
                                            .px(px(6.))
                                            .rounded(px(5.))
                                            .bg(dbg)
                                            .flex()
                                            .items_center()
                                            .child(pro_rolling_number(
                                                pro_eid(&self.id, "sp-dlt"),
                                                spend_delta.abs(),
                                                1,
                                                delta_sign,
                                                "%",
                                                12.,
                                                FontWeight::MEDIUM,
                                                dfg,
                                            )),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(var_seg)
                            .child(rng_seg),
                    ),
            )
            .child(main_plot);

        let sub_defs = [
            (
                1usize,
                "tok",
                tr(cx, "Tokens", "Token 量"),
                tok_headline,
                tok_dec,
                "",
                tok_suffix,
                tokens,
                "",
                "M",
                1usize,
                true,
            ),
            (
                2usize,
                "req",
                tr(cx, "Requests", "请求量"),
                req_headline,
                1usize,
                "",
                "k",
                reqs,
                "",
                "k",
                1usize,
                true,
            ),
            (
                3usize,
                "cpr",
                tr(cx, "Cost per 1k requests", "每千次请求成本"),
                cpr_headline,
                2usize,
                "$",
                "",
                cpr,
                "$",
                "",
                2usize,
                false,
            ),
        ];

        let mut mini_row = div().w_full().flex().flex_wrap().gap(px(12.));
        for (
            slot_i,
            sid,
            s_title,
            h_val,
            h_dec,
            h_pre,
            h_suf,
            vals,
            t_pre,
            t_suf,
            t_dec,
            zero_flag,
        ) in sub_defs
        {
            let sub_spec = TrendPlotSpec {
                id: pro_eid(&self.id, sid),
                aria_label: s_title.into(),
                variant: TrendVariant::Sparkline,
                axis: TrendAxisMode::None,
                zero: zero_flag,
                tooltip: true,
                height: 40.,
                labels: labels.clone(),
                tooltip_labels: tip_labels.clone(),
                series: vec![TrendSeries::new(sid, s_title, vals)],
                prefix: t_pre.into(),
                suffix: t_suf.into(),
                decimals: t_dec,
                active_idx,
                empty_text: empty_str.clone(),
                bounds_sink: self.plot_bounds[slot_i].clone(),
            };
            let sub_plot = self.render_plot_widget(sub_spec, card_bg, w, cx);
            mini_row = mini_row.child(
                div()
                    .flex_1()
                    .min_w(px(164.))
                    .rounded(px(10.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .when(!dark, |d| d.shadow_xs())
                    .px(px(14.))
                    .pt(px(12.))
                    .pb(px(10.))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.text)
                            .truncate()
                            .child(s_title),
                    )
                    .child(div().mt(px(2.)).mb(px(12.)).child(pro_rolling_number(
                        pro_eid(&self.id, format!("mh-{sid}")),
                        h_val,
                        h_dec,
                        h_pre,
                        h_suf,
                        15.,
                        FontWeight::MEDIUM,
                        p.strong,
                    )))
                    .child(sub_plot),
            );
        }

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .w_full()
            .max_w(px(580.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .on_key_down(cx.listener(Self::handle_key_down))
            .child(main_card)
            .child(mini_row)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{TrendChart, TrendSeries, TrendVariant, compute_axis_domain, sample_opt_series_at};
    use gpui::{AppContext, TestAppContext, px, size};
    use std::time::Duration;

    #[test]
    fn axis_domain_supports_zero_anchor_and_zoomed_rates() {
        let s = vec![TrendSeries::new("cpr", "Cost", vec![18.7, 19.4, 21.5])];
        let (lo_z, hi_z, ok_z) = compute_axis_domain(&s, true);
        assert!(ok_z);
        assert_eq!(lo_z, 0.0);
        assert!(hi_z >= 21.5);

        let (lo_zoom, hi_zoom, ok_zoom) = compute_axis_domain(&s, false);
        assert!(ok_zoom);
        assert!(lo_zoom > 15.0);
        assert!(hi_zoom < 25.0);
    }

    #[gpui::test]
    fn controlled_index_synchronizes_standalone_charts(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let chart = cx.new(|cx| {
            TrendChart::standalone("trc-test", cx)
                .with_variant(TrendVariant::Area)
                .with_labels(["Mar 3", "Mar 4", "Mar 5", "Mar 6"])
                .with_series(vec![
                    TrendSeries::new("this", "This week", vec![412., 455., 398., 470.]),
                    TrendSeries::new("last", "Last week", vec![356., 380., 371., 390.])
                        .with_previous(true),
                ])
        });
        chart.update(cx, |c, cx| {
            assert_eq!(c.active_index(), None);
            c.set_controlled_index(Some(2), cx);
            assert_eq!(c.active_index(), Some(2));
        });
    }

    #[gpui::test]
    fn showcase_variant_and_range_morph_without_collapsing(cx: &mut TestAppContext) {
        assert_eq!(
            sample_opt_series_at(&[Some(10.0), Some(20.0)], 0.5),
            Some(15.0)
        );
        cx.update(crate::init);
        let exec = cx.executor();
        let (chart, w) = cx.add_window_view(|_, cx| TrendChart::new("trc-showcase", cx));
        w.simulate_resize(size(px(600.), px(460.)));
        w.run_until_parked();

        w.update(|_, app| {
            chart.update(app, |c, cx| {
                c.variant = TrendVariant::Bar;
                c.showcase_range = "30d".into();
                c.set_hover_idx(Some(4), cx);
            });
        });
        w.run_until_parked();

        exec.advance_clock(Duration::from_millis(80));
        w.update(|win, app| win.simulate_next_frame(app));
        w.run_until_parked();

        w.update(|_, app| {
            let c = chart.read(app);
            assert_eq!(c.active_index(), Some(4));
            assert!(
                c.plot_bounds[0]
                    .get()
                    .is_some_and(|b| b.size.height > px(100.))
            );
        });
    }
}
