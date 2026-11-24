//! KPI stat card with rolling figure, directional delta badge (`lower_is_better`), sparkline/bar slot, and link/tab selection.
//!
//! Renders either a standalone [`StatCard`] (via [`StatCard::single`] or builder methods) or the synchronized
//! 4-card `"Usage"` (`7D` / `30D`) KPI showcase (`Spend`, `Tokens`, `Avg. latency`, `Error rate`) on [`StatCard::new`].

use crate::{
    components::pro_chrome::{
        ProPalette, pro_ease, pro_eid, pro_empty_state, pro_mark_selected, pro_pill_indicator,
        pro_rolling_number, pro_selection_sensor, pro_skeleton_bar, pro_spring_f32,
        pro_toast_banner, tr,
    },
    primitives::{
        family::{PhaseClock, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const DELTA_TRI_UP_SVG: &str = r#"<path d="M4 1.2 7.2 6.4H.8L4 1.2Z" fill="currentColor"></path>"#;
const DELTA_TRI_DOWN_SVG: &str = r#"<path d="M4 6.8.8 1.6h6.4L4 6.8Z" fill="currentColor"></path>"#;

/// Arrangement of the embedded mini chart relative to the headline figure in [`StatCard`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatCardLayout {
    /// Stack the sparkline or bar strip beneath the figure and delta row.
    #[default]
    Stacked,
    /// Place the sparkline or bar strip to the right of the figure (`44%` column aligned to bottom).
    Inline,
}

/// Built-in mini chart style for [`StatSparkSpec`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatSparkKind {
    /// Smooth stroke with vertical gradient fill.
    #[default]
    Area,
    /// Crisp stroke without area fill.
    Line,
    /// Rounded daily vertical bars.
    Bars,
}

/// Embedded sparkline or mini-bar dataset inside a [`StatCardItem`].
#[derive(Clone, Debug, PartialEq)]
pub struct StatSparkSpec {
    /// Visual rendering mode (`Area`, `Line`, or `Bars`).
    pub kind: StatSparkKind,
    /// Point date labels shown in the hover mini-tag.
    pub labels: Vec<SharedString>,
    /// Daily values matching `labels`.
    pub values: Vec<f64>,
    /// Optional custom stroke/bar color (defaults to palette blue).
    pub color: Option<Hsla>,
    /// Prefix used to format point values in the hover tag.
    pub prefix: SharedString,
    /// Suffix used to format point values in the hover tag.
    pub suffix: SharedString,
    /// Divisor applied to point values in the hover tag.
    pub divisor: f64,
    /// Decimal places in the hover tag.
    pub decimals: usize,
}

impl StatSparkSpec {
    /// Create a mini chart specification from `kind`, `labels`, and `values`.
    pub fn new(
        kind: StatSparkKind,
        labels: impl IntoIterator<Item = impl Into<SharedString>>,
        values: Vec<f64>,
    ) -> Self {
        Self {
            kind,
            labels: labels.into_iter().map(Into::into).collect(),
            values,
            color: None,
            prefix: "".into(),
            suffix: "".into(),
            divisor: 1.0,
            decimals: 0,
        }
    }

    /// Configure point value formatting in the hover tag.
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

    /// Override the sparkline stroke or bar fill color.
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

/// Complete specification for one KPI tile in [`StatCard`].
#[derive(Clone, Debug, PartialEq)]
pub struct StatCardItem {
    /// Stable card identifier.
    pub id: SharedString,
    /// Metric label above the number.
    pub label: SharedString,
    /// Numeric value rolled via [`pro_rolling_number`].
    pub value: f64,
    /// Optional static text override when `value` is non-numeric.
    pub text_override: Option<SharedString>,
    /// Scale divisor for the headline rolling number.
    pub divisor: f64,
    /// Decimal precision for the headline rolling number.
    pub decimals: usize,
    /// Prefix before the headline rolling number (for example `"$"`).
    pub prefix: SharedString,
    /// Suffix after the headline rolling number (for example `"k"`, `"M"`, `"s"`, `"%"`).
    pub suffix: SharedString,
    /// Percentage change vs. the prior period (`Some(16.5)` -> `▲ 16.5%`).
    pub change: Option<f64>,
    /// Quiet comparison phrase after the percentage badge (`"vs last week"`).
    pub change_label: Option<SharedString>,
    /// Whether negative deltas are good news (green) and positive deltas are bad news (red).
    pub lower_is_better: bool,
    /// Optional `16px` icon rendered before `label`.
    pub icon: Option<ProIcon>,
    /// Optional mini sparkline or bar chart rendered below or beside the figure.
    pub spark: Option<StatSparkSpec>,
    /// Card body layout (`Stacked` or `Inline`).
    pub layout: StatCardLayout,
    /// Optional bottom footer line separated by a hairline border.
    pub footer: Option<SharedString>,
    /// Optional link destination (`href`) or interactive button target.
    pub href: Option<SharedString>,
    /// Whether the whole card acts as an interactive button/link.
    pub interactive: bool,
}

impl StatCardItem {
    /// Construct a KPI stat card item with `id`, `label`, and `value`.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>, value: f64) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value,
            text_override: None,
            divisor: 1.0,
            decimals: 0,
            prefix: "".into(),
            suffix: "".into(),
            change: None,
            change_label: None,
            lower_is_better: false,
            icon: None,
            spark: None,
            layout: StatCardLayout::Stacked,
            footer: None,
            href: None,
            interactive: true,
        }
    }

    /// Format the rolling headline figure (`divisor`, `decimals`, `prefix`, `suffix`).
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

    /// Configure the percentage delta badge and comparison phrase.
    pub fn with_change(mut self, pct: f64, vs_label: impl Into<SharedString>) -> Self {
        self.change = Some(pct);
        let s: SharedString = vs_label.into();
        self.change_label = (!s.is_empty()).then_some(s);
        self
    }

    /// Treat decreases as green improvements and increases as red regressions (for latency, churn, or error rate).
    pub fn with_lower_is_better(mut self, lower_is_better: bool) -> Self {
        self.lower_is_better = lower_is_better;
        self
    }

    /// Attach a `16px` icon before the metric label.
    pub fn with_icon(mut self, icon: ProIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Attach a mini sparkline or bar chart to the card.
    pub fn with_spark(mut self, spark: StatSparkSpec) -> Self {
        self.spark = Some(spark);
        self
    }

    /// Choose `Stacked` (below) or `Inline` (right-hand) chart placement.
    pub fn with_layout(mut self, layout: StatCardLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Add a bottom footer caption row.
    pub fn with_footer(mut self, footer: impl Into<SharedString>) -> Self {
        let s: SharedString = footer.into();
        self.footer = (!s.is_empty()).then_some(s);
        self
    }

    /// Make the card a link to `href`.
    pub fn with_href(mut self, href: impl Into<SharedString>) -> Self {
        self.href = Some(href.into());
        self.interactive = true;
        self
    }

    /// Determine whether `change` represents a positive outcome (`Some(true)` = green, `Some(false)` = red, `None` = neutral `0.0%`).
    pub fn is_change_good(&self) -> Option<bool> {
        let chg = self.change?;
        if !chg.is_finite() || chg.abs() < 1e-6 {
            return None;
        }
        Some(if self.lower_is_better {
            chg < 0.0
        } else {
            chg > 0.0
        })
    }
}

/// Typed interaction events emitted by [`StatCard`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatCardEvent {
    /// Emitted when an interactive stat card or link card is clicked or activated with Enter/Space.
    Activated {
        id: SharedString,
        href: Option<SharedString>,
    },
    /// Emitted in showcase mode when switching between reporting ranges (`"7d"` / `"30d"`).
    RangeChanged(SharedString),
    /// Emitted when pointer scrubbing inspects a mini-chart day point.
    SparkHoverChanged {
        card_id: SharedString,
        index: Option<usize>,
    },
}

impl EventEmitter<StatCardEvent> for StatCard {}

fn resample_f32(src: &[f32], len: usize) -> Vec<f32> {
    if src.is_empty() || len == 0 {
        return vec![0.0; len];
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
            src[i0] + (src[i1] - src[i0]) * (pos - i0 as f32)
        })
        .collect()
}

