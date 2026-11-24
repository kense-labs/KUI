//! Interactive SaaS billing phase builder with cadence, timing, discounts, and rolling totals.
//!
//! Composes a contract billing phase from a start date, phase duration, recurring or one-off
//! product lines (`In advance` / `In arrears`), percentage discounts, and minimum spend
//! commitments while rolling first-invoice and phase totals live.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_check_draw, pro_eid, pro_empty_state, pro_fold, pro_icon_btn,
        pro_popover_presence, pro_rolling_number, pro_skeleton_bar, pro_swap_label, pro_tactile,
        pro_toast_banner, pro_view_transition, tr,
    },
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        family::disabled_motion,
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const SVG_REPEAT: &str = r#"<path d="M17 3.5 19.5 6 17 8.5M19.5 6H9a5 5 0 0 0-5 5M7 20.5 4.5 18 7 15.5M4.5 18H15a5 5 0 0 0 5-5" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/>"#;
const SVG_DISCOUNT: &str = r#"<path d="M9 15 15 9M9.5 9.5h.01M14.5 14.5h.01M12 3l2.1 1.6 2.6-.2.9 2.5 2.3 1.2-.4 2.6L21 12.6l-1.5 2.1.4 2.6-2.3 1.2-.9 2.5-2.6-.2L12 21.4l-2.1-1.6-2.6.2-.9-2.5L4.1 16.3l.4-2.6L3 12l1.5-2.1-.4-2.6 2.3-1.2.9-2.5 2.6.2Z" stroke="currentColor" stroke-width="1.65" stroke-linecap="round" stroke-linejoin="round"/>"#;
const SVG_MINIMUM: &str = r#"<path d="M5 19.5h14M7 15.5h10M9 11.5h6M10 7.5h4M12 4v1" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"/>"#;

/// Simple `YYYY-MM-DD` calendar date used by [`BillingSchedule`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BillingDate {
    /// Four-digit Gregorian year.
    pub year: i32,
    /// Month in `1..=12`.
    pub month: u8,
    /// Day of month in `1..=days_in_month(year, month)`.
    pub day: u8,
}

impl Default for BillingDate {
    fn default() -> Self {
        Self {
            year: 2026,
            month: 1,
            day: 1,
        }
    }
}

impl BillingDate {
    /// Construct a clamped [`BillingDate`].
    pub fn new(year: i32, month: u8, day: u8) -> Self {
        let m = month.clamp(1, 12);
        let max_d = days_in_month(year, m);
        Self {
            year,
            month: m,
            day: day.clamp(1, max_d),
        }
    }

    /// Parse `"YYYY-MM-DD"`, returning `None` if malformed.
    pub fn parse_iso(raw: &str) -> Option<Self> {
        let mut parts = raw.trim().split('-');
        let y: i32 = parts.next()?.parse().ok()?;
        let m: u8 = parts.next()?.parse().ok()?;
        let d: u8 = parts.next()?.parse().ok()?;
        if parts.next().is_some() || !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
            return None;
        }
        Some(Self::new(y, m, d))
    }

    /// Format as `"YYYY-MM-DD"`.
    pub fn to_iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// Add `months` calendar months, clamping day-of-month cleanly.
    pub fn add_months(self, months: i32) -> Self {
        let total = self.year * 12 + (i32::from(self.month) - 1) + months;
        let y = total.div_euclid(12);
        let m = (total.rem_euclid(12) + 1) as u8;
        let d = self.day.min(days_in_month(y, m));
        Self {
            year: y,
            month: m,
            day: d,
        }
    }

    /// Format for human display (`"1 Jan 2026"` in English, `"2026年1月1日"` in Chinese).
    pub fn display(self, cx: &App) -> String {
        let en_months = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        let m_idx = usize::from(self.month.saturating_sub(1)).min(11);
        let en = format!("{} {} {}", self.day, en_months[m_idx], self.year);
        let zh = format!("{}年{}月{}日", self.year, self.month, self.day);
        tr(cx, &en, &zh).to_string()
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 30,
    }
}

/// Weekday offset (`0 = Monday .. 6 = Sunday`) via Sakamoto's algorithm.
fn weekday_monday_zero(year: i32, month: u8, day: u8) -> usize {
    let t = [0i32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let mut y = year;
    if month < 3 {
        y -= 1;
    }
    let m_idx = usize::from(month.saturating_sub(1)).min(11);
    let sun_zero = (y + y / 4 - y / 100 + y / 400 + t[m_idx] + i32::from(day)).rem_euclid(7);
    ((sun_zero + 6) % 7) as usize
}

/// Phase duration options for [`BillingSchedule`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PhaseDuration {
    /// Single completion milestone without fixed calendar months.
    #[default]
    Milestone,
    /// 1-month billing phase.
    OneMonth,
    /// 3-month quarterly phase.
    ThreeMonths,
    /// 6-month semi-annual phase.
    SixMonths,
    /// 12-month annual phase.
    TwelveMonths,
    /// 24-month multi-year phase.
    TwentyFourMonths,
}

impl PhaseDuration {
    /// All selectable durations in menu order.
    pub const ALL: [Self; 6] = [
        Self::Milestone,
        Self::OneMonth,
        Self::ThreeMonths,
        Self::SixMonths,
        Self::TwelveMonths,
        Self::TwentyFourMonths,
    ];

    /// Number of calendar months (`None` for [`Self::Milestone`]).
    pub fn months(self) -> Option<u32> {
        match self {
            Self::Milestone => None,
            Self::OneMonth => Some(1),
            Self::ThreeMonths => Some(3),
            Self::SixMonths => Some(6),
            Self::TwelveMonths => Some(12),
            Self::TwentyFourMonths => Some(24),
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Milestone => "milestone",
            Self::OneMonth => "1m",
            Self::ThreeMonths => "3m",
            Self::SixMonths => "6m",
            Self::TwelveMonths => "12m",
            Self::TwentyFourMonths => "24m",
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Milestone => tr(cx, "Milestone", "里程碑结算"),
            Self::OneMonth => tr(cx, "1 month", "1 个月"),
            Self::ThreeMonths => tr(cx, "3 months", "3 个月"),
            Self::SixMonths => tr(cx, "6 months", "6 个月"),
            Self::TwelveMonths => tr(cx, "12 months", "12 个月"),
            Self::TwentyFourMonths => tr(cx, "24 months", "24 个月"),
        }
    }

    fn hint(self, cx: &App) -> &'static str {
        match self {
            Self::Milestone => tr(cx, "Single phase", "单次阶段交付"),
            Self::OneMonth => tr(cx, "1 cycle", "1 个月度周期"),
            Self::ThreeMonths => tr(cx, "1 quarter", "1 个季度周期"),
            Self::SixMonths => tr(cx, "2 quarters", "半年期合同"),
            Self::TwelveMonths => tr(cx, "Annual term", "年度标准合同"),
            Self::TwentyFourMonths => tr(cx, "2-year term", "两年期合同"),
        }
    }
}

/// Billing repeat cadence for a product line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RepeatCadence {
    /// Billed once during the phase.
    #[default]
    Once,
    /// Billed every month of the phase.
    Monthly,
    /// Billed every quarter of the phase.
    Quarterly,
    /// Billed every year of the phase.
    Yearly,
}

impl RepeatCadence {
    /// All selectable cadences in menu order.
    pub const ALL: [Self; 4] = [Self::Once, Self::Monthly, Self::Quarterly, Self::Yearly];

    /// Billing occurrences across a [`PhaseDuration`].
    pub fn occurrences(self, duration: PhaseDuration) -> u32 {
        let m = duration.months().unwrap_or(1).max(1);
        match self {
            Self::Once => 1,
            Self::Monthly => m,
            Self::Quarterly => m.div_ceil(3).max(1),
            Self::Yearly => m.div_ceil(12).max(1),
        }
    }

    fn short_label(self, cx: &App) -> &'static str {
        match self {
            Self::Once => tr(cx, "Once", "一次性"),
            Self::Monthly => tr(cx, "Monthly", "按月"),
            Self::Quarterly => tr(cx, "Quarterly", "按季"),
            Self::Yearly => tr(cx, "Yearly", "按年"),
        }
    }

    fn menu_label(self, cx: &App) -> &'static str {
        match self {
            Self::Once => tr(cx, "Once", "一次性计费"),
            Self::Monthly => tr(cx, "Every month", "每月重复"),
            Self::Quarterly => tr(cx, "Every quarter", "每季度重复"),
            Self::Yearly => tr(cx, "Every year", "每年重复"),
        }
    }

    fn unit_suffix(self, cx: &App) -> &'static str {
        match self {
            Self::Once => "",
            Self::Monthly => tr(cx, "/ mo", "/ 月"),
            Self::Quarterly => tr(cx, "/ qtr", "/ 季"),
            Self::Yearly => tr(cx, "/ yr", "/ 年"),
        }
    }
}

/// Invoice timing for a product row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BillingTiming {
    /// Billed up front at the beginning of each cycle (included on the first invoice).
    #[default]
    InAdvance,
    /// Billed at the end of each cycle (deferred from day-one first invoice).
    InArrears,
}

