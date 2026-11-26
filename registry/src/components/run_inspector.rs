//! Single-screen multi-agent run inspector with summary rollup, live connector tree, and inline call traces.
//!
//! Renders [`RunInspectorBlock`] with a six-metric summary strip (`Duration`, `Tokens`, `Cost`,
//! `LLM calls`, `Tool calls`, `Failures`), a live orchestrator-to-sub-agent tree joined by
//! [`pro_connector_pulse`] connectors, miniature colored call-duration bars inside each agent
//! card, and inline [`pro_fold`] call details with arguments, outputs, red error traces, and `Retry`.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_card_shadow,
        pro_check_draw, pro_connector_pulse, pro_eid, pro_empty_state, pro_fold, pro_mark_selected,
        pro_mix_color, pro_pill_indicator, pro_popover_presence, pro_rolling_number,
        pro_selection_sensor, pro_skeleton_bar, pro_spring_bar, pro_spring_f32, pro_swap_label,
        pro_tactile, pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashSet, rc::Rc};
use web_time::Instant;

const SVG_SPARK_RUN: &str = r#"<path d="M7 1.2c.4 2.9 1.9 4.4 4.8 4.8-2.9.4-4.4 1.9-4.8 4.8C6.6 7.9 5.1 6.4 2.2 6 5.1 5.6 6.6 4.1 7 1.2Z" fill="currentColor"></path><circle cx="11.6" cy="11.4" r="1.1" fill="currentColor" opacity="0.55"></circle>"#;
const SVG_SPIN_ARC: &str = r#"<circle cx="12" cy="12" r="8.5" stroke="currentColor" stroke-opacity="0.25" stroke-width="2.4"></circle><path d="M12 3.5A8.5 8.5 0 0 1 20.5 12" stroke="currentColor" stroke-linecap="round" stroke-width="2.4"></path>"#;

/// Span category inside a [`RunSpan`] hierarchy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunSpanKind {
    /// Autonomous orchestrator, supervisor, or worker sub-agent.
    #[default]
    Agent,
    /// Model reasoning or completion call (`llm`).
    Llm,
    /// External tool, query, or test runner invocation (`tool`).
    Tool,
}

/// Execution lifecycle status of a [`RunSpan`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RunSpanStatus {
    /// Queued awaiting upstream work.
    #[default]
    Queued,
    /// Actively running (`now_ms` advances its duration bar).
    Running,
    /// Paused awaiting human input or policy check.
    Waiting,
    /// Completed cleanly.
    Done,
    /// Terminated with an error trace.
    Failed,
}

impl RunSpanStatus {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Queued => tr(cx, "Queued", "排队中"),
            Self::Running => tr(cx, "Running", "运行中"),
            Self::Waiting => tr(cx, "Waiting", "待确认"),
            Self::Done => tr(cx, "Done", "已完成"),
            Self::Failed => tr(cx, "Failed", "失败"),
        }
    }
}

/// Recursive agent, LLM call, or tool call span inspected by [`RunInspectorBlock`].
#[derive(Clone, Debug, PartialEq)]
pub struct RunSpan {
    /// Stable span identifier (`"main"`, `"researcher"`, `"tests"`).
    pub id: SharedString,
    /// Display name of the agent or call.
    pub name: SharedString,
    /// Span category (`Agent`, `Llm`, or `Tool`).
    pub kind: RunSpanKind,
    /// Current lifecycle state.
    pub status: RunSpanStatus,
    /// Offset from run start in milliseconds.
    pub start_ms: u64,
    /// Completion offset in milliseconds (`None` while running).
    pub end_ms: Option<u64>,
    /// Model label (`"Opus 4.6"`, `"Sonnet 4.6"`, `"GPT-5.4"`).
    pub model: Option<SharedString>,
    /// Assigned goal shown on agent cards.
    pub task: Option<SharedString>,
    /// Tool identifier (`"web_search"`, `"sql_query"`, `"run_tests"`).
    pub tool: Option<SharedString>,
    /// Serialized arguments or prompt snippet shown when unfolded.
    pub args: Option<SharedString>,
    /// Returned summary or completion preview shown when unfolded.
    pub output: Option<SharedString>,
    /// Failure message rendered as a red error trace when `status == Failed`.
    pub error: Option<SharedString>,
    /// Prompt / input token count.
    pub tokens_in: u64,
    /// Completion / output token count.
    pub tokens_out: u64,
    /// Dollar cost of this span.
    pub cost: f64,
    /// Nested calls (`Llm` / `Tool`) and delegated sub-agents (`Agent`).
    pub children: Vec<RunSpan>,
}

impl RunSpan {
    /// Duration in milliseconds evaluated at `now_ms`.
    pub fn duration_ms(&self, now_ms: u64) -> u64 {
        let end = self.end_ms.unwrap_or_else(|| now_ms.max(self.start_ms));
        end.saturating_sub(self.start_ms)
    }

    /// Direct `Llm` and `Tool` calls belonging to this agent card.
    pub fn direct_calls(&self) -> impl Iterator<Item = &RunSpan> {
        self.children
            .iter()
            .filter(|c| matches!(c.kind, RunSpanKind::Llm | RunSpanKind::Tool))
    }

    /// Direct delegated child sub-agents (`RunSpanKind::Agent`).
    pub fn sub_agents(&self) -> impl Iterator<Item = &RunSpan> {
        self.children
            .iter()
            .filter(|c| c.kind == RunSpanKind::Agent)
    }
}

/// Aggregated run or subtree telemetry displayed in the top summary strip.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RunRollupMetrics {
    /// Elapsed wall-clock span in milliseconds.
    pub duration_ms: u64,
    /// Total input tokens.
    pub tokens_in: u64,
    /// Total output tokens.
    pub tokens_out: u64,
    /// Total USD cost.
    pub cost: f64,
    /// Count of `Llm` calls.
    pub llm_calls: usize,
    /// Count of `Tool` calls.
    pub tool_calls: usize,
    /// Count of failed calls.
    pub failures: usize,
    /// Count of agent nodes in scope.
    pub agent_count: usize,
    /// Count of currently running agent nodes.
    pub running_agents: usize,
}

impl RunRollupMetrics {
    /// Combined input + output tokens.
    pub fn total_tokens(&self) -> u64 {
        self.tokens_in + self.tokens_out
    }

    /// Combined LLM + tool calls.
    pub fn total_calls(&self) -> usize {
        self.llm_calls + self.tool_calls
    }
}

/// Events emitted by [`RunInspectorBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunInspectorEvent {
    /// User picked an agent card to scope metrics, or cleared the selection (`None`).
    SelectedChanged(Option<SharedString>),
    /// User unfolded or closed a call row (`None` when closed).
    CallChanged(Option<SharedString>),
    /// User clicked `Retry` on a failed call trace.
    RetryCall {
        /// Owning agent identifier.
        agent_id: SharedString,
        /// Retried call identifier.
        call_id: SharedString,
    },
    /// User folded or unfolded an agent's sub-agents.
    SubAgentsFolded {
        /// Parent agent identifier.
        agent_id: SharedString,
        /// Whether sub-agents are now hidden.
        folded: bool,
    },
    /// User paused or resumed the live run clock.
    LiveToggled(bool),
}

/// Compute rolled-up metrics across `root` (and optional `supervisor` when unscoped).
pub fn compute_run_rollup(
    root: &RunSpan,
    supervisor: Option<&RunSpan>,
    now_ms: u64,
) -> RunRollupMetrics {
    let mut m = RunRollupMetrics::default();
    let mut min_start = root.start_ms;
    let mut max_end = root.end_ms.unwrap_or(now_ms).max(root.start_ms);

    fn visit(
        span: &RunSpan,
        now_ms: u64,
        min_s: &mut u64,
        max_e: &mut u64,
        out: &mut RunRollupMetrics,
    ) {
        *min_s = (*min_s).min(span.start_ms);
        let e = span.end_ms.unwrap_or_else(|| now_ms.max(span.start_ms));
        *max_e = (*max_e).max(e);
        out.tokens_in += span.tokens_in;
        out.tokens_out += span.tokens_out;
        out.cost += span.cost;
        match span.kind {
            RunSpanKind::Agent => {
                out.agent_count += 1;
                if span.status == RunSpanStatus::Running {
                    out.running_agents += 1;
                }
            }
            RunSpanKind::Llm => {
                out.llm_calls += 1;
                if span.status == RunSpanStatus::Failed {
                    out.failures += 1;
                }
            }
            RunSpanKind::Tool => {
                out.tool_calls += 1;
                if span.status == RunSpanStatus::Failed {
                    out.failures += 1;
                }
            }
        }
        for ch in &span.children {
            visit(ch, now_ms, min_s, max_e, out);
        }
    }

    visit(root, now_ms, &mut min_start, &mut max_end, &mut m);
    if let Some(sup) = supervisor {
        visit(sup, now_ms, &mut min_start, &mut max_end, &mut m);
    }
    m.duration_ms = max_end.saturating_sub(min_start);
    m
}

