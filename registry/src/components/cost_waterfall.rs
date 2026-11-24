//! Financial and token waterfall of an agent run across LLM calls, tool calls, and sub-agents.
//!
//! Renders cumulative stepped waterfall bars for every step in an agent run with model artwork,
//! input/output token counts, dollar cost, elapsed duration, a `TOP` badge on the most expensive
//! step, collapsible sub-agent branches, expandable prompt/response details (`pro_fold`), and a
//! spring-animated switch (`pro_pill_indicator` + `pro_spring_f32`) between `Cost ($)` and `Tokens`.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_eid, pro_empty_state, pro_fold, pro_mark_selected, pro_mix_color,
        pro_pill_indicator, pro_rolling_number, pro_selection_sensor, pro_shimmer_text,
        pro_skeleton_bar, pro_spring_f32, tr,
    },
    primitives::{
        control::activate,
        family::disabled_motion,
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashSet, rc::Rc};
use web_time::Instant;

const SPARK_SVG: &str = r#"<path d="M7 1.2c.4 2.9 1.9 4.4 4.8 4.8-2.9.4-4.4 1.9-4.8 4.8C6.6 7.9 5.1 6.4 2.2 6 5.1 5.6 6.6 4.1 7 1.2Z" fill="currentColor"></path><circle cx="11.6" cy="11.4" r="1.1" fill="currentColor" opacity="0.55"></circle>"#;

/// Measurement axis displayed by [`CostWaterfall`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CostMeasureMode {
    /// Cumulative dollar cost (`$`).
    #[default]
    Cost,
    /// Cumulative total tokens (`in + out`).
    Tokens,
}

impl CostMeasureMode {
    fn key(self) -> &'static str {
        match self {
            Self::Cost => "cost",
            Self::Tokens => "tokens",
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Cost => tr(cx, "Cost", "费用"),
            Self::Tokens => tr(cx, "Tokens", "Token"),
        }
    }
}

/// Category of a [`CostStep`] determining bar tint and inspector headings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CostStepKind {
    /// Sub-agent container or autonomous worker (`Task` / `Result`).
    Agent,
    /// Model inference call (`Prompt` / `Response`, purple tint).
    #[default]
    Llm,
    /// External tool or search call (`Input` / `Output`, cyan tint).
    Tool,
}

/// Operational status of a [`CostStep`] or overall run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CostStepStatus {
    /// Step is still streaming tokens or waiting on a tool response.
    Running,
    /// Step finished normally.
    #[default]
    Done,
    /// Step terminated with an error.
    Failed,
}

/// Hierarchical step entry in [`CostWaterfall`].
#[derive(Clone, Debug, PartialEq)]
pub struct CostStep {
    /// Stable step identifier.
    pub id: SharedString,
    /// Step or tool call name.
    pub name: SharedString,
    /// Step category (`Agent`, `Llm`, or `Tool`).
    pub kind: CostStepKind,
    /// Step execution state (`Running`, `Done`, or `Failed`).
    pub status: CostStepStatus,
    /// Optional model label (such as `"Claude Opus 4.6"` or `"Gemini 2.5 Flash"`).
    pub model: Option<SharedString>,
    /// Optional secondary detail when `model` is absent (such as tool query summary).
    pub detail: Option<SharedString>,
    /// Optional explicit provider/tool icon key for [`pro_app_artwork`].
    pub icon: Option<SharedString>,
    /// Input/prompt tokens consumed directly by this step.
    pub tokens_in: u64,
    /// Output/completion tokens generated directly by this step.
    pub tokens_out: u64,
    /// Direct dollar cost of this step (negative values represent prompt-cache/batch credits).
    pub cost: f64,
    /// Step duration in milliseconds.
    pub duration_ms: u64,
    /// Optional prompt, task, or JSON input payload shown when expanded.
    pub input: Option<SharedString>,
    /// Optional completion, result, or tool output shown when expanded.
    pub output: Option<SharedString>,
    /// Optional error message shown when `status` is [`CostStepStatus::Failed`].
    pub error: Option<SharedString>,
    /// Nested child steps owned by a sub-agent (`CostStepKind::Agent`).
    pub children: Vec<CostStep>,
}

impl CostStep {
    /// Construct a new [`CostStep`] with `Done` status and zeroed counters.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        kind: CostStepKind,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            status: CostStepStatus::Done,
            model: None,
            detail: None,
            icon: None,
            tokens_in: 0,
            tokens_out: 0,
            cost: 0.0,
            duration_ms: 0,
            input: None,
            output: None,
            error: None,
            children: Vec::new(),
        }
    }

    /// Set the step execution state.
    pub fn with_status(mut self, status: CostStepStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the model label and infer provider artwork.
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the short secondary detail text.
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Override the provider/tool artwork key (`"anthropic"`, `"openai"`, `"gemini"`, `"slack"`).
    pub fn with_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set input and output token counts for this step.
    pub fn with_tokens(mut self, tokens_in: u64, tokens_out: u64) -> Self {
        self.tokens_in = tokens_in;
        self.tokens_out = tokens_out;
        self
    }

    /// Set direct cost in USD (supports negative credit adjustments).
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.cost = cost;
        self
    }

    /// Set elapsed or completed duration in milliseconds.
    pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Set input/prompt and output/response text shown inside the step inspector.
    pub fn with_io(
        mut self,
        input: impl Into<SharedString>,
        output: impl Into<SharedString>,
    ) -> Self {
        self.input = Some(input.into());
        self.output = Some(output.into());
        self
    }

    /// Attach a failure message and mark the step as [`CostStepStatus::Failed`].
    pub fn with_error(mut self, error: impl Into<SharedString>) -> Self {
        self.status = CostStepStatus::Failed;
        self.error = Some(error.into());
        self
    }

    /// Attach nested child steps under this sub-agent.
    pub fn with_children(mut self, children: Vec<CostStep>) -> Self {
        self.children = children;
        self
    }
}

/// Events emitted by [`CostWaterfall`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CostWaterfallEvent {
    /// Emitted when switching between `Cost` and `Tokens` modes.
    ModeChanged(CostMeasureMode),
    /// Emitted when opening or closing a step's inspector drawer.
    OpenChanged(Option<SharedString>),
    /// Emitted when folding or unfolding a sub-agent branch.
    CollapsedChanged(Vec<SharedString>),
}

impl EventEmitter<CostWaterfallEvent> for CostWaterfall {}

#[derive(Clone, Debug)]
struct FlatCostRow {
    id: SharedString,
    parent_id: Option<SharedString>,
    name: SharedString,
    kind: CostStepKind,
    status: CostStepStatus,
    sub_label: Option<SharedString>,
    artwork_key: Option<SharedString>,
    level: usize,
    has_children: bool,
    child_count: usize,
    folded: bool,
    visible: bool,
    tokens_in: u64,
    tokens_out: u64,
    cost: f64,
    duration_ms: u64,
    cost_start: f64,
    cost_delta: f64,
    tok_start: f64,
    tok_delta: f64,
    is_credit: bool,
    input: Option<SharedString>,
    output: Option<SharedString>,
    error: Option<SharedString>,
}

#[derive(Clone, Debug)]
struct WaterfallSummary {
    rows: Vec<FlatCostRow>,
    total_cost: f64,
    total_in: u64,
    total_out: u64,
    cost_axis_max: f64,
    tok_axis_max: f64,
    top_cost_leaf: Option<(SharedString, SharedString, f64)>,
    top_tok_leaf: Option<(SharedString, SharedString, u64)>,
    run_status: CostStepStatus,
}

fn infer_provider_key(step: &CostStep) -> Option<SharedString> {
    if let Some(ic) = &step.icon {
        return Some(ic.clone());
    }
    if let Some(model) = &step.model {
        let lower = model.to_ascii_lowercase();
        if lower.contains("claude")
            || lower.contains("opus")
            || lower.contains("sonnet")
            || lower.contains("haiku")
        {
            return Some("claude".into());
        }
        if lower.contains("gemini") {
            return Some("gemini".into());
        }
        if lower.contains("gpt")
            || lower.contains("o1")
            || lower.contains("o3")
            || lower.contains("o4")
            || lower.contains("openai")
        {
            return Some("openai".into());
        }
        if lower.contains("grok") {
            return Some("xai".into());
        }
        if lower.contains("mistral") {
            return Some("mistral".into());
        }
    }
    let name_lower = step.name.to_ascii_lowercase();
    if name_lower.contains("github") {
        Some("github".into())
    } else if name_lower.contains("slack") {
        Some("slack".into())
    } else if name_lower.contains("stripe") {
        Some("stripe".into())
    } else {
        None
    }
}

