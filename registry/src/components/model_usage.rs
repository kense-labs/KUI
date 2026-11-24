//! AI provider usage breakdown with stacked model segments, metric switching, and task filtering.
//!
//! Displays the period total and percentage change on top with a `Tokens` / `Cost` / `Requests` pill switcher,
//! one stacked bar per AI company shaded from darkest (top model) to lighter tints, a side tooltip per provider,
//! and a dynamic 2x2 task-category breakdown (`Search`, `Analyze`, `Code`, `Test`) below.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_ease, pro_eid, pro_empty_state, pro_mark_selected, pro_mix_color,
        pro_pill_indicator, pro_rolling_number, pro_selection_sensor, pro_skeleton_bar,
        pro_spring_f32, tr,
    },
    primitives::{
        family::disabled_motion,
        pro_icons::{ProIcon, pro_app_artwork, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::collections::BTreeMap;
use web_time::Instant;

const TASK_SEARCH_SVG: &str = r#"<path d="M17 17l4 4M19 11a8 8 0 1 0-16 0 8 8 0 0 0 16 0Z" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const TASK_ANALYZE_SVG: &str = r#"<path d="M3 3v15a3 3 0 0 0 3 3h15M8 16v-4M12.5 16V8M17 16v-6" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const TASK_CODE_SVG: &str = r#"<path d="M7.5 7.5 3 12l4.5 4.5M16.5 7.5 21 12l-4.5 4.5M14 4l-4 16" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const TASK_TEST_SVG: &str = r#"<path d="M9 3h6M10 3v6.5L4.6 18.4A1.7 1.7 0 0 0 6 21h12a1.7 1.7 0 0 0 1.4-2.6L14 9.5V3M7 15h10" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"></path>"#;

/// Selectable usage unit in the top switcher of [`ModelUsage`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum UsageMetric {
    /// Token volume (`9.76M`).
    #[default]
    Tokens,
    /// Dollar spend (`$68.40`).
    Cost,
    /// API request count (`3.91k`).
    Requests,
}

impl UsageMetric {
    fn key(self) -> &'static str {
        match self {
            Self::Tokens => "tokens",
            Self::Cost => "cost",
            Self::Requests => "requests",
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Tokens => tr(cx, "Tokens", "Token"),
            Self::Cost => tr(cx, "Cost", "费用"),
            Self::Requests => tr(cx, "Requests", "请求数"),
        }
    }
}

/// Measured usage tuple across tokens, dollar cost, and request count.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UsageAmounts {
    /// Total tokens processed.
    pub tokens: f64,
    /// Total cost in USD.
    pub cost: f64,
    /// Total number of requests.
    pub requests: f64,
}

impl UsageAmounts {
    /// Construct a usage tuple from `tokens`, `cost`, and `requests`.
    pub const fn new(tokens: f64, cost: f64, requests: f64) -> Self {
        Self {
            tokens,
            cost,
            requests,
        }
    }

    /// Extract the non-negative finite value for `metric`.
    pub fn get(self, metric: UsageMetric) -> f64 {
        let raw = match metric {
            UsageMetric::Tokens => self.tokens,
            UsageMetric::Cost => self.cost,
            UsageMetric::Requests => self.requests,
        };
        if raw.is_finite() && raw > 0.0 {
            raw
        } else {
            0.0
        }
    }
}

/// Single model segment inside a [`UsageCompany`] stacked column.
#[derive(Clone, Debug, PartialEq)]
pub struct UsageModel {
    /// Stable model identifier.
    pub id: SharedString,
    /// Human-readable model name (for example `"Claude Sonnet 4.6"`).
    pub name: SharedString,
    /// Aggregate usage across measures.
    pub usage: UsageAmounts,
    /// Per-task usage split keyed by task kind (`"search"`, `"analyze"`, `"code"`, `"test"`).
    pub tasks: BTreeMap<SharedString, UsageAmounts>,
}

impl UsageModel {
    /// Construct a model entry with `id`, `name`, and aggregate `usage`.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        usage: UsageAmounts,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            usage,
            tasks: BTreeMap::new(),
        }
    }

    /// Attach usage attributed to a task category `kind` (for example `"code"` or `"search"`).
    pub fn with_task(mut self, kind: impl Into<SharedString>, amounts: UsageAmounts) -> Self {
        self.tasks.insert(kind.into(), amounts);
        self
    }
}

/// One AI provider stacked bar in [`ModelUsage`].
#[derive(Clone, Debug, PartialEq)]
pub struct UsageCompany {
    /// Stable provider identifier (`"anthropic"`, `"openai"`, `"google"`, `"deepseek"`).
    pub id: SharedString,
    /// Provider display name.
    pub name: SharedString,
    /// Optional brand key passed to [`pro_app_artwork`] (falls back to `id`).
    pub brand: Option<SharedString>,
    /// Optional custom signature hue for this provider's stacked bar.
    pub color: Option<Hsla>,
    /// Models belonging to this provider.
    pub models: Vec<UsageModel>,
}

impl UsageCompany {
    /// Construct a provider entry with `id` and `name`.
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            brand: None,
            color: None,
            models: Vec::new(),
        }
    }

    /// Override the brand mark lookup key.
    pub fn with_brand(mut self, brand: impl Into<SharedString>) -> Self {
        self.brand = Some(brand.into());
        self
    }

    /// Override the provider's signature bar color.
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    /// Populate this provider's models.
    pub fn with_models(mut self, models: Vec<UsageModel>) -> Self {
        self.models = models;
        self
    }

    /// Sum usage across all models in this company for `metric`.
    pub fn total(&self, metric: UsageMetric) -> f64 {
        self.models.iter().map(|m| m.usage.get(metric)).sum()
    }
}