fn find_agent_span<'a>(root: &'a RunSpan, target_id: &str) -> Option<&'a RunSpan> {
    if root.kind == RunSpanKind::Agent && root.id.as_ref() == target_id {
        return Some(root);
    }
    for ch in &root.children {
        if let Some(found) = find_agent_span(ch, target_id) {
            return Some(found);
        }
    }
    None
}

fn retry_call_in_span(
    span: &mut RunSpan,
    target_call_id: &str,
    now_ms: u64,
) -> Option<SharedString> {
    let owner_id = span.id.clone();
    let mut fixed_here = false;
    for ch in &mut span.children {
        if ch.id.as_ref() == target_call_id && ch.kind != RunSpanKind::Agent {
            ch.status = RunSpanStatus::Done;
            ch.error = None;
            ch.output =
                Some("14 of 14 checkout tests passed (WINTER20 annual price = $79/mo)".into());
            ch.end_ms = Some(now_ms.max(ch.start_ms + 1800));
            ch.cost += 0.01;
            fixed_here = true;
            break;
        }
    }
    if fixed_here {
        let any_failed = span
            .children
            .iter()
            .any(|c| c.status == RunSpanStatus::Failed);
        if !any_failed && span.status == RunSpanStatus::Failed {
            span.status = RunSpanStatus::Done;
            span.end_ms = Some(now_ms);
        }
        return Some(owner_id);
    }
    for ch in &mut span.children {
        if ch.kind == RunSpanKind::Agent
            && let Some(agent_id) = retry_call_in_span(ch, target_call_id, now_ms)
        {
            return Some(agent_id);
        }
    }
    None
}

/// Default multi-agent pricing-launch run with an orchestrator, supervisor, and three sub-agents.
pub fn default_run_span() -> (RunSpan, RunSpan) {
    let supervisor = RunSpan {
        id: "inspector".into(),
        name: "Inspector".into(),
        kind: RunSpanKind::Agent,
        status: RunSpanStatus::Running,
        start_ms: 400,
        end_ms: None,
        model: Some("Haiku 4.5".into()),
        task: Some("Guard token budget & production flags".into()),
        tool: None,
        args: None,
        output: None,
        error: None,
        tokens_in: 0,
        tokens_out: 0,
        cost: 0.0,
        children: vec![
            RunSpan {
                id: "budget-check".into(),
                name: "Check budget".into(),
                kind: RunSpanKind::Tool,
                status: RunSpanStatus::Done,
                start_ms: 800,
                end_ms: Some(3200),
                model: None,
                task: None,
                tool: Some("policy_guard".into()),
                args: Some("{\"cap_usd\": 2.00, \"spent_usd\": 0.48}".into()),
                output: Some("Within $2.00 run cap (24% used) · deploy gate armed".into()),
                error: None,
                tokens_in: 900,
                tokens_out: 180,
                cost: 0.01,
                children: Vec::new(),
            },
            RunSpan {
                id: "pii-audit".into(),
                name: "Audit Stripe env keys".into(),
                kind: RunSpanKind::Tool,
                status: RunSpanStatus::Done,
                start_ms: 9200,
                end_ms: Some(11400),
                model: None,
                task: None,
                tool: Some("secret_scan".into()),
                args: Some("{\"paths\": [\"apps/web\", \"checkout.ts\"]}".into()),
                output: Some("0 live keys exposed · sandbox sk_test_* verified".into()),
                error: None,
                tokens_in: 620,
                tokens_out: 140,
                cost: 0.01,
                children: Vec::new(),
            },
        ],
    };

    let run = RunSpan {
        id: "main".into(),
        name: "Orchestrator".into(),
        kind: RunSpanKind::Agent,
        status: RunSpanStatus::Running,
        start_ms: 0,
        end_ms: None,
        model: Some("Opus 4.6".into()),
        task: Some("Launch the new pricing page by Friday".into()),
        tool: None,
        args: None,
        output: None,
        error: None,
        tokens_in: 0,
        tokens_out: 0,
        cost: 0.0,
        children: vec![
            RunSpan {
                id: "plan".into(),
                name: "Plan the launch".into(),
                kind: RunSpanKind::Llm,
                status: RunSpanStatus::Done,
                start_ms: 200,
                end_ms: Some(7100),
                model: Some("Opus 4.6".into()),
                task: None,
                tool: None,
                args: Some("prompt: Decompose pricing launch into research, copy, and checkout tasks".into()),
                output: Some("Delegated 3 parallel workstreams to Researcher, Writer, and Coder".into()),
                error: None,
                tokens_in: 6200,
                tokens_out: 940,
                cost: 0.08,
                children: Vec::new(),
            },
            RunSpan {
                id: "verify-gate".into(),
                name: "Coordinate merge readiness".into(),
                kind: RunSpanKind::Llm,
                status: RunSpanStatus::Running,
                start_ms: 17800,
                end_ms: None,
                model: Some("Opus 4.6".into()),
                task: None,
                tool: None,
                args: Some("prompt: Await Coder checkout test fix before opening release PR".into()),
                output: Some("Holding release branch until checkout coupon assertion passes…".into()),
                error: None,
                tokens_in: 4100,
                tokens_out: 520,
                cost: 0.06,
                children: Vec::new(),
            },
            RunSpan {
                id: "researcher".into(),
                name: "Researcher".into(),
                kind: RunSpanKind::Agent,
                status: RunSpanStatus::Running,
                start_ms: 7300,
                end_ms: None,
                model: Some("Sonnet 4.6".into()),
                task: Some("Compare the pricing of five competitors".into()),
                tool: None,
                args: None,
                output: None,
                error: None,
                tokens_in: 0,
                tokens_out: 0,
                cost: 0.0,
                children: vec![
                    RunSpan {
                        id: "search-linear".into(),
                        name: "Search Linear pricing".into(),
                        kind: RunSpanKind::Tool,
                        status: RunSpanStatus::Done,
                        start_ms: 7500,
                        end_ms: Some(12900),
                        model: None,
                        task: None,
                        tool: Some("web_search".into()),
                        args: Some("{\"query\": \"linear workspace pricing annual discount\"}".into()),
                        output: Some("Captured Free, Basic ($10), Business ($16), Enterprise (-20% yearly)".into()),
                        error: None,
                        tokens_in: 3800,
                        tokens_out: 720,
                        cost: 0.04,
                        children: Vec::new(),
                    },
                    RunSpan {
                        id: "query-usage".into(),
                        name: "Query plan usage".into(),
                        kind: RunSpanKind::Tool,
                        status: RunSpanStatus::Done,
                        start_ms: 13100,
                        end_ms: Some(16600),
                        model: None,
                        task: None,
                        tool: Some("sql_query".into()),
                        args: Some("{\"sql\": \"SELECT tier, p90_events FROM workspaces GROUP BY 1\"}".into()),
                        output: Some("Starter p90 = 8.4k events · Growth p90 = 82k events".into()),
                        error: None,
                        tokens_in: 2900,
                        tokens_out: 610,
                        cost: 0.03,
                        children: Vec::new(),
                    },
                    RunSpan {
                        id: "search-notion".into(),
                        name: "Search Notion pricing".into(),
                        kind: RunSpanKind::Tool,
                        status: RunSpanStatus::Running,
                        start_ms: 16900,
                        end_ms: None,
                        model: None,
                        task: None,
                        tool: Some("web_search".into()),
                        args: Some("{\"query\": \"notion plus business seats annual billing\"}".into()),
                        output: Some("Parsing seat proration FAQ table…".into()),
                        error: None,
                        tokens_in: 3400,
                        tokens_out: 480,
                        cost: 0.04,
                        children: Vec::new(),
                    },
                ],
            },
            RunSpan {
                id: "writer".into(),
                name: "Writer".into(),
                kind: RunSpanKind::Agent,
                status: RunSpanStatus::Done,
                start_ms: 7400,
                end_ms: Some(16900),
                model: Some("Opus 4.6".into()),
                task: Some("Write the copy for the three plans".into()),
                tool: None,
                args: None,
                output: None,
                error: None,
                tokens_in: 0,
                tokens_out: 0,
                cost: 0.0,
                children: vec![
                    RunSpan {
                        id: "read-voice".into(),
                        name: "Read brand voice".into(),
                        kind: RunSpanKind::Tool,
                        status: RunSpanStatus::Done,
                        start_ms: 7600,
                        end_ms: Some(10400),
                        model: None,
                        task: None,
                        tool: Some("docs_read".into()),
                        args: Some("{\"doc\": \"brand/pricing-voice.md\"}".into()),
                        output: Some("Plain nouns, capsule -20% yearly badge, 14-day free trial".into()),
                        error: None,
                        tokens_in: 3100,
                        tokens_out: 510,
                        cost: 0.03,
                        children: Vec::new(),
                    },
                    RunSpan {
                        id: "draft-tiers".into(),
                        name: "Draft tier headlines".into(),
                        kind: RunSpanKind::Llm,
                        status: RunSpanStatus::Done,
                        start_ms: 10600,
                        end_ms: Some(16700),
                        model: Some("Opus 4.6".into()),
                        task: None,
                        tool: None,
                        args: Some("prompt: Write Starter ($24), Growth ($79 Popular), Scale ($199) copy".into()),
                        output: Some("Finalized headlines, 3 quota rows, and 6-row feature checklist".into()),
                        error: None,
                        tokens_in: 5400,
                        tokens_out: 1280,
                        cost: 0.07,
                        children: Vec::new(),
                    },
                ],
            },
            RunSpan {
                id: "coder".into(),
                name: "Coder".into(),
                kind: RunSpanKind::Agent,
                status: RunSpanStatus::Failed,
                start_ms: 7500,
                end_ms: Some(20400),
                model: Some("GPT-5.4".into()),
                task: Some("Build the page and wire up checkout".into()),
                tool: None,
                args: None,
                output: None,
                error: None,
                tokens_in: 0,
                tokens_out: 0,
                cost: 0.0,
                children: vec![
                    RunSpan {
                        id: "impl-grid".into(),
                        name: "Build PricingPlans grid".into(),
                        kind: RunSpanKind::Llm,
                        status: RunSpanStatus::Done,
                        start_ms: 7700,
                        end_ms: Some(11900),
                        model: Some("GPT-5.4".into()),
                        task: None,
                        tool: None,
                        args: Some("prompt: Implement responsive 3-card layout and yearly coupon math".into()),
                        output: Some("Wrote PricingPlans component and checkout session handler".into()),
                        error: None,
                        tokens_in: 6800,
                        tokens_out: 2950,
                        cost: 0.07,
                        children: Vec::new(),
                    },
                    RunSpan {
                        id: "tests".into(),
                        name: "Run checkout tests".into(),
                        kind: RunSpanKind::Tool,
                        status: RunSpanStatus::Failed,
                        start_ms: 12000,
                        end_ms: Some(20400),
                        model: None,
                        task: None,
                        tool: Some("run_tests".into()),
                        args: Some("command: \"pnpm test checkout\"".into()),
                        output: None,
                        error: Some(
                            "2 of 14 checkout tests fail — annual coupon WINTER20 expected $79/mo, received $99/mo"
                                .into(),
                        ),
                        tokens_in: 2200,
                        tokens_out: 510,
                        cost: 0.04,
                        children: Vec::new(),
                    },
                ],
            },
        ],
    };

    (run, supervisor)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum InspectorCallTab {
    #[default]
    All,
    Llm,
    Tool,
    Failures,
}

impl InspectorCallTab {
    const ALL: [Self; 4] = [Self::All, Self::Llm, Self::Tool, Self::Failures];

    fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Llm => "llm",
            Self::Tool => "tool",
            Self::Failures => "fail",
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::All => tr(cx, "All calls", "全部调用"),
            Self::Llm => tr(cx, "LLM", "模型"),
            Self::Tool => tr(cx, "Tools", "工具"),
            Self::Failures => tr(cx, "Failures", "失败"),
        }
    }

    fn matches(self, call: &RunSpan) -> bool {
        match self {
            Self::All => true,
            Self::Llm => call.kind == RunSpanKind::Llm,
            Self::Tool => call.kind == RunSpanKind::Tool,
            Self::Failures => call.status == RunSpanStatus::Failed,
        }
    }
}