struct SparkMorphState {
    from_vals: Vec<f32>,
    to_vals: Vec<f32>,
    clock: PhaseClock,
}

#[derive(IntoElement)]
struct StatSparkCanvas {
    id: ElementId,
    spec: StatSparkSpec,
    hovered_idx: Option<usize>,
    card_bg: Hsla,
    bounds_sink: Rc<Cell<Option<Bounds<Pixels>>>>,
}

impl RenderOnce for StatSparkCanvas {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let dark = p.dark;
        let reduced = disabled_motion(cx);
        let target_f32: Vec<f32> = self
            .spec
            .values
            .iter()
            .map(|&v| if v.is_finite() { v as f32 } else { 0.0 })
            .collect();
        let init_v = target_f32.clone();
        let st = w.use_keyed_state(pro_eid(&self.id, "sp-morph"), cx, |_, _| SparkMorphState {
            from_vals: init_v.clone(),
            to_vals: init_v,
            clock: PhaseClock::default(),
        });
        let values: Vec<f32> = st.update(cx, |s, cx| {
            let age = s.clock.time(0.32, w, cx);
            let cur_t = pro_ease((age / 0.32).clamp(0., 1.));
            if s.to_vals != target_f32 {
                let tlen = target_f32.len();
                let pa = resample_f32(&s.from_vals, tlen);
                let pb = resample_f32(&s.to_vals, tlen);
                s.from_vals = pa
                    .into_iter()
                    .zip(pb)
                    .map(|(a, b)| a + (b - a) * cur_t)
                    .collect();
                s.to_vals = target_f32.clone();
                s.clock.restart(cx);
            }
            let age2 = s.clock.time(0.32, w, cx);
            let t = if reduced {
                1.0
            } else {
                pro_ease((age2 / 0.32).clamp(0., 1.))
            };
            if t >= 0.999 || s.from_vals.len() != s.to_vals.len() {
                s.to_vals.clone()
            } else {
                s.from_vals
                    .iter()
                    .zip(&s.to_vals)
                    .map(|(&a, &b)| a + (b - a) * t)
                    .collect()
            }
        });