/// Task category definition in the bottom breakdown of [`ModelUsage`].
#[derive(Clone, Debug, PartialEq)]
pub struct UsageTaskKind {
    /// Stable task key (`"search"`, `"analyze"`, `"code"`, `"test"`).
    pub id: SharedString,
    /// Display label.
    pub label: SharedString,
    /// Accent color for the task icon tile and progress bar.
    pub color: Hsla,
}

impl UsageTaskKind {
    /// Construct a task category definition.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>, color: Hsla) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            color,
        }
    }
}

/// Typed interaction events emitted by [`ModelUsage`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelUsageEvent {
    /// Emitted when switching between `Tokens`, `Cost`, and `Requests`.
    MetricChanged(UsageMetric),
    /// Emitted when hovering or keyboard-walking highlights a provider column (`None` on clear).
    HoverCompanyChanged(Option<SharedString>),
}

impl EventEmitter<ModelUsageEvent> for ModelUsage {}

/// Format a usage figure concisely for any scale (`9.76M`, `$68.40`, `3.91k`).
pub fn format_usage_short(value: f64, metric: UsageMetric) -> String {
    let v = if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    };
    let prefix = if metric == UsageMetric::Cost { "$" } else { "" };
    if v == 0.0 {
        return match metric {
            UsageMetric::Cost => "$0".to_string(),
            _ => "0".to_string(),
        };
    }
    if v >= 1_000_000_000_000.0 {
        format!("{prefix}{:.2}T", v / 1_000_000_000_000.0)
    } else if v >= 1_000_000_000.0 {
        format!("{prefix}{:.2}B", v / 1_000_000_000.0)
    } else if v >= 1_000_000.0 {
        let m = v / 1_000_000.0;
        if (m.fract()).abs() < 0.01 {
            format!("{prefix}{m:.0}M")
        } else {
            format!("{prefix}{m:.2}M")
        }
    } else if v >= 1_000.0 {
        let k = v / 1_000.0;
        if (k.fract()).abs() < 0.05 || k >= 100.0 {
            format!("{prefix}{k:.0}k")
        } else {
            let trimmed = format!("{prefix}{k:.2}")
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string();
            format!("{trimmed}k")
        }
    } else if metric == UsageMetric::Cost {
        if (v.fract()).abs() < 0.005 {
            format!("${v:.0}")
        } else {
            format!("${v:.2}")
        }
    } else {
        format!("{v:.0}")
    }
}

fn format_compact_clean(value: f64, metric: UsageMetric) -> String {
    let v = if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    };
    let prefix = if metric == UsageMetric::Cost { "$" } else { "" };
    if v == 0.0 {
        return format!("{prefix}0");
    }
    let trim_num = |n: f64, unit: &str| -> String {
        let s = if n >= 100.0 {
            format!("{n:.0}")
        } else if n >= 10.0 {
            format!("{n:.1}")
        } else {
            format!("{n:.2}")
        };
        let trimmed = if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.')
        } else {
            s.as_str()
        };
        format!("{prefix}{trimmed}{unit}")
    };
    if v >= 1_000_000_000_000.0 {
        trim_num(v / 1_000_000_000_000.0, "T")
    } else if v >= 1_000_000_000.0 {
        trim_num(v / 1_000_000_000.0, "B")
    } else if v >= 1_000_000.0 {
        trim_num(v / 1_000_000.0, "M")
    } else if v >= 1_000.0 {
        trim_num(v / 1_000.0, "k")
    } else if metric == UsageMetric::Cost {
        format!("${v:.2}")
    } else {
        format!("{v:.0}")
    }
}

fn default_company_color(id: &str, index: usize) -> Hsla {
    match id.to_ascii_lowercase().as_str() {
        "anthropic" | "claude" => rgb(0xf04438).into(),
        "openai" | "gpt" => rgb(0xe8b000).into(),
        "google" | "gemini" => rgb(0x5fc88a).into(),
        "deepseek" | "xai" | "grok" => rgb(0x0190fe).into(),
        "mistral" => rgb(0xf28a2e).into(),
        _ => match index % 5 {
            0 => rgb(0xf04438).into(),
            1 => rgb(0xe8b000).into(),
            2 => rgb(0x5fc88a).into(),
            3 => rgb(0x0190fe).into(),
            _ => rgb(0xa651f0).into(),
        },
    }
}

fn nice_company_ceiling(max_val: f64) -> f64 {
    if !max_val.is_finite() || max_val <= 0.0 {
        return 100.0;
    }
    let exp = 10f64.powf(max_val.log10().floor());
    let norm = max_val / exp;
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
    } else if norm <= 8.0 {
        8.0
    } else {
        10.0
    };
    step * exp
}

fn scaled_rolling_spec(val: f64, metric: UsageMetric) -> (f64, usize, &'static str, &'static str) {
    let prefix = if metric == UsageMetric::Cost { "$" } else { "" };
    let v = if val.is_finite() && val > 0.0 {
        val
    } else {
        0.0
    };
    if v >= 1_000_000_000.0 {
        (v / 1_000_000_000.0, 2, prefix, "B")
    } else if v >= 1_000_000.0 {
        (v / 1_000_000.0, 2, prefix, "M")
    } else if v >= 10_000.0 {
        (v / 1_000.0, 1, prefix, "k")
    } else if v >= 1_000.0 {
        (v / 1_000.0, 2, prefix, "k")
    } else if metric == UsageMetric::Cost {
        (v, 2, "$", "")
    } else {
        (v, 0, "", "")
    }
}