/// Complete single-screen multi-agent run inspector with live tree connectors and foldable call bars.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use kui::components::run_inspector::RunInspectorBlock;
///
/// let inspector = cx.new(|cx| RunInspectorBlock::new("run-inspector", cx));
/// # }
/// ```
pub struct RunInspectorBlock {
    id: ElementId,
    run: RunSpan,
    supervisor: Option<RunSpan>,
    base_now_ms: u64,
    started_at_ms: u64,
    live: bool,
    selected_agent_id: Option<SharedString>,
    call_tab: InspectorCallTab,
    open_call_id: Option<SharedString>,
    folded_agents: HashSet<SharedString>,
    root_sensor: Rc<Cell<Option<Bounds<Pixels>>>>,
    sup_sensor: Rc<Cell<Option<Bounds<Pixels>>>>,
    child_sensors: Vec<Rc<Cell<Option<Bounds<Pixels>>>>>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
    boot_at: Instant,
    focus: FocusHandle,
    measured_width: Rc<Cell<f32>>,
}

impl EventEmitter<RunInspectorEvent> for RunInspectorBlock {}

impl RunInspectorBlock {
    /// Create a [`RunInspectorBlock`] populated with the five-agent pricing-launch run.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let (run, supervisor) = default_run_span();
        Self {
            id: id.into(),
            run,
            supervisor: Some(supervisor),
            base_now_ms: 24_800,
            started_at_ms: 0,
            live: true,
            selected_agent_id: None,
            call_tab: InspectorCallTab::All,
            open_call_id: Some("tests".into()),
            folded_agents: HashSet::new(),
            root_sensor: Rc::new(Cell::new(None)),
            sup_sensor: Rc::new(Cell::new(None)),
            child_sensors: (0..8).map(|_| Rc::new(Cell::new(None))).collect(),
            toast: None,
            loading: false,
            empty: false,
            boot_at: Instant::now(),
            focus: cx.focus_handle(),
            measured_width: Rc::new(Cell::new(894.0)),
        }
    }

    /// Replace the root orchestrator [`RunSpan`].
    pub fn with_run(mut self, run: RunSpan) -> Self {
        self.run = run;
        self
    }

    /// Configure or clear the side-by-side supervisor [`RunSpan`].
    pub fn with_supervisor(mut self, supervisor: Option<RunSpan>) -> Self {
        self.supervisor = supervisor;
        self
    }

    /// Set the baseline run clock in milliseconds (`24_800` by default).
    pub fn with_now_ms(mut self, now_ms: u64) -> Self {
        self.base_now_ms = now_ms;
        self
    }

    /// Set the run start epoch offset in milliseconds.
    pub fn with_started_at_ms(mut self, started_at_ms: u64) -> Self {
        self.started_at_ms = started_at_ms;
        self
    }

    /// Pre-select an agent card to scope the summary strip.
    pub fn with_selected(mut self, agent_id: Option<SharedString>) -> Self {
        self.selected_agent_id = agent_id;
        self
    }

    /// Control which call row starts unfolded (`Some("tests")` by default).
    pub fn with_open_call(mut self, call_id: Option<SharedString>) -> Self {
        self.open_call_id = call_id;
        self
    }

    /// Enable or pause live clock progression (`true` by default).
    pub fn with_live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }

    /// Configure skeleton loading placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the currently selected agent id, if any.
    pub fn selected_agent(&self) -> Option<&SharedString> {
        self.selected_agent_id.as_ref()
    }

    /// Return the currently unfolded call id, if any.
    pub fn open_call(&self) -> Option<&SharedString> {
        self.open_call_id.as_ref()
    }

    /// Return the configured run start epoch offset in milliseconds.
    pub fn started_at_ms(&self) -> u64 {
        self.started_at_ms
    }

    /// Effective run clock in milliseconds relative to `started_at_ms`.
    pub fn current_now_ms(&self, reduced_motion: bool) -> u64 {
        let raw = if self.live && !reduced_motion {
            self.base_now_ms + (self.boot_at.elapsed().as_millis() as u64).min(120_000)
        } else {
            self.base_now_ms
        };
        raw.saturating_sub(self.started_at_ms)
    }

    /// Compute summary metrics for either the selected agent or the whole run.
    pub fn metrics(&self) -> RunRollupMetrics {
        let now = self.base_now_ms;
        if let Some(sel_id) = &self.selected_agent_id {
            if let Some(target) = find_agent_span(&self.run, sel_id) {
                return compute_run_rollup(target, None, now);
            }
            if let Some(sup) = &self.supervisor
                && sup.id == *sel_id
            {
                return compute_run_rollup(sup, None, now);
            }
        }
        compute_run_rollup(&self.run, self.supervisor.as_ref(), now)
    }

    /// Pick or clear the selected agent card.
    pub fn select_agent(&mut self, agent_id: Option<SharedString>, cx: &mut Context<Self>) {
        let next = match (&self.selected_agent_id, agent_id) {
            (Some(cur), Some(req)) if cur == &req => None,
            (_, other) => other,
        };
        if self.selected_agent_id != next {
            self.selected_agent_id = next.clone();
            cx.emit(RunInspectorEvent::SelectedChanged(next));
            cx.notify();
        }
    }

    /// Unfold or collapse a call row beneath its miniature duration bar.
    pub fn toggle_call(&mut self, call_id: impl Into<SharedString>, cx: &mut Context<Self>) {
        let cid = call_id.into();
        let next = if self.open_call_id.as_ref() == Some(&cid) {
            None
        } else {
            Some(cid)
        };
        self.open_call_id = next.clone();
        cx.emit(RunInspectorEvent::CallChanged(next));
        cx.notify();
    }

    /// Retry a failed call by id, clearing its failure and updating parent status.
    pub fn retry_call(&mut self, call_id: &str, cx: &mut Context<Self>) -> bool {
        let now = self.base_now_ms + 1_800;
        let owner = retry_call_in_span(&mut self.run, call_id, now).or_else(|| {
            self.supervisor
                .as_mut()
                .and_then(|s| retry_call_in_span(s, call_id, now))
        });
        if let Some(agent_id) = owner {
            self.base_now_ms = now;
            self.toast = Some((
                tr(
                    cx,
                    "Retried call · 14/14 checkout tests passed",
                    "重试成功 · 14/14 结账测试全部通过",
                )
                .into(),
                Instant::now(),
            ));
            cx.emit(RunInspectorEvent::RetryCall {
                agent_id,
                call_id: SharedString::from(call_id.to_owned()),
            });
            cx.notify();
            true
        } else {
            false
        }
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        match ev.keystroke.key.as_str() {
            "escape" => {
                if self.open_call_id.is_some() {
                    self.open_call_id = None;
                    cx.emit(RunInspectorEvent::CallChanged(None));
                    cx.notify();
                } else if self.selected_agent_id.is_some() {
                    self.selected_agent_id = None;
                    cx.emit(RunInspectorEvent::SelectedChanged(None));
                    cx.notify();
                }
            }
            " " if ev.keystroke.modifiers.shift => {
                self.live = !self.live;
                cx.emit(RunInspectorEvent::LiveToggled(self.live));
                cx.notify();
            }
            _ => {}
        }
    }

    fn render_status_pill(id: ElementId, status: RunSpanStatus, p: &ProPalette, cx: &App) -> Div {
        let (bg, fg) = match status {
            RunSpanStatus::Running => (p.accent_soft, p.blue),
            RunSpanStatus::Waiting => (p.amber_soft, p.amber_ink),
            RunSpanStatus::Done => (p.green_soft, p.green_ink),
            RunSpanStatus::Failed => (p.red_soft, p.red_ink),
            RunSpanStatus::Queued => (p.chip, p.muted),
        };
        let lead = match status {
            RunSpanStatus::Running => pro_svg("0 0 24 24", SVG_SPIN_ARC, 10., fg),
            RunSpanStatus::Done => {
                pro_check_draw(pro_eid(&id, "st-chk"), true, 12., false, p.green)
            }
            RunSpanStatus::Failed => pro_icon(ProIcon::AlertTriangle, 10., fg),
            RunSpanStatus::Waiting => pro_icon(ProIcon::Clock, 10., fg),
            RunSpanStatus::Queued => div().size(px(5.)).rounded_full().bg(fg).into_any_element(),
        };
        div()
            .flex_none()
            .h(px(19.))
            .pl(px(5.))
            .pr(px(6.5))
            .rounded(px(5.))
            .bg(bg)
            .flex()
            .items_center()
            .gap(px(4.))
            .child(lead)
            .child(pro_swap_label(
                pro_eid(&id, "st-lbl"),
                status.label(cx),
                11.,
                FontWeight::MEDIUM,
                fg,
            ))
    }

    fn call_icon(call: &RunSpan, color: Hsla, cx: &App) -> AnyElement {
        if call.kind == RunSpanKind::Llm {
            return pro_icon(ProIcon::Sparkles, 11.5, color);
        }
        let lower_name = call.name.to_ascii_lowercase();
        let tool_slug = call.tool.as_deref().unwrap_or("");
        if lower_name.contains("notion") || tool_slug == "docs_read" {
            return pro_app_tile("Notion", 14., cx);
        }
        if lower_name.contains("stripe") {
            return pro_app_tile("Stripe", 14., cx);
        }
        if lower_name.contains("github") {
            return pro_app_tile("GitHub", 14., cx);
        }
        if lower_name.contains("figma") {
            return pro_app_tile("Figma", 14., cx);
        }
        if lower_name.contains("slack") {
            return pro_app_tile("Slack", 14., cx);
        }
        let icon = match tool_slug {
            "web_search" => ProIcon::Search,
            "sql_query" => ProIcon::Database,
            "policy_guard" | "secret_scan" => ProIcon::Shield,
            "run_tests" => ProIcon::Terminal,
            _ => ProIcon::Bolt,
        };
        pro_icon(icon, 11.5, color)
    }

    #[allow(clippy::too_many_arguments)]
    fn render_call_row(
        &self,
        agent_start_ms: u64,
        agent_window_ms: u64,
        call: &RunSpan,
        now_ms: u64,
        sheen_phase: f32,
        reduced: bool,
        p: &ProPalette,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let is_open = self.open_call_id.as_ref() == Some(&call.id);
        let tab_match = self.call_tab.matches(call);
        let row_op = pro_spring_f32(
            pro_eid(&self.id, format!("ri-cop-{}", call.id)),
            if tab_match || is_open { 1.0 } else { 0.34 },
            0.20,
            w,
            cx,
        );
        let dur_ms = call.duration_ms(now_ms);
        let win = agent_window_ms.max(1000) as f32;
        let rel_left =
            ((call.start_ms.saturating_sub(agent_start_ms)) as f32 / win).clamp(0.0, 0.88);
        let raw_w = (dur_ms as f32 / win).clamp(0.12, 1.0 - rel_left);
        let anim_w = pro_spring_f32(
            pro_eid(&self.id, format!("ri-barw-{}", call.id)),
            raw_w,
            0.28,
            w,
            cx,
        )
        .clamp(0.10, 1.0 - rel_left);

        let (bar_fill, bar_edge, icon_col) = if call.status == RunSpanStatus::Failed {
            (
                p.red.opacity(if p.dark { 0.22 } else { 0.16 }),
                p.red.opacity(0.52),
                p.red_ink,
            )
        } else if call.kind == RunSpanKind::Llm {
            (
                p.purple.opacity(if p.dark { 0.24 } else { 0.16 }),
                p.purple.opacity(0.48),
                p.purple,
            )
        } else {
            (
                p.cyan.opacity(if p.dark { 0.22 } else { 0.15 }),
                p.cyan.opacity(0.48),
                p.cyan,
            )
        };

        let is_running = call.status == RunSpanStatus::Running;
        let is_done = call.status == RunSpanStatus::Done;
        let track_el = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 2.0 || bh <= 2.0 {
                    return;
                }
                let x = rel_left * bw;
                let w_px = (anim_w * bw).max(5.0).min(bw - x);
                let bar_bounds = Bounds::new(
                    point(bounds.left() + px(x), bounds.top()),
                    size(px(w_px), px(bh)),
                );
                window.paint_quad(quad(
                    bar_bounds,
                    Corners::all(px(3.0)),
                    bar_fill,
                    Edges::all(px(1.0)),
                    bar_edge,
                    BorderStyle::Solid,
                ));
                if is_running && !reduced && w_px > 8.0 {
                    let sx = x + (w_px - 6.0) * sheen_phase;
                    let glint = Bounds::new(
                        point(bounds.left() + px(sx), bounds.top() + px(1.0)),
                        size(px(6.0), px((bh - 2.0).max(1.0))),
                    );
                    window.paint_quad(fill(glint, white().opacity(0.42)).corner_radii(px(2.0)));
                }
            },
        )
        .w(px(52.))
        .h(px(8.))
        .rounded(px(3.))
        .bg(p.chip)
        .flex_none();

        let took_txt = if call.status == RunSpanStatus::Running {
            format!("{:.1}s…", dur_ms as f32 / 1000.0)
        } else {
            format_duration_compact(dur_ms)
        };

        let cid_click = call.id.clone();
        let row_content = div()
            .h(px(24.))
            .px(px(6.))
            .flex()
            .items_center()
            .gap(px(5.))
            .text_size(px(12.))
            .child(Self::call_icon(call, icon_col, cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_color(if is_open { p.strong } else { p.ink })
                    .child(call.name.clone()),
            )
            .child(track_el)
            .child(
                div()
                    .w(px(38.))
                    .flex_none()
                    .text_right()
                    .text_size(px(11.))
                    .text_color(if call.status == RunSpanStatus::Failed {
                        p.red_ink
                    } else if is_running {
                        p.blue
                    } else {
                        p.muted
                    })
                    .child(took_txt.clone()),
            );

        let row_btn = pro_tactile(
            pro_eid(&self.id, format!("ri-call-{}", call.id)),
            format!("{}, {took_txt}", call.name),
            6.,
            if is_open {
                p.accent_soft
            } else {
                transparent_black()
            },
            if is_open {
                pro_mix_color(p.accent_soft, p.blue, 0.12)
            } else {
                p.hover
            },
            transparent_black(),
            if is_open {
                p.blue.opacity(0.28)
            } else {
                transparent_black()
            },
            is_open,
            row_content,
            cx,
            move |this, _, cx| {
                this.toggle_call(cid_click.clone(), cx);
            },
        )
        .w_full();

        let head_meta: SharedString = if let Some(tool) = &call.tool {
            format!(
                "{tool} · {} in → {} out",
                format_tokens_short(call.tokens_in),
                format_tokens_short(call.tokens_out)
            )
            .into()
        } else {
            format!(
                "{} · {} in → {} out",
                call.model.clone().unwrap_or_else(|| "model".into()),
                format_tokens_short(call.tokens_in),
                format_tokens_short(call.tokens_out)
            )
            .into()
        };
        let state_txt: SharedString = format!("{} · {took_txt}", call.status.label(cx)).into();
        let args_opt = call.args.clone();
        let out_opt = call.output.clone();
        let err_opt = call.error.clone();
        let is_failed = call.status == RunSpanStatus::Failed;
        let cid_retry = call.id.to_string();
        let cid_fold_chk = pro_eid(&self.id, format!("ri-fchk-{}", call.id));
        let cid_fold_lbl = pro_eid(&self.id, format!("ri-flbl-{}", call.id));
        let p_copy = *p;

        let retry_btn = is_failed.then(|| {
            pro_btn(
                pro_eid(&self.id, format!("ri-retry-{}", call.id)),
                tr(cx, "Retry", "重试"),
                Some(ProIcon::Refresh),
                ProBtnStyle::Secondary,
                true,
                cx,
                move |this, _, cx| {
                    this.retry_call(&cid_retry, cx);
                },
            )
            .into_any_element()
        });

        let detail_fold = pro_fold(
            pro_eid(&self.id, format!("ri-cfold-{}", call.id)),
            is_open,
            96.,
            0.,
            move || {
                div()
                    .mt(px(3.))
                    .mb(px(5.))
                    .px(px(9.))
                    .py(px(8.))
                    .rounded(px(7.))
                    .bg(if is_failed {
                        p_copy.red_soft
                    } else {
                        p_copy.chip
                    })
                    .border_1()
                    .border_color(if is_failed {
                        p_copy.red.opacity(0.32)
                    } else {
                        p_copy.line
                    })
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .text_size(px(11.5))
                    .line_height(px(16.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(6.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(10.5))
                                    .text_color(p_copy.muted)
                                    .child(head_meta),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .when(is_done, |d| {
                                        d.child(pro_check_draw(
                                            cid_fold_chk,
                                            true,
                                            12.,
                                            false,
                                            p_copy.green,
                                        ))
                                    })
                                    .child(pro_swap_label(
                                        cid_fold_lbl,
                                        state_txt,
                                        10.5,
                                        FontWeight::MEDIUM,
                                        if is_failed {
                                            p_copy.red_ink
                                        } else if is_done {
                                            p_copy.green_ink
                                        } else {
                                            p_copy.muted
                                        },
                                    )),
                            ),
                    )
                    .when_some(args_opt, |d, args| {
                        d.child(
                            div()
                                .truncate()
                                .text_size(px(11.))
                                .text_color(p_copy.text)
                                .child(args),
                        )
                    })
                    .when_some(err_opt, |d, err| {
                        d.child(
                            div()
                                .text_color(p_copy.red_ink)
                                .font_weight(FontWeight::MEDIUM)
                                .child(err),
                        )
                    })
                    .when_some(out_opt, |d, out| {
                        d.child(div().text_color(p_copy.ink).child(out))
                    })
                    .when_some(retry_btn, |d, btn| {
                        d.child(div().pt(px(3.)).flex().justify_end().child(btn))
                    })
                    .into_any_element()
            },
        );

        div()
            .w_full()
            .opacity(row_op)
            .flex()
            .flex_col()
            .child(row_btn)
            .child(detail_fold)
    }

    #[allow(clippy::too_many_arguments)]
    fn render_agent_card(
        &self,
        agent: &RunSpan,
        sensor: Option<Rc<Cell<Option<Bounds<Pixels>>>>>,
        card_width: Option<f32>,
        now_ms: u64,
        elapsed_ms: f32,
        sheen_phase: f32,
        reduced: bool,
        p: &ProPalette,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let is_selected = self.selected_agent_id.as_ref() == Some(&agent.id);
        let dimmed = self.selected_agent_id.is_some() && !is_selected;
        let card_op = pro_spring_f32(
            pro_eid(&self.id, format!("ri-adim-{}", agent.id)),
            if dimmed { 0.46 } else { 1.0 },
            0.22,
            w,
            cx,
        );
        let sub_count = agent.sub_agents().count();
        let is_folded = self.folded_agents.contains(&agent.id);

        let agent_rollup = compute_run_rollup(agent, None, now_ms);
        let calls: Vec<&RunSpan> = agent.direct_calls().collect();
        let agent_window_ms = calls
            .iter()
            .map(|c| c.end_ms.unwrap_or(now_ms).saturating_sub(agent.start_ms))
            .max()
            .unwrap_or(8_000)
            .max(2_000);

        let avatar_tint = match agent.id.as_ref() {
            "main" => p.blue,
            "inspector" => p.teal,
            "researcher" => p.cyan,
            "writer" => p.purple,
            "coder" => {
                if agent.status == RunSpanStatus::Failed {
                    p.red
                } else {
                    p.green
                }
            }
            _ => p.blue,
        };

        let mut call_rows = Vec::with_capacity(calls.len());
        for c in calls {
            call_rows.push(self.render_call_row(
                agent.start_ms,
                agent_window_ms,
                c,
                now_ms,
                sheen_phase,
                reduced,
                p,
                w,
                cx,
            ));
        }

        let aid_select = agent.id.clone();
        let aid_fold = agent.id.clone();

        let fold_btn = (sub_count > 0).then(|| {
            pro_tactile(
                pro_eid(&self.id, format!("ri-fold-sub-{}", agent.id)),
                tr(cx, "Fold sub-agents", "折叠或展开子智能体"),
                5.,
                p.chip,
                p.chip_hover,
                transparent_black(),
                p.line_strong,
                !is_folded,
                div()
                    .h(px(20.))
                    .px(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(3.))
                    .whitespace_nowrap()
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.text)
                    .child(format!("{sub_count}"))
                    .child(pro_icon(
                        if is_folded {
                            ProIcon::ChevronRight
                        } else {
                            ProIcon::ChevronDown
                        },
                        11.,
                        p.muted,
                    )),
                cx,
                move |this, _, cx| {
                    let folded = if this.folded_agents.contains(&aid_fold) {
                        this.folded_agents.remove(&aid_fold);
                        false
                    } else {
                        this.folded_agents.insert(aid_fold.clone());
                        true
                    };
                    cx.emit(RunInspectorEvent::SubAgentsFolded {
                        agent_id: aid_fold.clone(),
                        folded,
                    });
                    cx.notify();
                },
            )
            .flex_none()
        });

        let border_col = if is_selected {
            p.blue
        } else if agent.status == RunSpanStatus::Failed {
            p.red.opacity(0.42)
        } else {
            p.line
        };

        let avatar_seed = match &agent.model {
            Some(m) => format!("{} {m}", agent.name),
            None => agent.name.to_string(),
        };

        let head_content = div()
            .p(px(2.))
            .flex()
            .flex_col()
            .gap(px(5.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(pro_agent_avatar(
                        &avatar_seed,
                        avatar_tint,
                        24.,
                        agent.status == RunSpanStatus::Running,
                        elapsed_ms,
                        cx,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(13.))
                                    .line_height(px(16.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(agent.name.clone()),
                            )
                            .when_some(agent.model.clone(), |d, m| {
                                d.child(
                                    div()
                                        .truncate()
                                        .text_size(px(11.))
                                        .line_height(px(14.))
                                        .text_color(p.muted)
                                        .child(m),
                                )
                            }),
                    )
                    .child(Self::render_status_pill(
                        pro_eid(&self.id, format!("ri-ast-{}", agent.id)),
                        agent.status,
                        p,
                        cx,
                    )),
            )
            .when_some(agent.task.clone(), |d, t| {
                d.child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(p.text)
                        .child(t),
                )
            });

        div()
            .id(pro_eid(&self.id, format!("ri-agent-{}", agent.id)))
            .relative()
            .when_some(card_width, |d, cw| d.w(px(cw)).flex_none())
            .when(card_width.is_none(), |d| d.w_full())
            .p(px(11.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(border_col)
            .shadow(pro_card_shadow(p.dark))
            .opacity(card_op)
            .flex()
            .flex_col()
            .gap(px(6.))
            .when_some(sensor, |d, cell| {
                d.child(
                    canvas(
                        move |b, _, _| {
                            cell.set(Some(b));
                        },
                        |_, _, _, _| (),
                    )
                    .absolute()
                    .inset_0(),
                )
            })
            .child(
                pro_tactile(
                    pro_eid(&self.id, format!("ri-ahead-{}", agent.id)),
                    format!("{}: {}", agent.name, agent.status.label(cx)),
                    6.,
                    transparent_black(),
                    p.hover,
                    transparent_black(),
                    transparent_black(),
                    is_selected,
                    head_content,
                    cx,
                    move |this, _, cx| {
                        this.select_agent(Some(aid_select.clone()), cx);
                    },
                )
                .w_full(),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(6.))
                    .text_size(px(11.))
                    .text_color(p.muted)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, format!("ri-acost-{}", agent.id)),
                                agent_rollup.cost,
                                2,
                                "$",
                                "",
                                11.,
                                FontWeight::MEDIUM,
                                p.muted,
                            ))
                            .child(format!(
                                "· {} tok · {}",
                                format_tokens_short(agent_rollup.total_tokens()),
                                format_duration_compact(agent.duration_ms(now_ms))
                            )),
                    )
                    .when_some(fold_btn, |d, fb| d.child(fb)),
            )
            .when(!call_rows.is_empty(), |d| {
                d.child(
                    div()
                        .mt(px(2.))
                        .pt(px(6.))
                        .border_t_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .children(call_rows),
                )
            })
    }

    fn render_summary_strip(
        &self,
        rollup: &RunRollupMetrics,
        narrow: bool,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let thumb_bg: Hsla = if p.dark {
            rgb(0x2a2a2e).into()
        } else {
            rgb(0xffffff).into()
        };

        // Agent scope filter strip with pro_pill_indicator
        let mut agent_choices: Vec<(Option<SharedString>, SharedString)> =
            vec![(None, tr(cx, "Whole run", "全量运行").into())];
        agent_choices.push((Some(self.run.id.clone()), self.run.name.clone()));
        if let Some(sup) = &self.supervisor {
            agent_choices.push((Some(sup.id.clone()), sup.name.clone()));
        }
        for sub in self.run.sub_agents() {
            agent_choices.push((Some(sub.id.clone()), sub.name.clone()));
        }

        let scope_sensor = pro_selection_sensor();
        let active_scope_key: SharedString = self
            .selected_agent_id
            .clone()
            .unwrap_or_else(|| "whole".into());
        let mut scope_strip = div()
            .relative()
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.chip)
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "ri-scope-pill"),
                active_scope_key,
                scope_sensor.clone(),
                6.,
                thumb_bg,
                Some(p.line),
                true,
            ));

        for (opt_id, label) in agent_choices {
            let sel = self.selected_agent_id == opt_id;
            let id_slug = opt_id
                .as_ref()
                .map(|s| format!("ri-scope-{s}"))
                .unwrap_or_else(|| "ri-clear-scope".to_string());
            let target_opt = opt_id.clone();
            scope_strip = scope_strip.child(
                pro_tactile(
                    pro_eid(&self.id, id_slug),
                    label.clone(),
                    6.,
                    transparent_black(),
                    if sel { transparent_black() } else { p.hover },
                    transparent_black(),
                    transparent_black(),
                    false,
                    div()
                        .h(px(22.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .whitespace_nowrap()
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if sel { p.strong } else { p.muted })
                        .child(label),
                    cx,
                    move |this, _, cx| {
                        this.select_agent(target_opt.clone(), cx);
                    },
                )
                .relative()
                .flex_none()
                .when(sel, |d| d.child(pro_mark_selected(&scope_sensor))),
            );
        }

        // Call inspector filter tabs with pro_pill_indicator
        let tab_sensor = pro_selection_sensor();
        let mut tab_strip = div()
            .relative()
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.chip)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "ri-tab-pill"),
                self.call_tab.slug(),
                tab_sensor.clone(),
                6.,
                thumb_bg,
                Some(p.line),
                true,
            ));

        for tab in InspectorCallTab::ALL {
            let sel = self.call_tab == tab;
            let lbl = tab.label(cx);
            tab_strip = tab_strip.child(
                pro_tactile(
                    pro_eid(&self.id, format!("ri-tab-{}", tab.slug())),
                    lbl,
                    6.,
                    transparent_black(),
                    if sel { transparent_black() } else { p.hover },
                    transparent_black(),
                    transparent_black(),
                    false,
                    div()
                        .h(px(22.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .whitespace_nowrap()
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if sel { p.strong } else { p.muted })
                        .child(lbl),
                    cx,
                    move |this, _, cx| {
                        if this.call_tab != tab {
                            this.call_tab = tab;
                            cx.notify();
                        }
                    },
                )
                .relative()
                .flex_none()
                .when(sel, |d| d.child(pro_mark_selected(&tab_sensor))),
            );
        }

        let stat_cell = |id_sfx: &str, label: &'static str, val_el: AnyElement| {
            div()
                .id(pro_eid(&self.id, format!("ri-stat-{id_sfx}")))
                .flex_1()
                .min_w(px(if narrow { 92. } else { 88. }))
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(11.))
                        .line_height(px(15.))
                        .text_color(p.muted)
                        .child(label),
                )
                .child(val_el)
        };

        let dur_sec = rollup.duration_ms as f64 / 1000.0;
        let tok_k = rollup.total_tokens() as f64 / 1000.0;
        let fail_col = if rollup.failures > 0 {
            p.red_ink
        } else {
            p.ink
        };

        let cells = vec![
            stat_cell(
                "dur",
                tr(cx, "Duration", "总耗时"),
                pro_rolling_number(
                    pro_eid(&self.id, "ri-roll-dur"),
                    dur_sec,
                    1,
                    "",
                    "s",
                    13.5,
                    FontWeight::MEDIUM,
                    p.ink,
                ),
            ),
            stat_cell(
                "tok",
                tr(cx, "Tokens", "Token 用量"),
                pro_rolling_number(
                    pro_eid(&self.id, "ri-roll-tok"),
                    tok_k,
                    1,
                    "",
                    "k",
                    13.5,
                    FontWeight::MEDIUM,
                    p.ink,
                ),
            ),
            stat_cell(
                "cost",
                tr(cx, "Cost", "总成本"),
                pro_rolling_number(
                    pro_eid(&self.id, "ri-roll-cost"),
                    rollup.cost,
                    2,
                    "$",
                    "",
                    13.5,
                    FontWeight::MEDIUM,
                    p.ink,
                ),
            ),
            stat_cell(
                "llm",
                tr(cx, "LLM calls", "模型调用"),
                pro_rolling_number(
                    pro_eid(&self.id, "ri-roll-llm"),
                    rollup.llm_calls as f64,
                    0,
                    "",
                    "",
                    13.5,
                    FontWeight::MEDIUM,
                    p.ink,
                ),
            ),
            stat_cell(
                "tool",
                tr(cx, "Tool calls", "工具调用"),
                pro_rolling_number(
                    pro_eid(&self.id, "ri-roll-tool"),
                    rollup.tool_calls as f64,
                    0,
                    "",
                    "",
                    13.5,
                    FontWeight::MEDIUM,
                    p.ink,
                ),
            ),
            stat_cell(
                "fail",
                tr(cx, "Failures", "失败数"),
                pro_rolling_number(
                    pro_eid(&self.id, "ri-roll-fail"),
                    rollup.failures as f64,
                    0,
                    "",
                    "",
                    13.5,
                    FontWeight::MEDIUM,
                    fail_col,
                ),
            ),
        ];

        let budget_ratio = (rollup.cost as f32 / 2.00).clamp(0.0, 1.0);

        div()
            .w_full()
            .max_w(px(740.))
            .mx_auto()
            .px(px(if narrow { 14. } else { 20. }))
            .py(px(12.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .shadow(pro_card_shadow(p.dark))
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .pb(px(8.))
                    .border_b_1()
                    .border_color(p.line_soft)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(scope_strip)
                    .child(tab_strip),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_wrap()
                    .gap(px(12.))
                    .children(cells),
            )
            .child(
                div()
                    .w_full()
                    .pt(px(2.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().flex_1().min_w_0().child(pro_spring_bar(
                        pro_eid(&self.id, "ri-budget-bar"),
                        budget_ratio,
                        4.,
                        2.,
                        p.chip,
                        if rollup.failures > 0 { p.amber } else { p.blue },
                    )))
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(10.5))
                            .text_color(p.faint)
                            .child(format!(
                                "{:.0}% {}",
                                budget_ratio * 100.0,
                                tr(cx, "of $2.00 cap", "$2.00 预算上限")
                            )),
                    ),
            )
    }
}