        let color = self.spec.color.unwrap_or(p.blue);
        let kind = self.spec.kind;
        let hovered_idx = self.hovered_idx;
        let card_bg = self.card_bg;
        let sink = self.bounds_sink;

        canvas(
            move |bounds, _, _| {
                sink.set(Some(bounds));
            },
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 4.0 || bh <= 4.0 || values.is_empty() {
                    return;
                }

                if kind == StatSparkKind::Bars {
                    let max_v = values.iter().copied().fold(0.0f32, f32::max).max(1e-3);
                    let n = values.len();
                    let gap = if n > 16 { 2.0 } else { 3.5 };
                    let bar_w = ((bw - gap * (n.saturating_sub(1) as f32)) / n as f32).max(2.0);
                    let bot_y = f32::from(bounds.bottom());
                    for (i, &v) in values.iter().enumerate() {
                        let h_px = ((v.max(0.0) / max_v) * (bh - 2.0)).max(2.0);
                        let x0 = f32::from(bounds.left()) + i as f32 * (bar_w + gap);
                        let y0 = bot_y - h_px;
                        let dimmed = hovered_idx.is_some_and(|hi| hi != i);
                        let alpha = if dimmed { 0.36 } else { 0.88 };
                        let r = px(2.5f32.min(bar_w * 0.5).min(h_px * 0.5));
                        window.paint_quad(
                            fill(
                                Bounds::new(point(px(x0), px(y0)), size(px(bar_w), px(h_px))),
                                color.opacity(alpha),
                            )
                            .corner_radii(r),
                        );
                    }
                    return;
                }

                if values.len() < 2 {
                    return;
                }
                let mut min_v = f32::INFINITY;
                let mut max_v = f32::NEG_INFINITY;
                for &v in &values {
                    min_v = min_v.min(v);
                    max_v = max_v.max(v);
                }
                let span = (max_v - min_v).max(1e-3);
                let pad_y = 4.0f32;
                let map_pt = |i: usize, v: f32| -> Point<Pixels> {
                    let tx = i as f32 / (values.len() - 1) as f32;
                    let ty = 1.0 - ((v - min_v) / span).clamp(0.0, 1.0);
                    point(
                        bounds.left() + px(tx * bw),
                        bounds.top() + px(pad_y + ty * (bh - 2.0 * pad_y)),
                    )
                };

                if kind == StatSparkKind::Area {
                    let mut area = PathBuilder::fill();
                    let first = map_pt(0, values[0]);
                    let last = map_pt(values.len() - 1, values[values.len() - 1]);
                    area.move_to(point(first.x, bounds.bottom()));
                    area.line_to(first);
                    for (i, &v) in values.iter().enumerate().skip(1) {
                        area.line_to(map_pt(i, v));
                    }
                    area.line_to(point(last.x, bounds.bottom()));
                    area.close();
                    if let Ok(path) = area.build() {
                        window.paint_path(
                            path,
                            linear_gradient(
                                180.,
                                linear_color_stop(
                                    color.opacity(if dark { 0.22 } else { 0.16 }),
                                    0.,
                                ),
                                linear_color_stop(color.opacity(0.0), 1.),
                            ),
                        );
                    }
                }

                let mut line = PathBuilder::stroke(px(1.6));
                for (i, &v) in values.iter().enumerate() {
                    let pt = map_pt(i, v);
                    if i == 0 {
                        line.move_to(pt);
                    } else {
                        line.line_to(pt);
                    }
                }
                if let Ok(path) = line.build() {
                    window.paint_path(path, color);
                }

                if let Some(hi) = hovered_idx
                    && hi < values.len()
                {
                    let pt = map_pt(hi, values[hi]);
                    window.paint_quad(
                        fill(
                            Bounds::new(
                                point(pt.x - px(4.5), pt.y - px(4.5)),
                                size(px(9.), px(9.)),
                            ),
                            card_bg,
                        )
                        .corner_radii(px(99.)),
                    );
                    window.paint_quad(
                        fill(
                            Bounds::new(point(pt.x - px(3.), pt.y - px(3.)), size(px(6.), px(6.))),
                            color,
                        )
                        .corner_radii(px(99.)),
                    );
                }
            },
        )
        .w_full()
        .h(px(40.))
    }
}