fn nice_cost_ceiling(v: f64) -> f64 {
    let v = if v.is_finite() && v > 0.0 { v } else { 0.05 };
    let steps: [f64; 14] = [
        0.02, 0.05, 0.08, 0.10, 0.15, 0.20, 0.25, 0.40, 0.50, 0.80, 1.0, 2.0, 5.0, 10.0,
    ];
    for s in steps {
        if s >= v * 1.04 {
            return s;
        }
    }
    (v * 1.15 * 10.0_f64).ceil() / 10.0
}

fn nice_token_ceiling(v: f64) -> f64 {
    let v = if v.is_finite() && v > 0.0 {
        v
    } else {
        10_000.0
    };
    let steps: [f64; 12] = [
        2_000.0, 5_000.0, 10_000.0, 15_000.0, 20_000.0, 30_000.0, 40_000.0, 50_000.0, 80_000.0,
        100_000.0, 200_000.0, 500_000.0,
    ];
    for s in steps {
        if s >= v * 1.04 {
            return s;
        }
    }
    (v * 1.15 / 10_000.0_f64).ceil() * 10_000.0
}

pub(crate) fn format_usd(amount: f64) -> String {
    if !amount.is_finite() {
        return "–".to_string();
    }
    let neg = amount < -1e-7;
    let abs = amount.abs();
    let body = if abs < 1e-6 {
        "$0".to_string()
    } else if abs < 0.01 {
        format!("${abs:.4}")
    } else if abs < 1.0 {
        let s = format!("{abs:.3}");
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        format!("${trimmed}")
    } else {
        format!("${abs:.2}")
    };
    if neg { format!("-{body}") } else { body }
}

pub(crate) fn format_tokens(count: u64) -> String {
    if count == 0 {
        "0".to_string()
    } else if count < 1_000 {
        format!("{count}")
    } else if count < 100_000 {
        let k = count as f64 / 1_000.0;
        let s = format!("{k:.1}");
        format!("{}k", s.trim_end_matches(".0"))
    } else if count < 1_000_000 {
        format!("{}k", (count + 500) / 1_000)
    } else {
        format!("{:.2}M", count as f64 / 1_000_000.0)
    }
}

pub(crate) fn format_duration_ms(ms: u64) -> String {
    if ms == 0 {
        "–".to_string()
    } else if ms < 1_000 {
        format!("{ms}ms")
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1_000.0)
    } else {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1_000;
        format!("{mins}m {secs:02}s")
    }
}

fn compute_waterfall(steps: &[CostStep], collapsed: &HashSet<SharedString>) -> WaterfallSummary {
    let mut rows: Vec<FlatCostRow> = Vec::new();
    let mut cost_cursor = 0.0_f64;
    let mut tok_cursor = 0.0_f64;
    let mut peak_cost = 0.0_f64;
    let mut peak_tok = 0.0_f64;
    let mut total_in = 0_u64;
    let mut total_out = 0_u64;
    let mut top_cost_leaf: Option<(SharedString, SharedString, f64)> = None;
    let mut top_tok_leaf: Option<(SharedString, SharedString, u64)> = None;
    let mut has_running = false;
    let mut has_failed = false;

    #[allow(clippy::too_many_arguments)]
    fn walk(
        step: &CostStep,
        parent_id: Option<SharedString>,
        level: usize,
        ancestor_hidden: bool,
        collapsed: &HashSet<SharedString>,
        rows: &mut Vec<FlatCostRow>,
        cost_cursor: &mut f64,
        tok_cursor: &mut f64,
        peak_cost: &mut f64,
        peak_tok: &mut f64,
        total_in: &mut u64,
        total_out: &mut u64,
        top_cost_leaf: &mut Option<(SharedString, SharedString, f64)>,
        top_tok_leaf: &mut Option<(SharedString, SharedString, u64)>,
        has_running: &mut bool,
        has_failed: &mut bool,
    ) -> (u64, u64, f64, u64, usize) {
        match step.status {
            CostStepStatus::Running => *has_running = true,
            CostStepStatus::Failed => *has_failed = true,
            CostStepStatus::Done => {}
        }

        let own_cost = if step.cost.is_finite() {
            step.cost
        } else {
            0.0
        };
        let own_in = step.tokens_in;
        let own_out = step.tokens_out;
        let own_tok = (own_in + own_out) as f64;
        let folded = collapsed.contains(&step.id);
        let idx = rows.len();

        let sub_label = step.model.clone().or_else(|| step.detail.clone());
        let artwork_key = infer_provider_key(step);
        rows.push(FlatCostRow {
            id: step.id.clone(),
            parent_id: parent_id.clone(),
            name: step.name.clone(),
            kind: step.kind,
            status: step.status,
            sub_label,
            artwork_key,
            level,
            has_children: !step.children.is_empty(),
            child_count: 0,
            folded,
            visible: !ancestor_hidden,
            tokens_in: own_in,
            tokens_out: own_out,
            cost: own_cost,
            duration_ms: step.duration_ms,
            cost_start: *cost_cursor,
            cost_delta: 0.0,
            tok_start: *tok_cursor,
            tok_delta: 0.0,
            is_credit: false,
            input: step.input.clone(),
            output: step.output.clone(),
            error: step.error.clone(),
        });

        if step.children.is_empty() {
            let c_start = if own_cost >= 0.0 {
                *cost_cursor
            } else {
                (*cost_cursor + own_cost).max(0.0)
            };
            let c_delta = own_cost.abs();
            *cost_cursor = (*cost_cursor + own_cost).max(0.0);
            *peak_cost = peak_cost.max(*cost_cursor).max(c_start + c_delta);

            let t_start = *tok_cursor;
            *tok_cursor += own_tok;
            *peak_tok = peak_tok.max(*tok_cursor);

            *total_in += own_in;
            *total_out += own_out;

            rows[idx].cost_start = c_start;
            rows[idx].cost_delta = c_delta;
            rows[idx].tok_start = t_start;
            rows[idx].tok_delta = own_tok;
            rows[idx].is_credit = own_cost < -1e-7;

            if own_cost > 0.0
                && top_cost_leaf
                    .as_ref()
                    .is_none_or(|(_, _, best)| own_cost > *best)
            {
                *top_cost_leaf = Some((step.id.clone(), step.name.clone(), own_cost));
            }
            let leaf_tok = own_in + own_out;
            if leaf_tok > 0
                && top_tok_leaf
                    .as_ref()
                    .is_none_or(|(_, _, best)| leaf_tok > *best)
            {
                *top_tok_leaf = Some((step.id.clone(), step.name.clone(), leaf_tok));
            }

            (own_in, own_out, own_cost, step.duration_ms, 1)
        } else {
            let start_c = *cost_cursor;
            let start_t = *tok_cursor;
            let mut sum_in = own_in;
            let mut sum_out = own_out;
            let mut sum_cost = own_cost;
            let mut sum_dur = 0_u64;
            let mut desc_leaves = 0usize;

            if own_cost.abs() > 1e-8 || own_tok > 0.0 {
                *cost_cursor = (*cost_cursor + own_cost).max(0.0);
                *tok_cursor += own_tok;
                *peak_cost = peak_cost.max(*cost_cursor);
                *peak_tok = peak_tok.max(*tok_cursor);
                *total_in += own_in;
                *total_out += own_out;
            }

            for child in &step.children {
                let (ci, co, cc, cd, cl) = walk(
                    child,
                    Some(step.id.clone()),
                    level + 1,
                    ancestor_hidden || folded,
                    collapsed,
                    rows,
                    cost_cursor,
                    tok_cursor,
                    peak_cost,
                    peak_tok,
                    total_in,
                    total_out,
                    top_cost_leaf,
                    top_tok_leaf,
                    has_running,
                    has_failed,
                );
                sum_in += ci;
                sum_out += co;
                sum_cost += cc;
                sum_dur += cd;
                desc_leaves += cl;
            }

            let dur = if step.duration_ms > 0 {
                step.duration_ms
            } else {
                sum_dur
            };
            let end_c = *cost_cursor;
            let end_t = *tok_cursor;
            rows[idx].tokens_in = sum_in;
            rows[idx].tokens_out = sum_out;
            rows[idx].cost = sum_cost;
            rows[idx].duration_ms = dur;
            rows[idx].child_count = step.children.len();
            rows[idx].cost_start = start_c.min(end_c);
            rows[idx].cost_delta = (end_c - start_c).abs();
            rows[idx].tok_start = start_t;
            rows[idx].tok_delta = (end_t - start_t).max(0.0);
            rows[idx].is_credit = sum_cost < -1e-7;

            (sum_in, sum_out, sum_cost, dur, desc_leaves)
        }
    }

    for step in steps {
        walk(
            step,
            None,
            0,
            false,
            collapsed,
            &mut rows,
            &mut cost_cursor,
            &mut tok_cursor,
            &mut peak_cost,
            &mut peak_tok,
            &mut total_in,
            &mut total_out,
            &mut top_cost_leaf,
            &mut top_tok_leaf,
            &mut has_running,
            &mut has_failed,
        );
    }
    let actual_peak_tok = rows.iter().fold(peak_tok.max(tok_cursor), |acc, r| {
        acc.max(r.tok_start + r.tok_delta)
    });

    let run_status = if has_failed {
        CostStepStatus::Failed
    } else if has_running {
        CostStepStatus::Running
    } else {
        CostStepStatus::Done
    };

    WaterfallSummary {
        rows,
        total_cost: cost_cursor,
        total_in,
        total_out,
        cost_axis_max: nice_cost_ceiling(peak_cost),
        tok_axis_max: nice_token_ceiling(actual_peak_tok),
        top_cost_leaf,
        top_tok_leaf,
        run_status,
    }
}