impl Render for RunInspectorBlock {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let elapsed_s = self.boot_at.elapsed().as_secs_f32();
        let elapsed_ms = elapsed_s * 1000.0;
        let now_ms = self.current_now_ms(reduced);
        let sheen_phase = if reduced {
            0.5
        } else {
            (elapsed_s / 1.35).fract()
        };
        let pulse_phase = if reduced {
            None
        } else {
            Some((elapsed_s / 1.80).fract())
        };

        let rollup = if let Some(sel_id) = &self.selected_agent_id {
            if let Some(target) = find_agent_span(&self.run, sel_id) {
                compute_run_rollup(target, None, now_ms)
            } else if let Some(sup) = &self.supervisor
                && &sup.id == sel_id
            {
                compute_run_rollup(sup, None, now_ms)
            } else {
                compute_run_rollup(&self.run, self.supervisor.as_ref(), now_ms)
            }
        } else {
            compute_run_rollup(&self.run, self.supervisor.as_ref(), now_ms)
        };
        let whole_rollup = compute_run_rollup(&self.run, self.supervisor.as_ref(), now_ms);

        if ((self.live && whole_rollup.running_agents > 0) || self.loading) && !reduced {
            w.request_animation_frame();
        }
        if self
            .toast
            .as_ref()
            .is_some_and(|(_, t)| t.elapsed().as_secs_f32() >= 2.45)
        {
            self.toast = None;
        }

