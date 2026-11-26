//! Compact ranked breakdown card with period pill switcher, rolling total, and position-trading rows.
//!
//! Splits a headline metric into ranked parts across selectable periods (`7d`, `30d`, `90d`),
//! rolling the headline readout (`pro_rolling_number`), spring-morphing horizontal bar segments
//! (`pro_spring_f32`), and smoothly trading vertical row positions with `pro_tactile` selection.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_card_shadow, pro_ease, pro_eid, pro_empty_state, pro_mark_selected,
        pro_mix_color, pro_pill_indicator, pro_rolling_number, pro_selection_sensor,
        pro_skeleton_bar, pro_spring_f32, pro_swap_label, pro_tactile, pro_view_transition, tr,
    },
    primitives::{
        control::activate,
        family::{PhaseClock, Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};

const ROW_HEIGHT: f32 = 32.0;

/// Selectable time period option in [`CompactBreakdown`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompactPeriodOption {
    /// Period identifier (`"7d"`, `"30d"`, `"90d"`).
    pub id: SharedString,
    /// Short button label.
    pub label: SharedString,
}

impl CompactPeriodOption {
    /// Create a [`CompactPeriodOption`].
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

/// One constituent part of a period's total in [`CompactBreakdown`].
#[derive(Clone, Debug, PartialEq)]
pub struct CompactPart {
    /// Stable identifier across periods so bar segments and ranked rows animate smoothly.
    pub id: SharedString,
    /// English display label.
    pub label: SharedString,
    /// Localized Chinese display label.
    pub label_zh: SharedString,
    /// Raw numeric value for this period.
    pub value: f64,
    /// Optional explicit swatch color override.
    pub color: Option<Hsla>,
}

impl CompactPart {
    /// Create a [`CompactPart`].
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>, value: f64) -> Self {
        let label = label.into();
        Self {
            id: id.into(),
            label_zh: label.clone(),
            label,
            value: if value.is_finite() {
                value.max(0.0)
            } else {
                0.0
            },
            color: None,
        }
    }

    /// Set localized Chinese label.
    pub fn with_label_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.label_zh = zh.into();
        self
    }

    /// Override swatch & bar segment color.
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

/// Dataset for one selectable period inside [`CompactBreakdown`].
#[derive(Clone, Debug, PartialEq)]
pub struct CompactPeriodData {
    /// Constituent items for this period (automatically ranked descending by `value`).
    pub items: Vec<CompactPart>,
    /// Optional percentage delta vs. prior period (`+8.1` or `-2.6`).
    pub delta_pct: Option<f64>,
}

impl CompactPeriodData {
    /// Construct a [`CompactPeriodData`] slice.
    pub fn new(items: Vec<CompactPart>, delta_pct: Option<f64>) -> Self {
        Self { items, delta_pct }
    }

    /// Sum of finite positive item values.
    pub fn total(&self) -> f64 {
        self.items
            .iter()
            .map(|i| {
                if i.value.is_finite() {
                    i.value.max(0.0)
                } else {
                    0.0
                }
            })
            .sum()
    }
}

/// Typed events emitted by [`CompactBreakdown`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompactBreakdownEvent {
    /// Active period changed (`"7d"`, `"30d"`, `"90d"`).
    PeriodChanged(SharedString),
    /// User clicked or focused a ranked part row or bar segment.
    PartSelected {
        period: SharedString,
        part_id: SharedString,
    },
}

/// Compact part-of-whole breakdown card with gliding bar and position-trading ranked rows.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::compact_breakdown::{CompactBreakdown, CompactBreakdownEvent};
///
/// let card = cx.new(|cx| {
///     CompactBreakdown::new("channel-breakdown", cx)
///         .with_title("Revenue by channel")
///         .with_default_period("30d")
/// });
/// cx.subscribe(&card, |_, event: &CompactBreakdownEvent, _| {
///     let _ = event;
/// }).detach();
/// # }
/// ```
pub struct CompactBreakdown {
    id: ElementId,
    title: SharedString,
    title_zh: SharedString,
    total_label: SharedString,
    total_label_zh: SharedString,
    value_prefix: SharedString,
    value_suffix: SharedString,
    decimals: usize,
    show_footer: bool,
    periods: Vec<CompactPeriodOption>,
    active_period: SharedString,
    datasets: HashMap<SharedString, CompactPeriodData>,
    canonical_order: Vec<SharedString>,
    hovered_part: Option<SharedString>,
    selected_part: Option<SharedString>,
    part_spotlights: HashMap<SharedString, (Slide, Slide)>,
    loading: bool,
    empty: bool,
    width_cell: Rc<Cell<f32>>,
    focus: FocusHandle,
}