fn default_cost_steps(cx: &App) -> Vec<CostStep> {
    vec![
        CostStep::new(
            "plan",
            tr(cx, "Plan the report", "规划调研提纲"),
            CostStepKind::Llm,
        )
        .with_model("Claude Opus 4.6")
        .with_tokens(4_120, 960)
        .with_cost(0.044)
        .with_duration_ms(2_900)
        .with_io(
            tr(
                cx,
                "Write an executive benchmark report on 2026 frontier LLM API pricing, caching discounts, and batch tiers.",
                "撰写 2026 前沿大模型 API 定价、上下文缓存折扣与批处理阶梯价格基准报告。",
            ),
            "1. Gather provider pricing tables\n2. Normalize USD / 1M token rates\n3. Synthesize workload cost matrix",
        ),
        CostStep::new(
            "research",
            tr(cx, "Researcher", "价格检索子智能体"),
            CostStepKind::Agent,
        )
        .with_model("Gemini 2.5 Flash")
        .with_duration_ms(5_100)
        .with_io(
            tr(
                cx,
                "Collect official 2026 API rate cards across Anthropic, OpenAI, and Google Cloud.",
                "抓取 Anthropic、OpenAI 与 Google Cloud 官方最新 API 价目表。",
            ),
            tr(
                cx,
                "Indexed 9 frontier models with verified input, cached-input, and output rates.",
                "已归档 9 款前沿模型的输入、缓存命中与输出单价。",
            ),
        )
        .with_children(vec![
            CostStep::new("s1", "web_search", CostStepKind::Tool)
                .with_detail(tr(cx, "API prices 2026", "2026 API 价格表"))
                .with_tokens(620, 1_840)
                .with_cost(0.011)
                .with_duration_ms(1_280)
                .with_io(
                    "{ \"query\": \"frontier LLM API pricing per 1M tokens 2026\" }",
                    "12 official pricing documents matched",
                ),
            CostStep::new(
                "s2",
                tr(cx, "Extract rate cards", "提取结构化单价"),
                CostStepKind::Llm,
            )
            .with_model("Gemini 2.5 Flash")
            .with_tokens(6_800, 1_420)
            .with_cost(0.018)
            .with_duration_ms(2_140)
            .with_io(
                "Normalize input, cached-input, and output USD/1M rates into typed JSON.",
                "{ \"models\": 9, \"unit\": \"USD_per_1M\", \"cache_discount\": \"75%-90%\" }",
            ),
            CostStep::new("s3", "verify_citations", CostStepKind::Tool)
                .with_detail(tr(cx, "9 provider URLs", "核验 9 处官方来源"))
                .with_tokens(940, 510)
                .with_cost(0.007)
                .with_duration_ms(1_120)
                .with_io(
                    "{ \"urls\": 9, \"check_last_modified\": true }",
                    "9/9 rate cards verified (HTTP 200)",
                ),
        ]),
        CostStep::new(
            "synthesize",
            tr(cx, "Synthesize pricing matrix", "汇总对比矩阵"),
            CostStepKind::Llm,
        )
        .with_model("Claude Opus 4.6")
        .with_tokens(8_450, 2_180)
        .with_cost(0.062)
        .with_duration_ms(4_200)
        .with_io(
            tr(
                cx,
                "Build a side-by-side cost comparison across interactive chat, coding agents, and nightly batch pipelines.",
                "生成交互对话、编码智能体与夜间批处理三类负载的成本对比矩阵。",
            ),
            tr(
                cx,
                "Produced 9-model comparison table + caching break-even thresholds.",
                "已输出 9 模型单价对比表与缓存盈亏平衡阈值分析。",
            ),
        ),
        CostStep::new(
            "draft",
            tr(cx, "Draft executive summary", "生成执行摘要"),
            CostStepKind::Llm,
        )
        .with_status(CostStepStatus::Running)
        .with_model("GPT-5")
        .with_tokens(3_600, 1_140)
        .with_cost(0.024)
        .with_duration_ms(3_100)
        .with_io(
            tr(
                cx,
                "Summarize the key 2026 cost takeaways for engineering leadership in 3 crisp bullets.",
                "用三条精炼要点为研发负责人总结 2026 API 选型与降本建议。",
            ),
            tr(
                cx,
                "Streaming section 3 of 3: prompt-caching routing saves 64% on multi-turn agent loops…",
                "正在流式生成第 3/3 节：多轮智能体循环启用提示词缓存可节省 64% 成本…",
            ),
        ),
    ]
}

/// Financial and token waterfall card for inspecting multi-step agent run spend.
pub struct CostWaterfall {
    id: ElementId,
    title: Option<SharedString>,
    task: Option<SharedString>,
    status_override: Option<CostStepStatus>,
    steps: Vec<CostStep>,
    mode: CostMeasureMode,
    collapsed: HashSet<SharedString>,
    open_id: Option<SharedString>,
    focused_row: usize,
    max_height: Option<f32>,
    loading: bool,
    empty: bool,
    started_at: Instant,
    measured_w: Rc<Cell<f32>>,
    focus: FocusHandle,
}