        let width = self.measured_width.get();
        let narrow = width < 760.0;
        let measure_cell = self.measured_width.clone();

        // Reset bounds sensors before prepaint
        self.root_sensor.set(None);
        self.sup_sensor.set(None);
        for cs in &self.child_sensors {
            cs.set(None);
        }

        let body = if self.loading {
            let ph = (elapsed_s / 1.4).fract();
            div()
                .w_full()
                .max_w(px(740.))
                .mx_auto()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(16.))
                .child(pro_skeleton_bar(360., 16., 5., ph, cx))
                .child(pro_skeleton_bar(620., 56., 10., ph + 0.1, cx))
                .child(pro_skeleton_bar(268., 128., 10., ph + 0.2, cx))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .when(narrow, |d| d.flex_col())
                        .gap(px(10.))
                        .child(pro_skeleton_bar(240., 148., 10., ph + 0.25, cx))
                        .child(pro_skeleton_bar(240., 148., 10., ph + 0.3, cx))
                        .child(pro_skeleton_bar(240., 148., 10., ph + 0.35, cx)),
                )
                .into_any_element()
        } else if self.empty {
            pro_empty_state(
                ProIcon::Activity,
                tr(cx, "No active agent run", "暂无运行中的多智能体轨迹"),
                tr(
                    cx,
                    "Pass an orchestrator RunSpan to inspect delegated sub-agents, call bars, and failure traces.",
                    "传入编排器 RunSpan 即可检视子智能体调用树、耗时条与错误堆栈。",
                ),
                Some((
                    pro_eid(&self.id, "ri-restore"),
                    tr(cx, "Load sample run", "载入示例运行").into(),
                    Box::new(|this: &mut Self, _, cx| {
                        this.empty = false;
                        cx.notify();
                    }),
                )),
                cx,
            )
            .into_any_element()
        } else {
            let run_title = self
                .run
                .task
                .clone()
                .unwrap_or_else(|| self.run.name.clone());

            let fail_badge_txt: SharedString = format!(
                "{} {}",
                whole_rollup.failures.max(1),
                tr(cx, "failed", "失败")
            )
            .into();
            let fail_badge = pro_popover_presence(
                pro_eid(&self.id, "ri-hdr-fail-pres"),
                whole_rollup.failures > 0,
                move || {
                    div()
                        .px(px(6.))
                        .py(px(1.))
                        .rounded(px(4.))
                        .bg(p.red_soft)
                        .text_color(p.red_ink)
                        .font_weight(FontWeight::MEDIUM)
                        .child(fail_badge_txt)
                        .into_any_element()
                },
            );

            let live_toggle = pro_tactile(
                pro_eid(&self.id, "ri-live-toggle"),
                tr(cx, "Toggle live clock", "切换实时计时"),
                5.,
                p.chip,
                p.chip_hover,
                transparent_black(),
                p.line_strong,
                self.live,
                div()
                    .h(px(22.))
                    .px(px(7.))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .whitespace_nowrap()
                    .child(pro_icon(
                        if self.live {
                            ProIcon::Pause
                        } else {
                            ProIcon::Play
                        },
                        10.,
                        if self.live { p.blue } else { p.muted },
                    ))
                    .child(pro_swap_label(
                        pro_eid(&self.id, "ri-live-lbl"),
                        if self.live {
                            tr(cx, "Live", "实时")
                        } else {
                            tr(cx, "Paused", "已暂停")
                        },
                        11.,
                        FontWeight::MEDIUM,
                        if self.live { p.blue } else { p.muted },
                    )),
                cx,
                |this, _, cx| {
                    this.live = !this.live;
                    cx.emit(RunInspectorEvent::LiveToggled(this.live));
                    cx.notify();
                },
            )
            .flex_none();

            let header_bar = div()
                .w_full()
                .max_w(px(740.))
                .mx_auto()
                .flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(180.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_svg("0 0 14 14", SVG_SPARK_RUN, 12.5, p.blue))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(13.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(run_title),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(px(10.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(format!(
                            "{} {}",
                            whole_rollup.agent_count,
                            tr(cx, "agents", "智能体")
                        ))
                        .when(whole_rollup.running_agents > 0, |d| {
                            d.child(div().text_color(p.blue).child(format!(
                                "{} {}",
                                whole_rollup.running_agents,
                                tr(cx, "running", "运行中")
                            )))
                        })
                        .child(fail_badge)
                        .child(pro_rolling_number(
                            pro_eid(&self.id, "ri-hdr-cost"),
                            whole_rollup.cost,
                            2,
                            "$",
                            "",
                            12.,
                            FontWeight::MEDIUM,
                            p.ink,
                        ))
                        .child(live_toggle),
                );

            let summary_card = self.render_summary_strip(&rollup, narrow, &p, cx);

            let run_clone = self.run.clone();
            let sup_clone = self.supervisor.clone();
            let sub_agents: Vec<RunSpan> = run_clone.sub_agents().cloned().collect();
            while self.child_sensors.len() < sub_agents.len() {
                self.child_sensors.push(Rc::new(Cell::new(None)));
            }
            let root_folded = self.folded_agents.contains(&run_clone.id);

            let tree_el = if narrow {
                // Indented mobile stack with crisp vertical rail + pro_fold on sub-agents
                let root_card = self.render_agent_card(
                    &run_clone,
                    None,
                    None,
                    now_ms,
                    elapsed_ms,
                    sheen_phase,
                    reduced,
                    &p,
                    w,
                    cx,
                );
                let sup_el = sup_clone.as_ref().map(|sup| {
                    div()
                        .pl(px(18.))
                        .border_l_2()
                        .border_color(p.line_strong)
                        .child(self.render_agent_card(
                            sup,
                            None,
                            None,
                            now_ms,
                            elapsed_ms,
                            sheen_phase,
                            reduced,
                            &p,
                            w,
                            cx,
                        ))
                });
                let mut child_els = Vec::new();
                for sub in &sub_agents {
                    child_els.push(self.render_agent_card(
                        sub,
                        None,
                        None,
                        now_ms,
                        elapsed_ms,
                        sheen_phase,
                        reduced,
                        &p,
                        w,
                        cx,
                    ));
                }
                let sub_fold = (!sub_agents.is_empty()).then(|| {
                    pro_fold(
                        pro_eid(&self.id, "ri-sub-narrow-fold"),
                        !root_folded,
                        360.,
                        10.,
                        move || {
                            div()
                                .pl(px(18.))
                                .border_l_2()
                                .border_color(p.blue.opacity(0.35))
                                .flex()
                                .flex_col()
                                .gap(px(10.))
                                .children(child_els)
                                .into_any_element()
                        },
                    )
                });
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(root_card)
                    .when_some(sup_el, |d, s| d.child(s))
                    .when_some(sub_fold, |d, sf| d.child(sf))
                    .into_any_element()
            } else {
                // Wide hierarchical tree with live pro_connector_pulse links
                let sub_w =
                    ((width - 64.0) / (sub_agents.len().max(1) as f32) - 10.0).clamp(242.0, 276.0);
                let root_w = if sup_clone.is_some() { 268.0 } else { 284.0 };

                let root_card = self.render_agent_card(
                    &run_clone,
                    Some(self.root_sensor.clone()),
                    Some(root_w),
                    now_ms,
                    elapsed_ms,
                    sheen_phase,
                    reduced,
                    &p,
                    w,
                    cx,
                );
                let sup_card = sup_clone.as_ref().map(|sup| {
                    self.render_agent_card(
                        sup,
                        Some(self.sup_sensor.clone()),
                        Some(256.0),
                        now_ms,
                        elapsed_ms,
                        sheen_phase,
                        reduced,
                        &p,
                        w,
                        cx,
                    )
                });

                let mut sub_cards = Vec::new();
                let mut active_child_flags = Vec::new();
                for (idx, sub) in sub_agents.iter().enumerate() {
                    active_child_flags.push((
                        sub.status == RunSpanStatus::Running,
                        sub.status == RunSpanStatus::Failed,
                    ));
                    sub_cards.push(self.render_agent_card(
                        sub,
                        if root_folded {
                            None
                        } else {
                            Some(self.child_sensors[idx].clone())
                        },
                        Some(sub_w),
                        now_ms,
                        elapsed_ms,
                        sheen_phase,
                        reduced,
                        &p,
                        w,
                        cx,
                    ));
                }

                let r_cell = self.root_sensor.clone();
                let s_cell = self.sup_sensor.clone();
                let c_cells: Vec<_> = if root_folded {
                    Vec::new()
                } else {
                    self.child_sensors
                        .iter()
                        .take(sub_cards.len())
                        .cloned()
                        .collect()
                };
                let link_base = if p.dark {
                    p.blue.opacity(0.36)
                } else {
                    p.blue.opacity(0.42)
                };
                let link_muted = p.line_strong;
                let link_fail = p.red.opacity(0.48);
                let pulse_col = p.blue;
                let dot_ring = p.card;

                let connector_layer = canvas(
                    |_, _, _| (),
                    move |_, _, window, _| {
                        let Some(rb) = r_cell.get() else {
                            return;
                        };
                        if let Some(sb) = s_cell.get() {
                            let p0 = point(rb.right(), rb.center().y);
                            let p1 = point(sb.left(), sb.center().y);
                            paint_dashed_line(window, p0, p1, link_muted, 1.5, 3.5, 3.5);
                            paint_endpoint_dot(window, p0, link_muted, dot_ring);
                            paint_endpoint_dot(window, p1, link_muted, dot_ring);
                        }
                        let root_bottom = point(rb.center().x, rb.bottom());
                        for (idx, cell) in c_cells.iter().enumerate() {
                            let Some(cb) = cell.get() else {
                                continue;
                            };
                            let child_top = point(cb.center().x, cb.top());
                            let (is_run, is_fail) = active_child_flags
                                .get(idx)
                                .copied()
                                .unwrap_or((false, false));
                            let base_c = if is_fail {
                                link_fail
                            } else if is_run {
                                link_base
                            } else {
                                link_muted
                            };
                            let prog = if is_run {
                                pulse_phase.map(|ph| (ph + idx as f32 * 0.22).fract())
                            } else {
                                None
                            };
                            pro_connector_pulse(
                                root_bottom,
                                child_top,
                                10.0,
                                true,
                                base_c,
                                pulse_col,
                                prog,
                                1.5,
                                window,
                            );
                            paint_endpoint_dot(window, child_top, base_c, dot_ring);
                        }
                        if !c_cells.is_empty() {
                            paint_endpoint_dot(window, root_bottom, link_base, dot_ring);
                        }
                    },
                )
                .absolute()
                .inset_0();

                let sub_row_pres = (!sub_agents.is_empty()).then(|| {
                    pro_popover_presence(
                        pro_eid(&self.id, "ri-sub-wide-pres"),
                        !root_folded,
                        move || {
                            div()
                                .w_full()
                                .flex()
                                .items_start()
                                .justify_center()
                                .gap(px(10.))
                                .children(sub_cards)
                                .into_any_element()
                        },
                    )
                });

                div()
                    .relative()
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(36.))
                    .child(connector_layer)
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_start()
                            .justify_center()
                            .gap(px(40.))
                            .child(root_card)
                            .when_some(sup_card, |d, sc| d.child(sc)),
                    )
                    .when_some(sub_row_pres, |d, sp| d.child(sp))
                    .into_any_element()
            };

            div()
                .w_full()
                .max_w(px(820.))
                .mx_auto()
                .flex()
                .flex_col()
                .gap(px(18.))
                .child(header_bar)
                .child(summary_card)
                .child(tree_el)
                .into_any_element()
        };

        let mode_key: SharedString = if self.loading {
            "loading".into()
        } else if self.empty {
            "empty".into()
        } else {
            "ready".into()
        };

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .relative()
            .w_full()
            .min_w_0()
            .px(px(if narrow { 16. } else { 36. }))
            .py(px(if narrow { 20. } else { 32. }))
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
                .inset_0(),
            )
            .child(pro_view_transition(
                pro_eid(&self.id, "ri-mode-vtx"),
                mode_key,
                body,
            ))
            .child(pro_toast_banner(self.toast.clone()))
    }
}