impl BillingTiming {
    /// Both timing modes in menu order.
    pub const ALL: [Self; 2] = [Self::InAdvance, Self::InArrears];

    fn inline_label(self, cx: &App) -> &'static str {
        match self {
            Self::InAdvance => tr(cx, "billed in advance", "预付计费 (In advance)"),
            Self::InArrears => tr(cx, "billed in arrears", "后付计费 (In arrears)"),
        }
    }

    fn menu_label(self, cx: &App) -> &'static str {
        match self {
            Self::InAdvance => tr(cx, "In advance", "期初预付 (In advance)"),
            Self::InArrears => tr(cx, "In arrears", "期末后付 (In arrears)"),
        }
    }
}

/// Catalog entry offered under the `+ Add product` menu.
#[derive(Clone, Debug, PartialEq)]
pub struct BillingCatalogItem {
    /// Stable catalog identifier.
    pub id: SharedString,
    /// English display name.
    pub name: SharedString,
    /// Localized Chinese display name.
    pub name_zh: SharedString,
    /// Default billing repeat cadence.
    pub repeat: RepeatCadence,
    /// Default unit price.
    pub price: f64,
}

impl BillingCatalogItem {
    /// Construct a new [`BillingCatalogItem`].
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        repeat: RepeatCadence,
        price: f64,
    ) -> Self {
        let name = name.into();
        Self {
            id: id.into(),
            name_zh: name.clone(),
            name,
            repeat,
            price,
        }
    }

    /// Set localized Chinese product name.
    pub fn with_name_zh(mut self, zh: impl Into<SharedString>) -> Self {
        self.name_zh = zh.into();
        self
    }
}

/// Kind and parameters of a row in [`BillingSchedule`].
#[derive(Clone, Debug, PartialEq)]
pub enum ScheduleLineKind {
    /// Standard recurring or one-off product charge.
    Product {
        name: SharedString,
        name_zh: SharedString,
        repeat: RepeatCadence,
        timing: BillingTiming,
        price: f64,
    },
    /// Percentage discount with optional coupon code applied across the phase.
    Discount { code: SharedString, percent: f64 },
    /// Minimum spend floor across the phase.
    Minimum { amount: f64 },
}

/// One line item inside [`BillingSchedule`].
#[derive(Clone, Debug, PartialEq)]
pub struct ScheduleLine {
    /// Unique row identifier.
    pub id: SharedString,
    /// Line payload.
    pub kind: ScheduleLineKind,
}

impl ScheduleLine {
    /// Create a product schedule line.
    pub fn product(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        repeat: RepeatCadence,
        timing: BillingTiming,
        price: f64,
    ) -> Self {
        let name = name.into();
        Self {
            id: id.into(),
            kind: ScheduleLineKind::Product {
                name_zh: name.clone(),
                name,
                repeat,
                timing,
                price: if price.is_finite() {
                    price.max(0.0)
                } else {
                    0.0
                },
            },
        }
    }

    /// Attach a localized Chinese name to a product line.
    pub fn with_name_zh(mut self, zh: impl Into<SharedString>) -> Self {
        if let ScheduleLineKind::Product { name_zh, .. } = &mut self.kind {
            *name_zh = zh.into();
        }
        self
    }

    /// Create a percentage discount line (`0.0..=100.0`).
    pub fn discount(
        id: impl Into<SharedString>,
        code: impl Into<SharedString>,
        percent: f64,
    ) -> Self {
        let p = if percent.is_finite() {
            percent.clamp(0.0, 100.0)
        } else {
            0.0
        };
        Self {
            id: id.into(),
            kind: ScheduleLineKind::Discount {
                code: code.into(),
                percent: p,
            },
        }
    }

    /// Create a phase minimum commitment line.
    pub fn minimum(id: impl Into<SharedString>, amount: f64) -> Self {
        let a = if amount.is_finite() {
            amount.max(0.0)
        } else {
            0.0
        };
        Self {
            id: id.into(),
            kind: ScheduleLineKind::Minimum { amount: a },
        }
    }
}

/// Computed financial summary for [`BillingSchedule`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BillingTotals {
    /// Day-one invoice subtotal before discounts.
    pub first_subtotal: f64,
    /// Day-one discount deduction.
    pub first_discount: f64,
    /// Day-one net invoice amount.
    pub first_invoice: f64,
    /// Full phase subtotal before discounts and minimum floor.
    pub phase_subtotal: f64,
    /// Full phase discount deduction.
    pub phase_discount: f64,
    /// Full phase total after discounts and minimum commitment floor.
    pub phase_total: f64,
    /// Whether the minimum commitment raised `phase_total`.
    pub minimum_applied: bool,
}

/// Snapshot emitted via [`BillingScheduleEvent::Changed`].
#[derive(Clone, Debug, PartialEq)]
pub struct BillingScheduleSnapshot {
    /// Phase label.
    pub phase: SharedString,
    /// Phase start date (`YYYY-MM-DD`).
    pub start: String,
    /// Selected duration.
    pub duration: PhaseDuration,
    /// Active schedule lines.
    pub lines: Vec<ScheduleLine>,
    /// Computed financial totals.
    pub totals: BillingTotals,
}