impl CostWaterfall {
    /// Create a [`CostWaterfall`] preloaded with the `"Report on LLM API prices in 2026"` run.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let steps = default_cost_steps(cx);
        Self {
            id: id.into(),
            title: None,
            task: Some(
                tr(
                    cx,
                    "Report on LLM API prices in 2026",
                    "2026 大模型 API 定价基准报告",
                )
                .into(),
            ),
            status_override: None,
            steps,
            mode: CostMeasureMode::Cost,
            collapsed: HashSet::new(),
            open_id: None,
            focused_row: 0,
            max_height: None,
            loading: false,
            empty: false,
            started_at: Instant::now(),
            measured_w: Rc::new(Cell::new(756.0)),
            focus: cx.focus_handle(),
        }
    }

    /// Override the bar heading (`"Run cost"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the muted run task description next to the heading.
    pub fn with_task(mut self, task: impl Into<SharedString>) -> Self {
        self.task = Some(task.into());
        self
    }

    /// Supply the hierarchical list of [`CostStep`] items.
    pub fn with_steps(mut self, steps: Vec<CostStep>) -> Self {
        self.steps = steps;
        self
    }

    /// Set the initial measurement axis (`Cost` or `Tokens`).
    pub fn with_mode(mut self, mode: CostMeasureMode) -> Self {
        self.mode = mode;
        self
    }

    /// Override the overall run status pill (`Running`, `Done`, or `Failed`).
    pub fn with_status(mut self, status: Option<CostStepStatus>) -> Self {
        self.status_override = status;
        self
    }

    /// Pre-collapse specific sub-agent step ids.
    pub fn with_collapsed<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.collapsed = ids.into_iter().map(Into::into).collect();
        self
    }

    /// Pre-open a specific step's prompt/response inspector drawer.
    pub fn with_open_id(mut self, open_id: Option<SharedString>) -> Self {
        self.open_id = open_id;
        self
    }

    /// Cap the scrollable height of the waterfall rows in pixels.
    pub fn with_max_height(mut self, max_height: Option<f32>) -> Self {
        self.max_height = max_height;
        self
    }

    /// Render skeleton bars while run cost metrics are loading.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Render the empty waiting state (`"Waiting for the first step…"`).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    fn set_mode(&mut self, next: CostMeasureMode, cx: &mut Context<Self>) {
        if self.mode != next {
            self.mode = next;
            cx.emit(CostWaterfallEvent::ModeChanged(next));
            cx.notify();
        }
    }

    fn toggle_open(&mut self, id: SharedString, cx: &mut Context<Self>) {
        if self.open_id.as_ref() == Some(&id) {
            self.open_id = None;
        } else {
            self.open_id = Some(id);
        }
        cx.emit(CostWaterfallEvent::OpenChanged(self.open_id.clone()));
        cx.notify();
    }

    fn toggle_fold(&mut self, id: SharedString, cx: &mut Context<Self>) {
        if !self.collapsed.remove(&id) {
            self.collapsed.insert(id.clone());
            // If an open child lives under the newly folded parent, close its detail drawer.
            let summary = compute_waterfall(&self.steps, &self.collapsed);
            if let Some(open) = &self.open_id
                && summary.rows.iter().any(|r| &r.id == open && !r.visible)
            {
                self.open_id = None;
                cx.emit(CostWaterfallEvent::OpenChanged(None));
            }
        }
        let mut list: Vec<SharedString> = self.collapsed.iter().cloned().collect();
        list.sort();
        cx.emit(CostWaterfallEvent::CollapsedChanged(list));
        cx.notify();
    }

    fn jump_to_top_step(&mut self, target_id: SharedString, cx: &mut Context<Self>) {
        // Unfold any collapsed ancestors of `target_id`.
        let full = compute_waterfall(&self.steps, &HashSet::new());
        let mut cursor = Some(target_id.clone());
        let mut unfolded = false;
        while let Some(cid) = cursor {
            if let Some(r) = full.rows.iter().find(|r| r.id == cid) {
                if let Some(pid) = &r.parent_id
                    && self.collapsed.remove(pid)
                {
                    unfolded = true;
                }
                cursor = r.parent_id.clone();
            } else {
                break;
            }
        }
        if unfolded {
            let mut list: Vec<SharedString> = self.collapsed.iter().cloned().collect();
            list.sort();
            cx.emit(CostWaterfallEvent::CollapsedChanged(list));
        }
        let vis = compute_waterfall(&self.steps, &self.collapsed);
        if let Some(pos) = vis
            .rows
            .iter()
            .filter(|r| r.visible)
            .position(|r| r.id == target_id)
        {
            self.focused_row = pos;
        }
        self.open_id = Some(target_id);
        cx.emit(CostWaterfallEvent::OpenChanged(self.open_id.clone()));
        cx.notify();
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _w: &mut Window, cx: &mut Context<Self>) {
        let summary = compute_waterfall(&self.steps, &self.collapsed);
        let vis: Vec<&FlatCostRow> = summary.rows.iter().filter(|r| r.visible).collect();
        if vis.is_empty() {
            return;
        }
        let cur = self.focused_row.min(vis.len() - 1);
        match ev.keystroke.key.as_str() {
            "down" => {
                self.focused_row = (cur + 1).min(vis.len() - 1);
                cx.notify();
            }
            "up" => {
                self.focused_row = cur.saturating_sub(1);
                cx.notify();
            }
            "home" => {
                self.focused_row = 0;
                cx.notify();
            }
            "end" => {
                self.focused_row = vis.len() - 1;
                cx.notify();
            }
            "right" => {
                let row = vis[cur];
                if row.has_children && row.folded {
                    self.toggle_fold(row.id.clone(), cx);
                } else if row.has_children && cur + 1 < vis.len() {
                    self.focused_row = cur + 1;
                    cx.notify();
                }
            }
            "left" => {
                let row = vis[cur];
                if row.has_children && !row.folded {
                    self.toggle_fold(row.id.clone(), cx);
                } else if let Some(pid) = &row.parent_id
                    && let Some(p_pos) = vis.iter().position(|r| &r.id == pid)
                {
                    self.focused_row = p_pos;
                    cx.notify();
                }
            }
            "enter" | "space" => {
                let id = vis[cur].id.clone();
                self.toggle_open(id, cx);
            }
            "escape" if self.open_id.take().is_some() => {
                cx.emit(CostWaterfallEvent::OpenChanged(None));
                cx.notify();
            }
            _ => {}
        }
    }
}

impl Render for CostWaterfall {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let elapsed = self.started_at.elapsed().as_secs_f32();
        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let title_str: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Run cost", "运行成本").into());

        if self.loading {
            if !reduced {
                window.request_animation_frame();
            }
            let phase = (elapsed / 1.4).fract();
            return div()
                .id(self.id.clone())
                .role(Role::Group)
                .aria_label(title_str)
                .w_full()
                .rounded(px(12.0))
                .bg(card_bg)
                .border_1()
                .border_color(p.line)
                .overflow_hidden()
                .flex()
                .flex_col()
                .child(
                    div()
                        .px(px(16.0))
                        .py(px(10.0))
                        .border_b_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(pro_skeleton_bar(180.0, 16.0, 5.0, phase, cx))
                        .child(pro_skeleton_bar(120.0, 24.0, 6.0, phase + 0.15, cx)),
                )
                .child(div().p(px(16.0)).flex().flex_col().gap(px(10.0)).children(
                    (0..5usize).map(|i| {
                        pro_skeleton_bar(
                            320.0 + (i % 3) as f32 * 60.0,
                            22.0,
                            5.0,
                            phase + i as f32 * 0.08,
                            cx,
                        )
                    }),
                ));
        }

        let summary = if self.empty {
            compute_waterfall(&[], &self.collapsed)
        } else {
            compute_waterfall(&self.steps, &self.collapsed)
        };
        let run_status = self.status_override.unwrap_or(summary.run_status);
        let is_live = run_status == CostStepStatus::Running && !summary.rows.is_empty();
        if is_live && !reduced {
            window.request_animation_frame();
        }

        let width_px = self.measured_w.get().max(280.0);
        let is_stack = width_px < 520.0;
        let hide_in_out = width_px < 720.0;

