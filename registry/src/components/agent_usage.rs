//! Monthly agent spend forecast, project and model cost breakdown, and anomaly inspector.
//!
//! Displays cumulative day-by-day spend against an editable budget cap, end-of-month
//! pace projection, breakdown bars by project or model provider, notable spend anomalies,
//! and a right-hand details inspector with daily bars and highest-cost tasks.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_app_tile, pro_btn, pro_ease, pro_eid, pro_empty_state,
            pro_mark_selected, pro_pill_indicator, pro_presence, pro_selection_sensor,
            pro_skeleton_bar, pro_spring_bar, pro_spring_f32, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const SVG_TREND_UP: &str = r#"<path d="M3 17L7 13C7.88256 12.1174 8.32385 11.6762 8.86543 11.6274C8.95496 11.6193 9.04504 11.6193 9.13457 11.6274C9.67615 11.6762 10.1174 12.1174 11 13C11.8826 13.8826 12.3238 14.3238 12.8654 14.3726C12.955 14.3807 13.045 14.3807 13.1346 14.3726C13.6762 14.3238 14.1174 13.8826 15 13L20 8" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M16 7.27657C16 7.27657 20.101 6.65426 20.7234 7.27661C21.3458 7.89896 20.7234 12 20.7234 12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

/// Active grouping dimension for the `Where it went` breakdown card.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UsageBreakdownView {
    /// Group spend by workspace project.
    #[default]
    Projects,
    /// Group spend by model provider (`Claude`, `OpenAI`, `Gemini`).
    Models,
}

/// Selected time window for spend aggregation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UsagePeriod {
    /// Rolling last 7 days.
    SevenDays,
    /// Rolling last 30 days.
    ThirtyDays,
    /// Current monthly billing cycle (`November`).
    #[default]
    BillingCycle,
}

impl UsagePeriod {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::SevenDays => tr(cx, "7d", "近 7 天"),
            Self::ThirtyDays => tr(cx, "30d", "近 30 天"),
            Self::BillingCycle => tr(cx, "Billing cycle", "账单周期"),
        }
    }
}

/// Palette tone for project or model breakdown bars.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UsageGroupTone {
    #[default]
    Orange,
    Purple,
    Teal,
    Blue,
    Amber,
    Pink,
    Green,
    Gray,
}

impl UsageGroupTone {
    fn resolve(self, p: &ProPalette) -> Hsla {
        match self {
            Self::Orange => p.orange,
            Self::Purple => p.purple,
            Self::Teal => p.teal,
            Self::Blue => p.blue,
            Self::Amber => p.amber,
            Self::Pink => p.pink,
            Self::Green => p.green,
            Self::Gray => p.faint,
        }
    }
}

/// Top-cost task entry inside a project or model inspector.
#[derive(Clone, Debug, PartialEq)]
pub struct UsageTaskCost {
    pub title: SharedString,
    pub runs: u32,
    pub cost: f64,
}

impl UsageTaskCost {
    /// Construct a task cost breakdown row.
    pub fn new(title: impl Into<SharedString>, runs: u32, cost: f64) -> Self {
        Self {
            title: title.into(),
            runs,
            cost,
        }
    }
}

/// Spend breakdown group representing either a project or an AI model.
#[derive(Clone, Debug, PartialEq)]
pub struct UsageBreakdownItem {
    pub id: SharedString,
    pub name: SharedString,
    /// Optional provider brand mark (`"Claude"`, `"OpenAI"`, `"Gemini"`).
    pub provider_app: Option<SharedString>,
    pub amount: f64,
    pub change_pct: i32,
    pub tone: UsageGroupTone,
    pub daily_costs: Vec<f64>,
    pub top_tasks: Vec<UsageTaskCost>,
}

/// Category of a notable spend observation in `Worth a look`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UsageNoteKind {
    /// Single-day or single-task cost spike.
    #[default]
    Spike,
    /// Month-over-month spend jump.
    Jump,
    /// Project approaching its soft budget allocation.
    Budget,
}

/// Actionable spend observation surfaced under `Worth a look`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageNote {
    pub id: SharedString,
    pub kind: UsageNoteKind,
    pub text: SharedString,
    pub detail: SharedString,
    pub badge: SharedString,
    pub target_view: UsageBreakdownView,
    pub target_id: SharedString,
}

/// Summary counters displayed along the foot of the monthly spend card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsageSummaryStats {
    pub tasks: u32,
    pub runs: u32,
    pub tokens: u64,
}

impl Default for UsageSummaryStats {
    fn default() -> Self {
        Self {
            tasks: 186,
            runs: 1_284,
            tokens: 48_200_000,
        }
    }
}

/// Notification payload for a newly completed run increment.
#[derive(Clone, Debug, PartialEq)]
pub struct UsageLatestRun {
    pub id: SharedString,
    pub task: SharedString,
    pub cost: f64,
}

/// Events emitted by [`AgentUsageScreen`].
#[derive(Clone, Debug, PartialEq)]
pub enum AgentUsageEvent {
    /// Sidebar navigation item clicked.
    Navigate(SharedString),
    /// Breakdown view or selected group changed.
    Select {
        view: UsageBreakdownView,
        id: SharedString,
    },
    /// `Open` clicked on an anomaly note.
    OpenNote(SharedString),
    /// Reporting period switched (`7d`, `30d`, `BillingCycle`).
    PeriodChanged(UsagePeriod),
    /// Monthly budget alert cap updated (`None` when cleared).
    BudgetChanged(Option<f64>),
}

impl EventEmitter<AgentUsageEvent> for AgentUsageScreen {}

/// Forecast calculation result for a monthly spend series.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpendForecast {
    pub spent_so_far: f64,
    pub projected_total: f64,
    pub budget_pct: Option<u32>,
    /// 1-based day of the month on which cumulative spend crosses `budget` at the current pace.
    pub over_budget_day: Option<usize>,
}

/// Compute cumulative spend, linear end-of-month pace forecast, and budget crossing day.
pub fn compute_spend_forecast(
    days: &[f64],
    days_in_month: usize,
    budget: Option<f64>,
) -> SpendForecast {
    let spent_so_far: f64 = days.iter().copied().sum();
    let elapsed = days.len();
    let total_days = days_in_month.max(elapsed).max(1);
    let daily_pace = if elapsed > 0 {
        spent_so_far / elapsed as f64
    } else {
        0.0
    };
    let projected_total = if elapsed >= total_days {
        spent_so_far
    } else {
        spent_so_far + daily_pace * (total_days - elapsed) as f64
    };

    let budget_pct = budget.and_then(|b| {
        if b > 0.0 {
            Some(((spent_so_far / b) * 100.0).round() as u32)
        } else {
            None
        }
    });

    let over_budget_day = budget.and_then(|b| {
        if b <= 0.0 || projected_total <= b {
            return None;
        }
        let mut acc = 0.0;
        for (idx, &d) in days.iter().enumerate() {
            acc += d;
            if acc >= b {
                return Some(idx + 1);
            }
        }
        if daily_pace > 1e-6 {
            let rem = b - spent_so_far;
            let extra_days = (rem / daily_pace).ceil().max(1.0) as usize;
            Some((elapsed + extra_days).min(total_days))
        } else {
            None
        }
    });

    SpendForecast {
        spent_so_far,
        projected_total,
        budget_pct,
        over_budget_day,
    }
}

fn currency_symbol(code: &str) -> &str {
    match code.trim().to_ascii_uppercase().as_str() {
        "EUR" => "€",
        "USD" => "$",
        "GBP" => "£",
        "JPY" | "CNY" => "¥",
        _ => "€",
    }
}

fn format_money(amount: f64, currency: &str) -> String {
    let sym = currency_symbol(currency);
    let zero_dec = matches!(currency.trim().to_ascii_uppercase().as_str(), "JPY");
    if zero_dec {
        format!("{sym}{}", format_int_grouped(amount.round() as u64))
    } else if amount >= 1_000_000.0 {
        format!("{sym}{:.2}M", amount / 1_000_000.0)
    } else {
        format!("{sym}{amount:.2}")
    }
}

fn format_money_whole(amount: f64, currency: &str) -> String {
    let sym = currency_symbol(currency);
    format!(
        "{sym}{}",
        format_int_grouped(amount.round().max(0.0) as u64)
    )
}