/// Typed events emitted by [`BillingSchedule`].
#[derive(Clone, Debug, PartialEq)]
pub enum BillingScheduleEvent {
    /// Emitted whenever phase metadata, dates, duration, or line items change.
    Changed(BillingScheduleSnapshot),
    /// Emitted when a product, discount, or minimum line is added.
    LineAdded(ScheduleLine),
    /// Emitted when a line is removed by id.
    LineRemoved(SharedString),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ActivePopover {
    Calendar { view_year: i32, view_month: u8 },
    Duration,
    Catalog,
    LineRepeat(SharedString),
    LineTiming(SharedString),
}

struct LineEditorSlot {
    line_id: SharedString,
    value_editor: Entity<Editor>,
    code_editor: Option<Entity<Editor>>,
}

/// Interactive SaaS billing schedule and contract phase composer.
pub struct BillingSchedule {
    id: ElementId,
    title: SharedString,
    title_zh: SharedString,
    phase: SharedString,
    currency: SharedString,
    start: BillingDate,
    duration: PhaseDuration,
    catalog: Vec<BillingCatalogItem>,
    lines: Vec<ScheduleLine>,
    entering_ids: Vec<(SharedString, bool)>,
    exiting_lines: Vec<(ScheduleLine, Instant)>,
    next_seq: usize,
    popover: Option<ActivePopover>,
    hovered_row: Option<SharedString>,
    phase_editor: Option<Entity<Editor>>,
    line_editors: Vec<LineEditorSlot>,
    loading: bool,
    empty: bool,
    width_cell: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    focus: FocusHandle,
}

impl EventEmitter<BillingScheduleEvent> for BillingSchedule {}

impl BillingSchedule {
    /// Construct a [`BillingSchedule`] with canonical default product catalog and initial phase.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            title: "Create a billing schedule".into(),
            title_zh: "创建计费排期阶段".into(),
            phase: "Build-out phase".into(),
            currency: "$".into(),
            start: BillingDate::default(),
            duration: PhaseDuration::Milestone,
            catalog: default_catalog(),
            lines: default_schedule_lines(),
            entering_ids: Vec::new(),
            exiting_lines: Vec::new(),
            next_seq: 10,
            popover: None,
            hovered_row: None,
            phase_editor: None,
            line_editors: Vec::new(),
            loading: false,
            empty: false,
            width_cell: Rc::new(Cell::new(560.)),
            toast: None,
            focus: cx.focus_handle(),
        }
    }

    /// Customize the card heading.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        let t = title.into();
        self.title_zh = t.clone();
        self.title = t;
        self
    }

    /// Customize the initial editable phase name.
    pub fn with_phase(mut self, phase: impl Into<SharedString>) -> Self {
        self.phase = phase.into();
        self
    }

    /// Customize the currency symbol (default `"$"`).
    pub fn with_currency(mut self, currency: impl Into<SharedString>) -> Self {
        self.currency = currency.into();
        self
    }

    /// Set the initial start date from `"YYYY-MM-DD"`.
    pub fn with_default_start(mut self, iso_date: &str) -> Self {
        if let Some(parsed) = BillingDate::parse_iso(iso_date) {
            self.start = parsed;
        }
        self
    }

    /// Set the initial [`PhaseDuration`].
    pub fn with_duration(mut self, duration: PhaseDuration) -> Self {
        self.duration = duration;
        self
    }

    /// Override the product catalog offered under `+ Add product`.
    pub fn with_catalog(mut self, catalog: Vec<BillingCatalogItem>) -> Self {
        self.catalog = catalog;
        self
    }

    /// Override the initial schedule lines.
    pub fn with_lines(mut self, lines: Vec<ScheduleLine>) -> Self {
        self.lines = lines;
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

    /// Return current phase name.
    pub fn phase(&self) -> &str {
        &self.phase
    }

    /// Return current start date.
    pub fn start_date(&self) -> BillingDate {
        self.start
    }

    /// Return current phase duration.
    pub fn duration(&self) -> PhaseDuration {
        self.duration
    }

    /// Return current schedule lines.
    pub fn lines(&self) -> &[ScheduleLine] {
        &self.lines
    }

    /// Return whether a discount row is currently present.
    pub fn has_discount(&self) -> bool {
        self.lines
            .iter()
            .any(|l| matches!(l.kind, ScheduleLineKind::Discount { .. }))
    }

    /// Return whether a minimum commitment row is currently present.
    pub fn has_minimum(&self) -> bool {
        self.lines
            .iter()
            .any(|l| matches!(l.kind, ScheduleLineKind::Minimum { .. }))
    }

    /// Compute live first-invoice and phase totals.
    pub fn totals(&self) -> BillingTotals {
        let mut first_subtotal = 0.0_f64;
        let mut phase_subtotal = 0.0_f64;
        let mut discount_pct = 0.0_f64;
        let mut min_floor = 0.0_f64;

        for line in &self.lines {
            match &line.kind {
                ScheduleLineKind::Product {
                    repeat,
                    timing,
                    price,
                    ..
                } => {
                    let unit = if price.is_finite() {
                        price.max(0.0)
                    } else {
                        0.0
                    };
                    let count = f64::from(repeat.occurrences(self.duration));
                    phase_subtotal += unit * count;
                    if *timing == BillingTiming::InAdvance {
                        first_subtotal += unit;
                    }
                }
                ScheduleLineKind::Discount { percent, .. } => {
                    if percent.is_finite() {
                        discount_pct = (discount_pct + percent.clamp(0.0, 100.0)).clamp(0.0, 100.0);
                    }
                }
                ScheduleLineKind::Minimum { amount } => {
                    if amount.is_finite() {
                        min_floor = min_floor.max(amount.max(0.0));
                    }
                }
            }
        }

        let rate = discount_pct / 100.0;
        let first_discount = first_subtotal * rate;
        let first_invoice = (first_subtotal - first_discount).max(0.0);

        let phase_discount = phase_subtotal * rate;
        let discounted_phase = (phase_subtotal - phase_discount).max(0.0);
        let minimum_applied = min_floor > discounted_phase + 0.004;
        let phase_total = discounted_phase.max(min_floor);

        BillingTotals {
            first_subtotal,
            first_discount,
            first_invoice,
            phase_subtotal,
            phase_discount,
            phase_total,
            minimum_applied,
        }
    }

    /// Build a full snapshot of the current schedule state.
    pub fn snapshot(&self) -> BillingScheduleSnapshot {
        BillingScheduleSnapshot {
            phase: self.phase.clone(),
            start: self.start.to_iso(),
            duration: self.duration,
            lines: self.lines.clone(),
            totals: self.totals(),
        }
    }

    /// Programmatically set the phase start date.
    pub fn set_start_date(&mut self, date: BillingDate, cx: &mut Context<Self>) {
        self.start = date;
        self.popover = None;
        self.emit_changed(cx);
    }

    /// Programmatically set the phase duration.
    pub fn set_duration(&mut self, duration: PhaseDuration, cx: &mut Context<Self>) {
        self.duration = duration;
        self.popover = None;
        self.emit_changed(cx);
    }

    /// Add a product from the catalog by index.
    pub fn add_catalog_product(&mut self, index: usize, cx: &mut Context<Self>) -> bool {
        let Some(item) = self.catalog.get(index).cloned() else {
            return false;
        };
        self.next_seq += 1;
        let id = SharedString::from(format!("prod-{}", self.next_seq));
        let line = ScheduleLine::product(
            id,
            item.name,
            item.repeat,
            BillingTiming::InAdvance,
            item.price,
        )
        .with_name_zh(item.name_zh);
        self.empty = false;
        self.popover = None;
        self.entering_ids.push((line.id.clone(), false));
        self.lines.push(line.clone());
        cx.emit(BillingScheduleEvent::LineAdded(line));
        self.emit_changed(cx);
        true
    }

    /// Add a percentage discount row if none exists yet.
    pub fn add_discount_line(&mut self, cx: &mut Context<Self>) -> bool {
        if self.has_discount() {
            return false;
        }
        self.next_seq += 1;
        let line = ScheduleLine::discount(format!("disc-{}", self.next_seq), "ANNUAL15", 15.0);
        self.empty = false;
        self.popover = None;
        self.entering_ids.push((line.id.clone(), false));
        self.lines.push(line.clone());
        cx.emit(BillingScheduleEvent::LineAdded(line));
        self.emit_changed(cx);
        true
    }

    /// Add a minimum spend commitment row if none exists yet.
    pub fn add_minimum_line(&mut self, cx: &mut Context<Self>) -> bool {
        if self.has_minimum() {
            return false;
        }
        self.next_seq += 1;
        let line = ScheduleLine::minimum(format!("min-{}", self.next_seq), 10_000.0);
        self.empty = false;
        self.popover = None;
        self.entering_ids.push((line.id.clone(), false));
        self.lines.push(line.clone());
        cx.emit(BillingScheduleEvent::LineAdded(line));
        self.emit_changed(cx);
        true
    }

    /// Remove a line item by id.
    pub fn remove_line(&mut self, line_id: &str, cx: &mut Context<Self>) -> bool {
        let Some(pos) = self.lines.iter().position(|l| l.id.as_ref() == line_id) else {
            return false;
        };
        let removed = self.lines.remove(pos);
        self.entering_ids.retain(|(id, _)| id.as_ref() != line_id);
        self.exiting_lines.retain(|(l, _)| l.id.as_ref() != line_id);
        self.exiting_lines.push((removed, Instant::now()));
        self.line_editors.retain(|s| s.line_id.as_ref() != line_id);
        self.popover = None;
        cx.emit(BillingScheduleEvent::LineRemoved(SharedString::from(
            line_id.to_string(),
        )));
        self.emit_changed(cx);
        true
    }

    /// Update a product line's repeat cadence.
    pub fn set_line_repeat(
        &mut self,
        line_id: &str,
        repeat: RepeatCadence,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut updated = false;
        for line in &mut self.lines {
            if line.id.as_ref() == line_id
                && let ScheduleLineKind::Product { repeat: r, .. } = &mut line.kind
            {
                *r = repeat;
                updated = true;
            }
        }
        self.popover = None;
        if updated {
            self.emit_changed(cx);
        }
        updated
    }

    /// Update a product line's billing timing (`InAdvance` / `InArrears`).
    pub fn set_line_timing(
        &mut self,
        line_id: &str,
        timing: BillingTiming,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut updated = false;
        for line in &mut self.lines {
            if line.id.as_ref() == line_id
                && let ScheduleLineKind::Product { timing: t, .. } = &mut line.kind
            {
                *t = timing;
                updated = true;
            }
        }
        self.popover = None;
        if updated {
            self.emit_changed(cx);
        }
        updated
    }

    /// Update a line's numeric value (unit price, discount percentage, or minimum spend).
    pub fn set_line_amount(
        &mut self,
        line_id: &str,
        raw_value: f64,
        w: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !raw_value.is_finite() {
            return false;
        }
        let mut formatted_sync = None;
        for line in &mut self.lines {
            if line.id.as_ref() == line_id {
                match &mut line.kind {
                    ScheduleLineKind::Product { price, .. } => {
                        *price = raw_value.clamp(0.0, 9_999_999.0);
                        formatted_sync = Some(format_number(*price, 2));
                    }
                    ScheduleLineKind::Discount { percent, .. } => {
                        *percent = raw_value.clamp(0.0, 100.0);
                        formatted_sync = Some(format_clean_pct(*percent));
                    }
                    ScheduleLineKind::Minimum { amount } => {
                        *amount = raw_value.clamp(0.0, 9_999_999.0);
                        formatted_sync = Some(format_number(*amount, 2));
                    }
                }
            }
        }
        let Some(text) = formatted_sync else {
            return false;
        };
        if let Some(win) = w
            && let Some(slot) = self
                .line_editors
                .iter()
                .find(|s| s.line_id.as_ref() == line_id)
        {
            slot.value_editor
                .update(cx, |e, cx| e.set_value(text, win, cx));
        }
        self.emit_changed(cx);
        true
    }

    fn emit_changed(&mut self, cx: &mut Context<Self>) {
        let snap = self.snapshot();
        cx.emit(BillingScheduleEvent::Changed(snap));
        cx.notify();
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.phase_editor.is_none() {
            let init_phase = self.phase.to_string();
            let ed = cx.new(|cx| {
                let mut e = Editor::new("Phase name", InputMode::Single, w, cx);
                e.set_placeholder("Phase name");
                e.set_value(init_phase, w, cx);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => {
                    let next = ed_ent.read(cx).value().trim().to_string();
                    if !next.is_empty() {
                        this.phase = SharedString::from(next);
                        this.emit_changed(cx);
                    }
                }
                EditEvent::Submitted | EditEvent::Blurred => {
                    let next = ed_ent.read(cx).value().trim().to_string();
                    if next.is_empty() {
                        let fallback = this.phase.to_string();
                        ed_ent.update(cx, |e, cx| e.set_value(fallback, w, cx));
                    } else {
                        this.phase = SharedString::from(next);
                        this.emit_changed(cx);
                    }
                    if matches!(ev, EditEvent::Submitted) {
                        this.focus.focus(w, cx);
                    }
                }
                EditEvent::Escape => {
                    let restore = this.phase.to_string();
                    ed_ent.update(cx, |e, cx| e.set_value(restore, w, cx));
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.phase_editor = Some(ed);
        }

        self.line_editors
            .retain(|slot| self.lines.iter().any(|l| l.id == slot.line_id));

        for idx in 0..self.lines.len() {
            let line_id = self.lines[idx].id.clone();
            if self.line_editors.iter().any(|s| s.line_id == line_id) {
                continue;
            }
            let (init_val, init_code) = match &self.lines[idx].kind {
                ScheduleLineKind::Product { price, .. } => (format_number(*price, 2), None),
                ScheduleLineKind::Discount { code, percent } => {
                    (format_clean_pct(*percent), Some(code.to_string()))
                }
                ScheduleLineKind::Minimum { amount } => (format_number(*amount, 2), None),
            };
            let val_ed = cx.new(|cx| {
                let mut e = Editor::new("Amount", InputMode::Single, w, cx);
                e.set_value(init_val, w, cx);
                e.font_size = 13.;
                e
            });
            let lid_val = line_id.clone();
            cx.subscribe_in(
                &val_ed,
                w,
                move |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                    EditEvent::Changed => {
                        let cleaned = ed_ent.read(cx).value().replace([',', ' ', '$', '%'], "");
                        if let Ok(parsed) = cleaned.parse::<f64>()
                            && parsed.is_finite()
                        {
                            for line in &mut this.lines {
                                if line.id == lid_val {
                                    match &mut line.kind {
                                        ScheduleLineKind::Product { price, .. } => {
                                            *price = parsed.clamp(0.0, 9_999_999.0);
                                        }
                                        ScheduleLineKind::Discount { percent, .. } => {
                                            *percent = parsed.clamp(0.0, 100.0);
                                        }
                                        ScheduleLineKind::Minimum { amount } => {
                                            *amount = parsed.clamp(0.0, 9_999_999.0);
                                        }
                                    }
                                }
                            }
                            cx.notify();
                        }
                    }
                    EditEvent::Submitted | EditEvent::Blurred => {
                        let cleaned = ed_ent.read(cx).value().replace([',', ' ', '$', '%'], "");
                        if let Ok(parsed) = cleaned.parse::<f64>() {
                            this.set_line_amount(&lid_val, parsed, Some(w), cx);
                        } else if let Some(line) = this.lines.iter().find(|l| l.id == lid_val) {
                            let restore = line_formatted_value(line);
                            ed_ent.update(cx, |e, cx| e.set_value(restore, w, cx));
                            cx.notify();
                        }
                        if matches!(ev, EditEvent::Submitted) {
                            this.focus.focus(w, cx);
                        }
                    }
                    EditEvent::Escape => {
                        if let Some(line) = this.lines.iter().find(|l| l.id == lid_val) {
                            let restore = line_formatted_value(line);
                            ed_ent.update(cx, |e, cx| e.set_value(restore, w, cx));
                        }
                        this.focus.focus(w, cx);
                        cx.notify();
                    }
                    _ => {}
                },
            )
            .detach();

            let code_editor = init_code.map(|code_str| {
                let ced = cx.new(|cx| {
                    let mut e = Editor::new("Coupon code", InputMode::Single, w, cx);
                    e.set_placeholder("COUPON");
                    e.set_value(code_str, w, cx);
                    e.font_size = 11.5;
                    e
                });
                let lid_code = line_id.clone();
                cx.subscribe_in(&ced, w, move |this, ed_ent, ev: &EditEvent, w, cx| {
                    if matches!(
                        ev,
                        EditEvent::Changed | EditEvent::Submitted | EditEvent::Blurred
                    ) {
                        let raw = ed_ent.read(cx).value().trim().to_ascii_uppercase();
                        for line in &mut this.lines {
                            if line.id == lid_code
                                && let ScheduleLineKind::Discount { code, .. } = &mut line.kind
                            {
                                *code = SharedString::from(raw.clone());
                            }
                        }
                        if matches!(ev, EditEvent::Submitted | EditEvent::Blurred) {
                            this.emit_changed(cx);
                        } else {
                            cx.notify();
                        }
                        if matches!(ev, EditEvent::Submitted) {
                            this.focus.focus(w, cx);
                        }
                    }
                })
                .detach();
                ced
            });

            self.line_editors.push(LineEditorSlot {
                line_id,
                value_editor: val_ed,
                code_editor,
            });
        }
    }

    fn render_calendar_popover(
        &self,
        view_year: i32,
        view_month: u8,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let en_months = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        let m_idx = usize::from(view_month.saturating_sub(1)).min(11);
        let month_heading = tr(
            cx,
            &format!("{} {}", en_months[m_idx], view_year),
            &format!("{}年 {}月", view_year, view_month),
        )
        .to_string();

        let prev_btn = activate(
            div()
                .id(pro_eid(&self.id, "cal-prev"))
                .role(Role::Button)
                .aria_label(tr(cx, "Previous month", "上个月"))
                .tab_index(0)
                .size(px(28.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover))
                .child(pro_icon(ProIcon::ChevronLeft, 15., p.muted)),
            cx,
            move |this, _, cx| {
                let prev = BillingDate::new(view_year, view_month, 1).add_months(-1);
                this.popover = Some(ActivePopover::Calendar {
                    view_year: prev.year,
                    view_month: prev.month,
                });
                cx.notify();
            },
        );

        let next_btn = activate(
            div()
                .id(pro_eid(&self.id, "cal-next"))
                .role(Role::Button)
                .aria_label(tr(cx, "Next month", "下个月"))
                .tab_index(0)
                .size(px(28.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover))
                .child(pro_icon(ProIcon::ChevronRight, 15., p.muted)),
            cx,
            move |this, _, cx| {
                let next = BillingDate::new(view_year, view_month, 1).add_months(1);
                this.popover = Some(ActivePopover::Calendar {
                    view_year: next.year,
                    view_month: next.month,
                });
                cx.notify();
            },
        );

        let dows = [
            tr(cx, "Mo", "一"),
            tr(cx, "Tu", "二"),
            tr(cx, "We", "三"),
            tr(cx, "Th", "四"),
            tr(cx, "Fr", "五"),
            tr(cx, "Sa", "六"),
            tr(cx, "Su", "日"),
        ];
        let mut dow_row = div().w_full().flex().items_center();
        for d in dows {
            dow_row = dow_row.child(
                div()
                    .flex_1()
                    .h(px(24.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(11.))
                    .text_color(p.muted)
                    .child(d),
            );
        }

        let start_offset = weekday_monday_zero(view_year, view_month, 1);
        let cur_days = days_in_month(view_year, view_month);
        let prev_month_date = BillingDate::new(view_year, view_month, 1).add_months(-1);
        let prev_days = days_in_month(prev_month_date.year, prev_month_date.month);
        let next_month_date = BillingDate::new(view_year, view_month, 1).add_months(1);

        let mut grid = div().w_full().flex().flex_col().gap(px(2.));
        for week in 0..6usize {
            let mut row = div().w_full().flex().items_center().gap(px(2.));
            for col in 0..7usize {
                let cell_idx = week * 7 + col;
                let (cell_date, in_month) = if cell_idx < start_offset {
                    let d = prev_days - (start_offset - 1 - cell_idx) as u8;
                    (
                        BillingDate::new(prev_month_date.year, prev_month_date.month, d),
                        false,
                    )
                } else if cell_idx - start_offset < usize::from(cur_days) {
                    let d = (cell_idx - start_offset + 1) as u8;
                    (BillingDate::new(view_year, view_month, d), true)
                } else {
                    let d = (cell_idx - start_offset - usize::from(cur_days) + 1) as u8;
                    (
                        BillingDate::new(next_month_date.year, next_month_date.month, d),
                        false,
                    )
                };

                let picked = cell_date == self.start;
                let day_btn = activate(
                    div()
                        .id(pro_eid(&self.id, format!("cal-d-{cell_idx}")))
                        .role(Role::Button)
                        .aria_label(cell_date.to_iso())
                        .tab_index(0)
                        .flex_1()
                        .h(px(30.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.5))
                        .font_weight(if picked {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .bg(if picked {
                            p.primary
                        } else {
                            transparent_black()
                        })
                        .text_color(if picked {
                            p.on_primary
                        } else if in_month {
                            p.ink
                        } else {
                            p.faint
                        })
                        .cursor_pointer()
                        .when(!picked, move |s| s.hover(move |h| h.bg(p.hover)))
                        .child(cell_date.day.to_string()),
                    cx,
                    move |this, _, cx| {
                        this.set_start_date(cell_date, cx);
                    },
                );
                row = row.child(day_btn);
            }
            grid = grid.child(row);
        }

        div()
            .w(px(252.))
            .p(px(8.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .flex()
            .flex_col()
            .gap(px(4.))
            .child(
                div()
                    .w_full()
                    .pl(px(6.))
                    .pb(px(2.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(month_heading),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .child(prev_btn)
                            .child(next_btn),
                    ),
            )
            .child(dow_row)
            .child(grid)
            .into_any_element()
    }

    fn render_line_row(
        &self,
        line: &ScheduleLine,
        narrow: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let lid = line.id.clone();
        let is_hovered = self.hovered_row.as_ref() == Some(&lid);
        let slot = self.line_editors.iter().find(|s| s.line_id == lid);
        let currency = self.currency.clone();

        let remove_btn = div()
            .flex_none()
            .opacity(if is_hovered || narrow { 1.0 } else { 0.48 })
            .hover(|s| s.opacity(1.0))
            .child(pro_icon_btn(
                pro_eid(&self.id, format!("rm-{lid}")),
                format!("{} {}", tr(cx, "Remove", "移除"), lid),
                ProIcon::Close,
                26.,
                false,
                cx,
                {
                    let lid = lid.clone();
                    move |this, _, cx| {
                        this.remove_line(&lid, cx);
                    }
                },
            ));

        let (name_cell, repeat_cell, price_cell): (AnyElement, AnyElement, AnyElement) = match &line
            .kind
        {
            ScheduleLineKind::Product {
                name,
                name_zh,
                repeat,
                timing,
                price,
            } => {
                let display_name = tr(cx, name.as_ref(), name_zh.as_ref()).to_string();
                let repeat_open = self.popover == Some(ActivePopover::LineRepeat(lid.clone()));
                let timing_open = self.popover == Some(ActivePopover::LineTiming(lid.clone()));
                let rep_val = *repeat;
                let tim_val = *timing;
                let occ = rep_val.occurrences(self.duration);
                let safe_unit = if price.is_finite() {
                    price.max(0.0)
                } else {
                    0.0
                };
                let line_subtotal = safe_unit * f64::from(occ);

                let name_el = div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(display_name),
                    )
                    .when(occ > 1, |d| {
                        d.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .text_size(px(11.))
                                .text_color(p.muted)
                                .child(format!("{occ}× ·"))
                                .child(pro_rolling_number(
                                    pro_eid(&self.id, format!("roll-line-{lid}")),
                                    line_subtotal,
                                    2,
                                    currency.clone(),
                                    "",
                                    11.,
                                    FontWeight::MEDIUM,
                                    p.text,
                                )),
                        )
                    })
                    .into_any_element();

                let repeat_btn = pro_tactile(
                    pro_eid(&self.id, format!("rep-btn-{lid}")),
                    format!(
                        "{} {}",
                        tr(cx, "Repeats:", "计费频率:"),
                        rep_val.short_label(cx)
                    ),
                    6.,
                    if repeat_open {
                        p.chip
                    } else {
                        transparent_black()
                    },
                    p.chip,
                    transparent_black(),
                    transparent_black(),
                    repeat_open,
                    div()
                        .h(px(24.))
                        .px(px(6.))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(12.5))
                        .text_color(if repeat_open { p.ink } else { p.text })
                        .whitespace_nowrap()
                        .child(pro_svg("0 0 24 24", SVG_REPEAT, 13.5, p.muted))
                        .child(pro_swap_label(
                            pro_eid(&self.id, format!("rep-lbl-{lid}")),
                            rep_val.short_label(cx),
                            12.5,
                            FontWeight::NORMAL,
                            if repeat_open { p.ink } else { p.text },
                        ))
                        .child(pro_icon(
                            if repeat_open {
                                ProIcon::ChevronUp
                            } else {
                                ProIcon::ChevronDown
                            },
                            12.,
                            p.muted,
                        )),
                    cx,
                    {
                        let lid = lid.clone();
                        move |this, _, cx| {
                            let target = ActivePopover::LineRepeat(lid.clone());
                            this.popover = if this.popover == Some(target.clone()) {
                                None
                            } else {
                                Some(target)
                            };
                            cx.notify();
                        }
                    },
                )
                .flex_none();

                let mut repeat_menu = div()
                    .w(px(180.))
                    .p(px(4.))
                    .rounded(px(9.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .gap(px(1.));
                for opt in RepeatCadence::ALL {
                    let active = opt == rep_val;
                    let lid_opt = lid.clone();
                    repeat_menu = repeat_menu.child(activate(
                        div()
                            .id(pro_eid(
                                &self.id,
                                format!("rep-opt-{lid}-{}", opt.short_label(cx)),
                            ))
                            .role(Role::Button)
                            .aria_label(opt.menu_label(cx))
                            .tab_index(0)
                            .h(px(28.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(12.5))
                            .text_color(p.ink)
                            .cursor_pointer()
                            .hover(move |s| s.bg(p.hover))
                            .child(opt.menu_label(cx))
                            .child(pro_check_draw(
                                pro_eid(&self.id, format!("rep-chk-{lid}-{}", opt.short_label(cx))),
                                active,
                                13.,
                                false,
                                p.blue,
                            )),
                        cx,
                        move |this, _, cx| {
                            this.set_line_repeat(&lid_opt, opt, cx);
                        },
                    ));
                }

                let repeat_wrap = div()
                    .w(px(132.))
                    .flex_none()
                    .relative()
                    .flex()
                    .items_center()
                    .child(repeat_btn)
                    .child(pro_popover_presence(
                        pro_eid(&self.id, format!("rep-pop-{lid}")),
                        repeat_open,
                        move || {
                            div()
                                .occlude()
                                .absolute()
                                .top(px(28.))
                                .left_0()
                                .child(repeat_menu)
                                .into_any_element()
                        },
                    ))
                    .into_any_element();

                let val_box = if let Some(s) = slot {
                    div()
                        .w(px(74.))
                        .h(px(24.))
                        .px(px(5.))
                        .rounded(px(5.))
                        .bg(p.subtle)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .child(s.value_editor.clone())
                        .into_any_element()
                } else {
                    div().into_any_element()
                };

                let unit_sfx = rep_val.unit_suffix(cx);
                let timing_btn = pro_tactile(
                    pro_eid(&self.id, format!("tim-btn-{lid}")),
                    tim_val.inline_label(cx),
                    5.,
                    if timing_open {
                        p.chip
                    } else {
                        transparent_black()
                    },
                    p.chip,
                    transparent_black(),
                    transparent_black(),
                    timing_open,
                    div()
                        .h(px(20.))
                        .px(px(5.))
                        .flex()
                        .items_center()
                        .gap(px(3.))
                        .text_size(px(11.5))
                        .text_color(if timing_open { p.ink } else { p.muted })
                        .whitespace_nowrap()
                        .child(pro_swap_label(
                            pro_eid(&self.id, format!("tim-lbl-{lid}")),
                            tim_val.inline_label(cx),
                            11.5,
                            FontWeight::NORMAL,
                            if timing_open { p.ink } else { p.muted },
                        ))
                        .child(pro_icon(ProIcon::ChevronDown, 11., p.muted)),
                    cx,
                    {
                        let lid = lid.clone();
                        move |this, _, cx| {
                            let target = ActivePopover::LineTiming(lid.clone());
                            this.popover = if this.popover == Some(target.clone()) {
                                None
                            } else {
                                Some(target)
                            };
                            cx.notify();
                        }
                    },
                )
                .flex_none();

                let mut timing_menu = div()
                    .w(px(176.))
                    .p(px(4.))
                    .rounded(px(9.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .gap(px(1.));
                for opt in BillingTiming::ALL {
                    let active = opt == tim_val;
                    let lid_opt = lid.clone();
                    timing_menu = timing_menu.child(activate(
                        div()
                            .id(pro_eid(
                                &self.id,
                                format!("tim-opt-{lid}-{}", opt.menu_label(cx)),
                            ))
                            .role(Role::Button)
                            .aria_label(opt.menu_label(cx))
                            .tab_index(0)
                            .h(px(28.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(12.5))
                            .text_color(p.ink)
                            .cursor_pointer()
                            .hover(move |s| s.bg(p.hover))
                            .child(opt.menu_label(cx))
                            .child(pro_check_draw(
                                pro_eid(&self.id, format!("tim-chk-{lid}-{}", opt.menu_label(cx))),
                                active,
                                13.,
                                false,
                                p.blue,
                            )),
                        cx,
                        move |this, _, cx| {
                            this.set_line_timing(&lid_opt, opt, cx);
                        },
                    ));
                }

                let price_el = div()
                    .w(if narrow { px(220.) } else { px(214.) })
                    .flex_none()
                    .flex()
                    .flex_col()
                    .items_end()
                    .gap(px(2.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(currency),
                            )
                            .child(val_box)
                            .when(!unit_sfx.is_empty(), |d| {
                                d.child(
                                    div().text_size(px(12.)).text_color(p.muted).child(unit_sfx),
                                )
                            })
                            .child(remove_btn),
                    )
                    .child(div().pr(px(30.)).relative().child(timing_btn).child(
                        pro_popover_presence(
                            pro_eid(&self.id, format!("tim-pop-{lid}")),
                            timing_open,
                            move || {
                                div()
                                    .occlude()
                                    .absolute()
                                    .top(px(24.))
                                    .right_0()
                                    .child(timing_menu)
                                    .into_any_element()
                            },
                        ),
                    ))
                    .into_any_element();

                (name_el, repeat_wrap, price_el)
            }
            ScheduleLineKind::Discount { .. } => {
                let code_chip = if let Some(ced) = slot.and_then(|s| s.code_editor.clone()) {
                    div()
                        .w(px(78.))
                        .h(px(21.))
                        .px(px(5.))
                        .rounded(px(5.))
                        .bg(p.accent_soft)
                        .border_1()
                        .border_color(p.blue.opacity(0.32))
                        .flex()
                        .items_center()
                        .text_color(p.blue)
                        .child(ced)
                        .into_any_element()
                } else {
                    div().into_any_element()
                };

                let name_el = div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .child(
                        div()
                            .size(px(20.))
                            .flex_none()
                            .rounded(px(5.))
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_svg("0 0 24 24", SVG_DISCOUNT, 13.5, p.blue)),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tr(cx, "Discount", "优惠折扣")),
                    )
                    .child(code_chip)
                    .into_any_element();

                let rep_el = div()
                    .w(px(132.))
                    .flex_none()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(tr(cx, "Entire phase", "整个阶段"))
                    .into_any_element();

                let val_box = if let Some(s) = slot {
                    div()
                        .w(px(48.))
                        .h(px(24.))
                        .px(px(5.))
                        .rounded(px(5.))
                        .bg(p.subtle)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .child(s.value_editor.clone())
                        .into_any_element()
                } else {
                    div().into_any_element()
                };

                let price_el = div()
                    .w(if narrow { px(190.) } else { px(214.) })
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(4.))
                    .child(val_box)
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "% off", "% 折扣")),
                    )
                    .child(remove_btn)
                    .into_any_element();

                (name_el, rep_el, price_el)
            }
            ScheduleLineKind::Minimum { .. } => {
                let name_el = div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .size(px(20.))
                            .flex_none()
                            .rounded(px(5.))
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_svg("0 0 24 24", SVG_MINIMUM, 13.5, p.muted)),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tr(cx, "Minimum spend", "最低消费承诺")),
                    )
                    .into_any_element();

                let rep_el = div()
                    .w(px(132.))
                    .flex_none()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(tr(cx, "Phase floor", "阶段保底"))
                    .into_any_element();

                let val_box = if let Some(s) = slot {
                    div()
                        .w(px(78.))
                        .h(px(24.))
                        .px(px(5.))
                        .rounded(px(5.))
                        .bg(p.subtle)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .child(s.value_editor.clone())
                        .into_any_element()
                } else {
                    div().into_any_element()
                };

                let price_el = div()
                    .w(if narrow { px(190.) } else { px(214.) })
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(currency),
                    )
                    .child(val_box)
                    .child(remove_btn)
                    .into_any_element();

                (name_el, rep_el, price_el)
            }
        };

        let lid_hov = lid.clone();
        div()
            .id(pro_eid(&self.id, format!("row-{lid}")))
            .w_full()
            .min_h(px(52.))
            .pl(px(14.))
            .pr(px(10.))
            .py(px(7.))
            .border_b_1()
            .border_color(p.line)
            .bg(if is_hovered {
                p.hover
            } else {
                transparent_black()
            })
            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                if *inside {
                    this.hovered_row = Some(lid_hov.clone());
                } else if this.hovered_row.as_ref() == Some(&lid_hov) {
                    this.hovered_row = None;
                }
                cx.notify();
            }))
            .flex()
            .when(narrow, |d| d.flex_col().items_stretch().gap(px(6.)))
            .when(!narrow, |d| d.items_center().gap(px(10.)))
            .child(name_cell)
            .when(!narrow, |d| d.child(repeat_cell))
            .child(price_cell)
            .into_any_element()
    }
}