impl EventEmitter<CompactBreakdownEvent> for CompactBreakdown {}

impl CompactBreakdown {
    /// Create a [`CompactBreakdown`] seeded with `7d`, `30d`, and `90d` channel revenue datasets.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let periods = vec![
            CompactPeriodOption::new("7d", "7d"),
            CompactPeriodOption::new("30d", "30d"),
            CompactPeriodOption::new("90d", "90d"),
        ];
        let datasets = default_channel_datasets();
        let canonical_order = vec![
            "direct".into(),
            "organic".into(),
            "referrals".into(),
            "social".into(),
            "email".into(),
        ];
        Self {
            id: id.into(),
            title: "Revenue by channel".into(),
            title_zh: "各渠道收入构成".into(),
            total_label: "Total revenue".into(),
            total_label_zh: "总计收入".into(),
            value_prefix: "$".into(),
            value_suffix: "".into(),
            decimals: 0,
            show_footer: true,
            periods,
            active_period: "7d".into(),
            datasets,
            canonical_order,
            hovered_part: None,
            selected_part: None,
            part_spotlights: HashMap::new(),
            loading: false,
            empty: false,
            width_cell: Rc::new(Cell::new(360.)),
            focus: cx.focus_handle(),
        }
    }

    /// Customize card title.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        let t = title.into();
        self.title_zh = t.clone();
        self.title = t;
        self
    }

    /// Customize the caption under the big total readout.
    pub fn with_total_label(mut self, label: impl Into<SharedString>) -> Self {
        let l = label.into();
        self.total_label_zh = l.clone();
        self.total_label = l;
        self
    }

    /// Configure numeric prefix, suffix, and decimal places (`"$", "", 0` by default).
    pub fn with_number_format(
        mut self,
        prefix: impl Into<SharedString>,
        suffix: impl Into<SharedString>,
        decimals: usize,
    ) -> Self {
        self.value_prefix = prefix.into();
        self.value_suffix = suffix.into();
        self.decimals = decimals;
        self
    }

    /// Set initial active period id (`"7d"`, `"30d"`, `"90d"`).
    pub fn with_default_period(mut self, period_id: impl Into<SharedString>) -> Self {
        let pid = period_id.into();
        if self.periods.iter().any(|p| p.id == pid) {
            self.active_period = pid;
        }
        self
    }

    /// Supply custom period options and datasets.
    pub fn with_datasets(
        mut self,
        periods: Vec<CompactPeriodOption>,
        datasets: HashMap<SharedString, CompactPeriodData>,
    ) -> Self {
        let mut order = Vec::new();
        for p in &periods {
            if let Some(ds) = datasets.get(&p.id) {
                for item in &ds.items {
                    if !order.contains(&item.id) {
                        order.push(item.id.clone());
                    }
                }
            }
        }
        if let Some(first) = periods.first() {
            self.active_period = first.id.clone();
        }
        self.periods = periods;
        self.datasets = datasets;
        self.selected_part = None;
        if !order.is_empty() {
            self.canonical_order = order;
        }
        self
    }

    /// Show or hide the `"Largest: <part>"` footer tray.
    pub fn with_footer(mut self, show: bool) -> Self {
        self.show_footer = show;
        self
    }

    /// Toggle skeleton loading preview state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle forced empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return current active period id.
    pub fn active_period(&self) -> &str {
        &self.active_period
    }

    /// Switch active period by id.
    pub fn set_period(&mut self, period_id: &str, cx: &mut Context<Self>) -> bool {
        let Some(found) = self.periods.iter().find(|p| p.id.as_ref() == period_id) else {
            return false;
        };
        if self.active_period == found.id {
            return true;
        }
        self.active_period = found.id.clone();
        cx.emit(CompactBreakdownEvent::PeriodChanged(
            self.active_period.clone(),
        ));
        cx.notify();
        true
    }

    fn toggle_select_part(&mut self, part_id: SharedString, cx: &mut Context<Self>) {
        if self.selected_part.as_ref() == Some(&part_id) {
            self.selected_part = None;
        } else {
            self.selected_part = Some(part_id.clone());
        }
        cx.emit(CompactBreakdownEvent::PartSelected {
            period: self.active_period.clone(),
            part_id,
        });
        cx.notify();
    }

    fn active_spotlight_part(&self) -> Option<&SharedString> {
        self.hovered_part.as_ref().or(self.selected_part.as_ref())
    }

    /// Return items for the active period sorted descending by `value` (tie-broken by canonical order).
    pub fn ranked_items(&self) -> Vec<CompactPart> {
        let Some(ds) = self.datasets.get(&self.active_period) else {
            return Vec::new();
        };
        let mut items = ds.items.clone();
        items.sort_by(|a, b| {
            b.value
                .partial_cmp(&a.value)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    let ia = self
                        .canonical_order
                        .iter()
                        .position(|id| id == &a.id)
                        .unwrap_or(usize::MAX);
                    let ib = self
                        .canonical_order
                        .iter()
                        .position(|id| id == &b.id)
                        .unwrap_or(usize::MAX);
                    ia.cmp(&ib)
                })
        });
        items
    }

    /// Return total for the active period.
    pub fn current_total(&self) -> f64 {
        self.datasets
            .get(&self.active_period)
            .map(CompactPeriodData::total)
            .unwrap_or(0.0)
    }

    fn part_color(&self, part: &CompactPart, p: &ProPalette) -> Hsla {
        if let Some(c) = part.color {
            return c;
        }
        let slot = self
            .canonical_order
            .iter()
            .position(|id| id == &part.id)
            .unwrap_or(0);
        let ramp: [Hsla; 5] = if p.dark {
            [
                p.blue,
                p.blue.opacity(0.68),
                p.blue.opacity(0.45),
                p.blue.opacity(0.26),
                hsla(0., 0., 1., 0.18),
            ]
        } else {
            [
                rgb(0x0068dc).into(),
                rgba(0x0060e29e).into(),
                rgba(0x0060e266).into(),
                rgba(0x0060e236).into(),
                hsla(0., 0., 0., 0.16),
            ]
        };
        ramp[slot % ramp.len()]
    }
}