fn paint_endpoint_dot(window: &mut Window, center: Point<Pixels>, stroke_c: Hsla, fill_c: Hsla) {
    let r = px(3.25);
    let bounds = Bounds::new(center - point(r, r), size(r * 2.0, r * 2.0));
    window.paint_quad(quad(
        bounds,
        Corners::all(r),
        fill_c,
        Edges::all(px(1.5)),
        stroke_c,
        BorderStyle::Solid,
    ));
}

fn paint_dashed_line(
    window: &mut Window,
    p0: Point<Pixels>,
    p1: Point<Pixels>,
    color: Hsla,
    stroke_w: f32,
    dash: f32,
    gap: f32,
) {
    let dx = f32::from(p1.x - p0.x);
    let dy = f32::from(p1.y - p0.y);
    let dist = dx.hypot(dy);
    if dist <= 1.0 {
        return;
    }
    let step = dash + gap;
    let mut d = 0.0_f32;
    let mut pb = PathBuilder::stroke(px(stroke_w));
    while d < dist {
        let t0 = d / dist;
        let t1 = ((d + dash) / dist).min(1.0);
        pb.move_to(point(p0.x + px(dx * t0), p0.y + px(dy * t0)));
        pb.line_to(point(p0.x + px(dx * t1), p0.y + px(dy * t1)));
        d += step;
    }
    if let Ok(path) = pb.build() {
        window.paint_path(path, color);
    }
}