/// AI provider model usage breakdown component (`model-usage`).
pub struct ModelUsage {
    id: ElementId,
    title: SharedString,
    since: SharedString,
    previous: Option<UsageAmounts>,
    metrics: Vec<UsageMetric>,
    active_metric: UsageMetric,
    companies: Vec<UsageCompany>,
    task_kinds: Vec<UsageTaskKind>,
    hovered_company: Option<usize>,
    loading: bool,
    empty: bool,
    focus: FocusHandle,
    started_at: Instant,
}

impl ModelUsage {
    /// Create a [`ModelUsage`] card preloaded with `Anthropic`, `OpenAI`, `Google`, and `DeepSeek` model & task data.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let companies = vec![
            UsageCompany::new("anthropic", "Anthropic")
                .with_brand("anthropic")
                .with_color(rgb(0xf04438).into())
                .with_models(vec![
                    UsageModel::new(
                        "claude-sonnet",
                        "Claude Sonnet 4.6",
                        UsageAmounts::new(2_180_000., 13.08, 1_420.),
                    )
                    .with_task("search", UsageAmounts::new(720_000., 4.32, 490.))
                    .with_task("analyze", UsageAmounts::new(480_000., 2.88, 310.))
                    .with_task("code", UsageAmounts::new(690_000., 4.14, 430.))
                    .with_task("test", UsageAmounts::new(290_000., 1.74, 190.)),
                    UsageModel::new(
                        "claude-opus",
                        "Claude Opus 4.6",
                        UsageAmounts::new(1_260_000., 18.90, 520.),
                    )
                    .with_task("search", UsageAmounts::new(340_000., 5.10, 140.))
                    .with_task("analyze", UsageAmounts::new(420_000., 6.30, 175.))
                    .with_task("code", UsageAmounts::new(360_000., 5.40, 145.))
                    .with_task("test", UsageAmounts::new(140_000., 2.10, 60.)),
                    UsageModel::new(
                        "claude-haiku",
                        "Claude Haiku 4.5",
                        UsageAmounts::new(620_000., 1.86, 610.),
                    )
                    .with_task("search", UsageAmounts::new(360_000., 1.08, 355.))
                    .with_task("analyze", UsageAmounts::new(120_000., 0.36, 118.))
                    .with_task("code", UsageAmounts::new(70_000., 0.21, 69.))
                    .with_task("test", UsageAmounts::new(70_000., 0.21, 68.)),
                ]),
            UsageCompany::new("openai", "OpenAI")
                .with_brand("openai")
                .with_color(rgb(0xe8b000).into())
                .with_models(vec![
                    UsageModel::new("gpt-5", "GPT-5", UsageAmounts::new(1_540_000., 15.40, 810.))
                        .with_task("search", UsageAmounts::new(620_000., 6.20, 325.))
                        .with_task("analyze", UsageAmounts::new(410_000., 4.10, 215.))
                        .with_task("code", UsageAmounts::new(330_000., 3.30, 175.))
                        .with_task("test", UsageAmounts::new(180_000., 1.80, 95.)),
                    UsageModel::new(
                        "o4-mini",
                        "o4-mini",
                        UsageAmounts::new(910_000., 4.55, 490.),
                    )
                    .with_task("search", UsageAmounts::new(310_000., 1.55, 168.))
                    .with_task("analyze", UsageAmounts::new(220_000., 1.10, 118.))
                    .with_task("code", UsageAmounts::new(240_000., 1.20, 128.))
                    .with_task("test", UsageAmounts::new(140_000., 0.70, 76.)),
                    UsageModel::new(
                        "gpt-5-mini",
                        "GPT-5 mini",
                        UsageAmounts::new(310_000., 0.93, 280.),
                    )
                    .with_task("search", UsageAmounts::new(180_000., 0.54, 162.))
                    .with_task("analyze", UsageAmounts::new(60_000., 0.18, 54.))
                    .with_task("code", UsageAmounts::new(40_000., 0.12, 36.))
                    .with_task("test", UsageAmounts::new(30_000., 0.09, 28.)),
                ]),
            UsageCompany::new("google", "Google")
                .with_brand("gemini")
                .with_color(rgb(0x5fc88a).into())
                .with_models(vec![
                    UsageModel::new(
                        "gemini-pro",
                        "Gemini 2.5 Pro",
                        UsageAmounts::new(1_140_000., 7.98, 540.),
                    )
                    .with_task("search", UsageAmounts::new(560_000., 3.92, 265.))
                    .with_task("analyze", UsageAmounts::new(310_000., 2.17, 147.))
                    .with_task("code", UsageAmounts::new(170_000., 1.19, 80.))
                    .with_task("test", UsageAmounts::new(100_000., 0.70, 48.)),
                    UsageModel::new(
                        "gemini-flash",
                        "Gemini 2.5 Flash",
                        UsageAmounts::new(840_000., 2.10, 620.),
                    )
                    .with_task("search", UsageAmounts::new(490_000., 1.22, 360.))
                    .with_task("analyze", UsageAmounts::new(170_000., 0.43, 126.))
                    .with_task("code", UsageAmounts::new(100_000., 0.25, 74.))
                    .with_task("test", UsageAmounts::new(80_000., 0.20, 60.)),
                ]),
            UsageCompany::new("deepseek", "DeepSeek")
                .with_brand("deepseek")
                .with_color(rgb(0x0190fe).into())
                .with_models(vec![
                    UsageModel::new(
                        "deepseek-v3",
                        "DeepSeek V3.2",
                        UsageAmounts::new(560_000., 1.68, 340.),
                    )
                    .with_task("search", UsageAmounts::new(190_000., 0.57, 115.))
                    .with_task("analyze", UsageAmounts::new(110_000., 0.33, 67.))
                    .with_task("code", UsageAmounts::new(160_000., 0.48, 97.))
                    .with_task("test", UsageAmounts::new(100_000., 0.30, 61.)),
                    UsageModel::new(
                        "deepseek-r1",
                        "DeepSeek R1",
                        UsageAmounts::new(400_000., 1.92, 210.),
                    )
                    .with_task("search", UsageAmounts::new(120_000., 0.58, 63.))
                    .with_task("analyze", UsageAmounts::new(60_000., 0.29, 31.))
                    .with_task("code", UsageAmounts::new(120_000., 0.57, 63.))
                    .with_task("test", UsageAmounts::new(100_000., 0.48, 53.)),
                ]),
        ];

        let task_kinds = vec![
            UsageTaskKind::new("search", "Search", rgb(0xf04438).into()),
            UsageTaskKind::new("analyze", "Analyze", rgb(0xe8b000).into()),
            UsageTaskKind::new("code", "Code", rgb(0x5fc88a).into()),
            UsageTaskKind::new("test", "Test", rgb(0x0190fe).into()),
        ];

        Self {
            id: id.into(),
            title: "This week".into(),
            since: "since last week".into(),
            previous: Some(UsageAmounts::new(7_072_000., 51.80, 4_390.)),
            metrics: vec![
                UsageMetric::Tokens,
                UsageMetric::Cost,
                UsageMetric::Requests,
            ],
            active_metric: UsageMetric::Tokens,
            companies,
            task_kinds,
            hovered_company: None,
            loading: false,
            empty: false,
            focus: cx.focus_handle().tab_stop(true),
            started_at: Instant::now(),
        }
    }

    /// Set the period heading shown beside the rolling total (`"This week"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Customize the comparison phrase inside the delta pill (`"since last week"` by default).
    pub fn with_since(mut self, since: impl Into<SharedString>) -> Self {
        self.since = since.into();
        self
    }

    /// Set or clear the previous-period totals used to compute `%` deltas.
    pub fn with_previous(mut self, previous: Option<UsageAmounts>) -> Self {
        self.previous = previous;
        self
    }

    /// Replace the AI provider columns and their model lists.
    pub fn with_companies(mut self, companies: Vec<UsageCompany>) -> Self {
        self.companies = companies;
        self.hovered_company = None;
        self
    }

    /// Configure which measures appear in the top switcher (a single metric hides the switcher).
    pub fn with_metrics(mut self, metrics: Vec<UsageMetric>) -> Self {
        if let Some(&first) = metrics.first()
            && !metrics.contains(&self.active_metric)
        {
            self.active_metric = first;
        }
        self.metrics = metrics;
        self
    }

    /// Set the initial active measure (`Tokens`, `Cost`, or `Requests`).
    pub fn with_default_metric(mut self, metric: UsageMetric) -> Self {
        self.active_metric = metric;
        self
    }

    /// Override the task categories rendered below the bars.
    pub fn with_task_kinds(mut self, kinds: Vec<UsageTaskKind>) -> Self {
        self.task_kinds = kinds;
        self
    }

    /// Render skeleton placeholders while usage data loads.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Render the empty-usage state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the currently selected measure.
    pub fn active_metric(&self) -> UsageMetric {
        self.active_metric
    }

    /// Return the currently hovered/inspected company id, if any.
    pub fn hovered_company(&self) -> Option<&str> {
        self.hovered_company
            .and_then(|idx| self.companies.get(idx))
            .map(|c| c.id.as_ref())
    }

    /// Sum usage across every company and model for `metric`.
    pub fn period_total(&self, metric: UsageMetric) -> f64 {
        self.companies.iter().map(|c| c.total(metric)).sum()
    }

    /// Order a company's models by the first configured measure (falling back to `Tokens`) so segment order stays stable when switching measures.
    fn ordered_models<'a>(&self, company: &'a UsageCompany) -> Vec<&'a UsageModel> {
        let primary = self.metrics.first().copied().unwrap_or(UsageMetric::Tokens);
        let mut refs: Vec<&UsageModel> = company.models.iter().collect();
        refs.sort_by(|a, b| {
            b.usage
                .get(primary)
                .partial_cmp(&a.usage.get(primary))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        refs
    }

    /// Compute `(task_id, raw_amount, whole_pct)` for `company_idx` (`None` aggregates across all companies).
    pub fn task_shares(
        &self,
        company_idx: Option<usize>,
        metric: UsageMetric,
    ) -> Vec<(SharedString, f64, u32)> {
        let mut totals: Vec<f64> = Vec::with_capacity(self.task_kinds.len());
        for tk in &self.task_kinds {
            let mut sum = 0.0;
            for (ci, comp) in self.companies.iter().enumerate() {
                if company_idx.is_some_and(|target| target != ci) {
                    continue;
                }
                for m in &comp.models {
                    if let Some(amts) = m.tasks.get(&tk.id) {
                        sum += amts.get(metric);
                    }
                }
            }
            totals.push(sum);
        }

        let grand: f64 = totals.iter().sum();
        if grand <= 1e-9 {
            return self
                .task_kinds
                .iter()
                .map(|tk| (tk.id.clone(), 0.0, 0))
                .collect();
        }

        let mut floors = Vec::with_capacity(totals.len());
        let mut rems = Vec::with_capacity(totals.len());
        let mut sum_f = 0u32;
        for (i, &v) in totals.iter().enumerate() {
            let exact = (v / grand) * 100.0;
            let fl = exact.floor() as u32;
            floors.push(fl);
            sum_f += fl;
            rems.push((i, exact - fl as f64));
        }
        rems.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let deficit = 100u32.saturating_sub(sum_f) as usize;
        for &(idx, _) in rems.iter().take(deficit) {
            floors[idx] += 1;
        }

        self.task_kinds
            .iter()
            .zip(totals)
            .zip(floors)
            .map(|((tk, raw), pct)| (tk.id.clone(), raw, pct))
            .collect()
    }

    fn set_metric(&mut self, metric: UsageMetric, cx: &mut Context<Self>) {
        if self.active_metric != metric {
            self.active_metric = metric;
            cx.emit(ModelUsageEvent::MetricChanged(metric));
            cx.notify();
        }
    }

    fn set_hovered_company(&mut self, next: Option<usize>, cx: &mut Context<Self>) {
        if self.hovered_company != next {
            self.hovered_company = next;
            let cid = next
                .and_then(|i| self.companies.get(i))
                .map(|c| c.id.clone());
            cx.emit(ModelUsageEvent::HoverCompanyChanged(cid));
            cx.notify();
        }
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.companies.is_empty() {
            return;
        }
        let max_i = self.companies.len() - 1;
        match ev.keystroke.key.as_str() {
            "left" => {
                let next = match self.hovered_company {
                    Some(i) => i.saturating_sub(1),
                    None => max_i,
                };
                self.set_hovered_company(Some(next), cx);
            }
            "right" => {
                let next = match self.hovered_company {
                    Some(i) => (i + 1).min(max_i),
                    None => 0,
                };
                self.set_hovered_company(Some(next), cx);
            }
            "home" => self.set_hovered_company(Some(0), cx),
            "end" => self.set_hovered_company(Some(max_i), cx),
            "escape" => self.set_hovered_company(None, cx),
            _ => {}
        }
    }

    fn render_task_icon(&self, task_id: &str, color: Hsla) -> AnyElement {
        let icon_el = match task_id.to_ascii_lowercase().as_str() {
            "search" | "read" => pro_svg("0 0 24 24", TASK_SEARCH_SVG, 13., color),
            "analyze" | "plan" => pro_svg("0 0 24 24", TASK_ANALYZE_SVG, 13., color),
            "code" | "write" => pro_svg("0 0 24 24", TASK_CODE_SVG, 13., color),
            "test" | "check" => pro_svg("0 0 24 24", TASK_TEST_SVG, 13., color),
            _ => div()
                .size(px(8.))
                .rounded(px(2.))
                .bg(color)
                .into_any_element(),
        };
        div()
            .size(px(20.))
            .flex_none()
            .rounded(px(5.))
            .bg(color.opacity(0.13))
            .flex()
            .items_center()
            .justify_center()
            .child(icon_el)
            .into_any_element()
    }
}