        // Segmented switch: Cost | Tokens.
        let mode_sensor = pro_selection_sensor();
        let cur_mode = self.mode;
        let switch_el = div()
            .id(pro_eid(&self.id, "mode-switch"))
            .role(Role::Group)
            .aria_label(tr(cx, "Measure by", "统计维度"))
            .relative()
            .ml_auto()
            .p(px(2.0))
            .rounded(px(7.0))
            .bg(p.chip)
            .flex()
            .items_center()
            .child(pro_pill_indicator(
                pro_eid(&self.id, "mode-pill"),
                cur_mode.key(),
                mode_sensor.clone(),
                5.0,
                card_bg,
                Some(p.line),
                true,
            ))
            .children(
                [CostMeasureMode::Cost, CostMeasureMode::Tokens]
                    .into_iter()
                    .map(|m| {
                        let active = cur_mode == m;
                        let focus_c = p.focus;
                        activate(
                            div()
                                .id(pro_eid(&self.id, format!("mode-{}", m.key())))
                                .role(Role::Button)
                                .aria_label(m.label(cx))
                                .tab_index(0)
                                .relative()
                                .h(px(24.0))
                                .px(px(10.0))
                                .rounded(px(5.0))
                                .text_size(px(12.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(if active { p.ink } else { p.text })
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .focus_visible(move |s| s.border_1().border_color(focus_c))
                                .when(active, |d| d.child(pro_mark_selected(&mode_sensor)))
                                .child(m.label(cx)),
                            cx,
                            move |this, _, cx| this.set_mode(m, cx),
                        )
                    }),
            );

        let pulse_alpha = if reduced {
            1.0
        } else {
            0.40 + 0.60 * (0.5 + 0.5 * ((elapsed / 1.4) * std::f32::consts::TAU).cos())
        };
        let (pill_bg, pill_fg, pill_lbl, pill_dot) = match run_status {
            CostStepStatus::Running => (p.accent_soft, p.blue, tr(cx, "Live", "实时"), true),
            CostStepStatus::Done => (p.green_soft, p.green_ink, tr(cx, "Done", "完成"), false),
            CostStepStatus::Failed => (p.red_soft, p.red_ink, tr(cx, "Failed", "失败"), false),
        };

        let measure_cell = self.measured_w.clone();
        let head_bar = div()
            .w_full()
            .px(px(if is_stack { 12.0 } else { 16.0 }))
            .py(px(10.0))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.0))
            .child(pro_svg("0 0 14 14", SPARK_SVG, 14.0, p.blue))
            .child(
                div()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(title_str.clone()),
            )
            .when_some((!is_stack).then(|| self.task.clone()).flatten(), |d, t| {
                d.child(
                    div()
                        .flex_1()
                        .min_w(px(80.0))
                        .text_size(px(13.0))
                        .text_color(p.muted)
                        .truncate()
                        .child(t),
                )
            })
            .child(switch_el)
            .child(
                div()
                    .id(pro_eid(&self.id, "status-pill"))
                    .role(Role::Status)
                    .h(px(20.0))
                    .px(px(7.0))
                    .rounded(px(5.0))
                    .bg(pill_bg)
                    .text_color(pill_fg)
                    .text_size(px(12.0))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .when(pill_dot, |d| {
                        d.child(
                            div()
                                .size(px(6.0))
                                .rounded_full()
                                .bg(pill_fg.opacity(pulse_alpha)),
                        )
                    })
                    .child(pill_lbl),
            )
            .when_some(is_stack.then(|| self.task.clone()).flatten(), |d, t| {
                d.child(
                    div()
                        .w_full()
                        .text_size(px(12.5))
                        .text_color(p.muted)
                        .truncate()
                        .child(t),
                )
            });

        // Summary stats strip: Total cost | Total tokens | Most expensive.
        let total_tokens = summary.total_in + summary.total_out;
        let top_entry: Option<(SharedString, SharedString, String)> = match cur_mode {
            CostMeasureMode::Cost => summary
                .top_cost_leaf
                .as_ref()
                .map(|(id, nm, c)| (id.clone(), nm.clone(), format_usd(*c))),
            CostMeasureMode::Tokens => summary
                .top_tok_leaf
                .as_ref()
                .map(|(id, nm, t)| (id.clone(), nm.clone(), format_tokens(*t))),
        };
        let top_flag_id: Option<SharedString> = top_entry.as_ref().map(|(id, _, _)| id.clone());