impl Render for CompactBreakdown {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let width_cell = self.width_cell.clone();
        let very_narrow = width_cell.get() < 260.;

        let sensor = canvas(
            move |bounds, _, _| {
                let bw = f32::from(bounds.size.width);
                if bw > 10. {
                    width_cell.set(bw);
                }
            },
            |_, _, _, _| (),
        )
        .w_full()
        .h(px(0.));

        if self.loading {
            let phase = if disabled_motion(cx) { 0.3 } else { 0.5 };
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(380.))
                .mx_auto()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(pro_skeleton_bar(180., 16., 4., phase, cx))
                .child(pro_skeleton_bar(130., 26., 5., phase, cx))
                .child(pro_skeleton_bar(320., 6., 3., phase, cx))
                .child(pro_skeleton_bar(320., 28., 5., phase, cx))
                .child(pro_skeleton_bar(320., 28., 5., phase, cx))
                .child(pro_skeleton_bar(320., 28., 5., phase, cx))
                .into_any_element();
        }

        let ranked = self.ranked_items();
        if self.empty || ranked.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(380.))
                .mx_auto()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::UsageChart,
                    tr(cx, "No breakdown data", "暂无构成拆分数据"),
                    tr(
                        cx,
                        "Provide channel or category parts to compare shares.",
                        "传入各分项数值以查看占比条与排名。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-seed"),
                        SharedString::from(tr(cx, "Load channel data", "加载示例数据")),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            if this.datasets.is_empty() {
                                this.datasets = default_channel_datasets();
                            }
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        let total = self.current_total();
        let active_ds = self.datasets.get(&self.active_period);
        let delta_pct = active_ds.and_then(|d| d.delta_pct);

        let heading = tr(cx, self.title.as_ref(), self.title_zh.as_ref()).to_string();

        // Period pill switcher (`7d`, `30d`, `90d` with liquid `pro_pill_indicator`)
        let period_sensor = pro_selection_sensor();
        let pill_fill: Hsla = if p.dark { rgb(0x28282c).into() } else { p.card };
        let period_switch = if self.periods.is_empty() {
            div().into_any_element()
        } else {
            let mut strip = div()
                .p(px(2.))
                .rounded(px(7.))
                .bg(p.subtle)
                .border_1()
                .border_color(p.line)
                .relative()
                .flex()
                .items_center()
                .gap(px(2.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "period-pill"),
                    self.active_period.clone(),
                    period_sensor.clone(),
                    5.,
                    pill_fill,
                    Some(p.line),
                    !p.dark,
                ));

            for opt in &self.periods {
                let selected = opt.id == self.active_period;
                let pid = opt.id.to_string();
                strip = strip.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("p-opt-{}", opt.id)))
                        .role(Role::Button)
                        .aria_label(opt.label.clone())
                        .tab_index(0)
                        .h(px(24.))
                        .px(px(8.))
                        .relative()
                        .rounded(px(5.))
                        .flex()
                        .items_center()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if selected { p.ink } else { p.text })
                        .cursor_pointer()
                        .when(!selected, move |d| {
                            d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                        })
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .when(selected, |d| d.child(pro_mark_selected(&period_sensor)))
                        .child(opt.label.clone()),
                    cx,
                    move |this, _, cx| {
                        this.set_period(&pid, cx);
                    },
                ));
            }
            strip.into_any_element()
        };

        let head_row = div()
            .w_full()
            .min_h(px(28.))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(px(15.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(heading),
            )
            .child(period_switch);

        // Determine headline figure (rolls to hovered or selected part's value via `pro_rolling_number`!)
        let active_part_id = self.active_spotlight_part().cloned();
        let spotlight_item = active_part_id
            .as_ref()
            .and_then(|aid| ranked.iter().find(|i| &i.id == aid));

        let shown_big_value = spotlight_item.map(|i| i.value).unwrap_or(total);
        let caption_text: SharedString = if let Some(hi) = spotlight_item {
            let pct = if total > 0.0 {
                (hi.value / total) * 100.0
            } else {
                0.0
            };
            let plbl = tr(cx, hi.label.as_ref(), hi.label_zh.as_ref());
            if self.selected_part.as_ref() == Some(&hi.id) && self.hovered_part.is_none() {
                format!(
                    "{} \u{00b7} {plbl} \u{00b7} {pct:.1}%",
                    tr(cx, "Selected", "已选")
                )
                .into()
            } else {
                format!("{plbl} \u{00b7} {pct:.1}%").into()
            }
        } else {
            tr(cx, self.total_label.as_ref(), self.total_label_zh.as_ref()).into()
        };

        let delta_badge = if spotlight_item.is_none()
            && let Some(dp) = delta_pct
            && dp.is_finite()
        {
            let is_up = dp >= 0.0;
            let (bg, fg) = if is_up {
                (p.green_soft, p.green_ink)
            } else {
                (p.red_soft, p.red_ink)
            };
            div()
                .h(px(20.))
                .pl(px(5.))
                .pr(px(6.))
                .rounded(px(5.))
                .bg(bg)
                .flex()
                .items_center()
                .gap(px(2.))
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(fg)
                .child(pro_icon(
                    if is_up {
                        ProIcon::ArrowUp
                    } else {
                        ProIcon::ChevronDown
                    },
                    11.,
                    fg,
                ))
                .child(pro_rolling_number(
                    pro_eid(&self.id, "delta-roll"),
                    dp.abs(),
                    1,
                    "",
                    "%",
                    11.5,
                    FontWeight::MEDIUM,
                    fg,
                ))
                .into_any_element()
        } else {
            div().into_any_element()
        };

        let figure_block = div()
            .mt(px(14.))
            .w_full()
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(comma_rolling_number(
                        pro_eid(&self.id, "big-total"),
                        shown_big_value,
                        self.decimals,
                        self.value_prefix.clone(),
                        self.value_suffix.clone(),
                        24.,
                        FontWeight::MEDIUM,
                        p.strong,
                    ))
                    .child(delta_badge),
            )
            .child(pro_swap_label(
                pro_eid(&self.id, "cap-swap"),
                caption_text,
                12.,
                FontWeight::NORMAL,
                p.muted,
            ));

        // Spring-Morphing Multi-Segment Proportion Bar (`pro_spring_f32` per segment share + `Slide::snappy` spotlight!)
        let mut prop_bar = div()
            .mt(px(15.))
            .w_full()
            .flex_none()
            .h(px(10.))
            .flex()
            .items_center()
            .gap(px(2.));

        let ordered_parts: Vec<CompactPart> = self
            .canonical_order
            .iter()
            .filter_map(|cid| ranked.iter().find(|i| &i.id == cid).cloned())
            .collect();
        let bar_parts = if ordered_parts.is_empty() {
            ranked.clone()
        } else {
            ordered_parts
        };

        // Pre-sample snappy `(opacity_slide, height_slide)` per part id.
        let mut part_anim_map: HashMap<SharedString, (f32, f32)> =
            HashMap::with_capacity(bar_parts.len());
        for part in &bar_parts {
            let is_spot = active_part_id.as_ref() == Some(&part.id);
            let dimmed = active_part_id.is_some() && !is_spot;
            let tgt_alpha = if dimmed { 0.26 } else { 1.0 };
            let tgt_h = if is_spot { 10.0 } else { 6.0 };
            let entry = self
                .part_spotlights
                .entry(part.id.clone())
                .or_insert_with(|| (Slide::snappy(1.0), Slide::snappy(6.0)));
            entry.0.set(tgt_alpha, cx);
            entry.1.set(tgt_h, cx);
            part_anim_map.insert(
                part.id.clone(),
                (
                    entry.0.get(w, cx).clamp(0.20, 1.0),
                    entry.1.get(w, cx).clamp(5.0, 11.0),
                ),
            );
        }

        for (b_idx, part) in bar_parts.iter().enumerate() {
            let target_share = if total > 0.0 {
                (part.value / total) as f32
            } else {
                0.0
            };
            let anim_share = pro_spring_f32(
                pro_eid(&self.id, format!("seg-w-{}", part.id)),
                target_share.clamp(0.0, 1.0),
                0.32,
                w,
                cx,
            )
            .clamp(0.0, 1.0);

            if anim_share <= 0.004 {
                continue;
            }

            let (seg_alpha, seg_h) = part_anim_map.get(&part.id).copied().unwrap_or((1.0, 6.0));
            let seg_color = self.part_color(part, &p);
            let pid_hov = part.id.clone();
            let pid_click = part.id.clone();

            prop_bar = prop_bar.child(
                div()
                    .id(pro_eid(&self.id, format!("bseg-{b_idx}-{}", part.id)))
                    .w(relative(anim_share))
                    .h(px(seg_h))
                    .rounded(px(2.5))
                    .bg(seg_color)
                    .opacity(seg_alpha)
                    .cursor_pointer()
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        if *inside {
                            this.hovered_part = Some(pid_hov.clone());
                        } else if this.hovered_part.as_ref() == Some(&pid_hov) {
                            this.hovered_part = None;
                        }
                        cx.notify();
                    }))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.toggle_select_part(pid_click.clone(), cx);
                    })),
            );
        }

        // Ranked Rows Smoothly Trading Vertical Positions via `pro_spring_f32` y-offset + `pro_tactile`!
        let rank_map: HashMap<SharedString, usize> = ranked
            .iter()
            .enumerate()
            .map(|(rk, item)| (item.id.clone(), rk))
            .collect();

        let list_h = (ranked.len() as f32) * ROW_HEIGHT;
        let mut rows_stage = div()
            .mt(px(10.))
            .w_full()
            .flex_none()
            .h(px(list_h))
            .relative();

        // Iterate in stable canonical order so GPUI element identity never swaps when ranks trade places!
        for part in &bar_parts {
            let Some(&rank_idx) = rank_map.get(&part.id) else {
                continue;
            };
            let target_y = (rank_idx as f32) * ROW_HEIGHT;
            let anim_y = pro_spring_f32(
                pro_eid(&self.id, format!("row-y-{}", part.id)),
                target_y,
                0.36,
                w,
                cx,
            );

            let is_sel = self.selected_part.as_ref() == Some(&part.id);
            let is_spot = active_part_id.as_ref() == Some(&part.id);
            let (seg_alpha, _) = part_anim_map.get(&part.id).copied().unwrap_or((1.0, 6.0));
            let row_alpha = (0.42 + 0.58 * seg_alpha).clamp(0.42, 1.0);
            let swatch_c = self.part_color(part, &p);
            let share_pct = if total > 0.0 {
                (part.value / total) * 100.0
            } else {
                0.0
            };
            let plabel = tr(cx, part.label.as_ref(), part.label_zh.as_ref()).to_string();
            let pid_hov = part.id.clone();
            let pid_click = part.id.clone();

            let rest_bg = if is_sel {
                pro_mix_color(
                    transparent_black(),
                    swatch_c,
                    if p.dark { 0.16 } else { 0.08 },
                )
            } else {
                transparent_black()
            };
            let hover_bg = if is_sel {
                pro_mix_color(p.hover, swatch_c, if p.dark { 0.20 } else { 0.11 })
            } else {
                p.hover
            };
            let rest_border = if is_sel {
                swatch_c.opacity(0.36)
            } else {
                transparent_black()
            };

            let row_inner = div()
                .w_full()
                .h(px(ROW_HEIGHT))
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(9.))
                .child(div().size(px(8.)).flex_none().rounded(px(2.)).bg(swatch_c))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(13.))
                        .font_weight(if is_spot {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(if is_spot { p.strong } else { p.ink })
                        .child(plabel.clone()),
                )
                .child(comma_rolling_number(
                    pro_eid(&self.id, format!("rval-{}", part.id)),
                    part.value,
                    self.decimals,
                    self.value_prefix.clone(),
                    self.value_suffix.clone(),
                    13.,
                    FontWeight::MEDIUM,
                    p.ink,
                ))
                .when(!very_narrow, |d| {
                    d.child(div().w(px(36.)).flex_none().flex().justify_end().child(
                        pro_rolling_number(
                            pro_eid(&self.id, format!("rshr-{}", part.id)),
                            share_pct,
                            0,
                            "",
                            "%",
                            12.,
                            FontWeight::NORMAL,
                            p.muted,
                        ),
                    ))
                });

            let row_el = pro_tactile(
                pro_eid(&self.id, format!("rank-row-{}", part.id)),
                format!(
                    "{plabel}: {}{}{} ({share_pct:.0}%)",
                    self.value_prefix,
                    format_comma_f64(part.value, self.decimals),
                    self.value_suffix
                ),
                6.,
                rest_bg,
                hover_bg,
                rest_border,
                swatch_c.opacity(0.28),
                is_sel,
                row_inner,
                cx,
                move |this, _, cx| {
                    this.toggle_select_part(pid_click.clone(), cx);
                },
            )
            .absolute()
            .left(px(-8.))
            .right(px(-8.))
            .top(px(anim_y))
            .h(px(ROW_HEIGHT))
            .opacity(row_alpha)
            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                if *inside {
                    this.hovered_part = Some(pid_hov.clone());
                } else if this.hovered_part.as_ref() == Some(&pid_hov) {
                    this.hovered_part = None;
                }
                cx.notify();
            }));

            rows_stage = rows_stage.child(row_el);
        }

        let footer = if self.show_footer
            && let Some(leader) = ranked.first()
        {
            let leader_name = tr(cx, leader.label.as_ref(), leader.label_zh.as_ref()).to_string();
            div()
                .w_full()
                .min_h(px(40.))
                .px(px(16.))
                .border_t_1()
                .border_color(p.line)
                .bg(p.subtle)
                .flex()
                .items_center()
                .child(pro_view_transition(
                    pro_eid(&self.id, "foot-leader-tx"),
                    format!("{}-{}", self.active_period, leader.id),
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(tr(cx, "Largest:", "最大占比:"))
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(leader_name),
                        ),
                ))
                .into_any_element()
        } else {
            div().into_any_element()
        };

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                if ev.keystroke.key.as_str() == "escape" && this.selected_part.is_some() {
                    this.selected_part = None;
                    cx.notify();
                    return;
                }
                let n = this.periods.len();
                if n == 0 {
                    return;
                }
                let Some(cur) = this.periods.iter().position(|o| o.id == this.active_period) else {
                    return;
                };
                let next_idx = match ev.keystroke.key.as_str() {
                    "left" => Some((cur + n - 1) % n),
                    "right" => Some((cur + 1) % n),
                    "home" => Some(0),
                    "end" => Some(n - 1),
                    _ => None,
                };
                if let Some(idx) = next_idx {
                    let pid = this.periods[idx].id.to_string();
                    this.set_period(&pid, cx);
                }
            }))
            .w_full()
            .relative()
            .flex()
            .flex_col()
            .child(sensor)
            .child(
                div()
                    .w_full()
                    .max_w(px(380.))
                    .mx_auto()
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .shadow(pro_card_shadow(p.dark))
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .w_full()
                            .pt(px(16.))
                            .px(px(16.))
                            .pb(px(10.))
                            .flex()
                            .flex_col()
                            .child(head_row)
                            .child(figure_block)
                            .child(prop_bar)
                            .child(rows_stage),
                    )
                    .child(footer),
            )
            .into_any_element()
    }
}