fn format_int_grouped(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

fn format_tokens_millions(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

fn format_change(pct: i32) -> String {
    if pct >= 1000 {
        let factor = ((pct + 100) as f64 / 100.0).round() as i32;
        format!("↑{factor}×")
    } else if pct > 0 {
        format!("↑{pct}%")
    } else if pct < 0 {
        format!("↓{}%", pct.abs())
    } else {
        "0%".to_owned()
    }
}

fn default_days() -> Vec<f64> {
    // Sums to €270.40 across Nov 1..Nov 19, matching the exact cumulative SVG curve.
    vec![
        18.10, 12.70, 10.50, 16.90, 18.20, 4.60, 5.50, 19.80, 16.90, 11.00, 14.30, 18.10, 5.10,
        2.10, 16.90, 19.00, 13.10, 28.20, 19.40,
    ]
}

fn default_projects() -> Vec<UsageBreakdownItem> {
    vec![
        UsageBreakdownItem {
            id: "northwind".into(),
            name: "Northwind".into(),
            provider_app: None,
            amount: 118.40,
            change_pct: 23,
            tone: UsageGroupTone::Orange,
            daily_costs: vec![
                7.80, 6.10, 2.20, 4.30, 8.20, 3.00, 1.30, 8.70, 10.00, 5.20, 3.90, 6.90, 2.60,
                0.50, 4.30, 8.70, 6.90, 18.40, 9.40,
            ],
            top_tasks: vec![
                UsageTaskCost::new("Competitor scan", 6, 31.20),
                UsageTaskCost::new("Ingest pipeline", 41, 27.80),
                UsageTaskCost::new("Client dashboard", 18, 22.60),
                UsageTaskCost::new("Weekly update", 3, 4.10),
            ],
        },
        UsageBreakdownItem {
            id: "platform".into(),
            name: "Platform".into(),
            provider_app: None,
            amount: 82.10,
            change_pct: -7,
            tone: UsageGroupTone::Purple,
            daily_costs: vec![
                5.80, 4.10, 4.80, 6.40, 5.10, 0.90, 2.20, 6.10, 4.20, 3.30, 5.40, 5.90, 1.40, 0.90,
                6.20, 4.80, 3.40, 5.30, 5.90,
            ],
            top_tasks: vec![
                UsageTaskCost::new("PR security review", 64, 29.40),
                UsageTaskCost::new("Schema migration check", 19, 21.80),
                UsageTaskCost::new("Release changelog", 12, 16.50),
                UsageTaskCost::new("Flaky test triage", 28, 14.40),
            ],
        },
        UsageBreakdownItem {
            id: "evalkit".into(),
            name: "Evalkit".into(),
            provider_app: None,
            amount: 46.30,
            change_pct: 102,
            tone: UsageGroupTone::Teal,
            daily_costs: vec![
                2.40, 1.20, 2.10, 3.80, 2.90, 0.40, 1.10, 2.80, 1.50, 1.40, 3.20, 3.90, 0.70, 0.40,
                4.80, 4.10, 1.90, 3.60, 4.10,
            ],
            top_tasks: vec![
                UsageTaskCost::new("Nightly regression suite", 19, 24.60),
                UsageTaskCost::new("Hallucination judge", 34, 13.90),
                UsageTaskCost::new("Prompt diff benchmark", 9, 7.80),
            ],
        },
        UsageBreakdownItem {
            id: "internal".into(),
            name: "Internal".into(),
            provider_app: None,
            amount: 23.60,
            change_pct: -22,
            tone: UsageGroupTone::Gray,
            daily_costs: vec![
                2.10, 1.30, 1.40, 2.40, 2.00, 0.30, 0.90, 2.20, 1.20, 1.10, 1.80, 1.40, 0.40, 0.30,
                1.60, 1.40, 0.90, 0.90, 0.00,
            ],
            top_tasks: vec![
                UsageTaskCost::new("Support inbox digest", 22, 11.40),
                UsageTaskCost::new("Vendor contract summary", 7, 7.30),
                UsageTaskCost::new("Standup notes", 15, 4.90),
            ],
        },
    ]
}

fn default_models() -> Vec<UsageBreakdownItem> {
    vec![
        UsageBreakdownItem {
            id: "claude-sonnet".into(),
            name: "Claude Sonnet 5.5".into(),
            provider_app: Some("Claude".into()),
            amount: 142.80,
            change_pct: 18,
            tone: UsageGroupTone::Orange,
            daily_costs: vec![
                9.60, 6.80, 5.90, 9.10, 9.80, 2.60, 3.10, 10.40, 9.20, 6.10, 7.90, 9.40, 2.90,
                1.20, 8.60, 9.80, 7.10, 13.40, 9.90,
            ],
            top_tasks: vec![
                UsageTaskCost::new("Ingest pipeline", 41, 27.80),
                UsageTaskCost::new("PR security review", 64, 26.10),
                UsageTaskCost::new("Client dashboard", 18, 22.60),
                UsageTaskCost::new("Flaky test triage", 28, 14.40),
            ],
        },
        UsageBreakdownItem {
            id: "claude-opus".into(),
            name: "Claude Opus 5.5".into(),
            provider_app: Some("Claude".into()),
            amount: 64.20,
            change_pct: 94,
            tone: UsageGroupTone::Purple,
            daily_costs: vec![
                2.10, 1.80, 1.40, 2.60, 2.80, 0.60, 0.80, 3.20, 2.70, 1.90, 2.40, 4.60, 1.10, 0.40,
                5.10, 5.80, 3.90, 12.20, 8.80,
            ],
            top_tasks: vec![
                UsageTaskCost::new("Competitor scan", 6, 31.20),
                UsageTaskCost::new("Nightly regression suite", 19, 21.40),
                UsageTaskCost::new("Schema migration check", 19, 11.60),
            ],
        },
        UsageBreakdownItem {
            id: "openai-gpt".into(),
            name: "OpenAI GPT-5.4".into(),
            provider_app: Some("OpenAI".into()),
            amount: 41.60,
            change_pct: -11,
            tone: UsageGroupTone::Teal,
            daily_costs: vec![
                4.10, 2.70, 2.00, 3.30, 3.60, 0.90, 1.00, 3.90, 3.10, 1.90, 2.50, 2.60, 0.70, 0.30,
                2.10, 2.20, 1.40, 1.80, 1.50,
            ],
            top_tasks: vec![
                UsageTaskCost::new("Hallucination judge", 34, 13.90),
                UsageTaskCost::new("Release changelog", 12, 12.80),
                UsageTaskCost::new("Support inbox digest", 22, 9.40),
            ],
        },
        UsageBreakdownItem {
            id: "gemini-pro".into(),
            name: "Gemini 2.5 Pro".into(),
            provider_app: Some("Gemini".into()),
            amount: 21.80,
            change_pct: -6,
            tone: UsageGroupTone::Blue,
            daily_costs: vec![
                2.30, 1.40, 1.20, 1.90, 2.00, 0.50, 0.60, 2.30, 1.90, 1.10, 1.50, 1.50, 0.40, 0.20,
                1.10, 1.20, 0.70, 0.80, 0.00,
            ],
            top_tasks: vec![
                UsageTaskCost::new("Long-document indexing", 14, 11.90),
                UsageTaskCost::new("Vendor contract summary", 7, 6.40),
                UsageTaskCost::new("Weekly update", 3, 3.50),
            ],
        },
    ]
}

fn default_notes() -> Vec<UsageNote> {
    vec![
        UsageNote {
            id: "note-competitor-scan".into(),
            kind: UsageNoteKind::Spike,
            text: "Competitor scan cost €18.40 yesterday".into(),
            detail: "4× what it usually costs – it read 212 pages instead of 40".into(),
            badge: "4×".into(),
            target_view: UsageBreakdownView::Projects,
            target_id: "northwind".into(),
        },
        UsageNote {
            id: "note-evalkit-jump".into(),
            kind: UsageNoteKind::Jump,
            text: "Evalkit is up 102% on October".into(),
            detail: "Its evals moved to Claude Opus 5.5 on Nov 12".into(),
            badge: "+102%".into(),
            target_view: UsageBreakdownView::Projects,
            target_id: "evalkit".into(),
        },
        UsageNote {
            id: "note-northwind-cap".into(),
            kind: UsageNoteKind::Budget,
            text: "Northwind has used 85% of its €140".into(),
            detail: "At this pace it reaches it around Nov 23".into(),
            badge: "85%".into(),
            target_view: UsageBreakdownView::Projects,
            target_id: "northwind".into(),
        },
    ]
}

/// KUI Pro screen tracking monthly agent spend, forecast pace, model/project breakdown, and cost anomalies.
pub struct AgentUsageScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    focus: FocusHandle,
    standalone: bool,
    month_name: SharedString,
    month_short: SharedString,
    days_in_month: usize,
    days: Vec<f64>,
    budget: Option<f64>,
    currency: SharedString,
    period: UsagePeriod,
    view: UsageBreakdownView,
    selected_id: Option<SharedString>,
    projects: Vec<UsageBreakdownItem>,
    models: Vec<UsageBreakdownItem>,
    notes: Vec<UsageNote>,
    stats: UsageSummaryStats,
    latest_run: Option<UsageLatestRun>,
    scrub_day: Option<usize>,
    panel_scrub_day: Option<usize>,
    editing_budget: bool,
    budget_editor: Option<Entity<Editor>>,
    loading: bool,
    empty: bool,
    measured_width: Rc<Cell<f32>>,
    boot_at: Instant,
    flash_at: Option<Instant>,
    live_seq: u32,
}

impl AgentUsageScreen {
    /// Construct a new [`AgentUsageScreen`] mounted in [`AppShellPreset::Agent`] with `Usage` active.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Agent, shell_cx)
                .with_active("Usage")
                .with_work_area(true, true)
                .with_panel_width(332.);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_inspector(w, cx));
            s
        });

        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                cx.emit(AgentUsageEvent::Navigate(label.clone()));
            }
            AppShellEvent::PanelOpen(_) => {
                cx.notify();
            }
            _ => {}
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            shell,
            focus: cx.focus_handle(),
            standalone: false,
            month_name: "November".into(),
            month_short: "Nov".into(),
            days_in_month: 30,
            days: default_days(),
            budget: Some(500.0),
            currency: "EUR".into(),
            period: UsagePeriod::BillingCycle,
            view: UsageBreakdownView::Projects,
            selected_id: Some("northwind".into()),
            projects: default_projects(),
            models: default_models(),
            notes: default_notes(),
            stats: UsageSummaryStats::default(),
            latest_run: None,
            scrub_day: None,
            panel_scrub_day: None,
            editing_budget: false,
            budget_editor: None,
            loading: false,
            empty: false,
            measured_width: Rc::new(Cell::new(640.)),
            boot_at: Instant::now(),
            flash_at: None,
            live_seq: 1,
        }
    }

    /// Render without the surrounding [`AppShell`] chrome.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Override the daily spend series (last entry represents today).
    pub fn with_days(mut self, days: Vec<f64>, days_in_month: usize) -> Self {
        self.days = days;
        self.days_in_month = days_in_month.max(1);
        self
    }

    /// Set or clear (`None`) the monthly budget cap.
    pub fn with_budget(mut self, budget: Option<f64>) -> Self {
        self.budget = budget;
        self
    }

    /// Set the ISO 4217 currency code (`"EUR"`, `"USD"`, `"GBP"`, `"JPY"`).
    pub fn with_currency(mut self, currency: impl Into<SharedString>) -> Self {
        self.currency = currency.into();
        self
    }

    /// Set the initial reporting period (`SevenDays`, `ThirtyDays`, `BillingCycle`).
    pub fn with_period(mut self, period: UsagePeriod) -> Self {
        self.period = period;
        self
    }

    /// Set which breakdown tab (`Projects` or `Models`) opens first.
    pub fn with_default_view(mut self, view: UsageBreakdownView) -> Self {
        self.view = view;
        self.selected_id = match view {
            UsageBreakdownView::Projects => self.projects.first().map(|p| p.id.clone()),
            UsageBreakdownView::Models => self.models.first().map(|m| m.id.clone()),
        };
        self
    }

    /// Replace the project breakdown items.
    pub fn with_projects(mut self, projects: Vec<UsageBreakdownItem>) -> Self {
        self.projects = projects;
        self
    }

    /// Replace the model breakdown items.
    pub fn with_models(mut self, models: Vec<UsageBreakdownItem>) -> Self {
        self.models = models;
        self
    }

    /// Replace the `Worth a look` anomaly notes.
    pub fn with_notes(mut self, notes: Vec<UsageNote>) -> Self {
        self.notes = notes;
        self
    }

    /// Enable or disable skeleton placeholder rendering.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Enable or disable the zero-spend empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Update loading skeleton state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Update empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        cx.notify();
    }

    /// Return the active monthly budget cap, if configured.
    pub fn budget(&self) -> Option<f64> {
        self.budget
    }

    /// Record a newly completed run, incrementing today's spend and flashing the run badge.
    pub fn record_run(&mut self, task: impl Into<SharedString>, cost: f64, cx: &mut Context<Self>) {
        if cost <= 0.0 {
            return;
        }
        let task_name: SharedString = task.into();
        if let Some(last) = self.days.last_mut() {
            *last += cost;
        } else {
            self.days.push(cost);
        }
        if let Some(proj) = self.projects.first_mut() {
            proj.amount += cost;
            if let Some(d) = proj.daily_costs.last_mut() {
                *d += cost;
            }
        }
        if let Some(mdl) = self.models.first_mut() {
            mdl.amount += cost;
            if let Some(d) = mdl.daily_costs.last_mut() {
                *d += cost;
            }
        }
        self.stats.runs = self.stats.runs.saturating_add(1);
        self.stats.tokens = self.stats.tokens.saturating_add(38_000);
        let seq = self.live_seq;
        self.live_seq = self.live_seq.wrapping_add(1);
        self.latest_run = Some(UsageLatestRun {
            id: format!("run-live-{seq}").into(),
            task: task_name,
            cost,
        });
        self.flash_at = Some(Instant::now());
        cx.notify();
    }

    fn period_scale(&self) -> f64 {
        match self.period {
            UsagePeriod::BillingCycle | UsagePeriod::ThirtyDays => 1.0,
            UsagePeriod::SevenDays => {
                let total: f64 = self.days.iter().copied().sum();
                let last7: f64 = self.days.iter().rev().take(7).copied().sum();
                if total > 1e-6 { last7 / total } else { 1.0 }
            }
        }
    }

    fn active_days(&self) -> (Vec<f64>, usize) {
        match self.period {
            UsagePeriod::SevenDays => {
                let tail: Vec<f64> = self
                    .days
                    .iter()
                    .rev()
                    .take(7)
                    .copied()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                let len = tail.len().max(7);
                (tail, len)
            }
            UsagePeriod::ThirtyDays | UsagePeriod::BillingCycle => {
                (self.days.clone(), self.days_in_month)
            }
        }
    }

    fn active_groups(&self) -> &[UsageBreakdownItem] {
        match self.view {
            UsageBreakdownView::Projects => &self.projects,
            UsageBreakdownView::Models => &self.models,
        }
    }

    fn commit_budget_edit(&mut self, cx: &mut Context<Self>) {
        if let Some(ed) = &self.budget_editor {
            let raw = ed.read(cx).value().trim().to_owned();
            let cleaned: String = raw
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if cleaned.is_empty() {
                self.budget = None;
                cx.emit(AgentUsageEvent::BudgetChanged(None));
            } else if let Ok(val) = cleaned.parse::<f64>()
                && val > 0.0
            {
                self.budget = Some(val);
                cx.emit(AgentUsageEvent::BudgetChanged(Some(val)));
            }
        }
        self.editing_budget = false;
        cx.notify();
    }

    fn handle_key(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        if self.editing_budget {
            if key == "escape" {
                self.editing_budget = false;
                cx.notify();
            }
            return;
        }

        if self
            .budget_editor
            .as_ref()
            .is_some_and(|ed| ed.read(cx).focus.is_focused(w))
        {
            return;
        }

        let (days, _) = self.active_days();
        match key {
            "escape" => {
                if self.scrub_day.is_some() || self.panel_scrub_day.is_some() {
                    self.scrub_day = None;
                    self.panel_scrub_day = None;
                    cx.notify();
                }
            }
            "left" => {
                if !days.is_empty() {
                    let cur = self.scrub_day.unwrap_or(days.len() - 1);
                    self.scrub_day = Some(cur.saturating_sub(1));
                    cx.notify();
                }
            }
            "right" => {
                if !days.is_empty() {
                    let last = days.len() - 1;
                    let cur = self.scrub_day.unwrap_or(0);
                    self.scrub_day = Some((cur + 1).min(last));
                    cx.notify();
                }
            }
            "home" => {
                if !days.is_empty() {
                    self.scrub_day = Some(0);
                    cx.notify();
                }
            }
            "end" => {
                if !days.is_empty() {
                    self.scrub_day = Some(days.len() - 1);
                    cx.notify();
                }
            }
            "j" | "down" => {
                let list = self.active_groups();
                if !list.is_empty() {
                    let next_idx = self
                        .selected_id
                        .as_ref()
                        .and_then(|sid| list.iter().position(|it| &it.id == sid))
                        .map_or(0, |i| (i + 1).min(list.len() - 1));
                    let next_id = list[next_idx].id.clone();
                    self.selected_id = Some(next_id.clone());
                    self.panel_scrub_day = None;
                    self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
                    cx.emit(AgentUsageEvent::Select {
                        view: self.view,
                        id: next_id,
                    });
                    cx.notify();
                }
            }
            "k" | "up" => {
                let list = self.active_groups();
                if !list.is_empty() {
                    let prev_idx = self
                        .selected_id
                        .as_ref()
                        .and_then(|sid| list.iter().position(|it| &it.id == sid))
                        .map_or(0, |i| i.saturating_sub(1));
                    let prev_id = list[prev_idx].id.clone();
                    self.selected_id = Some(prev_id.clone());
                    self.panel_scrub_day = None;
                    self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
                    cx.emit(AgentUsageEvent::Select {
                        view: self.view,
                        id: prev_id,
                    });
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_spend_card(
        &self,
        narrow: bool,
        reveal: f32,
        pulse_t: f32,
        skel_phase: f32,
        reduced: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);

        if self.loading {
            return div()
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .pt(px(16.))
                .overflow_hidden()
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(
                    div()
                        .px(px(16.))
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .child(format!(
                                    "{} {}",
                                    tr(cx, "Spent in", "本月支出 ·"),
                                    self.month_name
                                )),
                        )
                        .child(pro_skeleton_bar(148., 28., 6., skel_phase, cx))
                        .child(pro_skeleton_bar(196., 13., 4., skel_phase, cx)),
                )
                .child(
                    div()
                        .px(px(16.))
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(260., 118., 6., skel_phase, cx))
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .child(pro_skeleton_bar(34., 10., 3., skel_phase, cx))
                                .child(pro_skeleton_bar(34., 10., 3., skel_phase, cx))
                                .child(pro_skeleton_bar(34., 10., 3., skel_phase, cx)),
                        ),
                )
                .child(
                    div()
                        .border_t_1()
                        .border_color(p.line)
                        .flex()
                        .children((0..4).map(|i| {
                            div()
                                .flex_1()
                                .px(px(16.))
                                .py(px(12.))
                                .when(i > 0, |d| d.border_l_1().border_color(p.line))
                                .flex()
                                .flex_col()
                                .gap(px(5.))
                                .child(pro_skeleton_bar(42., 10., 3., skel_phase, cx))
                                .child(pro_skeleton_bar(56., 16., 4., skel_phase, cx))
                        })),
                );
        }

        let (days, total_days) = self.active_days();
        let active_budget = match self.period {
            UsagePeriod::SevenDays => self.budget.map(|b| b * (7.0 / 30.0)),
            UsagePeriod::ThirtyDays | UsagePeriod::BillingCycle => self.budget,
        };
        let forecast = compute_spend_forecast(&days, total_days, active_budget);

        let spent_heading = match self.period {
            UsagePeriod::SevenDays => tr(cx, "Spent in last 7 days", "近 7 天累计支出").to_owned(),
            UsagePeriod::ThirtyDays => {
                tr(cx, "Spent in last 30 days", "近 30 天累计支出").to_owned()
            }
            UsagePeriod::BillingCycle => {
                format!("{} {}", tr(cx, "Spent in", "本月支出 ·"), self.month_name)
            }
        };

        let big_val = format_money(forecast.spent_so_far, &self.currency);
        let budget_ctrl: AnyElement = if self.editing_budget {
            div()
                .flex()
                .items_center()
                .gap(px(5.))
                .child(
                    div()
                        .w(px(88.))
                        .h(px(26.))
                        .px(px(7.))
                        .rounded(px(6.))
                        .bg(p.subtle)
                        .border_1()
                        .border_color(p.focus)
                        .flex()
                        .items_center()
                        .gap(px(3.))
                        .text_size(px(12.5))
                        .text_color(p.muted)
                        .child(currency_symbol(&self.currency).to_owned())
                        .when_some(self.budget_editor.clone(), |d, ed| {
                            d.child(div().flex_1().min_w_0().child(ed))
                        }),
                )
                .child(pro_btn(
                    pro_eid(&self.id, "save-budget"),
                    tr(cx, "Save", "保存"),
                    None,
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    |this, _, cx| this.commit_budget_edit(cx),
                ))
                .into_any_element()
        } else {
            let cap_label = if let Some(b) = active_budget {
                let pct = forecast.budget_pct.unwrap_or(0);
                format!(
                    "{} {} · {pct}%",
                    tr(cx, "of", "/ 预算"),
                    format_money_whole(b, &self.currency)
                )
            } else {
                tr(cx, "Set budget cap", "设置预算上限").to_owned()
            };
            let current_raw = self
                .budget
                .map(|b| format!("{}", b.round() as u64))
                .unwrap_or_default();
            activate(
                div()
                    .id(pro_eid(&self.id, "edit-budget-btn"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Edit monthly budget threshold", "编辑月度预算阈值"))
                    .tab_index(0)
                    .px(px(5.))
                    .py(px(1.))
                    .rounded(px(5.))
                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .text_size(px(13.))
                    .text_color(p.muted)
                    .cursor_pointer()
                    .child(cap_label),
                cx,
                move |this, w, cx| {
                    this.editing_budget = true;
                    if let Some(ed) = &this.budget_editor {
                        let init = current_raw.clone();
                        ed.update(cx, |e, cx| {
                            e.set_value(&init, w, cx);
                            e.select_all(w, cx);
                            e.focus.focus(w, cx);
                        });
                    }
                    cx.notify();
                },
            )
            .into_any_element()
        };

        let show_flash = self
            .flash_at
            .is_some_and(|t| t.elapsed().as_secs_f32() < 3.0);
        let flash_pill: Option<AnyElement> = self.latest_run.as_ref().map(|run| {
            let cost_txt = format!("+{}", format_money(run.cost, &self.currency));
            let task_txt = format!("· {}", run.task);
            pro_presence(
                pro_eid(&self.id, "flash-run-pres"),
                show_flash,
                0.16,
                0.24,
                move |amount| {
                    div()
                        .ml(px(-4. * (1. - amount)))
                        .opacity(amount)
                        .px(px(8.))
                        .py(px(2.))
                        .rounded(px(6.))
                        .bg(p.accent_soft)
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(12.))
                        .text_color(p.blue)
                        .child(div().font_weight(FontWeight::MEDIUM).child(cost_txt))
                        .child(div().max_w(px(150.)).truncate().child(task_txt))
                        .into_any_element()
                },
            )
        });

        let proj_str = format_money_whole(forecast.projected_total, &self.currency);
        let end_date_str = if self.period == UsagePeriod::SevenDays {
            tr(cx, "end of week", "本周末").to_owned()
        } else {
            format!("{} {}", self.month_short, total_days)
        };
        let is_over = forecast.over_budget_day.is_some();
        let pace_line = if let Some(cross_day) = forecast.over_budget_day
            && let Some(b) = active_budget
        {
            let b_str = format_money_whole(b, &self.currency);
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(13.))
                .text_color(p.amber_ink)
                .child(pro_icon(ProIcon::AlertTriangle, 13., p.amber_ink))
                .child(format!(
                    "{} {b_str} {} {} {cross_day} ({proj_str} {} {end_date_str})",
                    tr(cx, "Passes", "按当前增速将于"),
                    tr(cx, "around", "前后突破预算 ·"),
                    self.month_short,
                    tr(cx, "by", "预计")
                ))
        } else {
            div().text_size(px(13.)).text_color(p.muted).child(format!(
                "{} {proj_str} {} {end_date_str}",
                tr(cx, "On track for", "按当前节奏预计月底达"),
                tr(cx, "by", "·")
            ))
        };

        // Cumulative curve geometry
        let mut cumul = Vec::with_capacity(days.len());
        let mut running_sum = 0.0_f64;
        for &d in &days {
            running_sum += d;
            cumul.push(running_sum as f32);
        }
        let max_val = active_budget
            .unwrap_or(0.0)
            .max(forecast.projected_total)
            .max(forecast.spent_so_far)
            .max(10.0) as f32
            * 1.12;

        let line_color = p.blue;
        let card_bg = p.card;
        let base_stroke = p.line_soft;
        let budget_stroke = p.faint.opacity(0.65);
        let budget_val_f32 = active_budget.map(|b| b as f32);
        let proj_f32 = forecast.projected_total as f32;
        let scrub = self.scrub_day.filter(|&d| d < cumul.len());
        let total_slots = total_days.max(cumul.len()).max(2);

        let cumul_for_scrub = cumul.clone();
        let chart_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let w = f32::from(bounds.size.width);
                let h = f32::from(bounds.size.height);
                if w <= 8. || h <= 16. {
                    return;
                }
                let pad_top = 20.0_f32;
                let base_y = h - 4.0_f32;
                let plot_h = (base_y - pad_top).max(20.);

                let x_of = |day_idx: usize| -> Pixels {
                    let t = day_idx as f32 / (total_slots - 1) as f32;
                    bounds.left() + px(1. + t * (w - 2.))
                };
                let y_of = |val: f32| -> Pixels {
                    let frac = (val / max_val).clamp(0., 1.);
                    bounds.top() + px(base_y - frac * plot_h)
                };

                // Baseline
                let mut base_pb = PathBuilder::stroke(px(1.));
                base_pb.move_to(point(bounds.left(), bounds.top() + px(base_y)));
                base_pb.line_to(point(bounds.right(), bounds.top() + px(base_y)));
                if let Ok(path) = base_pb.build() {
                    window.paint_path(path, base_stroke);
                }

                // Dashed budget line
                if let Some(bv) = budget_val_f32 {
                    let by = y_of(bv);
                    let dash = 4.0_f32;
                    let gap = 4.0_f32;
                    let mut x = 0.0_f32;
                    let mut b_pb = PathBuilder::stroke(px(1.));
                    while x < w {
                        let x2 = (x + dash).min(w);
                        b_pb.move_to(point(bounds.left() + px(x), by));
                        b_pb.line_to(point(bounds.left() + px(x2), by));
                        x += dash + gap;
                    }
                    if let Ok(path) = b_pb.build() {
                        window.paint_path(path, budget_stroke);
                    }
                }

                if cumul.is_empty() {
                    return;
                }

                let vis_n = if cumul.len() <= 1 {
                    1
                } else {
                    (((cumul.len() - 1) as f32 * reveal).ceil() as usize + 1).clamp(1, cumul.len())
                };

                // Area gradient fill under cumulative curve
                if vis_n >= 2 {
                    let mut area = PathBuilder::fill();
                    let first_pt = point(x_of(0), y_of(cumul[0]));
                    let last_pt = point(x_of(vis_n - 1), y_of(cumul[vis_n - 1]));
                    let floor_y = bounds.top() + px(base_y);
                    area.move_to(point(first_pt.x, floor_y));
                    area.line_to(first_pt);
                    for (i, &cv) in cumul.iter().enumerate().take(vis_n).skip(1) {
                        area.line_to(point(x_of(i), y_of(cv)));
                    }
                    area.line_to(point(last_pt.x, floor_y));
                    area.close();
                    if let Ok(path) = area.build() {
                        window.paint_path(
                            path,
                            linear_gradient(
                                180.,
                                linear_color_stop(line_color.opacity(0.17), 0.),
                                linear_color_stop(line_color.opacity(0.0), 1.),
                            ),
                        );
                    }
                }

                // Dashed forecast segment from today to end of month
                if reveal >= 0.85 && cumul.len() < total_slots {
                    let today_idx = cumul.len() - 1;
                    let p0 = point(x_of(today_idx), y_of(cumul[today_idx]));
                    let p1 = point(x_of(total_slots - 1), y_of(proj_f32));
                    let dx = f32::from(p1.x - p0.x);
                    let dy = f32::from(p1.y - p0.y);
                    let dist = dx.hypot(dy);
                    if dist > 4. {
                        let steps = (dist / 8.).floor().max(2.) as usize;
                        let mut f_pb = PathBuilder::stroke(px(1.5));
                        for s in 0..steps {
                            let t0 = s as f32 / steps as f32;
                            let t1 = (t0 + 0.5 / steps as f32).min(1.0);
                            f_pb.move_to(point(p0.x + px(dx * t0), p0.y + px(dy * t0)));
                            f_pb.line_to(point(p0.x + px(dx * t1), p0.y + px(dy * t1)));
                        }
                        if let Ok(path) = f_pb.build() {
                            let forecast_c = if is_over {
                                line_color.opacity(0.55)
                            } else {
                                line_color.opacity(0.40)
                            };
                            window.paint_path(path, forecast_c);
                        }
                    }
                }

                // Solid cumulative polyline
                if vis_n >= 2 {
                    let mut line = PathBuilder::stroke(px(2.0));
                    for (i, &cv) in cumul.iter().enumerate().take(vis_n) {
                        let pt = point(x_of(i), y_of(cv));
                        if i == 0 {
                            line.move_to(pt);
                        } else {
                            line.line_to(pt);
                        }
                    }
                    if let Ok(path) = line.build() {
                        window.paint_path(path, line_color);
                    }
                }

                // Today's dot + breathing pulse
                let today_idx = vis_n - 1;
                let today_pt = point(x_of(today_idx), y_of(cumul[today_idx]));
                if !reduced {
                    let halo_r = px(4. + pulse_t * 8.);
                    let halo_alpha = (1. - pulse_t) * 0.34;
                    let halo_rect = Bounds::new(
                        today_pt - point(halo_r, halo_r),
                        size(halo_r * 2., halo_r * 2.),
                    );
                    window.paint_quad(
                        fill(halo_rect, line_color.opacity(halo_alpha)).corner_radii(halo_r),
                    );
                }
                let outer_r = px(5.);
                let inner_r = px(3.5);
                window.paint_quad(
                    fill(
                        Bounds::new(
                            today_pt - point(outer_r, outer_r),
                            size(outer_r * 2., outer_r * 2.),
                        ),
                        card_bg,
                    )
                    .corner_radii(outer_r),
                );
                window.paint_quad(
                    fill(
                        Bounds::new(
                            today_pt - point(inner_r, inner_r),
                            size(inner_r * 2., inner_r * 2.),
                        ),
                        line_color,
                    )
                    .corner_radii(inner_r),
                );

                // Optional keyboard/click day scrubber cursor
                if let Some(di) = scrub {
                    let sx = x_of(di);
                    let sy = y_of(cumul[di]);
                    let mut cur_pb = PathBuilder::stroke(px(1.));
                    cur_pb.move_to(point(sx, bounds.top() + px(pad_top)));
                    cur_pb.line_to(point(sx, bounds.top() + px(base_y)));
                    if let Ok(path) = cur_pb.build() {
                        window.paint_path(path, budget_stroke);
                    }
                    let cr = px(4.);
                    window.paint_quad(
                        fill(
                            Bounds::new(point(sx, sy) - point(cr, cr), size(cr * 2., cr * 2.)),
                            line_color,
                        )
                        .corner_radii(cr),
                    );
                }
            },
        )
        .w_full()
        .h(px(126.));

        let budget_tag_top = active_budget.map(|b| {
            let frac = ((b as f32) / max_val).clamp(0., 1.);
            let base_y = 122.0_f32;
            let plot_h = 102.0_f32;
            let line_y = base_y - frac * plot_h;
            (
                (line_y - 16.).max(0.),
                format!(
                    "{} {}",
                    tr(cx, "Budget", "预算"),
                    format_money_whole(b, &self.currency)
                ),
            )
        });

        let scrub_readout: Option<String> = scrub.map(|di| {
            let day_num = if self.period == UsagePeriod::SevenDays {
                self.days.len().saturating_sub(days.len()) + di + 1
            } else {
                di + 1
            };
            format!(
                "{} {day_num} · {} ({} {})",
                self.month_short,
                format_money(days[di], &self.currency),
                format_money(cumul_for_scrub[di] as f64, &self.currency),
                tr(cx, "so far", "累计")
            )
        });

        // Interactive invisible day hit columns over the curve for mouse click/scrub
        let hit_cols = days.len();
        let hit_overlay = div()
            .absolute()
            .inset_0()
            .flex()
            .children((0..hit_cols).map(|di| {
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("chart-day-{di}")))
                        .flex_1()
                        .h_full()
                        .cursor_pointer(),
                    cx,
                    move |this, _, cx| {
                        if this.scrub_day == Some(di) {
                            this.scrub_day = None;
                        } else {
                            this.scrub_day = Some(di);
                        }
                        cx.notify();
                    },
                )
            }));

        let ticks: [String; 5] = if self.period == UsagePeriod::SevenDays {
            let start_d = self.days.len().saturating_sub(6).max(1);
            let end_d = self.days.len().max(7);
            [
                format!("{} {start_d}", self.month_short),
                format!("{} {}", self.month_short, start_d + 1),
                format!("{} {}", self.month_short, start_d + 3),
                format!("{} {}", self.month_short, start_d + 5),
                format!("{} {end_d}", self.month_short),
            ]
        } else {
            [
                format!("{} 1", self.month_short),
                format!("{} 8", self.month_short),
                format!("{} 15", self.month_short),
                format!("{} 23", self.month_short),
                format!("{} {}", self.month_short, total_days),
            ]
        };

        let scale = self.period_scale();
        let scaled_tasks = ((self.stats.tasks as f64 * scale).round() as u32).max(1);
        let scaled_runs = ((self.stats.runs as f64 * scale).round() as u32).max(1);
        let scaled_tokens = (self.stats.tokens as f64 * scale).round() as u64;
        let per_task = forecast.spent_so_far / scaled_tasks as f64;

        let kpi_items = [
            (
                tr(cx, "Tasks", "任务数"),
                format_int_grouped(u64::from(scaled_tasks)),
            ),
            (
                tr(cx, "Runs", "运行次数"),
                format_int_grouped(u64::from(scaled_runs)),
            ),
            (
                tr(cx, "Per task", "单任务均价"),
                format_money(per_task, &self.currency),
            ),
            (
                tr(cx, "Tokens", "词元消耗"),
                format_tokens_millions(scaled_tokens),
            ),
        ];

        let stats_footer = if narrow {
            div()
                .border_t_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .border_b_1()
                        .border_color(p.line)
                        .child(self.render_kpi_cell(kpi_items[0].0, &kpi_items[0].1, false, cx))
                        .child(self.render_kpi_cell(kpi_items[1].0, &kpi_items[1].1, true, cx)),
                )
                .child(
                    div()
                        .flex()
                        .child(self.render_kpi_cell(kpi_items[2].0, &kpi_items[2].1, false, cx))
                        .child(self.render_kpi_cell(kpi_items[3].0, &kpi_items[3].1, true, cx)),
                )
        } else {
            div().border_t_1().border_color(p.line).flex().children(
                kpi_items
                    .into_iter()
                    .enumerate()
                    .map(|(i, (k, v))| self.render_kpi_cell(k, &v, i > 0, cx)),
            )
        };

        div()
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .pt(px(16.))
            .overflow_hidden()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                div()
                    .px(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(spent_heading),
                            )
                            .when_some(scrub_readout, |d, txt| {
                                d.child(
                                    div()
                                        .px(px(7.))
                                        .py(px(1.))
                                        .rounded(px(5.))
                                        .bg(p.chip)
                                        .text_size(px(11.5))
                                        .text_color(p.ink)
                                        .child(txt),
                                )
                            }),
                    )
                    .child(
                        div()
                            .pt(px(2.))
                            .flex()
                            .flex_wrap()
                            .items_baseline()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(28.))
                                    .line_height(px(34.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(big_val),
                            )
                            .child(budget_ctrl)
                            .when_some(flash_pill, |d, fp| d.child(fp)),
                    )
                    .child(pace_line)
                    .when_some(active_budget, |d, b| {
                        let ratio = ((forecast.spent_so_far / b.max(1e-6)) as f32).clamp(0., 1.);
                        let fill_c = if is_over { p.amber } else { p.blue };
                        d.child(div().pt(px(4.)).child(pro_spring_bar(
                            pro_eid(&self.id, "spend-budget-bar"),
                            ratio,
                            6.,
                            3.,
                            p.line_soft,
                            fill_c,
                        )))
                    }),
            )
            .child(
                div()
                    .px(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .relative()
                            .w_full()
                            .h(px(126.))
                            .child(chart_canvas)
                            .when_some(budget_tag_top, |d, (top_y, lbl)| {
                                d.child(
                                    div()
                                        .absolute()
                                        .right(px(2.))
                                        .top(px(top_y))
                                        .text_size(px(11.))
                                        .text_color(p.faint)
                                        .child(lbl),
                                )
                            })
                            .child(hit_overlay),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(11.))
                            .text_color(p.faint)
                            .children(ticks.into_iter().map(|t| div().child(t))),
                    ),
            )
            .child(stats_footer)
    }

    fn render_kpi_cell(&self, label: &str, value: &str, left_border: bool, cx: &App) -> Div {
        let p = ProPalette::get(cx);
        div()
            .flex_1()
            .min_w_0()
            .px(px(16.))
            .pt(px(12.))
            .pb(px(14.))
            .when(left_border, |d| d.border_l_1().border_color(p.line))
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(p.muted)
                    .truncate()
                    .child(label.to_owned()),
            )
            .child(
                div()
                    .text_size(px(15.))
                    .line_height(px(22.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .truncate()
                    .child(value.to_owned()),
            )
    }

    fn render_breakdown_section(
        &self,
        narrow: bool,
        reveal: f32,
        skel_phase: f32,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let proj_active = self.view == UsageBreakdownView::Projects;
        let mod_active = self.view == UsageBreakdownView::Models;
        let view_sensor = pro_selection_sensor();
        let view_sensor_proj = view_sensor.clone();
        let view_sensor_mod = view_sensor.clone();
        let view_key: SharedString = format!("{:?}", self.view).into();

        let seg_switch = div()
            .relative()
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.chip)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "view-pill-ind"),
                view_key,
                view_sensor,
                6.,
                p.card,
                Some(p.line),
                true,
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "view-projects"))
                    .role(Role::Tab)
                    .tab_index(0)
                    .relative()
                    .h(px(24.))
                    .px(px(10.))
                    .rounded(px(6.))
                    .hover(move |s| {
                        if proj_active {
                            s.text_color(p.ink)
                        } else {
                            s.text_color(p.ink).bg(p.hover)
                        }
                    })
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if proj_active { p.ink } else { p.muted })
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .when(proj_active, |d| {
                        d.child(pro_mark_selected(&view_sensor_proj))
                    })
                    .child(tr(cx, "Projects", "按项目")),
                cx,
                |this, _, cx| {
                    this.view = UsageBreakdownView::Projects;
                    if let Some(first) = this.projects.first() {
                        let fid = first.id.clone();
                        this.selected_id = Some(fid.clone());
                        this.panel_scrub_day = None;
                        cx.emit(AgentUsageEvent::Select {
                            view: UsageBreakdownView::Projects,
                            id: fid,
                        });
                    }
                    cx.notify();
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "view-models"))
                    .role(Role::Tab)
                    .tab_index(0)
                    .relative()
                    .h(px(24.))
                    .px(px(10.))
                    .rounded(px(6.))
                    .hover(move |s| {
                        if mod_active {
                            s.text_color(p.ink)
                        } else {
                            s.text_color(p.ink).bg(p.hover)
                        }
                    })
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if mod_active { p.ink } else { p.muted })
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .when(mod_active, |d| d.child(pro_mark_selected(&view_sensor_mod)))
                    .child(tr(cx, "Models", "按模型")),
                cx,
                |this, _, cx| {
                    this.view = UsageBreakdownView::Models;
                    if let Some(first) = this.models.first() {
                        let fid = first.id.clone();
                        this.selected_id = Some(fid.clone());
                        this.panel_scrub_day = None;
                        cx.emit(AgentUsageEvent::Select {
                            view: UsageBreakdownView::Models,
                            id: fid,
                        });
                    }
                    cx.notify();
                },
            ));

        let head = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(tr(cx, "Where it went", "费用流向")),
            )
            .child(seg_switch);

        if self.loading {
            return div().flex().flex_col().gap(px(8.)).child(head).child(
                div()
                    .p(px(4.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .children((0..4).map(|i| {
                        div()
                            .h(px(44.))
                            .px(px(10.))
                            .when(i > 0, |d| d.border_t_1().border_color(p.line_soft))
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(pro_skeleton_bar(8., 8., 2., skel_phase, cx))
                            .child(pro_skeleton_bar(96., 12., 4., skel_phase, cx))
                            .when(!narrow, |d| {
                                d.child(
                                    div()
                                        .flex_1()
                                        .child(pro_skeleton_bar(140., 6., 3., skel_phase, cx)),
                                )
                            })
                            .child(pro_skeleton_bar(56., 12., 4., skel_phase, cx))
                            .child(pro_skeleton_bar(38., 11., 4., skel_phase, cx))
                    })),
            );
        }

        let scale = self.period_scale();
        let groups = self.active_groups();
        let total_amt: f64 = groups.iter().map(|g| g.amount * scale).sum();
        let max_amt: f64 = groups
            .iter()
            .map(|g| g.amount * scale)
            .fold(0.0_f64, f64::max)
            .max(1e-6);

        let mut list_card = div()
            .p(px(4.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col();

        for (idx, item) in groups.iter().cloned().enumerate() {
            let scaled_amt = item.amount * scale;
            let share_pct = if total_amt > 1e-6 {
                ((scaled_amt / total_amt) * 100.0).round() as u32
            } else {
                0
            };
            let bar_frac = (((scaled_amt / max_amt) as f32) * reveal).clamp(0.03, 1.0);
            let tone_c = item.tone.resolve(&p);
            let is_current = self.selected_id.as_ref() == Some(&item.id);
            let item_id = item.id.clone();
            let cur_view = self.view;

            let lead_mark: AnyElement = if let Some(app) = &item.provider_app {
                pro_app_tile(app, 20., cx)
            } else {
                div()
                    .size(px(8.))
                    .flex_none()
                    .rounded(px(2.))
                    .bg(tone_c)
                    .into_any_element()
            };

            let row_btn = activate(
                div()
                    .id(pro_eid(&self.id, format!("grp-row-{item_id}")))
                    .role(Role::Button)
                    .aria_label(format!(
                        "{}: {}, {}%",
                        item.name,
                        format_money(scaled_amt, &self.currency),
                        share_pct
                    ))
                    .tab_index(0)
                    .min_h(px(44.))
                    .px(px(10.))
                    .rounded(px(7.))
                    .when(is_current, |d| d.bg(p.hover))
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .cursor_pointer()
                    .child(lead_mark)
                    .child(
                        div()
                            .when(narrow, |d| d.flex_1().min_w_0())
                            .when(!narrow, |d| d.w(px(148.)).flex_none())
                            .text_size(px(13.))
                            .text_color(p.ink)
                            .truncate()
                            .child(item.name.clone()),
                    )
                    .when(!narrow, |d| {
                        d.child(div().flex_1().min_w(px(40.)).child(pro_spring_bar(
                            pro_eid(&self.id, format!("brk-bar-{idx}")),
                            bar_frac,
                            6.,
                            3.,
                            p.line_soft,
                            tone_c,
                        )))
                    })
                    .child(
                        div()
                            .w(px(78.))
                            .flex_none()
                            .flex()
                            .justify_end()
                            .text_size(px(13.))
                            .text_color(p.ink)
                            .child(format_money(scaled_amt, &self.currency)),
                    )
                    .when(!narrow, |d| {
                        d.child(
                            div()
                                .w(px(34.))
                                .flex_none()
                                .flex()
                                .justify_end()
                                .text_size(px(12.))
                                .text_color(p.faint)
                                .child(format!("{share_pct}%")),
                        )
                    })
                    .child(
                        div()
                            .w(px(50.))
                            .flex_none()
                            .flex()
                            .justify_end()
                            .text_size(px(12.))
                            .text_color(if item.change_pct > 0 {
                                p.amber_ink
                            } else {
                                p.muted
                            })
                            .child(format_change(item.change_pct)),
                    )
                    .child(pro_icon(ProIcon::ChevronRight, 12., p.faint)),
                cx,
                move |this, _, cx| {
                    this.selected_id = Some(item_id.clone());
                    this.panel_scrub_day = None;
                    this.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
                    cx.emit(AgentUsageEvent::Select {
                        view: cur_view,
                        id: item_id.clone(),
                    });
                    cx.notify();
                },
            );

            list_card = list_card.child(
                div()
                    .when(idx > 0, |d| d.border_t_1().border_color(p.line_soft))
                    .child(row_btn),
            );
        }

        let brk_vtx_key: SharedString = format!("{:?}-{:?}", self.view, self.period).into();
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(head)
            .child(pro_view_transition(
                pro_eid(&self.id, "usage-brk-vtx"),
                brk_vtx_key,
                list_card,
            ))
    }

    fn render_notes_section(&self, narrow: bool, skel_phase: f32, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let heading = div()
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(p.muted)
            .child(tr(cx, "Worth a look", "值得关注"));

        if self.loading {
            return div().flex().flex_col().gap(px(8.)).child(heading).child(
                div()
                    .px(px(12.))
                    .py(px(2.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .children((0..3).map(|i| {
                        div()
                            .min_h(px(56.))
                            .py(px(8.))
                            .when(i > 0, |d| d.border_t_1().border_color(p.line_soft))
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(pro_skeleton_bar(28., 28., 7., skel_phase, cx))
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(5.))
                                    .child(pro_skeleton_bar(190., 12., 4., skel_phase, cx))
                                    .child(pro_skeleton_bar(230., 10., 3., skel_phase, cx)),
                            )
                            .child(pro_skeleton_bar(36., 18., 4., skel_phase, cx))
                            .child(pro_skeleton_bar(52., 28., 6., skel_phase, cx))
                    })),
            );
        }

        let mut card = div()
            .px(px(12.))
            .py(px(2.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col();

        for (idx, note) in self.notes.iter().cloned().enumerate() {
            let is_spike = note.kind == UsageNoteKind::Spike;
            let is_warn_badge = matches!(note.kind, UsageNoteKind::Spike | UsageNoteKind::Jump);
            let icon_tile_bg = if is_spike { p.amber_soft } else { p.chip };
            let icon_fg = if is_spike { p.amber_ink } else { p.muted };
            let glyph: AnyElement = match note.kind {
                UsageNoteKind::Spike => pro_icon(ProIcon::AlertTriangle, 14., icon_fg),
                UsageNoteKind::Jump => pro_svg("0 0 24 24", SVG_TREND_UP, 14., icon_fg),
                UsageNoteKind::Budget => pro_icon(ProIcon::UsageChart, 14., icon_fg),
            };

            let nid = note.id.clone();
            let t_view = note.target_view;
            let t_id = note.target_id.clone();

            let badge_el = div()
                .px(px(6.))
                .h(px(18.))
                .flex_none()
                .rounded(px(4.))
                .bg(if is_warn_badge { p.amber_soft } else { p.chip })
                .text_color(if is_warn_badge { p.amber_ink } else { p.muted })
                .text_size(px(11.))
                .font_weight(FontWeight::MEDIUM)
                .flex()
                .items_center()
                .child(note.badge.clone());

            let open_btn = pro_btn(
                pro_eid(&self.id, format!("note-open-{nid}")),
                tr(cx, "Open", "查看"),
                None,
                ProBtnStyle::Secondary,
                true,
                cx,
                move |this, _, cx| {
                    this.view = t_view;
                    this.selected_id = Some(t_id.clone());
                    this.panel_scrub_day = None;
                    this.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
                    cx.emit(AgentUsageEvent::OpenNote(nid.clone()));
                    cx.notify();
                },
            );

            card = card.child(
                div()
                    .min_h(px(56.))
                    .py(px(8.))
                    .when(idx > 0, |d| d.border_t_1().border_color(p.line_soft))
                    .flex()
                    .when(narrow, |d| d.flex_wrap())
                    .items_center()
                    .gap(px(12.))
                    .child(
                        div()
                            .size(px(28.))
                            .flex_none()
                            .rounded(px(7.))
                            .bg(icon_tile_bg)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(glyph),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(160.))
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(19.))
                                    .text_color(p.ink)
                                    .truncate()
                                    .child(note.text.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .text_color(p.muted)
                                    .truncate()
                                    .child(note.detail.clone()),
                            ),
                    )
                    .child(badge_el)
                    .child(open_btn),
            );
        }

        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(heading)
            .child(card)
    }

    fn render_inspector(
        &mut self,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SharedString, ProIcon, AnyElement)> {
        let p = ProPalette::get(cx);
        let elapsed_secs = self.boot_at.elapsed().as_secs_f32();
        let skel_phase = if Theme::global(cx).reduced_motion {
            0.25
        } else {
            elapsed_secs * 0.85
        };

        if self.empty {
            let empty_body = div()
                .p(px(24.))
                .min_h(px(220.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .text_color(p.muted)
                .text_center()
                .child(tr(
                    cx,
                    "Select a project or model once spend begins.",
                    "产生费用后，可在此查看单日趋势与高成本任务。",
                ))
                .into_any_element();
            return Some((
                tr(cx, "Details", "详情").into(),
                ProIcon::UsageChart,
                empty_body,
            ));
        }

        if self.loading {
            let skel_body = div()
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(20.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(8., 8., 2., skel_phase, cx))
                        .child(pro_skeleton_bar(88., 15., 4., skel_phase, cx)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(pro_skeleton_bar(110., 22., 5., skel_phase, cx))
                        .child(pro_skeleton_bar(92., 12., 4., skel_phase, cx)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(68., 11., 3., skel_phase, cx))
                        .child(pro_skeleton_bar(240., 68., 6., skel_phase, cx)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(104., 11., 3., skel_phase, cx))
                        .children((0..4).map(|_| pro_skeleton_bar(240., 36., 6., skel_phase, cx))),
                )
                .into_any_element();
            return Some((tr(cx, "Details", "详情").into(), ProIcon::Folder, skel_body));
        }

        let scale = self.period_scale();
        let groups = self.active_groups();
        let selected = self
            .selected_id
            .as_ref()
            .and_then(|sid| groups.iter().find(|g| &g.id == sid))
            .or_else(|| groups.first())?
            .clone();

        let total_amt: f64 = groups.iter().map(|g| g.amount * scale).sum();
        let scaled_amt = selected.amount * scale;
        let share_pct = if total_amt > 1e-6 {
            ((scaled_amt / total_amt) * 100.0).round() as u32
        } else {
            0
        };
        let tone_c = selected.tone.resolve(&p);

        let tab_icon = if selected.provider_app.is_some() {
            ProIcon::Sparkles
        } else {
            ProIcon::Folder
        };
        let head_mark: AnyElement = if let Some(app) = &selected.provider_app {
            pro_app_tile(app, 22., cx)
        } else {
            div()
                .size(px(8.))
                .flex_none()
                .rounded(px(2.))
                .bg(tone_c)
                .into_any_element()
        };

        let bar_values: Vec<f64> = if self.period == UsagePeriod::SevenDays {
            selected
                .daily_costs
                .iter()
                .rev()
                .take(7)
                .copied()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect()
        } else {
            selected.daily_costs.clone()
        };
        let bar_slots = if self.period == UsagePeriod::SevenDays {
            7usize
        } else {
            self.days_in_month.max(bar_values.len())
        };
        let max_bar = bar_values.iter().copied().fold(0.0_f64, f64::max).max(1e-6);

        let active_scrub = self.panel_scrub_day.filter(|&i| i < bar_values.len());
        let bars_caption = if let Some(di) = active_scrub {
            let day_num = if self.period == UsagePeriod::SevenDays {
                selected.daily_costs.len().saturating_sub(bar_values.len()) + di + 1
            } else {
                di + 1
            };
            format!(
                "{} {day_num} · {}",
                self.month_short,
                format_money(bar_values[di], &self.currency)
            )
        } else if self.period == UsagePeriod::SevenDays {
            let start_d = selected.daily_costs.len().saturating_sub(6).max(1);
            let end_d = selected.daily_costs.len().max(7);
            format!(
                "{} {start_d} – {} {end_d} · {}",
                self.month_short,
                self.month_short,
                format_money(scaled_amt, &self.currency)
            )
        } else {
            format!(
                "{} 1 – {} {} · {}",
                self.month_short,
                self.month_short,
                bar_values.len(),
                format_money(scaled_amt, &self.currency)
            )
        };

        let mut bars_row = div().h(px(72.)).w_full().flex().items_end().gap(px(2.));

        for slot in 0..bar_slots {
            if slot < bar_values.len() {
                let v = bar_values[slot];
                let target_h = ((v / max_bar) as f32 * 68.).clamp(2., 70.);
                let h_px = pro_spring_f32(
                    pro_eid(&self.id, format!("pbar-h-{slot}")),
                    target_h,
                    0.24,
                    w,
                    cx,
                );
                let is_hi = active_scrub == Some(slot)
                    || (active_scrub.is_none() && slot + 1 == bar_values.len());
                let alpha = if is_hi { 1.0 } else { 0.74 };
                bars_row = bars_row.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("pbar-{slot}")))
                        .flex_1()
                        .h_full()
                        .flex()
                        .items_end()
                        .cursor_pointer()
                        .child(
                            div()
                                .w_full()
                                .h(px(h_px))
                                .rounded(px(2.))
                                .bg(tone_c.opacity(alpha)),
                        ),
                    cx,
                    move |this, _, cx| {
                        if this.panel_scrub_day == Some(slot) {
                            this.panel_scrub_day = None;
                        } else {
                            this.panel_scrub_day = Some(slot);
                        }
                        cx.notify();
                    },
                ));
            } else {
                bars_row = bars_row.child(
                    div()
                        .flex_1()
                        .h_full()
                        .flex()
                        .items_end()
                        .child(div().w_full().h(px(2.)).rounded(px(2.)).bg(p.line_soft)),
                );
            }
        }

        let max_tk_cost = selected
            .top_tasks
            .iter()
            .map(|t| t.cost * scale)
            .fold(0.0_f64, f64::max)
            .max(1e-6);

        let tasks_card = div()
            .px(px(12.))
            .py(px(2.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .children(selected.top_tasks.iter().enumerate().map(|(ti, tk)| {
                let tk_cost = tk.cost * scale;
                let tk_runs = ((tk.runs as f64 * scale).round() as u32).max(1);
                let tk_frac = ((tk_cost / max_tk_cost) as f32).clamp(0.04, 1.0);
                div()
                    .py(px(8.))
                    .when(ti > 0, |d| d.border_t_1().border_color(p.line_soft))
                    .flex()
                    .flex_col()
                    .gap(px(5.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(10.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(1.))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .line_height(px(19.))
                                            .text_color(p.ink)
                                            .truncate()
                                            .child(tk.title.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .line_height(px(16.))
                                            .text_color(p.muted)
                                            .child(format!(
                                                "{tk_runs} {}",
                                                if tk_runs == 1 {
                                                    tr(cx, "run", "次运行")
                                                } else {
                                                    tr(cx, "runs", "次运行")
                                                }
                                            )),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .text_size(px(13.))
                                    .text_color(p.ink)
                                    .child(format_money(tk_cost, &self.currency)),
                            ),
                    )
                    .child(pro_spring_bar(
                        pro_eid(&self.id, format!("tk-cost-bar-{ti}")),
                        tk_frac,
                        4.,
                        2.,
                        p.line_soft,
                        tone_c,
                    ))
            }));

        let share_scope = if self.period == UsagePeriod::SevenDays {
            tr(cx, "of 7d", "占近 7 天").to_owned()
        } else {
            format!("{} {}", tr(cx, "of", "占"), self.month_name)
        };

        let body = div()
            .p(px(16.))
            .flex()
            .flex_col()
            .gap(px(20.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(9.))
                    .child(head_mark)
                    .child(
                        div()
                            .text_size(px(15.))
                            .line_height(px(22.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .truncate()
                            .child(selected.name.clone()),
                    ),
            )
            .child(
                div()
                    .mt(px(-10.))
                    .flex()
                    .flex_col()
                    .gap(px(3.))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_baseline()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(22.))
                                    .line_height(px(28.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(format_money(scaled_amt, &self.currency)),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(p.faint)
                                    .child(format!("{share_pct}% {share_scope}")),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(if selected.change_pct > 0 {
                                p.amber_ink
                            } else {
                                p.muted
                            })
                            .child(format!(
                                "{} {}",
                                format_change(selected.change_pct),
                                tr(cx, "vs last month", "较上月")
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Day by day", "逐日分布")),
                    )
                    .child(bars_row)
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(bars_caption),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "What cost the most", "最高消耗任务")),
                    )
                    .child(tasks_card),
            );

        let insp_vtx_key: SharedString =
            format!("{:?}:{}:{:?}", self.view, selected.id, self.period).into();
        let body = pro_view_transition(pro_eid(&self.id, "usage-insp-vtx"), insp_vtx_key, body);

        Some((selected.name, tab_icon, body))
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.budget_editor.is_none() {
            let ph: SharedString = "500".into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 12.5;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| {
                if matches!(ev, EditEvent::Submitted) {
                    this.commit_budget_edit(cx);
                } else if matches!(ev, EditEvent::Escape) {
                    this.editing_budget = false;
                    cx.notify();
                }
            })
            .detach();
            self.budget_editor = Some(ed);
        }

        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let elapsed = self.boot_at.elapsed().as_secs_f32();

        let reveal = if reduced {
            1.0
        } else {
            pro_ease((elapsed / 0.85).clamp(0., 1.))
        };
        let pulse_t = if reduced {
            0.0
        } else {
            (elapsed / 2.2).fract()
        };
        let skel_phase = if reduced { 0.25 } else { elapsed * 0.85 };
        let flash_active = self
            .flash_at
            .is_some_and(|t| t.elapsed().as_secs_f32() < 3.0);

        if !reduced
            && w.is_visible()
            && !self.empty
            && (self.loading || elapsed < 0.9 || flash_active || !self.days.is_empty())
        {
            w.request_animation_frame();
        }

        let width = self.measured_width.get().max(320.);
        let narrow = width < 620.;
        let panel_closed = !self.standalone && !self.shell.read(cx).is_panel_open();

        // Capsule period selector (`7d` | `30d` | `Billing cycle` — `rounded_full` per DESIGN.md)
        let prd_sensor = pro_selection_sensor();
        let prd_key: SharedString = format!("{:?}", self.period).into();
        let period_pills = div()
            .relative()
            .p(px(2.))
            .rounded_full()
            .bg(p.chip)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "period-pill-ind"),
                prd_key,
                prd_sensor.clone(),
                999.,
                p.card,
                Some(p.line),
                true,
            ))
            .children(
                [
                    UsagePeriod::SevenDays,
                    UsagePeriod::ThirtyDays,
                    UsagePeriod::BillingCycle,
                ]
                .into_iter()
                .enumerate()
                .map(|(pi, prd)| {
                    let selected = self.period == prd;
                    let prd_sensor_item = prd_sensor.clone();
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("period-{pi}")))
                            .role(Role::Tab)
                            .tab_index(0)
                            .relative()
                            .h(px(22.))
                            .px(px(9.))
                            .rounded_full()
                            .hover(move |s| {
                                if selected {
                                    s.text_color(p.ink)
                                } else {
                                    s.text_color(p.ink).bg(p.hover)
                                }
                            })
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if selected { p.ink } else { p.muted })
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .when(selected, |d| d.child(pro_mark_selected(&prd_sensor_item)))
                            .child(prd.label(cx)),
                        cx,
                        move |this, _, cx| {
                            this.period = prd;
                            this.scrub_day = None;
                            this.panel_scrub_day = None;
                            cx.emit(AgentUsageEvent::PeriodChanged(prd));
                            cx.notify();
                        },
                    )
                }),
            );

        let header = div()
            .h(px(48.))
            .pl(px(if width < 680. && !self.standalone {
                46.
            } else {
                20.
            }))
            .pr(px(if panel_closed { 50. } else { 16. }))
            .flex_none()
            .border_b_1()
            .border_color(p.line)
            .flex()
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
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tr(cx, "Usage", "用量统计")),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(p.faint)
                            .child(self.month_name.clone()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(period_pills)
                    .when(!narrow && !self.loading && !self.empty, |d| {
                        d.child(pro_btn(
                            pro_eid(&self.id, "sim-live-run"),
                            "+€0.42",
                            Some(ProIcon::Bolt),
                            ProBtnStyle::Ghost,
                            true,
                            cx,
                            |this, _, cx| {
                                this.record_run("Ingest pipeline", 0.42, cx);
                            },
                        ))
                    }),
            );

        let page_body: Div = if self.empty {
            div()
                .w_full()
                .max_w(px(720.))
                .mx_auto()
                .px(px(24.))
                .py(px(36.))
                .child(pro_empty_state(
                    ProIcon::UsageChart,
                    tr(cx, "Nothing spent yet", "本月暂无支出"),
                    tr(
                        cx,
                        "Completed agent runs and tool calls appear here day by day against your monthly budget.",
                        "智能体完成任务运行后，其模型与工具消耗将按日累计并对照预算预测月底走势。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-start-task"),
                        tr(cx, "Start a task", "发起任务").into(),
                        Box::new(|_: &mut Self, _, cx| {
                            cx.emit(AgentUsageEvent::Navigate("Tasks".into()));
                        }),
                    )),
                    cx,
                ))
        } else {
            let pad = if narrow { 16. } else { 24. };
            div()
                .w_full()
                .max_w(px(720.))
                .mx_auto()
                .px(px(pad))
                .pt(px(pad))
                .pb(px(48.))
                .flex()
                .flex_col()
                .gap(px(if narrow { 24. } else { 28. }))
                .child(self.render_spend_card(narrow, reveal, pulse_t, skel_phase, reduced, cx))
                .child(self.render_breakdown_section(narrow, reveal, skel_phase, cx))
                .child(self.render_notes_section(narrow, skel_phase, cx))
        };

        let measure_cell = self.measured_width.clone();
        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .min_h(px(520.))
            .bg(p.panel)
            .overflow_hidden()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(Self::handle_key))
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
                .absolute()
                .size_0(),
            )
            .child(header)
            .child(
                div()
                    .id(pro_eid(&self.id, "usage-scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(page_body),
            )
    }
}