fn format_duration_compact(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms}ms")
    } else {
        format!("{:.1}s", ms as f32 / 1000.0)
    }
}

fn format_tokens_short(tok: u64) -> String {
    if tok >= 1_000_000 {
        format!("{:.1}M", tok as f64 / 1_000_000.0)
    } else if tok >= 1_000 {
        format!("{:.1}k", tok as f64 / 1_000.0)
    } else {
        format!("{tok}")
    }
}

#[cfg(test)]
mod tests {
    use super::{RunInspectorBlock, RunSpanStatus, compute_run_rollup, default_run_span};

    #[test]
    fn default_run_rollup_counts_five_agents_and_single_checkout_failure() {
        let (run, sup) = default_run_span();
        let rollup = compute_run_rollup(&run, Some(&sup), 24_800);
        assert_eq!(rollup.agent_count, 5);
        assert_eq!(rollup.llm_calls, 4);
        assert_eq!(rollup.tool_calls, 7);
        assert_eq!(rollup.failures, 1);
        assert!((rollup.cost - 0.48).abs() < 1e-6);
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn run_inspector_scopes_agent_and_retries_failed_checkout_call() {
        let mut app = gpui::TestAppContext::single();
        app.update(crate::init);
        let (inspector, w) =
            app.add_window_view(|_, cx| RunInspectorBlock::new("test-run-inspector", cx));
        w.update(|_, cx| {
            inspector.update(cx, |ins, cx| {
                assert_eq!(ins.open_call().map(|s| s.as_ref()), Some("tests"));
                assert_eq!(ins.metrics().failures, 1);

                ins.select_agent(Some("coder".into()), cx);
                assert_eq!(ins.metrics().agent_count, 1);
                assert_eq!(ins.metrics().failures, 1);

                assert!(ins.retry_call("tests", cx));
                assert_eq!(ins.metrics().failures, 0);

                ins.select_agent(None, cx);
                assert_eq!(ins.metrics().agent_count, 5);
                assert_eq!(ins.metrics().failures, 0);
                assert_eq!(ins.run.children[4].status, RunSpanStatus::Done);
            });
        });
    }
}