struct CommaRollState {
    target: f64,
    from: f64,
    dir: f32,
    clock: PhaseClock,
}

#[derive(IntoElement)]
struct CommaRollingNumber {
    id: ElementId,
    value: f64,
    decimals: usize,
    prefix: SharedString,
    suffix: SharedString,
    font_size: f32,
    weight: FontWeight,
    color: Hsla,
}

impl RenderOnce for CommaRollingNumber {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let val = if self.value.is_finite() {
            self.value
        } else {
            0.0
        };
        let st = w.use_keyed_state(pro_eid(&self.id, "crnum"), cx, |_, _| CommaRollState {
            target: val,
            from: val,
            dir: 1.,
            clock: PhaseClock::default(),
        });
        let (shown, dy) = st.update(cx, |s, cx| {
            let age_cur = s.clock.time(0.28, w, cx);
            let cur_t = pro_ease((age_cur / 0.28).clamp(0., 1.)) as f64;
            if (s.target - val).abs() > 1e-6 {
                s.from += (s.target - s.from) * cur_t;
                s.dir = if val >= s.target { 1. } else { -1. };
                s.target = val;
                s.clock.restart(cx);
            }
            let age = s.clock.time(0.28, w, cx);
            let t = pro_ease((age / 0.28).clamp(0., 1.));
            let v = s.from + (s.target - s.from) * (t as f64);
            let offset = if t >= 0.999 || disabled_motion(cx) {
                0.
            } else {
                2.5 * s.dir * (1. - t)
            };
            (v, offset)
        });
        let formatted = format!(
            "{}{}{}",
            self.prefix,
            format_comma_f64(shown, self.decimals),
            self.suffix
        );
        div()
            .relative()
            .top(px(dy))
            .text_size(px(self.font_size))
            .font_weight(self.weight)
            .text_color(self.color)
            .child(formatted)
            .into_any_element()
    }
}