impl Render for BillingSchedule {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let width_cell = self.width_cell.clone();
        let measured_w = width_cell.get();
        let narrow = measured_w < 500.;

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
                .max_w(px(560.))
                .mx_auto()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(pro_skeleton_bar(190., 16., 5., phase, cx))
                .child(pro_skeleton_bar(260., 12., 4., phase, cx))
                .child(
                    div()
                        .flex()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(240., 32., 8., phase, cx))
                        .child(pro_skeleton_bar(240., 32., 8., phase, cx)),
                )
                .child(pro_skeleton_bar(520., 56., 6., phase, cx))
                .child(pro_skeleton_bar(520., 38., 6., phase, cx))
                .into_any_element();
        }

        let heading = tr(cx, self.title.as_ref(), self.title_zh.as_ref()).to_string();
        let start_display = self.start.display(cx);
        let end_summary = match self.duration.months() {
            Some(m) => self.start.add_months(m as i32).display(cx),
            None => self.duration.label(cx).to_string(),
        };
        let end_state_key = format!("{}-{}", self.start.to_iso(), self.duration.key());

        let phase_box = if let Some(pe) = &self.phase_editor {
            div()
                .w(px(132.))
                .h(px(22.))
                .px(px(5.))
                .ml(px(-4.))
                .rounded(px(5.))
                .hover(move |s| s.bg(p.hover))
                .flex()
                .items_center()
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink)
                .child(pe.clone())
                .into_any_element()
        } else {
            div().child(self.phase.clone()).into_any_element()
        };

        let header = div()
            .w_full()
            .pt(px(14.))
            .px(px(14.))
            .pb(px(12.))
            .flex()
            .flex_col()
            .gap(px(3.))
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(heading),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .text_color(p.text)
                    .child(phase_box)
                    .child(div().text_color(p.faint).child("·"))
                    .child(div().child(start_display.clone()))
                    .child(pro_icon(ProIcon::ArrowRight, 13.5, p.faint))
                    .child(div().w(px(110.)).child(pro_view_transition(
                        pro_eid(&self.id, "end-tx"),
                        end_state_key,
                        div().text_color(p.text).child(end_summary),
                    ))),
            );

        let cal_open = matches!(self.popover, Some(ActivePopover::Calendar { .. }));
        let (cal_y, cal_m) = match self.popover {
            Some(ActivePopover::Calendar {
                view_year,
                view_month,
            }) => (view_year, view_month),
            _ => (self.start.year, self.start.month),
        };
        let cal_pop = self.render_calendar_popover(cal_y, cal_m, cx);

        let start_field = div()
            .flex_1()
            .min_w_0()
            .relative()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(div().text_size(px(12.)).text_color(p.text).child(tr(
                cx,
                "Start date",
                "开始日期",
            )))
            .child(pro_tactile(
                pro_eid(&self.id, "start-btn"),
                format!("{}: {}", tr(cx, "Start date", "开始日期"), start_display),
                8.,
                p.card,
                p.card,
                if cal_open { p.line_strong } else { p.line },
                p.line_strong,
                cal_open,
                div()
                    .w_full()
                    .h(px(32.))
                    .pl(px(10.))
                    .pr(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(13.))
                    .text_color(p.ink)
                    .child(pro_swap_label(
                        pro_eid(&self.id, "start-lbl"),
                        start_display,
                        13.,
                        FontWeight::NORMAL,
                        p.ink,
                    ))
                    .child(pro_icon(ProIcon::Calendar, 15.5, p.muted)),
                cx,
                |this, _, cx| {
                    this.popover = if matches!(this.popover, Some(ActivePopover::Calendar { .. })) {
                        None
                    } else {
                        Some(ActivePopover::Calendar {
                            view_year: this.start.year,
                            view_month: this.start.month,
                        })
                    };
                    cx.notify();
                },
            ))
            .child(pro_popover_presence(
                pro_eid(&self.id, "cal-pres"),
                cal_open,
                move || {
                    div()
                        .occlude()
                        .absolute()
                        .top(px(58.))
                        .left_0()
                        .child(cal_pop)
                        .into_any_element()
                },
            ));

        let dur_open = self.popover == Some(ActivePopover::Duration);
        let dur_label = self.duration.label(cx);
        let mut dur_menu = div()
            .w(px(236.))
            .p(px(4.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .flex()
            .flex_col()
            .gap(px(1.));
        for opt in PhaseDuration::ALL {
            let selected = opt == self.duration;
            dur_menu = dur_menu.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("dur-opt-{}", opt.key())))
                    .role(Role::Button)
                    .aria_label(opt.label(cx))
                    .tab_index(0)
                    .min_h(px(30.))
                    .px(px(8.))
                    .py(px(4.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(opt.label(cx)),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(p.muted)
                                    .child(opt.hint(cx)),
                            ),
                    )
                    .child(pro_check_draw(
                        pro_eid(&self.id, format!("dur-chk-{}", opt.key())),
                        selected,
                        13.5,
                        false,
                        p.blue,
                    )),
                cx,
                move |this, _, cx| {
                    this.set_duration(opt, cx);
                },
            ));
        }

        let dur_rest_bg = if self.duration != PhaseDuration::Milestone {
            p.accent_soft
        } else {
            p.card
        };
        let dur_rest_bdr: Hsla = if dur_open {
            p.line_strong
        } else if self.duration != PhaseDuration::Milestone {
            p.blue.opacity(0.38)
        } else {
            p.line
        };
        let dur_field = div()
            .flex_1()
            .min_w_0()
            .relative()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(div().text_size(px(12.)).text_color(p.text).child(tr(
                cx,
                "Duration",
                "持续周期",
            )))
            .child(pro_tactile(
                pro_eid(&self.id, "dur-btn"),
                format!("{}: {}", tr(cx, "Duration", "持续周期"), dur_label),
                8.,
                dur_rest_bg,
                dur_rest_bg,
                dur_rest_bdr,
                p.line_strong,
                dur_open,
                div()
                    .w_full()
                    .h(px(32.))
                    .pl(px(10.))
                    .pr(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(13.))
                    .text_color(p.ink)
                    .child(pro_swap_label(
                        pro_eid(&self.id, "dur-lbl"),
                        dur_label,
                        13.,
                        FontWeight::NORMAL,
                        p.ink,
                    ))
                    .child(pro_icon(
                        if dur_open {
                            ProIcon::ChevronUp
                        } else {
                            ProIcon::ChevronDown
                        },
                        15.,
                        p.muted,
                    )),
                cx,
                |this, _, cx| {
                    this.popover = if this.popover == Some(ActivePopover::Duration) {
                        None
                    } else {
                        Some(ActivePopover::Duration)
                    };
                    cx.notify();
                },
            ))
            .child(pro_popover_presence(
                pro_eid(&self.id, "dur-pres"),
                dur_open,
                move || {
                    div()
                        .occlude()
                        .absolute()
                        .top(px(58.))
                        .right_0()
                        .child(dur_menu)
                        .into_any_element()
                },
            ));

        let fields_row = div()
            .w_full()
            .px(px(14.))
            .pb(px(14.))
            .flex()
            .when(narrow, |d| d.flex_col())
            .gap(px(10.))
            .child(start_field)
            .child(dur_field);

        let reduced = disabled_motion(cx);
        if reduced {
            self.entering_ids.clear();
            self.exiting_lines.clear();
        } else {
            self.exiting_lines
                .retain(|(_, t)| t.elapsed().as_secs_f32() < 0.28);
            if !self.exiting_lines.is_empty() || self.entering_ids.iter().any(|(_, ready)| !*ready)
            {
                w.request_animation_frame();
            }
        }

        let table_section = if self.empty
            || (self.lines.is_empty() && self.exiting_lines.is_empty())
        {
            div()
                .w_full()
                .border_t_1()
                .border_b_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::Coins,
                    tr(cx, "No billing lines yet", "暂无计费明细行"),
                    tr(
                        cx,
                        "Add a product, discount, or minimum commitment to build this billing phase.",
                        "点击下方按钮添加订阅产品、优惠折扣或最低消费承诺。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-add"),
                        SharedString::from(tr(cx, "Add product", "添加产品")),
                        Box::new(|this: &mut Self, _, cx| {
                            this.add_catalog_product(0, cx);
                        }),
                    )),
                    cx,
                ))
                .into_any_element()
        } else {
            let head = div()
                .w_full()
                .h(px(32.))
                .pl(px(14.))
                .pr(px(40.))
                .border_t_1()
                .border_b_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .gap(px(10.))
                .text_size(px(10.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.muted)
                .child(div().flex_1().child(tr(cx, "NAME", "计费项目")))
                .when(!narrow, |d| {
                    d.child(
                        div()
                            .w(px(132.))
                            .flex_none()
                            .child(tr(cx, "REPEATS", "周期")),
                    )
                })
                .child(
                    div()
                        .w(px(184.))
                        .flex_none()
                        .flex()
                        .justify_end()
                        .child(tr(cx, "PRICE", "单价")),
                );

            let lines_snapshot = self.lines.clone();
            let exiting_snapshot: Vec<ScheduleLine> =
                self.exiting_lines.iter().map(|(l, _)| l.clone()).collect();
            let mut rows_col = div().w_full().flex().flex_col();
            for line in &lines_snapshot {
                let lid = line.id.clone();
                let fold_open =
                    if let Some(entry) = self.entering_ids.iter_mut().find(|(id, _)| *id == lid) {
                        if entry.1 {
                            true
                        } else {
                            entry.1 = true;
                            false
                        }
                    } else {
                        true
                    };
                let row_el = self.render_line_row(line, narrow, cx);
                rows_col = rows_col.child(pro_fold(
                    pro_eid(&self.id, format!("line-fold-{lid}")),
                    fold_open,
                    56.,
                    0.,
                    move || row_el,
                ));
            }
            self.entering_ids.retain(|(_, ready)| !*ready);
            for line in &exiting_snapshot {
                let lid = line.id.clone();
                let row_el = self.render_line_row(line, narrow, cx);
                rows_col = rows_col.child(pro_fold(
                    pro_eid(&self.id, format!("line-fold-{lid}")),
                    false,
                    56.,
                    0.,
                    move || row_el,
                ));
            }
            div()
                .w_full()
                .flex()
                .flex_col()
                .when(!narrow, |d| d.child(head))
                .when(narrow, |d| d.border_t_1().border_color(p.line))
                .child(rows_col)
                .into_any_element()
        };

        let cat_open = self.popover == Some(ActivePopover::Catalog);
        let mut cat_menu = div()
            .w(px(264.))
            .p(px(4.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .flex()
            .flex_col()
            .gap(px(1.));
        for (idx, item) in self.catalog.iter().enumerate() {
            let cname = tr(cx, item.name.as_ref(), item.name_zh.as_ref()).to_string();
            let price_tag = format!(
                "{}{} {}",
                self.currency,
                format_number(item.price, 2),
                item.repeat.short_label(cx)
            );
            cat_menu = cat_menu.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("cat-item-{idx}")))
                    .role(Role::Button)
                    .aria_label(cname.clone())
                    .tab_index(0)
                    .min_h(px(32.))
                    .px(px(8.))
                    .py(px(4.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(cname),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(11.5))
                            .text_color(p.muted)
                            .child(price_tag),
                    ),
                cx,
                move |this, _, cx| {
                    this.add_catalog_product(idx, cx);
                },
            ));
        }

        let add_product_wrap = div()
            .relative()
            .child(
                pro_tactile(
                    pro_eid(&self.id, "add-prod-btn"),
                    tr(cx, "Add product", "添加产品"),
                    6.,
                    p.card,
                    p.hover,
                    if cat_open { p.line_strong } else { p.line },
                    p.line_strong,
                    cat_open,
                    div()
                        .h(px(28.))
                        .pl(px(8.))
                        .pr(px(10.))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .whitespace_nowrap()
                        .child(pro_icon(ProIcon::Plus, 14., p.muted))
                        .child(tr(cx, "Add product", "添加产品")),
                    cx,
                    |this, _, cx| {
                        this.popover = if this.popover == Some(ActivePopover::Catalog) {
                            None
                        } else {
                            Some(ActivePopover::Catalog)
                        };
                        cx.notify();
                    },
                )
                .flex_none(),
            )
            .child(pro_popover_presence(
                pro_eid(&self.id, "cat-pres"),
                cat_open,
                move || {
                    div()
                        .occlude()
                        .absolute()
                        .bottom(px(34.))
                        .left_0()
                        .child(cat_menu)
                        .into_any_element()
                },
            ));

        let disc_disabled = self.has_discount();
        let add_disc_btn = if disc_disabled {
            div()
                .id(pro_eid(&self.id, "add-disc-btn"))
                .role(Role::Button)
                .aria_label(tr(cx, "Add discount", "添加折扣"))
                .tab_index(-1)
                .h(px(28.))
                .pl(px(8.))
                .pr(px(10.))
                .rounded(px(6.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .opacity(0.42)
                .flex()
                .items_center()
                .gap(px(5.))
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.strong)
                .child(pro_svg("0 0 24 24", SVG_DISCOUNT, 14., p.muted))
                .child(tr(cx, "Add discount", "添加折扣"))
                .into_any_element()
        } else {
            pro_tactile(
                pro_eid(&self.id, "add-disc-btn"),
                tr(cx, "Add discount", "添加折扣"),
                6.,
                p.card,
                p.hover,
                p.line,
                p.line_strong,
                false,
                div()
                    .h(px(28.))
                    .pl(px(8.))
                    .pr(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .whitespace_nowrap()
                    .child(pro_svg("0 0 24 24", SVG_DISCOUNT, 14., p.muted))
                    .child(tr(cx, "Add discount", "添加折扣")),
                cx,
                |this, _, cx| {
                    this.add_discount_line(cx);
                },
            )
            .flex_none()
            .into_any_element()
        };

        let min_disabled = self.has_minimum();
        let add_min_btn = if min_disabled {
            div()
                .id(pro_eid(&self.id, "add-min-btn"))
                .role(Role::Button)
                .aria_label(tr(cx, "Add minimum", "添加保底"))
                .tab_index(-1)
                .h(px(28.))
                .pl(px(8.))
                .pr(px(10.))
                .rounded(px(6.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .opacity(0.42)
                .flex()
                .items_center()
                .gap(px(5.))
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.strong)
                .child(pro_svg("0 0 24 24", SVG_MINIMUM, 14., p.muted))
                .child(tr(cx, "Add minimum", "添加保底"))
                .into_any_element()
        } else {
            pro_tactile(
                pro_eid(&self.id, "add-min-btn"),
                tr(cx, "Add minimum", "添加保底"),
                6.,
                p.card,
                p.hover,
                p.line,
                p.line_strong,
                false,
                div()
                    .h(px(28.))
                    .pl(px(8.))
                    .pr(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .whitespace_nowrap()
                    .child(pro_svg("0 0 24 24", SVG_MINIMUM, 14., p.muted))
                    .child(tr(cx, "Add minimum", "添加保底")),
                cx,
                |this, _, cx| {
                    this.add_minimum_line(cx);
                },
            )
            .flex_none()
            .into_any_element()
        };

        let actions_row = div()
            .w_full()
            .px(px(14.))
            .py(px(10.))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.))
            .child(add_product_wrap)
            .when(!narrow, |d| d.child(div().flex_1()))
            .child(add_disc_btn)
            .child(add_min_btn);

        let totals = self.totals();
        let cur_sym = self.currency.clone();
        let show_disc_pill = totals.phase_discount > 0.005;
        let show_min_pill = totals.minimum_applied;
        let show_subtotal = show_disc_pill || show_min_pill;

        let totals_bar = div()
            .w_full()
            .px(px(14.))
            .py(px(10.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .when(narrow, |d| d.flex_col().items_stretch().gap(px(6.)))
            .when(!narrow, |d| {
                d.flex_wrap().items_center().justify_between().gap(px(12.))
            })
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .justify_between()
                    .gap(px(8.))
                    .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                        cx,
                        "First invoice",
                        "首期账单",
                    )))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "roll-first"),
                        totals.first_invoice,
                        2,
                        cur_sym.clone(),
                        "",
                        15.,
                        FontWeight::MEDIUM,
                        p.strong,
                    )),
            )
            .when(show_subtotal, |d| {
                d.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(px(6.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .px(px(6.))
                                .py(px(2.))
                                .rounded(px(5.))
                                .bg(p.chip)
                                .text_size(px(11.))
                                .text_color(p.muted)
                                .child(tr(cx, "Subtotal", "阶段小计"))
                                .child(pro_rolling_number(
                                    pro_eid(&self.id, "roll-sub"),
                                    totals.phase_subtotal,
                                    2,
                                    cur_sym.clone(),
                                    "",
                                    11.,
                                    FontWeight::MEDIUM,
                                    p.text,
                                )),
                        )
                        .when(show_disc_pill, |el| {
                            el.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .px(px(6.))
                                    .py(px(2.))
                                    .rounded(px(5.))
                                    .bg(p.accent_soft)
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.blue)
                                    .child(tr(cx, "Saved", "已减免"))
                                    .child(pro_rolling_number(
                                        pro_eid(&self.id, "roll-disc"),
                                        totals.phase_discount,
                                        2,
                                        cur_sym.clone(),
                                        "",
                                        11.,
                                        FontWeight::MEDIUM,
                                        p.blue,
                                    )),
                            )
                        })
                        .when(show_min_pill, |el| {
                            el.child(
                                div()
                                    .px(px(6.))
                                    .py(px(2.))
                                    .rounded(px(5.))
                                    .bg(p.accent_soft)
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.blue)
                                    .child(tr(cx, "Min floor active", "已触发保底额")),
                            )
                        }),
                )
            })
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .justify_between()
                    .gap(px(8.))
                    .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                        cx,
                        "Phase total",
                        "阶段总计",
                    )))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "roll-phase"),
                        totals.phase_total,
                        2,
                        cur_sym,
                        "",
                        15.,
                        FontWeight::MEDIUM,
                        p.strong,
                    )),
            );

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                if ev.keystroke.key == "escape" && this.popover.is_some() {
                    this.popover = None;
                    cx.notify();
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
                    .max_w(px(560.))
                    .mx_auto()
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .when(!p.dark, |d| d.shadow_xs())
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(fields_row)
                    .child(table_section)
                    .child(actions_row)
                    .child(totals_bar),
            )
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