fn build_showcase_cards(range: &str) -> Vec<StatCardItem> {
    if range == "30d" {
        let days: Vec<String> = (1..=14).map(|d| format!("Feb {}", d * 2)).collect();
        vec![
            StatCardItem::new("spend", "Spend", 11_420.0)
                .with_format(1_000.0, 1, "$", "k")
                .with_change(21.8, "vs last month")
                .with_spark(
                    StatSparkSpec::new(
                        StatSparkKind::Area,
                        days.clone(),
                        vec![
                            320., 345., 310., 380., 410., 395., 430., 405., 450., 472., 440., 488.,
                            512., 540.,
                        ],
                    )
                    .with_format(1.0, 0, "$", ""),
                ),
            StatCardItem::new("tokens", "Tokens", 1.08)
                .with_format(1.0, 2, "", "B")
                .with_change(14.4, "vs last month")
                .with_spark(
                    StatSparkSpec::new(
                        StatSparkKind::Bars,
                        days.clone(),
                        vec![
                            28., 32., 35., 30., 38., 41., 36., 44., 40., 46., 49., 45., 52., 55.,
                        ],
                    )
                    .with_format(1.0, 1, "", "M"),
                ),
            StatCardItem::new("latency", "Avg. latency", 1.24)
                .with_format(1.0, 2, "", "s")
                .with_change(-4.2, "vs last month")
                .with_lower_is_better(true)
                .with_spark(
                    StatSparkSpec::new(
                        StatSparkKind::Area,
                        days.clone(),
                        vec![
                            1.36, 1.34, 1.31, 1.33, 1.29, 1.28, 1.30, 1.26, 1.25, 1.27, 1.23, 1.22,
                            1.21, 1.19,
                        ],
                    )
                    .with_format(1.0, 2, "", "s"),
                ),
            StatCardItem::new("errors", "Error rate", 0.29)
                .with_format(1.0, 2, "", "%")
                .with_change(-8.6, "vs last month")
                .with_lower_is_better(true)
                .with_spark(
                    StatSparkSpec::new(
                        StatSparkKind::Area,
                        days,
                        vec![
                            0.38, 0.36, 0.35, 0.39, 0.34, 0.33, 0.31, 0.32, 0.29, 0.30, 0.28, 0.27,
                            0.26, 0.25,
                        ],
                    )
                    .with_format(1.0, 2, "", "%"),
                ),
        ]
    } else {
        let days = [
            "Mar 3", "Mar 4", "Mar 5", "Mar 6", "Mar 7", "Mar 8", "Mar 9",
        ];
        vec![
            StatCardItem::new("spend", "Spend", 2_741.0)
                .with_format(1_000.0, 1, "$", "k")
                .with_change(16.5, "vs last week")
                .with_spark(
                    StatSparkSpec::new(
                        StatSparkKind::Area,
                        days,
                        vec![435., 412., 365., 402., 335., 318., 474.],
                    )
                    .with_format(1.0, 0, "$", ""),
                ),
            StatCardItem::new("tokens", "Tokens", 257.7)
                .with_format(1.0, 1, "", "M")
                .with_change(8.2, "vs last week")
                .with_spark(
                    StatSparkSpec::new(
                        StatSparkKind::Bars,
                        days,
                        vec![34.2, 38.5, 31.8, 36.4, 40.1, 35.6, 41.1],
                    )
                    .with_format(1.0, 1, "", "M"),
                ),
            StatCardItem::new("latency", "Avg. latency", 1.18)
                .with_format(1.0, 2, "", "s")
                .with_change(-6.1, "vs last week")
                .with_lower_is_better(true)
                .with_spark(
                    StatSparkSpec::new(
                        StatSparkKind::Area,
                        days,
                        vec![1.28, 1.24, 1.16, 1.18, 1.22, 1.19, 1.14],
                    )
                    .with_format(1.0, 2, "", "s"),
                ),
            StatCardItem::new("errors", "Error rate", 0.35)
                .with_format(1.0, 2, "", "%")
                .with_change(12.3, "vs last week")
                .with_lower_is_better(true)
                .with_spark(
                    StatSparkSpec::new(
                        StatSparkKind::Area,
                        days,
                        vec![0.31, 0.28, 0.30, 0.35, 0.33, 0.37, 0.41],
                    )
                    .with_format(1.0, 2, "", "%"),
                ),
        ]
    }
}

/// KPI stat card and synchronized 4-card KPI grid (`stat-card`).
pub struct StatCard {
    id: ElementId,
    showcase_mode: bool,
    showcase_title: SharedString,
    active_range: SharedString,
    single_item: StatCardItem,
    selected_single: Option<bool>,
    selected_showcase_id: Option<SharedString>,
    hovered_spark: Option<(SharedString, usize)>,
    spark_bounds: Vec<Rc<Cell<Option<Bounds<Pixels>>>>>,
    loading: bool,
    empty: bool,
    focus: FocusHandle,
    toast: Option<(SharedString, Instant)>,
    started_at: Instant,
}