impl Render for AgentUsageScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        compute_spend_forecast, default_days, default_models, default_projects, format_change,
    };

    #[test]
    fn november_forecast_matches_reference_spend_and_pace() {
        let days = default_days();
        assert_eq!(days.len(), 19);
        let fc = compute_spend_forecast(&days, 30, Some(500.0));
        assert!((fc.spent_so_far - 270.40).abs() < 1e-6);
        assert_eq!(fc.projected_total.round() as u32, 427);
        assert_eq!(fc.budget_pct, Some(54));
        assert_eq!(fc.over_budget_day, None);

        // Lowering budget cap below projected €427 flags exact November crossing day
        let tight = compute_spend_forecast(&days, 30, Some(350.0));
        assert_eq!(tight.over_budget_day, Some(25));
    }

    #[test]
    fn project_and_model_breakdowns_reconcile_to_monthly_total() {
        let proj_sum: f64 = default_projects().iter().map(|p| p.amount).sum();
        let mod_sum: f64 = default_models().iter().map(|m| m.amount).sum();
        assert!((proj_sum - 270.40).abs() < 1e-6);
        assert!((mod_sum - 270.40).abs() < 1e-6);
        assert_eq!(format_change(23), "↑23%");
        assert_eq!(format_change(-7), "↓7%");
        assert_eq!(format_change(1100), "↑12×");
    }
}