fn line_formatted_value(line: &ScheduleLine) -> String {
    match &line.kind {
        ScheduleLineKind::Product { price, .. } => format_number(*price, 2),
        ScheduleLineKind::Discount { percent, .. } => format_clean_pct(*percent),
        ScheduleLineKind::Minimum { amount } => format_number(*amount, 2),
    }
}

fn format_clean_pct(pct: f64) -> String {
    if (pct.fract()).abs() < 0.005 {
        format!("{:.0}", pct)
    } else {
        format!("{:.1}", pct)
    }
}

fn format_number(value: f64, decimals: usize) -> String {
    let safe = if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    };
    let raw = format!("{safe:.decimals$}");
    let (int_part, frac_part) = match raw.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (raw.as_str(), None),
    };
    let mut grouped = String::with_capacity(int_part.len() + int_part.len() / 3);
    for (idx, ch) in int_part.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let grouped_int: String = grouped.chars().rev().collect();
    match frac_part {
        Some(f) => format!("{grouped_int}.{f}"),
        None => grouped_int,
    }
}

fn default_schedule_lines() -> Vec<ScheduleLine> {
    vec![
        ScheduleLine::product(
            "setup",
            "Set-up fee",
            RepeatCadence::Once,
            BillingTiming::InAdvance,
            4_999.00,
        )
        .with_name_zh("实施部署费"),
    ]
}