impl Render for ModelUsage {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let dark = p.dark;
        let card_bg: Hsla = if dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let pop_bg: Hsla = if dark {
            rgb(0x242427).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border: Hsla = if dark {
            hsla(0., 0., 1., 0.09)
        } else {
            hsla(0., 0., 0., 0.065)
        };
        let line: Hsla = if dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let grid_c: Hsla = if dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.07)
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
                .max_w(px(560.))
                .rounded(px(12.))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .p(px(20.))
                .flex()
                .flex_col()
                .gap(px(18.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(pro_skeleton_bar(210., 22., 6., phase, cx))
                        .child(pro_skeleton_bar(168., 26., 6., phase, cx)),
                )
                .child(pro_skeleton_bar(516., 196., 8., phase, cx))
                .child(pro_skeleton_bar(516., 76., 6., phase, cx))
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(560.))
                .rounded(px(12.))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .child(pro_empty_state::<Self>(
                    ProIcon::Sparkles,
                    tr(cx, "No model usage", "暂无模型调用数据"),
                    tr(
                        cx,
                        "Token, cost, and request shares by provider appear once models run.",
                        "产生模型请求后，此处将按厂商与任务类型汇总 Token、费用与调用次数。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let reduced = disabled_motion(cx);
        let elapsed = self.started_at.elapsed().as_secs_f32();
        if !reduced && elapsed < 0.85 {
            w.request_animation_frame();
        }

        let metric = self.active_metric;
        let grand_total = self.period_total(metric);
        let (roll_val, roll_dec, roll_pre, roll_suf) = scaled_rolling_spec(grand_total, metric);

        let change_pill: Option<AnyElement> = self.previous.and_then(|prev| {
            let prev_v = prev.get(metric);
            if prev_v <= 1e-9 {
                return None;
            }
            let pct = ((grand_total - prev_v) / prev_v) * 100.0;
            let up = pct >= 0.0;
            let (cbg, cfg): (Hsla, Hsla) = if up {
                if dark {
                    (hsla(145. / 360., 0.48, 0.58, 0.14), rgb(0x5fd08f).into())
                } else {
                    (hsla(144. / 360., 0.69, 0.39, 0.10), rgb(0x1a7f45).into())
                }
            } else if dark {
                (hsla(4. / 360., 0.86, 0.58, 0.16), rgb(0xff7a6b).into())
            } else {
                (hsla(4. / 360., 0.86, 0.58, 0.10), rgb(0xc2341f).into())
            };
            let sign = if up { "+" } else { "\u{2212}" };
            Some(
                div()
                    .h(px(22.))
                    .px(px(7.))
                    .rounded(px(6.))
                    .bg(cbg)
                    .text_color(cfg)
                    .text_size(px(12.5))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "chg-pct"),
                        pct.abs(),
                        0,
                        sign,
                        "%",
                        12.5,
                        FontWeight::MEDIUM,
                        cfg,
                    ))
                    .child(self.since.clone())
                    .into_any_element(),
            )
        });