#[allow(clippy::too_many_arguments)]
fn comma_rolling_number(
    id: impl Into<ElementId>,
    value: f64,
    decimals: usize,
    prefix: impl Into<SharedString>,
    suffix: impl Into<SharedString>,
    font_size: f32,
    weight: FontWeight,
    color: Hsla,
) -> AnyElement {
    CommaRollingNumber {
        id: id.into(),
        value,
        decimals,
        prefix: prefix.into(),
        suffix: suffix.into(),
        font_size,
        weight,
        color,
    }
    .into_any_element()
}

fn format_comma_f64(value: f64, decimals: usize) -> String {
    let v = if value.is_finite() { value } else { 0.0 };
    let neg = v < -1e-9;
    let raw = format!("{:.*}", decimals, v.abs());
    let (int_part, frac_part) = match raw.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (raw.as_str(), None),
    };
    let mut rev = String::with_capacity(int_part.len() + 4);
    for (idx, ch) in int_part.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            rev.push(',');
        }
        rev.push(ch);
    }
    let grouped: String = rev.chars().rev().collect();
    let sign = if neg && (grouped != "0" || frac_part.is_some_and(|f| f.chars().any(|c| c != '0')))
    {
        "-"
    } else {
        ""
    };
    match frac_part {
        Some(f) => format!("{sign}{grouped}.{f}"),
        None => format!("{sign}{grouped}"),
    }
}