impl StatCard {
    /// Create a [`StatCard`] displaying the canonical 4-card `"Usage"` (`7D` / `30D`) KPI showcase;
    /// calling `.with_label(...)`, `.with_value(...)`, or [`StatCard::single`] configures standalone single-card mode.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let single_item = build_showcase_cards("7d")
            .into_iter()
            .next()
            .unwrap_or_else(|| StatCardItem::new("stat", "Spend", 2_741.0));
        Self {
            id: id.into(),
            showcase_mode: true,
            showcase_title: "Usage".into(),
            active_range: "7d".into(),
            single_item,
            selected_single: None,
            selected_showcase_id: None,
            hovered_spark: None,
            spark_bounds: (0..4).map(|_| Rc::new(Cell::new(None))).collect(),
            loading: false,
            empty: false,
            focus: cx.focus_handle().tab_stop(true),
            toast: None,
            started_at: Instant::now(),
        }
    }

    /// Create a standalone single [`StatCard`] with `label` and `value`.
    pub fn single(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        value: f64,
        cx: &mut Context<Self>,
    ) -> Self {
        let id = id.into();
        let mut this = Self::new(id.clone(), cx);
        this.showcase_mode = false;
        this.single_item = StatCardItem::new("single", label, value);
        this
    }

    /// Switch into standalone single-card mode and set the metric label.
    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.showcase_mode = false;
        self.single_item.label = label.into();
        self
    }

    /// Switch into standalone single-card mode and set the numeric rolling value.
    pub fn with_value(mut self, value: f64) -> Self {
        self.showcase_mode = false;
        self.single_item.value = value;
        self.single_item.text_override = None;
        self
    }

    /// Switch into standalone single-card mode and display a literal string value without numeric rolling.
    pub fn with_text_value(mut self, value: impl Into<SharedString>) -> Self {
        self.showcase_mode = false;
        self.single_item.text_override = Some(value.into());
        self
    }

    /// Configure numeric formatting for the standalone card.
    pub fn with_format(
        mut self,
        divisor: f64,
        decimals: usize,
        prefix: impl Into<SharedString>,
        suffix: impl Into<SharedString>,
    ) -> Self {
        self.showcase_mode = false;
        self.single_item = self
            .single_item
            .with_format(divisor, decimals, prefix, suffix);
        self
    }

    /// Configure percentage delta and comparison phrase on the standalone card.
    pub fn with_change(mut self, pct: f64, vs_label: impl Into<SharedString>) -> Self {
        self.showcase_mode = false;
        self.single_item = self.single_item.with_change(pct, vs_label);
        self
    }

    /// Mark negative deltas as green improvements (latency, churn, error rate).
    pub fn with_lower_is_better(mut self, lower_is_better: bool) -> Self {
        self.showcase_mode = false;
        self.single_item.lower_is_better = lower_is_better;
        self
    }

    /// Set a leading `16px` icon before the label on the standalone card.
    pub fn with_icon(mut self, icon: ProIcon) -> Self {
        self.showcase_mode = false;
        self.single_item.icon = Some(icon);
        self
    }

    /// Attach a mini sparkline or bar strip to the standalone card.
    pub fn with_spark(mut self, spark: StatSparkSpec) -> Self {
        self.showcase_mode = false;
        self.single_item.spark = Some(spark);
        self
    }

    /// Arrange the mini chart `Stacked` below or `Inline` beside the figure.
    pub fn with_layout(mut self, layout: StatCardLayout) -> Self {
        self.showcase_mode = false;
        self.single_item.layout = layout;
        self
    }

    /// Set a footer caption on the standalone card.
    pub fn with_footer(mut self, footer: impl Into<SharedString>) -> Self {
        self.showcase_mode = false;
        self.single_item = self.single_item.with_footer(footer);
        self
    }

    /// Turn the standalone card into an interactive link target.
    pub fn with_href(mut self, href: impl Into<SharedString>) -> Self {
        self.showcase_mode = false;
        self.single_item = self.single_item.with_href(href);
        self
    }

    /// Enable or disable whole-card click/button interactivity on the standalone card.
    pub fn with_interactive(mut self, interactive: bool) -> Self {
        self.showcase_mode = false;
        self.single_item.interactive = interactive;
        self
    }

    /// Control selected/dimmed state when multiple standalone `StatCard`s act as tabs (`Some(false)` dims to `0.45`).
    pub fn with_selected(mut self, selected: Option<bool>) -> Self {
        self.selected_single = selected;
        self
    }

    /// Render skeleton placeholders while keeping the card's exact footprint.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Render the empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the standalone item specification.
    pub fn item(&self) -> &StatCardItem {
        &self.single_item
    }

    fn render_one_tile(
        &self,
        idx: usize,
        item: &StatCardItem,
        dimmed: bool,
        phase: f32,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
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
        let card_hover_border: Hsla = if dark {
            hsla(0., 0., 1., 0.18)
        } else {
            hsla(0., 0., 0., 0.13)
        };
        let line: Hsla = if dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.06)
        };

        let tile_id = pro_eid(&self.id, format!("tile-{}", item.id));
        let dim_alpha = pro_spring_f32(
            pro_eid(&tile_id, "dim"),
            if dimmed { 0.46 } else { 1.0 },
            0.24,
            w,
            cx,
        );

        let label_row = div()
            .flex()
            .items_center()
            .gap(px(6.))
            .min_w_0()
            .when_some(item.icon, |d, ic| d.child(pro_icon(ic, 15., p.muted)))
            .child(
                div()
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.text)
                    .truncate()
                    .child(item.label.clone()),
            );

        if self.loading {
            return div()
                .id(tile_id)
                .w_full()
                .rounded(px(10.))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .px(px(16.))
                .pt(px(14.))
                .pb(px(16.))
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(label_row)
                .child(pro_skeleton_bar(96., 28., 5., phase, cx))
                .child(pro_skeleton_bar(124., 18., 5., phase, cx))
                .when(item.spark.is_some(), |d| {
                    d.child(pro_skeleton_bar(240., 40., 5., phase, cx))
                })
                .into_any_element();
        }

        let figure_el: AnyElement = if let Some(txt) = &item.text_override {
            div()
                .mt(px(2.))
                .text_size(px(24.))
                .line_height(px(32.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.strong)
                .child(txt.clone())
                .into_any_element()
        } else {
            div()
                .mt(px(2.))
                .child(pro_rolling_number(
                    pro_eid(&tile_id, "val"),
                    item.value / item.divisor,
                    item.decimals,
                    item.prefix.clone(),
                    item.suffix.clone(),
                    24.,
                    FontWeight::MEDIUM,
                    p.strong,
                ))
                .into_any_element()
        };

        let meta_row: Option<AnyElement> = item.change.map(|chg| {
            let good = item.is_change_good();
            let (cbg, cfg): (Hsla, Hsla) = match good {
                Some(true) => {
                    if dark {
                        (hsla(145. / 360., 0.46, 0.65, 0.13), rgb(0x5fc88a).into())
                    } else {
                        (hsla(145. / 360., 0.49, 0.58, 0.10), rgb(0x1e8a4f).into())
                    }
                }
                Some(false) => {
                    if dark {
                        (hsla(4. / 360., 1.0, 0.69, 0.13), rgb(0xff8a7a).into())
                    } else {
                        (hsla(4. / 360., 0.86, 0.58, 0.10), rgb(0xc4320a).into())
                    }
                }
                None => (
                    if dark {
                        hsla(0., 0., 1., 0.08)
                    } else {
                        hsla(0., 0., 0., 0.05)
                    },
                    p.text,
                ),
            };
            let arrow: Option<AnyElement> = if chg.abs() < 1e-6 {
                None
            } else if chg > 0.0 {
                Some(pro_svg("0 0 8 8", DELTA_TRI_UP_SVG, 7., cfg))
            } else {
                Some(pro_svg("0 0 8 8", DELTA_TRI_DOWN_SVG, 7., cfg))
            };

            div()
                .mt(px(4.))
                .flex()
                .items_center()
                .gap(px(6.))
                .min_w_0()
                .child(
                    div()
                        .h(px(20.))
                        .px(px(6.))
                        .flex_none()
                        .rounded(px(5.))
                        .bg(cbg)
                        .text_color(cfg)
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .when_some(arrow, |d, ar| d.child(ar))
                        .child(pro_rolling_number(
                            pro_eid(&tile_id, "chg"),
                            chg.abs(),
                            1,
                            "",
                            "%",
                            12.,
                            FontWeight::MEDIUM,
                            cfg,
                        )),
                )
                .when_some(item.change_label.clone(), |d, vs| {
                    d.child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.muted)
                            .truncate()
                            .child(vs),
                    )
                })
                .into_any_element()
        });

        let spark_el: Option<AnyElement> = item.spark.clone().map(|spec| {
            let sink = self
                .spark_bounds
                .get(idx)
                .cloned()
                .unwrap_or_else(|| Rc::new(Cell::new(None)));
            let cid = item.id.clone();
            let hovered_pt = self
                .hovered_spark
                .as_ref()
                .and_then(|(id, hi)| (id == &cid).then_some(*hi));
            let val_count = spec.values.len();

            let tag_el: Option<AnyElement> = hovered_pt.and_then(|hi| {
                let raw_v = *spec.values.get(hi)?;
                let lbl = spec.labels.get(hi).cloned().unwrap_or_default();
                let formatted = format!(
                    "{}{:.*}{}",
                    spec.prefix,
                    spec.decimals,
                    raw_v / spec.divisor,
                    spec.suffix
                );
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
                let bw = sink.get().map(|b| f32::from(b.size.width)).unwrap_or(260.);
                let frac = hi as f32 / (val_count.saturating_sub(1)).max(1) as f32;
                let tag_w = 88.0f32;
                let left_px = (frac * bw - tag_w * 0.5).clamp(0.0, (bw - tag_w).max(0.0));
                Some(
                    div()
                        .absolute()
                        .top(px(-22.))
                        .left(px(left_px))
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
                        .child(div().font_weight(FontWeight::MEDIUM).child(formatted))
                        .child(div().opacity(0.68).child(lbl))
                        .into_any_element(),
                )
            });

            let sink_move = sink.clone();
            let cid_move = cid.clone();
            let cid_leave = cid.clone();
            div()
                .id(pro_eid(&tile_id, "spark-hit"))
                .w_full()
                .h(px(40.))
                .relative()
                .cursor(CursorStyle::Crosshair)
                .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, _, cx| {
                    if val_count == 0 {
                        return;
                    }
                    let Some(b) = sink_move.get() else {
                        return;
                    };
                    let bw = f32::from(b.size.width).max(1.0);
                    let rx = f32::from(ev.position.x - b.left()).clamp(0.0, bw);
                    let hi = if val_count <= 1 {
                        0
                    } else {
                        ((rx / bw) * (val_count - 1) as f32).round() as usize
                    }
                    .min(val_count - 1);
                    let next = Some((cid_move.clone(), hi));
                    if this.hovered_spark != next {
                        this.hovered_spark = next;
                        cx.emit(StatCardEvent::SparkHoverChanged {
                            card_id: cid_move.clone(),
                            index: Some(hi),
                        });
                        cx.notify();
                    }
                }))
                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                    if !*inside
                        && this
                            .hovered_spark
                            .as_ref()
                            .is_some_and(|(id, _)| id == &cid_leave)
                    {
                        this.hovered_spark = None;
                        cx.emit(StatCardEvent::SparkHoverChanged {
                            card_id: cid_leave.clone(),
                            index: None,
                        });
                        cx.notify();
                    }
                }))
                .child(StatSparkCanvas {
                    id: pro_eid(&tile_id, "canvas"),
                    spec,
                    hovered_idx: hovered_pt,
                    card_bg,
                    bounds_sink: sink,
                })
                .when_some(tag_el, |d, t| d.child(t))
                .into_any_element()
        });

        let text_block = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(label_row)
            .child(figure_el)
            .when_some(meta_row, |d, m| d.child(m));

        let body_el = match (item.layout, spark_el) {
            (StatCardLayout::Inline, Some(sp)) => div()
                .px(px(16.))
                .pt(px(14.))
                .pb(px(16.))
                .opacity(dim_alpha)
                .flex()
                .items_end()
                .gap(px(16.))
                .child(text_block)
                .child(div().w(relative(0.44)).min_w(px(96.)).child(sp)),
            (_, maybe_sp) => div()
                .px(px(16.))
                .pt(px(14.))
                .pb(px(16.))
                .opacity(dim_alpha)
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(text_block)
                .when_some(maybe_sp, |d, sp| d.child(sp)),
        };

        let item_id = item.id.clone();
        let item_href = item.href.clone();
        let item_label = item.label.clone();
        let interactive = item.interactive || item.href.is_some();

        let mut card = div()
            .id(tile_id)
            .w_full()
            .rounded(px(10.))
            .bg(card_bg)
            .border_1()
            .border_color(card_border)
            .when(!dark, |d| d.shadow_xs())
            .flex()
            .flex_col();

        if interactive {
            let role = if item.href.is_some() {
                Role::Link
            } else {
                Role::Button
            };
            card = card
                .role(role)
                .aria_label(item_label.clone())
                .tab_index(0)
                .cursor_pointer()
                .hover(move |s| s.border_color(card_hover_border))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if this.showcase_mode {
                        if this.selected_showcase_id.as_ref() == Some(&item_id) {
                            this.selected_showcase_id = None;
                        } else {
                            this.selected_showcase_id = Some(item_id.clone());
                        }
                    } else if let Some(h) = &item_href {
                        this.toast = Some((
                            format!("{} {h}", tr(cx, "Opened", "已打开")).into(),
                            Instant::now(),
                        ));
                    }
                    cx.emit(StatCardEvent::Activated {
                        id: item_id.clone(),
                        href: item_href.clone(),
                    });
                    cx.notify();
                }));
        }

        card.child(body_el)
            .when_some(item.footer.clone(), |d, ft| {
                d.child(
                    div()
                        .mt_auto()
                        .px(px(16.))
                        .py(px(10.))
                        .border_t_1()
                        .border_color(line)
                        .opacity(dim_alpha)
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(p.muted)
                        .child(ft),
                )
            })
            .into_any_element()
    }
}