        let cost_stat_val: AnyElement = if summary.total_cost <= 1e-7 {
            div()
                .text_size(px(15.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink)
                .child("$0")
                .into_any_element()
        } else if is_live {
            pro_shimmer_text(
                pro_eid(&self.id, "live-cost"),
                format_usd(summary.total_cost),
                p.ink,
                p.strong,
                15.0,
                FontWeight::MEDIUM,
                true,
            )
        } else {
            pro_rolling_number(
                pro_eid(&self.id, "roll-cost"),
                summary.total_cost,
                3,
                "$",
                "",
                15.0,
                FontWeight::MEDIUM,
                p.ink,
            )
        };

        let top_stat_content: AnyElement = if let Some((tid, tname, tval)) = top_entry {
            let hover_bg = p.chip;
            let focus_c = p.focus;
            activate(
                div()
                    .id(pro_eid(&self.id, "most-expensive-btn"))
                    .role(Role::Button)
                    .aria_label(format!("{tname} ({tval})"))
                    .tab_index(0)
                    .mx(px(-6.0))
                    .px(px(6.0))
                    .py(px(1.0))
                    .rounded(px(6.0))
                    .flex()
                    .items_baseline()
                    .gap(px(8.0))
                    .min_w_0()
                    .cursor_pointer()
                    .hover(move |s| s.bg(hover_bg))
                    .focus_visible(move |s| s.border_1().border_color(focus_c))
                    .child(
                        div()
                            .text_size(px(15.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .truncate()
                            .child(tname),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(12.0))
                            .text_color(p.muted)
                            .child(tval),
                    ),
                cx,
                move |this, _, cx| this.jump_to_top_step(tid.clone(), cx),
            )
            .into_any_element()
        } else {
            div()
                .text_size(px(12.0))
                .text_color(p.muted)
                .child("–")
                .into_any_element()
        };

        let stats_strip = div()
            .w_full()
            .py(px(12.0))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .when(is_stack, |d| d.flex_wrap().gap_y(px(10.0)))
            .child(
                div()
                    .when(is_stack, |d| d.w_1_2().pl(px(16.0)).pr(px(12.0)))
                    .when(!is_stack, |d| {
                        d.flex_1().min_w_0().pl(px(22.0)).pr(px(16.0))
                    })
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "TOTAL COST", "总费用")),
                    )
                    .child(cost_stat_val),
            )
            .child(
                div()
                    .when(is_stack, |d| {
                        d.w_1_2().px(px(12.0)).border_l_1().border_color(p.line)
                    })
                    .when(!is_stack, |d| {
                        d.flex_1()
                            .min_w_0()
                            .px(px(16.0))
                            .border_l_1()
                            .border_color(p.line)
                    })
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "TOTAL TOKENS", "总 TOKEN")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap(px(8.0))
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(15.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(format_tokens(total_tokens)),
                            )
                            .when(!is_stack, |d| {
                                d.child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(format!(
                                            "{} in · {} out",
                                            format_tokens(summary.total_in),
                                            format_tokens(summary.total_out)
                                        )),
                                )
                            }),
                    ),
            )
            .child(
                div()
                    .when(is_stack, |d| d.w_full().px(px(16.0)))
                    .when(!is_stack, |d| {
                        d.w(relative(0.38))
                            .min_w_0()
                            .px(px(16.0))
                            .border_l_1()
                            .border_color(p.line)
                    })
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(match cur_mode {
                                CostMeasureMode::Cost => tr(cx, "MOST EXPENSIVE", "最高花费步骤"),
                                CostMeasureMode::Tokens => tr(cx, "MOST TOKENS", "最高消耗步骤"),
                            }),
                    )
                    .child(top_stat_content),
            );

        // Column widths matching reference CSS variables.
        let pad_l = 10.0_f32;
        let pad_r = if is_stack { 12.0 } else { 16.0 };
        let col_gap = 12.0_f32;
        let name_w = if hide_in_out { 196.0 } else { 228.0 };
        let num_w = 44.0_f32;
        let cost_w = 54.0_f32;
        let time_w = 50.0_f32;

        let axis_max = match cur_mode {
            CostMeasureMode::Cost => summary.cost_axis_max.max(0.01),
            CostMeasureMode::Tokens => summary.tok_axis_max.max(1.0),
        };
        let tick_labels: [String; 3] = match cur_mode {
            CostMeasureMode::Cost => [
                "$0".to_string(),
                format_usd(axis_max * 0.5),
                format_usd(axis_max),
            ],
            CostMeasureMode::Tokens => [
                "0".to_string(),
                format_tokens((axis_max * 0.5).round() as u64),
                format_tokens(axis_max.round() as u64),
            ],
        };

        let layer_left = if is_stack {
            pad_l
        } else {
            pad_l + name_w + col_gap
        };
        let layer_right = if is_stack {
            pad_r
        } else if hide_in_out {
            pad_r + cost_w + time_w + 2.0 * col_gap
        } else {
            pad_r + 2.0 * num_w + cost_w + time_w + 4.0 * col_gap
        };

        let is_empty_rows = summary.rows.is_empty();
        let grid_line_c = p.line_soft;
        let zero_line_c = p.line_strong;

        let tick_overlay = div()
            .absolute()
            .top_0()
            .when(!is_empty_rows && !is_stack, |d| d.bottom_0())
            .when(is_empty_rows || is_stack, |d| d.h(px(30.0)))
            .left(px(layer_left))
            .right(px(layer_right))
            .overflow_hidden()
            .child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left_0()
                    .w(px(1.0))
                    .bg(zero_line_c),
            )
            .child(
                div()
                    .absolute()
                    .top(px(7.0))
                    .left(px(6.0))
                    .text_size(px(11.5))
                    .text_color(p.muted)
                    .child(tick_labels[0].clone()),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(relative(0.5))
                    .w(px(1.0))
                    .bg(grid_line_c),
            )
            .child(
                div()
                    .absolute()
                    .top(px(7.0))
                    .left(relative(0.5))
                    .ml(px(6.0))
                    .text_size(px(11.5))
                    .text_color(p.muted)
                    .child(tick_labels[1].clone()),
            )
            .child(
                div()
                    .absolute()
                    .top(px(7.0))
                    .right(px(4.0))
                    .text_size(px(11.5))
                    .text_color(p.faint)
                    .child(tick_labels[2].clone()),
            );

        let col_header = div()
            .w_full()
            .h(px(30.0))
            .pl(px(pad_l))
            .pr(px(pad_r))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(col_gap))
            .text_size(px(10.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(p.muted)
            .when(!is_stack, |d| {
                d.child(
                    div()
                        .w(px(name_w))
                        .flex_none()
                        .pl(px(6.0))
                        .child(tr(cx, "STEP", "步骤")),
                )
                .child(div().flex_1().min_w_0())
                .when(!hide_in_out, |d| {
                    d.child(
                        div()
                            .w(px(num_w))
                            .flex_none()
                            .text_right()
                            .child(tr(cx, "IN", "输入")),
                    )
                    .child(
                        div()
                            .w(px(num_w))
                            .flex_none()
                            .text_right()
                            .child(tr(cx, "OUT", "输出")),
                    )
                })
                .child(div().w(px(cost_w)).flex_none().text_right().child(
                    if hide_in_out && cur_mode == CostMeasureMode::Tokens {
                        tr(cx, "TOKENS", "TOKEN")
                    } else {
                        tr(cx, "COST", "费用")
                    },
                ))
                .child(
                    div()
                        .w(px(time_w))
                        .flex_none()
                        .text_right()
                        .child(tr(cx, "TIME", "耗时")),
                )
            });

        let body_content: AnyElement = if is_empty_rows {
            div()
                .w_full()
                .py(px(22.0))
                .px(px(16.0))
                .child(pro_empty_state(
                    ProIcon::Coins,
                    tr(cx, "Waiting for the first step…", "等待首个步骤开始…"),
                    tr(
                        cx,
                        "LLM calls, tool invocations, and sub-agent token costs step across this waterfall as they run.",
                        "智能体运行中的模型推理、工具调用与子智能体开销将在此逐级展开。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element()
        } else {
            let sweep_phase = ((elapsed / 1.5).fract()).clamp(0.0, 1.0);
            let glow_alpha = if reduced {
                0.75
            } else {
                0.45 + 0.55 * (0.5 + 0.5 * ((elapsed / 1.1) * std::f32::consts::TAU).sin())
            };

            let mut rows_col = div().relative().w_full().flex().flex_col();
            let mut vis_index = 0usize;

            for row in &summary.rows {
                if !row.visible {
                    continue;
                }
                let my_vis_idx = vis_index;
                vis_index += 1;

                let row_id = row.id.clone();
                let is_open = self.open_id.as_ref() == Some(&row_id);
                let is_top = top_flag_id.as_ref() == Some(&row_id);

                // Resolve bar palette by kind & error status.
                let (bar_bg, bar_out, bar_edge, mark_c) = if row.status == CostStepStatus::Failed {
                    (
                        p.red.opacity(if p.dark { 0.17 } else { 0.10 }),
                        p.red.opacity(if p.dark { 0.22 } else { 0.16 }),
                        p.red.opacity(if p.dark { 0.42 } else { 0.34 }),
                        p.red.opacity(0.72),
                    )
                } else {
                    match row.kind {
                        CostStepKind::Llm => (
                            p.purple.opacity(if p.dark { 0.16 } else { 0.10 }),
                            p.purple.opacity(if p.dark { 0.24 } else { 0.20 }),
                            p.purple.opacity(if p.dark { 0.38 } else { 0.30 }),
                            p.purple.opacity(0.76),
                        ),
                        CostStepKind::Tool => (
                            p.cyan.opacity(if p.dark { 0.14 } else { 0.09 }),
                            p.cyan.opacity(if p.dark { 0.22 } else { 0.17 }),
                            p.cyan.opacity(if p.dark { 0.34 } else { 0.28 }),
                            p.cyan.opacity(0.68),
                        ),
                        CostStepKind::Agent => (
                            if p.dark {
                                hsla(0., 0., 1., 0.07)
                            } else {
                                hsla(0., 0., 0., 0.045)
                            },
                            if p.dark {
                                hsla(0., 0., 1., 0.09)
                            } else {
                                hsla(0., 0., 0., 0.065)
                            },
                            if p.dark {
                                hsla(0., 0., 1., 0.15)
                            } else {
                                hsla(0., 0., 0., 0.11)
                            },
                            p.muted,
                        ),
                    }
                };

                let (raw_start, raw_delta) = match cur_mode {
                    CostMeasureMode::Cost => (
                        (row.cost_start / axis_max) as f32,
                        (row.cost_delta / axis_max) as f32,
                    ),
                    CostMeasureMode::Tokens => (
                        (row.tok_start / axis_max) as f32,
                        (row.tok_delta / axis_max) as f32,
                    ),
                };
                let tot_tok = row.tokens_in + row.tokens_out;
                let raw_out_split = if tot_tok > 0 {
                    (row.tokens_out as f32 / tot_tok as f32).clamp(0.0, 1.0)
                } else {
                    0.0
                };

                let s_ratio = pro_spring_f32(
                    pro_eid(&self.id, format!("bs-{row_id}")),
                    raw_start.clamp(0.0, 0.99),
                    0.34,
                    window,
                    cx,
                )
                .clamp(0.0, 0.99);
                let d_ratio = pro_spring_f32(
                    pro_eid(&self.id, format!("bd-{row_id}")),
                    raw_delta.clamp(0.0, 1.0 - s_ratio),
                    0.34,
                    window,
                    cx,
                )
                .clamp(0.0, 1.0);
                let out_ratio = pro_spring_f32(
                    pro_eid(&self.id, format!("bo-{row_id}")),
                    raw_out_split,
                    0.30,
                    window,
                    cx,
                )
                .clamp(0.0, 1.0);

                let bar_h = if is_stack { 10.0 } else { 14.0 };
                let is_running = row.status == CostStepStatus::Running;
                let is_credit = row.is_credit && cur_mode == CostMeasureMode::Cost;
                let bar_border = if is_open { mark_c } else { bar_edge };

                let waterfall_track = canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let tw = f32::from(bounds.size.width);
                        let th = f32::from(bounds.size.height);
                        if tw <= 2.0 {
                            return;
                        }
                        let bx = (s_ratio * tw).clamp(0.0, (tw - 2.0).max(0.0));
                        let bw = (d_ratio * tw).max(2.5).min(tw - bx);
                        let by = (th - bar_h) * 0.5;
                        let bar_rect = Bounds::new(
                            point(bounds.left() + px(bx), bounds.top() + px(by)),
                            size(px(bw), px(bar_h)),
                        );
                        let r = px(3.5);
                        window.paint_quad(quad(
                            bar_rect,
                            Corners::all(r),
                            bar_bg,
                            Edges::all(px(1.0)),
                            bar_border,
                            BorderStyle::Solid,
                        ));

                        if is_credit {
                            // Subtle diagonal stripe cue for negative credit adjustments.
                            let mut x_str = 3.0_f32;
                            while x_str < bw - 2.0 {
                                let mut hatch = PathBuilder::stroke(px(1.2));
                                hatch.move_to(point(
                                    bar_rect.left() + px(x_str),
                                    bar_rect.bottom() - px(1.0),
                                ));
                                hatch.line_to(point(
                                    bar_rect.left() + px((x_str + 5.0).min(bw - 1.0)),
                                    bar_rect.top() + px(1.0),
                                ));
                                if let Ok(hp) = hatch.build() {
                                    window.paint_path(hp, bar_edge);
                                }
                                x_str += 6.0;
                            }
                        } else if out_ratio > 0.02 && bw > 5.0 {
                            let ow = (bw * out_ratio).clamp(1.5, bw);
                            let out_rect = Bounds::new(
                                point(bar_rect.right() - px(ow), bar_rect.top() + px(1.0)),
                                size(px(ow), px((bar_h - 2.0).max(1.0))),
                            );
                            window.paint_quad(fill(out_rect, bar_out).corner_radii(px(2.5)));
                        }

                        if is_running && !reduced {
                            let sweep_w = (bw * 0.45).clamp(12.0, 56.0);
                            let sx = -sweep_w + sweep_phase * (bw + sweep_w * 2.0);
                            let s_left = sx.max(0.0);
                            let s_right = (sx + sweep_w).min(bw);
                            if s_right > s_left + 1.0 {
                                let sheen_rect = Bounds::new(
                                    point(bar_rect.left() + px(s_left), bar_rect.top() + px(1.0)),
                                    size(px(s_right - s_left), px((bar_h - 2.0).max(1.0))),
                                );
                                window.paint_quad(fill(sheen_rect, card_bg.opacity(0.45)));
                            }
                            let tip_w = 18.0_f32.min(bw);
                            let tip_rect = Bounds::new(
                                point(bar_rect.right() - px(tip_w), bar_rect.top()),
                                size(px(tip_w), px(bar_h)),
                            );
                            window.paint_quad(
                                fill(
                                    tip_rect,
                                    linear_gradient(
                                        90.0,
                                        linear_color_stop(bar_edge.opacity(0.0), 0.0),
                                        linear_color_stop(mark_c.opacity(0.55 * glow_alpha), 1.0),
                                    ),
                                )
                                .corner_radii(r),
                            );
                        }
                    },
                )
                .w_full()
                .h(px(if is_stack { 12.0 } else { 32.0 }));

                // Indent guides + chevron + icon tile + label + badges.
                let depth = row.level.min(if is_stack { 4 } else { 6 });
                let mut guides = div().h_full().flex().items_stretch();
                for _ in 0..depth {
                    guides = guides.child(
                        div()
                            .w(px(14.0))
                            .h_full()
                            .flex()
                            .justify_center()
                            .child(div().w(px(1.0)).h_full().bg(p.line)),
                    );
                }

                let chev_el: AnyElement = if row.has_children {
                    let fold_id = row_id.clone();
                    let folded = row.folded;
                    let hover_bg = p.chip;
                    let focus_c = p.focus;
                    div()
                        .id(pro_eid(&self.id, format!("chev-{row_id}")))
                        .role(Role::Button)
                        .aria_label(if folded {
                            tr(cx, "Expand sub-agent", "展开子智能体")
                        } else {
                            tr(cx, "Collapse sub-agent", "折叠子智能体")
                        })
                        .aria_expanded(!folded)
                        .tab_index(0)
                        .w(px(16.0))
                        .h(px(20.0))
                        .mr(px(4.0))
                        .rounded(px(4.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(move |s| s.bg(hover_bg))
                        .focus_visible(move |s| s.border_1().border_color(focus_c))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.toggle_fold(fold_id.clone(), cx);
                        }))
                        .child(pro_icon(
                            if folded {
                                ProIcon::ChevronRight
                            } else {
                                ProIcon::ChevronDown
                            },
                            12.0,
                            p.muted,
                        ))
                        .into_any_element()
                } else {
                    div().w(px(16.0)).mr(px(4.0)).flex_none().into_any_element()
                };

                let icon_inner: AnyElement = if let Some(key) = &row.artwork_key
                    && let Some(art) = pro_app_artwork(key, 12.0, cx)
                {
                    art
                } else {
                    div()
                        .size(px(8.0))
                        .rounded(px(2.0))
                        .bg(mark_c)
                        .into_any_element()
                };

                let icon_tile = div()
                    .size(px(20.0))
                    .mr(px(8.0))
                    .flex_none()
                    .rounded(px(5.0))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon_inner);

                let name_cell = div()
                    .when(!is_stack, |d| d.w(px(name_w)).flex_none().h(px(32.0)))
                    .when(is_stack, |d| d.flex_1().min_w_0().h(px(28.0)))
                    .flex()
                    .items_center()
                    .min_w_0()
                    .child(guides)
                    .child(chev_el)
                    .child(icon_tile)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_baseline()
                            .gap(px(6.0))
                            .overflow_hidden()
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(if row.has_children {
                                        FontWeight::MEDIUM
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .text_color(if row.has_children { p.strong } else { p.ink })
                                    .truncate()
                                    .child(row.name.clone()),
                            )
                            .when_some(
                                (!is_stack).then(|| row.sub_label.clone()).flatten(),
                                |d, sub| {
                                    d.child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(sub),
                                    )
                                },
                            ),
                    )
                    .when(row.has_children && row.folded, |d| {
                        d.child(
                            div()
                                .ml(px(6.0))
                                .h(px(18.0))
                                .px(px(5.0))
                                .flex_none()
                                .rounded(px(4.0))
                                .bg(p.chip)
                                .text_size(px(11.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.text)
                                .flex()
                                .items_center()
                                .child(format!("+{}", row.child_count)),
                        )
                    })
                    .when(is_top, |d| {
                        d.child(
                            div()
                                .ml(px(6.0))
                                .h(px(16.0))
                                .px(px(5.0))
                                .flex_none()
                                .rounded(px(4.0))
                                .bg(p.amber_soft)
                                .text_size(px(10.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.amber_ink)
                                .flex()
                                .items_center()
                                .child(tr(cx, "TOP", "最高")),
                        )
                    })
                    .when(row.status == CostStepStatus::Failed, |d| {
                        d.child(div().ml(px(4.0)).flex_none().child(pro_icon(
                            ProIcon::AlertTriangle,
                            13.0,
                            p.red_ink,
                        )))
                    });

                let primary_metric_str =
                    if (is_stack || hide_in_out) && cur_mode == CostMeasureMode::Tokens {
                        format_tokens(row.tokens_in + row.tokens_out)
                    } else {
                        format_usd(row.cost)
                    };

                let hover_row_bg = p.hover;
                let open_row_bg = pro_mix_color(card_bg, p.ink, if p.dark { 0.04 } else { 0.03 });
                let focus_c = p.focus;
                let click_id = row_id.clone();
                let aria = format!(
                    "{}, {}, {}, {} tokens, {}",
                    row.name,
                    format_usd(row.cost),
                    format_duration_ms(row.duration_ms),
                    row.tokens_in + row.tokens_out,
                    match row.status {
                        CostStepStatus::Running => "running",
                        CostStepStatus::Done => "done",
                        CostStepStatus::Failed => "failed",
                    }
                );

                let row_button = activate(
                    div()
                        .id(pro_eid(&self.id, format!("row-{row_id}")))
                        .role(Role::ListBoxOption)
                        .aria_label(aria)
                        .aria_expanded(is_open)
                        .tab_index(if my_vis_idx == self.focused_row {
                            0
                        } else {
                            -1
                        })
                        .w_full()
                        .pl(px(pad_l))
                        .pr(px(pad_r))
                        .when(is_open, |d| d.bg(open_row_bg))
                        .hover(move |s| s.bg(hover_row_bg))
                        .focus_visible(move |s| s.border_1().border_color(focus_c))
                        .cursor_pointer()
                        .map(|d| {
                            if !is_stack {
                                d.h(px(32.0))
                                    .flex()
                                    .items_center()
                                    .gap(px(col_gap))
                                    .child(name_cell)
                                    .child(div().flex_1().min_w_0().child(waterfall_track))
                                    .when(!hide_in_out, |d| {
                                        d.child(
                                            div()
                                                .w(px(num_w))
                                                .flex_none()
                                                .text_right()
                                                .text_size(px(12.0))
                                                .text_color(p.text)
                                                .child(format_tokens(row.tokens_in)),
                                        )
                                        .child(
                                            div()
                                                .w(px(num_w))
                                                .flex_none()
                                                .text_right()
                                                .text_size(px(12.0))
                                                .text_color(p.text)
                                                .child(format_tokens(row.tokens_out)),
                                        )
                                    })
                                    .child(
                                        div()
                                            .w(px(cost_w))
                                            .flex_none()
                                            .text_right()
                                            .text_size(px(12.0))
                                            .font_weight(if row.has_children {
                                                FontWeight::MEDIUM
                                            } else {
                                                FontWeight::NORMAL
                                            })
                                            .text_color(p.ink)
                                            .child(primary_metric_str),
                                    )
                                    .child(
                                        div()
                                            .w(px(time_w))
                                            .flex_none()
                                            .text_right()
                                            .text_size(px(12.0))
                                            .text_color(if row.status == CostStepStatus::Failed {
                                                p.red_ink
                                            } else {
                                                p.text
                                            })
                                            .child(format_duration_ms(row.duration_ms)),
                                    )
                            } else {
                                d.py(px(4.0))
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .child(
                                        div()
                                            .w_full()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.0))
                                            .child(name_cell)
                                            .child(
                                                div()
                                                    .w(px(cost_w))
                                                    .flex_none()
                                                    .text_right()
                                                    .text_size(px(12.0))
                                                    .text_color(p.ink)
                                                    .child(primary_metric_str),
                                            )
                                            .child(
                                                div()
                                                    .w(px(time_w))
                                                    .flex_none()
                                                    .text_right()
                                                    .text_size(px(12.0))
                                                    .text_color(p.text)
                                                    .child(format_duration_ms(row.duration_ms)),
                                            ),
                                    )
                                    .child(waterfall_track)
                            }
                        }),
                    cx,
                    move |this, _, cx| {
                        this.focused_row = my_vis_idx;
                        this.toggle_open(click_id.clone(), cx);
                    },
                );

                // Step inspector drawer (`pro_fold`) showing Prompt/Response or Input/Output + Error.
                let (in_title, out_title) = match row.kind {
                    CostStepKind::Llm => {
                        (tr(cx, "PROMPT", "提示词"), tr(cx, "RESPONSE", "模型回复"))
                    }
                    CostStepKind::Tool => {
                        (tr(cx, "INPUT", "输入参数"), tr(cx, "OUTPUT", "返回结果"))
                    }
                    CostStepKind::Agent => (tr(cx, "TASK", "子任务"), tr(cx, "RESULT", "汇总结果")),
                };
                let in_txt = row.input.clone();
                let out_txt = row.output.clone();
                let err_txt = row.error.clone();
                let is_tool = row.kind == CostStepKind::Tool;
                let indent_px = pad_l + (depth as f32 * 14.0) + if is_stack { 8.0 } else { 44.0 };
                let detail_bg = open_row_bg;
                let box_line = p.line;
                let ink_c = p.ink;
                let muted_c = p.muted;
                let red_bg = p.red_soft;
                let red_fg = p.red_ink;
                let in_hdr: SharedString = in_title.into();
                let out_hdr: SharedString = out_title.into();

                let fold_el = pro_fold(
                    pro_eid(&self.id, format!("detail-{row_id}")),
                    is_open && (in_txt.is_some() || out_txt.is_some() || err_txt.is_some()),
                    96.0,
                    0.0,
                    move || {
                        let mut io_grid = div()
                            .w_full()
                            .flex()
                            .when(is_stack, |d| d.flex_col())
                            .gap(px(8.0));
                        if let Some(inp) = in_txt {
                            io_grid = io_grid.child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .p(px(9.0))
                                    .rounded(px(8.0))
                                    .bg(card_bg)
                                    .border_1()
                                    .border_color(box_line)
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .text_size(px(10.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(muted_c)
                                            .child(in_hdr),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(if is_tool { 11.5 } else { 12.0 }))
                                            .line_height(px(17.5))
                                            .text_color(ink_c)
                                            .child(inp),
                                    ),
                            );
                        }
                        if let Some(out) = out_txt {
                            io_grid = io_grid.child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .p(px(9.0))
                                    .rounded(px(8.0))
                                    .bg(card_bg)
                                    .border_1()
                                    .border_color(box_line)
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .text_size(px(10.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(muted_c)
                                            .child(out_hdr),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(if is_tool { 11.5 } else { 12.0 }))
                                            .line_height(px(17.5))
                                            .text_color(if is_running { muted_c } else { ink_c })
                                            .child(out),
                                    ),
                            );
                        }

                        let mut wrap = div()
                            .w_full()
                            .pl(px(indent_px))
                            .pr(px(pad_r))
                            .pt(px(4.0))
                            .pb(px(10.0))
                            .bg(detail_bg)
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(io_grid);

                        if let Some(err) = err_txt {
                            wrap = wrap.child(
                                div()
                                    .p(px(9.0))
                                    .rounded(px(8.0))
                                    .bg(red_bg)
                                    .flex()
                                    .items_start()
                                    .gap(px(8.0))
                                    .text_size(px(12.0))
                                    .text_color(red_fg)
                                    .child(pro_icon(ProIcon::AlertTriangle, 14.0, red_fg))
                                    .child(div().flex_1().min_w_0().child(err)),
                            );
                        }
                        wrap.into_any_element()
                    },
                );

                rows_col = rows_col.child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .child(row_button)
                        .child(fold_el),
                );
            }

            if let Some(mh) = self.max_height {
                div()
                    .id(pro_eid(&self.id, "rows-scroll"))
                    .w_full()
                    .max_h(px(mh))
                    .overflow_y_scroll()
                    .child(rows_col)
                    .into_any_element()
            } else {
                rows_col.into_any_element()
            }
        };

        div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(title_str)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .relative()
            .w_full()
            .min_w_0()
            .rounded(px(12.0))
            .bg(card_bg)
            .border_1()
            .border_color(p.line)
            .when(!p.dark, |d| d.shadow_xs())
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                canvas(
                    move |b, window, _| {
                        let nw = f32::from(b.size.width);
                        if nw > 40.0 && (measure_cell.get() - nw).abs() > 1.0 {
                            measure_cell.set(nw);
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .w_full()
                .h(px(0.0)),
            )
            .child(head_bar)
            .child(stats_strip)
            .child(
                div()
                    .relative()
                    .w_full()
                    .flex()
                    .flex_col()
                    .child(tick_overlay)
                    .child(col_header)
                    .child(body_content),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CostStep, CostStepKind, CostStepStatus, compute_waterfall, format_tokens, format_usd,
    };
    use gpui::SharedString;
    use std::collections::HashSet;

    #[test]
    fn sub_agent_sums_children_and_credits_step_waterfall_back() {
        let steps = vec![
            CostStep::new("plan", "Plan", CostStepKind::Llm)
                .with_tokens(2_000, 500)
                .with_cost(0.030),
            CostStep::new("sub", "Worker", CostStepKind::Agent).with_children(vec![
                CostStep::new("c1", "Search", CostStepKind::Tool)
                    .with_tokens(400, 600)
                    .with_cost(0.015),
                CostStep::new("c2", "Cache credit", CostStepKind::Tool)
                    .with_tokens(0, 0)
                    .with_cost(-0.005),
            ]),
        ];

        let summary = compute_waterfall(&steps, &HashSet::new());
        assert!((summary.total_cost - 0.040).abs() < 1e-6);
        assert_eq!(summary.total_in, 2_400);
        assert_eq!(summary.total_out, 1_100);
        assert_eq!(
            summary.top_cost_leaf.as_ref().map(|(id, _, _)| id.as_ref()),
            Some("plan")
        );
        // Sub-agent row aggregates its two children (`0.015 - 0.005 = 0.010`).
        assert!((summary.rows[1].cost - 0.010).abs() < 1e-6);
        assert!(summary.rows[3].is_credit);

        let mut folded = HashSet::new();
        folded.insert(SharedString::from("sub"));
        let collapsed_summary = compute_waterfall(&steps, &folded);
        assert!(collapsed_summary.rows[1].folded);
        assert!(!collapsed_summary.rows[2].visible);
        assert_eq!(format_usd(0.044), "$0.044");
        assert_eq!(format_tokens(28_400), "28.4k");
    }

    #[test]
    fn failed_child_propagates_failed_run_status() {
        let steps = vec![
            CostStep::new("a", "Step A", CostStepKind::Tool).with_error("HTTP 429 rate limited"),
        ];
        let summary = compute_waterfall(&steps, &HashSet::new());
        assert_eq!(summary.run_status, CostStepStatus::Failed);
    }
}