fn default_channel_datasets() -> HashMap<SharedString, CompactPeriodData> {
    let mut map = HashMap::with_capacity(3);
    // 7d: #1 Organic ($21,400), #2 Direct ($16,900), #3 Social ($8,900), #4 Referrals ($6,400), #5 Email ($4,000) = $57,600 (+8.1%)
    map.insert(
        "7d".into(),
        CompactPeriodData::new(
            vec![
                CompactPart::new("organic", "Organic search", 21_400.0).with_label_zh("自然搜索"),
                CompactPart::new("direct", "Direct", 16_900.0).with_label_zh("直接访问"),
                CompactPart::new("social", "Social", 8_900.0).with_label_zh("社媒引流"),
                CompactPart::new("referrals", "Referrals", 6_400.0).with_label_zh("外部引荐"),
                CompactPart::new("email", "Email", 4_000.0).with_label_zh("邮件触达"),
            ],
            Some(8.1),
        ),
    );
    // 30d: #1 Direct ($74,200), #2 Organic ($61,800), #3 Referrals ($31,500), #4 Social ($22,400), #5 Email ($14,900) = $204,800 (+12.4%)
    map.insert(
        "30d".into(),
        CompactPeriodData::new(
            vec![
                CompactPart::new("direct", "Direct", 74_200.0).with_label_zh("直接访问"),
                CompactPart::new("organic", "Organic search", 61_800.0).with_label_zh("自然搜索"),
                CompactPart::new("referrals", "Referrals", 31_500.0).with_label_zh("外部引荐"),
                CompactPart::new("social", "Social", 22_400.0).with_label_zh("社媒引流"),
                CompactPart::new("email", "Email", 14_900.0).with_label_zh("邮件触达"),
            ],
            Some(12.4),
        ),
    );
    // 90d: #1 Organic ($198,000), #2 Referrals ($142,500), #3 Direct ($136,000), #4 Email ($58,200), #5 Social ($44,300) = $579,000 (-2.6%)
    map.insert(
        "90d".into(),
        CompactPeriodData::new(
            vec![
                CompactPart::new("organic", "Organic search", 198_000.0).with_label_zh("自然搜索"),
                CompactPart::new("referrals", "Referrals", 142_500.0).with_label_zh("外部引荐"),
                CompactPart::new("direct", "Direct", 136_000.0).with_label_zh("直接访问"),
                CompactPart::new("email", "Email", 58_200.0).with_label_zh("邮件触达"),
                CompactPart::new("social", "Social", 44_300.0).with_label_zh("社媒引流"),
            ],
            Some(-2.6),
        ),
    );
    map
}