        let metric_tabs: Option<AnyElement> = (self.metrics.len() > 1).then(|| {
            let sensor = pro_selection_sensor();
            let sel_fill: Hsla = if dark {
                hsla(0., 0., 1., 0.10)
            } else {
                hsla(0., 0., 0., 0.06)
            };
            let mut row =
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .child(pro_pill_indicator(
                        pro_eid(&self.id, "mtabs"),
                        metric.key(),
                        sensor.clone(),
                        6.,
                        sel_fill,
                        None,
                        false,
                    ));
            for &m in &self.metrics {
                let selected = m == metric;
                row = row.child(
                    div()
                        .id(pro_eid(&self.id, format!("mtab-{}", m.key())))
                        .role(Role::Button)
                        .aria_label(m.label(cx))
                        .tab_index(0)
                        .h(px(26.))
                        .px(px(10.))
                        .relative()
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(13.))
                        .font_weight(if selected {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(if selected { p.strong } else { p.text })
                        .cursor_pointer()
                        .when(!selected, |d| d.hover(move |s| s.text_color(p.ink)))
                        .when(selected, |d| d.child(pro_mark_selected(&sensor)))
                        .on_click(cx.listener(move |this, _, _, cx| this.set_metric(m, cx)))
                        .child(m.label(cx)),
                );
            }
            row.into_any_element()
        });

        let max_comp = self
            .companies
            .iter()
            .map(|c| c.total(metric))
            .fold(0.0f64, f64::max);
        let has_usage = grand_total > 0.0 && max_comp > 0.0;
        let ceil = nice_company_ceiling(max_comp);
        let chart_h = 200.0f32;

        let mut cols_row = div()
            .relative()
            .size_full()
            .px(px(8.))
            .flex()
            .items_end()
            .gap(px(12.));

        for (ci, comp) in self.companies.iter().enumerate() {
            let comp_total = comp.total(metric);
            let target_h = if has_usage {
                ((comp_total / ceil) as f32 * chart_h)
                    .clamp(if comp_total > 0.0 { 4.0 } else { 0.0 }, chart_h)
            } else {
                0.0
            };
            let spring_h =
                pro_spring_f32(pro_eid(&self.id, format!("ch-{ci}")), target_h, 0.42, w, cx);
            let rise = if reduced {
                1.0
            } else {
                let delay = ci as f32 * 0.06;
                pro_ease(((elapsed - delay) / 0.50).clamp(0., 1.))
            };
            let bar_px = (spring_h * rise).max(0.0);

            let base_c = comp
                .color
                .unwrap_or_else(|| default_company_color(&comp.id, ci));
            let ordered = self.ordered_models(comp);
            let is_on = self.hovered_company == Some(ci);
            let is_dim = self.hovered_company.is_some_and(|hi| hi != ci);

            // Build segments top-to-bottom visually (lightest smallest on top, darkest largest at bottom).
            let mut stack_col = div()
                .w(px(64.))
                .max_w_full()
                .h(px(bar_px))
                .rounded_t(px(5.))
                .overflow_hidden()
                .flex()
                .flex_col()
                .gap(px(2.));

            let seg_count = ordered.len();
            for (rev_i, model) in ordered.iter().enumerate().rev() {
                let m_val = model.usage.get(metric);
                let share = if comp_total > 1e-9 {
                    (m_val / comp_total) as f32
                } else {
                    0.0
                };
                let sp_share = pro_spring_f32(
                    pro_eid(&self.id, format!("seg-{ci}-{}", model.id)),
                    share,
                    0.38,
                    w,
                    cx,
                )
                .clamp(0.0, 1.0);
                if sp_share <= 0.002 {
                    continue;
                }
                let strength = match rev_i {
                    0 => 1.0,
                    1 => 0.54,
                    2 => 0.32,
                    _ => (0.22 - (rev_i.saturating_sub(3) as f32 * 0.04)).max(0.14),
                };
                let seg_color = if seg_count <= 1 || rev_i == 0 {
                    base_c
                } else {
                    pro_mix_color(card_bg, base_c, strength)
                };
                stack_col = stack_col.child(
                    div()
                        .w_full()
                        .flex_grow(1.0)
                        .flex_basis(relative(sp_share))
                        .min_h(px(3.))
                        .bg(seg_color),
                );
            }

            let val_str = format_compact_clean(comp_total, metric);
            cols_row = cols_row.child(
                div()
                    .id(pro_eid(&self.id, format!("col-{}", comp.id)))
                    .role(Role::Button)
                    .aria_label(format!("{}: {val_str}", comp.name))
                    .tab_index(0)
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .relative()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_end()
                    .opacity(if is_dim { 0.36 } else { 1.0 })
                    .cursor_pointer()
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        if *inside {
                            this.set_hovered_company(Some(ci), cx);
                        } else if this.hovered_company == Some(ci) {
                            this.set_hovered_company(None, cx);
                        }
                    }))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let next = if this.hovered_company == Some(ci) {
                            None
                        } else {
                            Some(ci)
                        };
                        this.set_hovered_company(next, cx);
                    }))
                    .child(
                        div()
                            .mb(px(6.))
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .font_weight(if is_on {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if is_on { p.strong } else { p.muted })
                            .child(val_str),
                    )
                    .child(stack_col),
            );
        }

        let tip_overlay: Option<AnyElement> = self.hovered_company.and_then(|ci| {
            let comp = self.companies.get(ci)?;
            let comp_total = comp.total(metric);
            let base_c = comp
                .color
                .unwrap_or_else(|| default_company_color(&comp.id, ci));
            let ordered = self.ordered_models(comp);
            let brand_key = comp.brand.as_ref().unwrap_or(&comp.id);
            let logo_el = pro_app_artwork(brand_key, 13., cx);

            let mut model_rows = div().flex().flex_col().gap(px(3.));
            for (mi, m) in ordered.iter().enumerate() {
                let mv = m.usage.get(metric);
                let pct_txt = if comp_total <= 1e-9 || mv <= 0.0 {
                    "0%".to_string()
                } else {
                    let exact = (mv / comp_total) * 100.0;
                    if exact > 0.0 && exact < 1.0 {
                        "<1%".to_string()
                    } else {
                        format!("{:.0}%", exact.round())
                    }
                };
                let strength = match mi {
                    0 => 1.0,
                    1 => 0.54,
                    2 => 0.32,
                    _ => 0.20,
                };
                let swatch_c = if ordered.len() <= 1 || mi == 0 {
                    base_c
                } else {
                    pro_mix_color(card_bg, base_c, strength)
                };
                model_rows = model_rows.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .child(
                            div()
                                .size(px(8.))
                                .flex_none()
                                .rounded(px(2.))
                                .bg(swatch_c)
                                .border_1()
                                .border_color(line),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_color(p.text)
                                .truncate()
                                .child(m.name.clone()),
                        )
                        .child(div().text_color(p.muted).child(pct_txt))
                        .child(
                            div()
                                .min_w(px(44.))
                                .text_right()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(format_compact_clean(mv, metric)),
                        ),
                );
            }

            let n_cols = self.companies.len().max(1);
            let on_right_half = ci * 2 >= n_cols;
            let tip_card = div()
                .w(px(238.))
                .px(px(12.))
                .py(px(10.))
                .rounded(px(10.))
                .bg(pop_bg)
                .border_1()
                .border_color(p.line_strong)
                .shadow_lg()
                .child(
                    div()
                        .mb(px(6.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .when_some(logo_el, |d, lg| d.child(lg))
                        .child(comp.name.clone()),
                )
                .child(model_rows)
                .child(
                    div()
                        .mt(px(6.))
                        .pt(px(6.))
                        .border_t_1()
                        .border_color(line)
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_size(px(12.))
                        .child(div().text_color(p.muted).child(tr(cx, "Total", "小计")))
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(format_compact_clean(comp_total, metric)),
                        ),
                );

            let frac = (ci as f32 + 0.5) / n_cols as f32;
            if on_right_half {
                Some(
                    div()
                        .absolute()
                        .top(px(24.))
                        .left(relative((frac - 0.54).clamp(0.02, 0.48)))
                        .child(tip_card)
                        .into_any_element(),
                )
            } else {
                Some(
                    div()
                        .absolute()
                        .top(px(24.))
                        .left(relative((frac + 0.08).clamp(0.22, 0.50)))
                        .child(tip_card)
                        .into_any_element(),
                )
            }
        });

        let mut axis_row = div()
            .ml(px(44.))
            .mt(px(10.))
            .px(px(8.))
            .flex()
            .items_center()
            .gap(px(12.));
        for (ci, comp) in self.companies.iter().enumerate() {
            let is_on = self.hovered_company == Some(ci);
            let is_dim = self.hovered_company.is_some_and(|hi| hi != ci);
            let brand_key = comp.brand.as_ref().unwrap_or(&comp.id);
            let logo = pro_app_artwork(brand_key, 13., cx);
            axis_row = axis_row.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(5.))
                    .opacity(if is_dim { 0.48 } else { 1.0 })
                    .when_some(logo, |d, lg| d.child(lg))
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(18.))
                            .font_weight(if is_on {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if is_on { p.strong } else { p.text })
                            .truncate()
                            .child(comp.name.clone()),
                    ),
            );
        }

        let shares = self.task_shares(self.hovered_company, metric);
        let tasks_heading: AnyElement = if let Some(ci) = self.hovered_company
            && let Some(comp) = self.companies.get(ci)
        {
            let brand_key = comp.brand.as_ref().unwrap_or(&comp.id);
            let logo = pro_app_artwork(brand_key, 12., cx);
            div()
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(5.))
                .text_size(px(12.))
                .text_color(p.muted)
                .child(tr(cx, "Tasks for", "任务分布 ·"))
                .when_some(logo, |d, lg| d.child(lg))
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(comp.name.clone()),
                )
                .into_any_element()
        } else {
            div()
                .px(px(8.))
                .text_size(px(12.))
                .text_color(p.muted)
                .child(tr(cx, "Tasks across all models", "全部模型任务分布"))
                .into_any_element()
        };

        let mut task_pairs = Vec::with_capacity(self.task_kinds.len());
        for (ti, tk) in self.task_kinds.iter().enumerate() {
            let (_, raw_amt, pct) = shares
                .get(ti)
                .cloned()
                .unwrap_or_else(|| (tk.id.clone(), 0.0, 0));
            let sp_bar = pro_spring_f32(
                pro_eid(&self.id, format!("tbar-{}", tk.id)),
                (pct as f32 / 100.0).clamp(0., 1.),
                0.36,
                w,
                cx,
            );
            let row_el = div()
                .flex_1()
                .min_w(px(180.))
                .h(px(30.))
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(8.))
                .opacity(if raw_amt <= 0.0 && has_usage {
                    0.42
                } else {
                    1.0
                })
                .child(self.render_task_icon(&tk.id, tk.color))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(13.))
                        .text_color(p.text)
                        .truncate()
                        .child(tk.label.clone()),
                )
                .child(
                    div()
                        .w(px(56.))
                        .h(px(4.))
                        .flex_none()
                        .rounded(px(2.))
                        .bg(if dark {
                            hsla(0., 0., 1., 0.07)
                        } else {
                            hsla(0., 0., 0., 0.06)
                        })
                        .overflow_hidden()
                        .child(
                            div()
                                .h_full()
                                .w(relative(sp_bar))
                                .rounded(px(2.))
                                .bg(tk.color),
                        ),
                )
                .child(
                    div()
                        .min_w(px(34.))
                        .flex()
                        .justify_end()
                        .child(pro_rolling_number(
                            pro_eid(&self.id, format!("tpct-{}", tk.id)),
                            pct as f64,
                            0,
                            "",
                            "%",
                            13.,
                            FontWeight::NORMAL,
                            p.ink,
                        )),
                );
            task_pairs.push(row_el);
        }

        let mut tasks_grid = div().w_full().flex().flex_col().gap(px(2.));
        let mut iter = task_pairs.into_iter();
        while let Some(left_item) = iter.next() {
            let mut line_row = div().w_full().flex().flex_wrap().gap_x(px(16.));
            line_row = line_row.child(left_item);
            if let Some(right_item) = iter.next() {
                line_row = line_row.child(right_item);
            }
            tasks_grid = tasks_grid.child(line_row);
        }

        let y_top = format_compact_clean(ceil, metric);
        let y_mid = format_compact_clean(ceil * 0.5, metric);
        let y_bot = if metric == UsageMetric::Cost {
            "$0"
        } else {
            "0"
        };

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .role(Role::Group)
            .aria_label(self.title.clone())
            .w_full()
            .max_w(px(560.))
            .rounded(px(12.))
            .bg(card_bg)
            .border_1()
            .border_color(card_border)
            .when(!dark, |d| d.shadow_xs())
            .px(px(20.))
            .pt(px(18.))
            .pb(px(14.))
            .flex()
            .flex_col()
            .on_key_down(cx.listener(Self::handle_key_down))
            .child(
                div()
                    .mb(px(28.))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(8.))
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(p.text)
                                    .child(self.title.clone()),
                            )
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "grand-tot"),
                                roll_val,
                                roll_dec,
                                roll_pre,
                                roll_suf,
                                13.5,
                                FontWeight::MEDIUM,
                                p.strong,
                            ))
                            .when_some(change_pill, |d, chg| d.child(chg)),
                    )
                    .when_some(metric_tabs, |d, tabs| d.child(tabs)),
            )
            .child(
                div()
                    .w_full()
                    .h(px(chart_h))
                    .flex()
                    .items_stretch()
                    .gap(px(10.))
                    .child(
                        div()
                            .w(px(34.))
                            .h_full()
                            .flex_none()
                            .flex()
                            .flex_col()
                            .justify_between()
                            .items_end()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.muted)
                            .child(y_top)
                            .child(y_mid)
                            .child(y_bot),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .relative()
                            .child(
                                div()
                                    .absolute()
                                    .inset_0()
                                    .flex()
                                    .flex_col()
                                    .justify_between()
                                    .child(div().w_full().h(px(1.)).bg(grid_c))
                                    .child(div().w_full().h(px(1.)).bg(grid_c))
                                    .child(div().w_full().h(px(1.)).bg(grid_c)),
                            )
                            .child(cols_row)
                            .when(!has_usage, |d| {
                                d.child(
                                    div()
                                        .absolute()
                                        .inset_0()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            div()
                                                .px(px(10.))
                                                .bg(card_bg)
                                                .text_size(px(12.5))
                                                .text_color(p.muted)
                                                .child(tr(
                                                    cx,
                                                    "No usage this week",
                                                    "本周暂无用量",
                                                )),
                                        ),
                                )
                            })
                            .when_some(tip_overlay, |d, tip| d.child(tip)),
                    ),
            )
            .child(axis_row)
            .when(!self.task_kinds.is_empty(), |d| {
                d.child(
                    div()
                        .mt(px(16.))
                        .mx(px(-8.))
                        .pt(px(12.))
                        .border_t_1()
                        .border_color(line)
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(tasks_heading)
                        .child(tasks_grid),
                )
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{ModelUsage, UsageMetric, format_usage_short};
    use gpui::{AppContext, TestAppContext};

    #[test]
    fn short_formatter_handles_tokens_cost_and_requests() {
        assert_eq!(
            format_usage_short(9_760_000.0, UsageMetric::Tokens),
            "9.76M"
        );
        assert_eq!(format_usage_short(68.4, UsageMetric::Cost), "$68.40");
        assert_eq!(format_usage_short(0.0, UsageMetric::Cost), "$0");
    }

    #[gpui::test]
    fn task_shares_filter_dynamically_by_hovered_provider(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx.new(|cx| ModelUsage::new("mu-test", cx));
        view.update(cx, |mu, cx| {
            let all_shares = mu.task_shares(None, UsageMetric::Tokens);
            let sum_all: u32 = all_shares.iter().map(|(_, _, p)| *p).sum();
            assert_eq!(sum_all, 100);

            let anth_shares = mu.task_shares(Some(0), UsageMetric::Tokens);
            let sum_anth: u32 = anth_shares.iter().map(|(_, _, p)| *p).sum();
            assert_eq!(sum_anth, 100);
            assert_ne!(all_shares[2].2, anth_shares[2].2);

            mu.set_metric(UsageMetric::Cost, cx);
            assert_eq!(mu.active_metric(), UsageMetric::Cost);
            assert!(mu.period_total(UsageMetric::Cost) > 50.0);
        });
    }
}