impl Render for StatCard {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let dark = p.dark;
        let phase = if self.loading {
            if disabled_motion(cx) {
                0.25
            } else {
                w.request_animation_frame();
                (self.started_at.elapsed().as_secs_f32() / 1.4).fract()
            }
        } else {
            0.0
        };

        if self.empty {
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
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(if self.showcase_mode { 640. } else { 320. }))
                .rounded(px(10.))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .child(pro_empty_state::<Self>(
                    ProIcon::UsageChart,
                    tr(cx, "No metric available", "暂无指标数据"),
                    tr(
                        cx,
                        "Connect a metric feed to display headline figures and period deltas.",
                        "接入指标数据源后将在此展示核心数值与环比涨跌。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        if !self.showcase_mode {
            let item = self.single_item.clone();
            let dimmed = self.selected_single == Some(false);
            return div()
                .id(self.id.clone())
                .track_focus(&self.focus)
                .w_full()
                .max_w(px(340.))
                .relative()
                .child(self.render_one_tile(0, &item, dimmed, phase, w, cx))
                .child(pro_toast_banner(self.toast.clone()))
                .into_any_element();
        }

        let sensor = pro_selection_sensor();
        let pill_fill: Hsla = if dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let mut range_seg = div()
            .relative()
            .p(px(2.))
            .rounded(px(7.))
            .bg(if dark {
                hsla(0., 0., 1., 0.04)
            } else {
                hsla(0., 0., 0., 0.03)
            })
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "rng-pill"),
                self.active_range.clone(),
                sensor.clone(),
                5.5,
                pill_fill,
                None,
                false,
            ));

        for (rid, rlbl) in [("7d", "7D"), ("30d", "30D")] {
            let selected = self.active_range.as_ref() == rid;
            range_seg = range_seg.child(
                div()
                    .id(pro_eid(&self.id, format!("rng-{rid}")))
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
                    .when(selected, |d| d.child(pro_mark_selected(&sensor)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.active_range.as_ref() != rid {
                            this.active_range = rid.into();
                            this.hovered_spark = None;
                            cx.emit(StatCardEvent::RangeChanged(rid.into()));
                            cx.notify();
                        }
                    }))
                    .child(rlbl),
            );
        }

        let cards = build_showcase_cards(&self.active_range);
        let mut tiles = Vec::with_capacity(cards.len());
        for (i, item) in cards.iter().enumerate() {
            let dimmed = self
                .selected_showcase_id
                .as_ref()
                .is_some_and(|sel| sel != &item.id);
            tiles.push(self.render_one_tile(i, item, dimmed, phase, w, cx));
        }

        let mut iter = tiles.into_iter();
        let row1_a = iter.next().unwrap_or_else(|| div().into_any_element());
        let row1_b = iter.next().unwrap_or_else(|| div().into_any_element());
        let row2_a = iter.next().unwrap_or_else(|| div().into_any_element());
        let row2_b = iter.next().unwrap_or_else(|| div().into_any_element());

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .w_full()
            .max_w(px(640.))
            .relative()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(self.showcase_title.clone()),
                    )
                    .child(range_seg),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_wrap()
                    .gap(px(12.))
                    .child(div().flex_1().min_w(px(240.)).child(row1_a))
                    .child(div().flex_1().min_w(px(240.)).child(row1_b)),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_wrap()
                    .gap(px(12.))
                    .child(div().flex_1().min_w(px(240.)).child(row2_a))
                    .child(div().flex_1().min_w(px(240.)).child(row2_b)),
            )
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{StatCard, StatCardItem, StatCardLayout};
    use gpui::{AppContext, TestAppContext};

    #[test]
    fn lower_is_better_inverts_good_and_bad_delta_semantics() {
        let spend = StatCardItem::new("spend", "Spend", 2741.0).with_change(16.5, "vs last week");
        assert_eq!(spend.is_change_good(), Some(true));

        let latency = StatCardItem::new("lat", "Avg. latency", 1.18)
            .with_change(-6.1, "vs last week")
            .with_lower_is_better(true);
        assert_eq!(latency.is_change_good(), Some(true));

        let errors = StatCardItem::new("err", "Error rate", 0.35)
            .with_change(12.3, "vs last week")
            .with_lower_is_better(true);
        assert_eq!(errors.is_change_good(), Some(false));
    }

    #[gpui::test]
    fn standalone_single_card_supports_inline_layout_and_link_mode(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let card = cx.new(|cx| {
            StatCard::single("spend-card", "Spend", 2_741.0, cx)
                .with_format(1_000.0, 1, "$", "k")
                .with_change(16.5, "vs last week")
                .with_layout(StatCardLayout::Inline)
                .with_href("/analytics/spend")
        });
        card.update(cx, |sc, _| {
            assert_eq!(sc.item().label.as_ref(), "Spend");
            assert_eq!(sc.item().layout, StatCardLayout::Inline);
            assert_eq!(sc.item().href.as_deref(), Some("/analytics/spend"));
        });
    }
}