#[cfg(test)]
mod tests {
    use super::CompactBreakdown;
    use gpui::{AppContext, TestAppContext, px, size};
    use std::time::Duration;

    #[gpui::test]
    fn switching_periods_reorders_ranked_rows_and_updates_totals(cx: &mut TestAppContext) {
        let card = cx.new(|cx| CompactBreakdown::new("test-compact-breakdown", cx));

        card.update(cx, |cb, cx| {
            assert_eq!(cb.active_period(), "7d");
            assert!((cb.current_total() - 57_600.0).abs() < 1e-4);
            let r7 = cb.ranked_items();
            assert_eq!(r7[0].id.as_ref(), "organic");
            assert_eq!(r7[1].id.as_ref(), "direct");

            // Switch to 30d: Direct overtakes Organic search at #1, Referrals overtakes Social at #3
            assert!(cb.set_period("30d", cx));
            assert!((cb.current_total() - 204_800.0).abs() < 1e-4);
            let r30 = cb.ranked_items();
            assert_eq!(r30[0].id.as_ref(), "direct");
            assert_eq!(r30[1].id.as_ref(), "organic");
            assert_eq!(r30[2].id.as_ref(), "referrals");

            // Switch to 90d: Organic #1, Referrals #2, Direct #3
            assert!(cb.set_period("90d", cx));
            assert!((cb.current_total() - 579_000.0).abs() < 1e-4);
            let r90 = cb.ranked_items();
            assert_eq!(r90[0].id.as_ref(), "organic");
            assert_eq!(r90[1].id.as_ref(), "referrals");
            assert_eq!(r90[2].id.as_ref(), "direct");
        });
    }

    #[gpui::test]
    fn tactile_row_selection_highlights_segment_with_snappy_spring(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let exec = cx.executor();
        let (card, w) = cx.add_window_view(|_, cx| CompactBreakdown::new("cb-sel", cx));
        w.simulate_resize(size(px(400.), px(380.)));
        w.run_until_parked();

        w.update(|_, app| {
            card.update(app, |cb, cx| {
                cb.toggle_select_part("direct".into(), cx);
                assert_eq!(cb.selected_part.as_deref(), Some("direct"));
            });
        });
        w.run_until_parked();

        exec.advance_clock(Duration::from_millis(60));
        w.update(|win, app| win.simulate_next_frame(app));
        w.run_until_parked();

        w.update(|_, app| {
            let cb = card.read(app);
            let (sel_alpha, sel_h) = cb.part_spotlights["direct"];
            let (dim_alpha, _) = cb.part_spotlights["organic"];
            assert!(sel_h.value(app) > 6.5);
            assert!(sel_alpha.value(app) > dim_alpha.value(app));
        });
    }
}