fn default_catalog() -> Vec<BillingCatalogItem> {
    vec![
        BillingCatalogItem::new(
            "platform",
            "Platform license",
            RepeatCadence::Monthly,
            1_200.00,
        )
        .with_name_zh("平台基础订阅许可"),
        BillingCatalogItem::new(
            "seats",
            "Enterprise seats (25)",
            RepeatCadence::Monthly,
            750.00,
        )
        .with_name_zh("企业版席位包 (25 席)"),
        BillingCatalogItem::new(
            "support",
            "Dedicated support",
            RepeatCadence::Quarterly,
            2_400.00,
        )
        .with_name_zh("专属技术支持"),
        BillingCatalogItem::new(
            "setup-kit",
            "Set-up & onboarding",
            RepeatCadence::Once,
            4_999.00,
        )
        .with_name_zh("实施与导入服务"),
        BillingCatalogItem::new("soc2", "Compliance & SSO", RepeatCadence::Yearly, 3_600.00)
            .with_name_zh("合规审计与单点登录"),
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        BillingDate, BillingSchedule, BillingTiming, PhaseDuration, RepeatCadence, format_number,
    };
    use gpui::{AppContext, TestAppContext};

    #[test]
    fn date_math_clamps_month_ends_and_formats_iso() {
        let jan_end = BillingDate::parse_iso("2026-01-31").expect("valid iso date");
        let feb_end = jan_end.add_months(1);
        assert_eq!(feb_end.to_iso(), "2026-02-28");

        let leap = BillingDate::new(2028, 1, 31).add_months(1);
        assert_eq!(leap.to_iso(), "2028-02-29");
        assert_eq!(format_number(4_999.0, 2), "4,999.00");
    }

    #[gpui::test]
    fn adding_products_discounts_and_minimum_updates_first_and_phase_totals(
        cx: &mut TestAppContext,
    ) {
        let sched = cx.new(|cx| BillingSchedule::new("test-billing-schedule", cx));

        sched.update(cx, |bs, cx| {
            let init = bs.totals();
            assert!((init.first_invoice - 4_999.0).abs() < 1e-4);
            assert!((init.phase_total - 4_999.0).abs() < 1e-4);

            // Switch to 12-month term and add Platform license ($1,200 / mo -> $14,400 phase)
            bs.set_duration(PhaseDuration::TwelveMonths, cx);
            assert!(bs.add_catalog_product(0, cx));
            let after_prod = bs.totals();
            assert!((after_prod.first_invoice - 6_199.0).abs() < 1e-4);
            assert!((after_prod.phase_total - 19_399.0).abs() < 1e-4);

            // Defer set-up fee to in-arrears: drops off day-one first invoice
            assert!(bs.set_line_timing("setup", BillingTiming::InArrears, cx));
            assert!((bs.totals().first_invoice - 1_200.0).abs() < 1e-4);
            assert!((bs.totals().phase_total - 19_399.0).abs() < 1e-4);

            // Add 15% discount -> 19,399 * 0.85 = 16,489.15
            assert!(bs.add_discount_line(cx));
            assert!(!bs.add_discount_line(cx));
            assert_eq!(
                RepeatCadence::Quarterly.occurrences(PhaseDuration::TwelveMonths),
                4
            );
            assert!((bs.totals().phase_total - 16_489.15).abs() < 1e-3);

            // Add minimum floor and raise it to $25,000
            assert!(bs.add_minimum_line(cx));
            let min_id = bs.lines().last().expect("minimum line").id.to_string();
            assert!(bs.set_line_amount(&min_id, 25_000.0, None, cx));
            let floored = bs.totals();
            assert!(floored.minimum_applied);
            assert!((floored.phase_total - 25_000.0).abs() < 1e-4);
        });
    }
}
