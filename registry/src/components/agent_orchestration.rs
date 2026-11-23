//! Multi-agent orchestration tree canvas, live trace waterfall, token ledger, and span inspector.
//!
//! Renders a live hierarchical agent run inside [`AppShell`] with pan/zoom canvas controls,
//! curved orthogonal connectors with active flow pulses, a resizable three-tab run panel
//! (`Trace`, `Tokens & cost`, `Events`), interactive human-in-the-loop approvals, and a
//! floating agent/span inspector.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_agent_avatar, pro_btn, pro_eid, pro_empty_state, pro_fold,
            pro_icon_btn, pro_mark_selected, pro_pill_indicator, pro_presence, pro_progress_ring,
            pro_selection_sensor, pro_skeleton_bar, pro_spring_bar, pro_toast_banner,
            pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        clipboard,
        control::activate,
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashSet, rc::Rc};
use web_time::Instant;

const LAYERS_SVG: &str = r#"<path d="M8.64298 3.14559L6.93816 3.93362C4.31272 5.14719 3 5.75397 3 6.75C3 7.74603 4.31272 8.35281 6.93817 9.56638L8.64298 10.3544C10.2952 11.1181 11.1214 11.5 12 11.5C12.8786 11.5 13.7048 11.1181 15.357 10.3544L17.0618 9.56638C19.6873 8.35281 21 7.74603 21 6.75C21 5.75397 19.6873 5.14719 17.0618 3.93362L15.357 3.14559C13.7048 2.38186 12.8786 2 12 2C11.1214 2 10.2952 2.38186 8.64298 3.14559Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M20.788 11.0972C20.9293 11.2959 21 11.5031 21 11.7309C21 12.7127 19.6873 13.3109 17.0618 14.5072L15.357 15.284C13.7048 16.0368 12.8786 16.4133 12 16.4133C11.1214 16.4133 10.2952 16.0368 8.64298 15.284L6.93817 14.5072C4.31272 13.3109 3 12.7127 3 11.7309C3 11.5031 3.07067 11.2959 3.212 11.0972" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M20.3767 16.2661C20.7922 16.5971 21 16.927 21 17.3176C21 18.2995 19.6873 18.8976 17.0618 20.0939L15.357 20.8707C13.7048 21.6236 12.8786 22 12 22C11.1214 22 10.2952 21.6236 8.64298 20.8707L6.93817 20.0939C4.31272 18.8976 3 18.2995 3 17.3176C3 16.927 3.20778 16.5971 3.62334 16.2661" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const STOP_SQUARE_SVG: &str = r#"<path d="M4 12C4 8.72077 4 7.08116 4.81382 5.91891C5.1149 5.48891 5.48891 5.1149 5.91891 4.81382C7.08116 4 8.72077 4 12 4C15.2792 4 16.9188 4 18.0811 4.81382C18.5111 5.1149 18.8851 5.48891 19.1862 5.91891C20 7.08116 20 8.72077 20 12C20 15.2792 20 16.9188 19.1862 18.0811C18.8851 18.5111 18.5111 18.8851 18.0811 19.1862C16.9188 20 15.2792 20 12 20C8.72077 20 7.08116 20 5.91891 19.1862C5.48891 18.8851 5.1149 18.5111 4.81382 18.0811C4 16.9188 4 15.2792 4 12Z" stroke="currentColor" stroke-width="1.9"></path>"#;

const FIT_VIEW_SVG: &str = r#"<path d="M3 4V20" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M21 4V20" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M15.6216 9L16.5002 9.87868C17.5002 10.8787 18.0002 11.3787 18.0002 12C18.0002 12.6213 17.5002 13.1213 16.5002 14.1213L15.6216 15M8.37892 9L7.50024 9.87868C6.50024 10.8787 6.00024 11.3787 6.00024 12C6.00024 12.6213 6.50024 13.1213 7.50024 14.1213L8.37892 15M6.37892 12H17.6216" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const LOCATE_PIN_SVG: &str = r#"<path d="M13.6177 21.367C13.1841 21.773 12.6044 22 12.0011 22C11.3978 22 10.8182 21.773 10.3845 21.367C6.41302 17.626 1.09076 13.4469 3.68627 7.37966C5.08963 4.09916 8.45834 2 12.0011 2C15.5439 2 18.9126 4.09916 20.316 7.37966C22.9082 13.4393 17.599 17.6389 13.6177 21.367Z" stroke="currentColor" stroke-width="1.9"></path><path d="M15.5 11C15.5 12.933 13.933 14.5 12 14.5C10.067 14.5 8.5 12.933 8.5 11C8.5 9.067 10.067 7.5 12 7.5C13.933 7.5 15.5 9.067 15.5 11Z" stroke="currentColor" stroke-width="1.9"></path>"#;

const DASHED_RING_SVG: &str = r#"<circle cx="12" cy="12" r="8" stroke="currentColor" stroke-width="2.2" stroke-dasharray="3 3.3"></circle>"#;

const CARD_W: f32 = 240.0;
const CARD_H: f32 = 110.0;

/// Lifecycle state of an orchestrated [`AoAgent`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AoAgentStatus {
    /// Waiting for an upstream dependency or slot.
    #[default]
    Queued,
    /// Actively executing LLM or tool calls.
    Running,
    /// Paused awaiting human approval (`Needs you`).
    Waiting,
    /// Completed successfully.
    Done,
    /// Terminated with an error.
    Failed,
    /// Cancelled by the operator.
    Stopped,
}

/// Span kind in the orchestration waterfall.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AoSpanKind {
    /// Full lifetime bar of an agent.
    #[default]
    Agent,
    /// Model reasoning/completion span.
    Llm,
    /// Tool execution span.
    Tool,
}

/// Active tab of the bottom run details panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AoRunTab {
    /// Hierarchical agent + call waterfall trace.
    #[default]
    Trace,
    /// Per-agent token and cost breakdown table.
    TokensAndCost,
    /// Chronological run event stream.
    Events,
}

/// Active tab of the floating right-hand agent inspector.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AoInspectorTab {
    /// Task list, metrics, and human approval prompt.
    #[default]
    Tasks,
    /// Handoff lineage and shared context artifacts.
    Context,
    /// Prompt input, tool call JSON payload, and output preview.
    Payloads,
}

/// Run-level playback action triggered from the top-right controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AoRunAction {
    /// Start an unstarted run from zero.
    Start,
    /// Freeze the live clock and flows.
    Pause,
    /// Resume a paused run.
    Resume,
    /// Terminate all running and waiting agents.
    Stop,
    /// Replay the orchestration from the beginning.
    Restart,
}

/// Event category shown in the `Events` tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AoEventKind {
    /// Agent started executing.
    Start,
    /// Parent delegated work to a sub-agent.
    Handoff,
    /// Tool call completed.
    Tool,
    /// Agent paused for operator approval.
    Approval,
    /// Tool or verification call failed.
    Error,
    /// Retry branch spawned with backoff.
    Retry,
    /// Agent finished its assigned task.
    Done,
}

/// One atomic sub-task executed by an [`AoAgent`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AoTask {
    /// Stable sub-task id.
    pub id: SharedString,
    /// Human-readable task description.
    pub title: SharedString,
    /// Tool icon key (`"web"`, `"fetch"`, `"code"`, `"sql"`, `"chart"`, `"doc"`, `"cite"`).
    pub tool_id: SharedString,
    /// Display name of the invoked tool.
    pub tool_label: SharedString,
    /// Short outcome summary.
    pub result: SharedString,
    /// Sub-task status.
    pub status: AoAgentStatus,
    /// Start offset in milliseconds from run start.
    pub start_ms: u64,
    /// Total duration in milliseconds (split into an LLM call + tool call in the trace).
    pub duration_ms: u64,
    /// Tokens consumed by this sub-task.
    pub tokens: u64,
}

impl AoTask {
    /// Create a sub-task record.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        tool_id: impl Into<SharedString>,
        tool_label: impl Into<SharedString>,
        result: impl Into<SharedString>,
        status: AoAgentStatus,
        start_ms: u64,
        duration_ms: u64,
        tokens: u64,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            tool_id: tool_id.into(),
            tool_label: tool_label.into(),
            result: result.into(),
            status,
            start_ms,
            duration_ms,
            tokens,
        }
    }
}

/// Shared context document or upstream artifact attached to an [`AoAgent`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AoContextItem {
    /// Artifact or memory key title.
    pub name: SharedString,
    /// Size, row count, or source label.
    pub meta: SharedString,
    /// Leading glyph.
    pub icon: ProIcon,
}

impl AoContextItem {
    /// Create a context entry.
    pub fn new(
        name: impl Into<SharedString>,
        meta: impl Into<SharedString>,
        icon: ProIcon,
    ) -> Self {
        Self {
            name: name.into(),
            meta: meta.into(),
            icon,
        }
    }
}

/// One agent node in the [`AgentOrchestrationScreen`] hierarchy.
#[derive(Clone, Debug, PartialEq)]
pub struct AoAgent {
    /// Unique agent identifier.
    pub id: SharedString,
    /// Parent agent id (`None` for the top-level orchestrator).
    pub parent: Option<SharedString>,
    /// Display name (`"Researcher"`, `"Data analyst"`, etc.).
    pub name: SharedString,
    /// Role badge (`"Orchestrator"`, `"Research"`, `"Analysis"`, `"Writing"`, `"Review"`).
    pub role: SharedString,
    /// One-line task description on the canvas card.
    pub task: SharedString,
    /// Model label (`"Opus 5.5"`, `"Sonnet 5.5"`, `"Gemini 3.8 Flash"`, `"GPT-6 Sol"`, `"GPT-6 Mini"`).
    pub model: SharedString,
    /// Provider key for artwork lookup (`"claude"`, `"openai"`, `"gemini"`).
    pub maker: SharedString,
    /// Tool ids shown as mini tiles on the card footer.
    pub tools: Vec<SharedString>,
    /// Current execution status.
    pub status: AoAgentStatus,
    /// Start offset in milliseconds from the beginning of the run.
    pub start_ms: Option<u64>,
    /// End offset in milliseconds when finished or failed.
    pub end_ms: Option<u64>,
    /// Input prompt tokens.
    pub tokens_in: u64,
    /// Output completion tokens.
    pub tokens_out: u64,
    /// Cached prompt tokens.
    pub tokens_cached: u64,
    /// Agent's own monetary cost in USD (branch cost sums descendants).
    pub cost: f64,
    /// Optional human-in-the-loop question when `status == AoAgentStatus::Waiting`.
    pub ask: Option<SharedString>,
    /// Optional failure message when `status == AoAgentStatus::Failed`.
    pub error: Option<SharedString>,
    /// Optional id of the failed agent that this agent retries.
    pub retry_of: Option<SharedString>,
    /// Prompt input passed into this agent.
    pub prompt_input: SharedString,
    /// Latest tool call JSON payload for span inspection.
    pub tool_payload: SharedString,
    /// Latest output excerpt produced by this agent.
    pub output_preview: SharedString,
    /// Sub-tasks executed by this agent.
    pub tasks: Vec<AoTask>,
    /// Context artifacts read or produced by this agent.
    pub context: Vec<AoContextItem>,
}

impl AoAgent {
    /// Total input + output tokens for this agent alone.
    pub const fn total_tokens(&self) -> u64 {
        self.tokens_in.saturating_add(self.tokens_out)
    }
}

/// Timestamped entry in the orchestration `Events` log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AoEvent {
    /// Event identifier.
    pub id: SharedString,
    /// Timestamp in milliseconds from run start.
    pub at_ms: u64,
    /// Associated agent id.
    pub agent_id: SharedString,
    /// Associated agent display name.
    pub agent_name: SharedString,
    /// Event type.
    pub kind: AoEventKind,
    /// Description text.
    pub text: SharedString,
}

impl AoEvent {
    /// Construct a timestamped orchestration event.
    pub fn new(
        id: impl Into<SharedString>,
        at_ms: u64,
        agent_id: impl Into<SharedString>,
        agent_name: impl Into<SharedString>,
        kind: AoEventKind,
        text: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            at_ms,
            agent_id: agent_id.into(),
            agent_name: agent_name.into(),
            kind,
            text: text.into(),
        }
    }
}

/// Events emitted by [`AgentOrchestrationScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentOrchestrationEvent {
    /// Sidebar item clicked.
    Navigate(SharedString),
    /// Agent inspector opened (`Some(id)`) or closed (`None`).
    SelectAgent(Option<SharedString>),
    /// Operator approved or rejected a waiting agent.
    Resolve {
        /// Waiting agent identifier.
        agent_id: SharedString,
        /// `true` when approved, `false` when rejected.
        approved: bool,
    },
    /// Operator triggered a run playback action.
    RunControl(AoRunAction),
    /// Operator switched the active project from the path breadcrumb.
    SwitchProject(SharedString),
    /// Operator switched the active run from the path breadcrumb.
    SwitchRun(SharedString),
}

impl EventEmitter<AgentOrchestrationEvent> for AgentOrchestrationScreen {}

/// Positioned node in world coordinates computed by the hierarchical layout.
#[derive(Clone, Debug)]
pub(crate) struct PositionedAgent {
    pub index: usize,
    pub depth: usize,
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug)]
struct ConnectorLink {
    points: Vec<(f32, f32)>,
    muted: bool,
    flowing: bool,
    faded: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DragKind {
    CanvasPan {
        start_mouse_x: i32,
        start_mouse_y: i32,
        orig_pan_x: f32,
        orig_pan_y: f32,
    },
    ResizeRunPanel {
        start_mouse_y: i32,
        orig_h: f32,
    },
}

/// Compute exact world-space `(x, y)` coordinates for the orchestrator and its column branches.
pub(crate) fn layout_agent_tree(agents: &[AoAgent]) -> (Vec<PositionedAgent>, f32, f32) {
    if agents.is_empty() {
        return (Vec::new(), 0.0, 0.0);
    }
    let root_idx = agents.iter().position(|a| a.parent.is_none()).unwrap_or(0);
    let root_id = &agents[root_idx].id;

    let top_branches: Vec<usize> = agents
        .iter()
        .enumerate()
        .filter(|&(i, a)| i != root_idx && a.parent.as_ref() == Some(root_id))
        .map(|(i, _)| i)
        .collect();

    let mut nodes: Vec<Option<PositionedAgent>> = vec![None; agents.len()];
    let mut cursor_x = 0.0_f32;
    let mut max_bottom = CARD_H;

    let mut first_col_cx = 0.0_f32;
    let mut last_col_cx = 0.0_f32;

    for (bi, &branch_idx) in top_branches.iter().enumerate() {
        let col_x = cursor_x;
        let col_cx = col_x + CARD_W * 0.5;
        if bi == 0 {
            first_col_cx = col_cx;
        }
        last_col_cx = col_cx;

        let mut stack: Vec<(usize, usize)> = vec![(branch_idx, 1)];
        let mut row_slot = 0usize;
        let mut col_max_right = col_x + CARD_W;

        while let Some((cur_idx, depth)) = stack.pop() {
            let x = col_x + (depth.saturating_sub(1) as f32) * 28.0;
            let y = 182.0 + (row_slot as f32) * 150.0;
            col_max_right = col_max_right.max(x + CARD_W);
            max_bottom = max_bottom.max(y + CARD_H);
            nodes[cur_idx] = Some(PositionedAgent {
                index: cur_idx,
                depth,
                x,
                y,
            });
            row_slot += 1;

            let cur_id = &agents[cur_idx].id;
            let children: Vec<usize> = agents
                .iter()
                .enumerate()
                .filter(|&(_, a)| a.parent.as_ref() == Some(cur_id))
                .map(|(i, _)| i)
                .collect();
            for &ch in children.iter().rev() {
                stack.push((ch, depth + 1));
            }
        }

        cursor_x = col_max_right + 20.0;
    }

    let root_x = if top_branches.is_empty() {
        0.0
    } else {
        ((first_col_cx + last_col_cx) * 0.5 - CARD_W * 0.5).max(0.0)
    };
    nodes[root_idx] = Some(PositionedAgent {
        index: root_idx,
        depth: 0,
        x: root_x,
        y: 0.0,
    });

    // Place any orphan nodes gracefully if present
    let mut fallback_y = max_bottom + 40.0;
    for (i, slot) in nodes.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(PositionedAgent {
                index: i,
                depth: 1,
                x: 0.0,
                y: fallback_y,
            });
            fallback_y += 150.0;
            max_bottom = max_bottom.max(fallback_y);
        }
    }

    // Order in pre-order traversal matching Trace & Tokens tables
    let mut ordered = Vec::with_capacity(agents.len());
    let mut dfs = vec![root_idx];
    let mut visited = vec![false; agents.len()];
    while let Some(idx) = dfs.pop() {
        if visited[idx] {
            continue;
        }
        visited[idx] = true;
        if let Some(ref pos) = nodes[idx] {
            ordered.push(pos.clone());
        }
        let pid = &agents[idx].id;
        let children: Vec<usize> = agents
            .iter()
            .enumerate()
            .filter(|&(_, a)| a.parent.as_ref() == Some(pid))
            .map(|(i, _)| i)
            .collect();
        for &ch in children.iter().rev() {
            dfs.push(ch);
        }
    }
    for (i, pos_opt) in nodes.into_iter().enumerate() {
        if !visited[i]
            && let Some(pos) = pos_opt
        {
            ordered.push(pos);
        }
    }

    let world_w = ordered.iter().fold(CARD_W, |acc, n| acc.max(n.x + CARD_W));
    (ordered, world_w, max_bottom)
}

/// Recursively sum the branch cost (own + all descendants) of `agent_id`.
pub(crate) fn branch_cost(agents: &[AoAgent], agent_id: &str) -> f64 {
    let mut total = 0.0;
    let mut stack = vec![agent_id];
    while let Some(cur) = stack.pop() {
        for a in agents {
            if a.id.as_ref() == cur {
                total += a.cost;
            } else if a.parent.as_deref() == Some(cur) {
                stack.push(a.id.as_ref());
            }
        }
    }
    total
}

/// Format milliseconds as `01:35` clock readout.
pub(crate) fn format_clock_ms(ms: u64) -> String {
    let secs = ms / 1000;
    let mins = secs / 60;
    let rem = secs % 60;
    format!("{mins:02}:{rem:02}")
}

/// Format duration in milliseconds (`"7.1s"`, `"41s"`, `"1m 35s"`).
pub(crate) fn format_duration_ms(ms: u64) -> String {
    if ms < 10_000 {
        let tenths = (ms as f64 / 100.0).round() / 10.0;
        if (tenths - tenths.round()).abs() < 0.04 {
            return format!("{}s", tenths.round() as u64);
        }
        return format!("{tenths:.1}s");
    }
    let secs = ((ms as f64) / 1000.0).round() as u64;
    if secs < 60 {
        format!("{secs}s")
    } else {
        let mins = secs / 60;
        let rem = secs % 60;
        if rem == 0 {
            format!("{mins}m")
        } else {
            format!("{mins}m {rem}s")
        }
    }
}

/// Format token counts compactly (`"579.2K"`, `"84.2K"`).
pub(crate) fn format_tokens_k(n: u64) -> String {
    if n == 0 {
        return "–".to_string();
    }
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn tool_icon(tool_id: &str) -> ProIcon {
    match tool_id {
        "web" => ProIcon::Globe,
        "fetch" => ProIcon::Link,
        "code" => ProIcon::Code,
        "sql" => ProIcon::Database,
        "chart" => ProIcon::UsageChart,
        "doc" => ProIcon::PageDoc,
        "cite" => ProIcon::Note,
        "delegate" => ProIcon::GitBranch,
        _ => ProIcon::Bolt,
    }
}

#[allow(clippy::too_many_lines)]
fn default_market_research_agents() -> Vec<AoAgent> {
    vec![
        AoAgent {
            id: "orchestrator".into(),
            parent: None,
            name: "Orchestrator".into(),
            role: "Orchestrator".into(),
            task: "Write the Q4 market report on European e-bike makers".into(),
            model: "Opus 5.5".into(),
            maker: "claude".into(),
            tools: vec!["delegate".into(), "doc".into(), "cite".into()],
            status: AoAgentStatus::Running,
            start_ms: Some(0),
            end_ms: None,
            tokens_in: 62_800,
            tokens_out: 14_200,
            tokens_cached: 42_000,
            cost: 0.62,
            ask: None,
            error: None,
            retry_of: None,
            prompt_input: "Produce an executive-ready Q4 European e-bike market report covering unit sales, pricing tiers, annual filings, 2028 forecasts, and verified citations.".into(),
            tool_payload: "{\n  \"tool\": \"spawn_branch\",\n  \"branches\": [\"researcher\", \"analyst\", \"writer\", \"fact-checker\", \"reviewer\"]\n}".into(),
            output_preview: "4/5 branches dispatched · Research & Analysis merged · awaiting Writer approval & Fact-checker retry".into(),
            tasks: vec![
                AoTask::new("o-1", "Decompose brief into five workstreams", "delegate", "Planner", "5 branches scheduled", AoAgentStatus::Done, 400, 6_800, 12_400),
                AoTask::new("o-2", "Dispatch Research and Analysis in parallel", "delegate", "Handoff", "Researcher + Data analyst started", AoAgentStatus::Done, 7_800, 5_400, 18_600),
                AoTask::new("o-3", "Merge findings into Writer & Fact-checker", "doc", "Workspace", "24 tables + 6 charts linked", AoAgentStatus::Done, 43_000, 8_200, 28_400),
                AoTask::new("o-4", "Coordinate sign-off and final assembly", "cite", "Orchestrator", "Waiting on Writer & Claim check", AoAgentStatus::Running, 72_000, 23_000, 23_200),
            ],
            context: vec![
                AoContextItem::new("Q4 Research Brief.md", "4.2 KB · input", ProIcon::PageDoc),
                AoContextItem::new("EU Maker Coverage List.csv", "12 makers · sheet", ProIcon::Database),
                AoContextItem::new("Merged Findings Bundle", "18 sources · 6 charts", ProIcon::Folder),
            ],
        },
        AoAgent {
            id: "researcher".into(),
            parent: Some("orchestrator".into()),
            name: "Researcher".into(),
            role: "Research".into(),
            task: "Find what each maker sold, at what price, and why".into(),
            model: "Sonnet 5.5".into(),
            maker: "claude".into(),
            tools: vec!["web".into(), "delegate".into()],
            status: AoAgentStatus::Done,
            start_ms: Some(1_200),
            end_ms: Some(42_200),
            tokens_in: 48_200,
            tokens_out: 9_800,
            tokens_cached: 24_100,
            cost: 0.27,
            ask: None,
            error: None,
            retry_of: None,
            prompt_input: "Collect 2025 unit volumes, retail pricing, and channel mix across the top 12 European e-bike manufacturers.".into(),
            tool_payload: "{\n  \"tool\": \"delegate_subagents\",\n  \"targets\": [\"web-search\", \"competitor-scan\"]\n}".into(),
            output_preview: "Compiled 12-maker unit matrix (3.84M units) and 64-model retail price catalog.".into(),
            tasks: vec![
                AoTask::new("r-1", "Spawn Web search and Competitor scan", "delegate", "Handoff", "2 sub-agents completed", AoAgentStatus::Done, 1_800, 9_400, 18_200),
                AoTask::new("r-2", "Normalize OEM unit volumes & currencies", "web", "Synthesizer", "12 makers reconciled in EUR", AoAgentStatus::Done, 28_000, 13_500, 39_800),
            ],
            context: vec![
                AoContextItem::new("12-Maker Sales Matrix", "3.84M units · verified", ProIcon::Database),
                AoContextItem::new("OEM Channel Notes.md", "9 retail networks", ProIcon::PageDoc),
            ],
        },
        AoAgent {
            id: "web-search".into(),
            parent: Some("researcher".into()),
            name: "Web search".into(),
            role: "Research".into(),
            task: "Find 2025 sales figures for the twelve largest makers".into(),
            model: "Gemini 3.8 Flash".into(),
            maker: "gemini".into(),
            tools: vec!["web".into()],
            status: AoAgentStatus::Done,
            start_ms: Some(2_500),
            end_ms: Some(14_500),
            tokens_in: 24_600,
            tokens_out: 4_400,
            tokens_cached: 8_000,
            cost: 0.02,
            ask: None,
            error: None,
            retry_of: None,
            prompt_input: "Query trade associations (CONEBI, ZIV, RAI) and press releases for 2025 unit shipments.".into(),
            tool_payload: "{\n  \"tool\": \"web_search\",\n  \"queries\": [\"ZIV 2025 e-bike market share\", \"CONEBI European e-bike sales 2025\"]\n}".into(),
            output_preview: "Ranked 42 candidate URLs; top 18 trade bulletins forwarded to Page reader.".into(),
            tasks: vec![
                AoTask::new("ws-1", "Search ZIV, CONEBI & BOVAG trade bulletins", "web", "Web search", "42 results ranked", AoAgentStatus::Done, 2_800, 5_600, 14_200),
                AoTask::new("ws-2", "Filter primary disclosures for Page reader", "web", "Filter", "18 authoritative URLs", AoAgentStatus::Done, 8_800, 5_200, 14_800),
            ],
            context: vec![AoContextItem::new("18 Primary Source URLs", "ZIV · CONEBI · Accell", ProIcon::Globe)],
        },
        AoAgent {
            id: "page-reader".into(),
            parent: Some("web-search".into()),
            name: "Page reader".into(),
            role: "Research".into(),
            task: "Read the 18 best sources in full and pull the numbers".into(),
            model: "Gemini 3.8 Flash".into(),
            maker: "gemini".into(),
            tools: vec!["fetch".into()],
            status: AoAgentStatus::Done,
            start_ms: Some(6_200),
            end_ms: Some(13_300),
            tokens_in: 38_100,
            tokens_out: 5_900,
            tokens_cached: 16_400,
            cost: 0.04,
            ask: None,
            error: None,
            retry_of: None,
            prompt_input: "Fetch full HTML/PDF bodies of the 18 selected sources and extract exact unit tables.".into(),
            tool_payload: "{\n  \"tool\": \"fetch_pages\",\n  \"concurrency\": 6,\n  \"urls_count\": 18\n}".into(),
            output_preview: "Extracted 94 cited figures across Germany, Netherlands, France, and Belgium.".into(),
            tasks: vec![
                AoTask::new("pr-1", "Fetch 18 trade PDFs & press releases", "fetch", "Page reader", "18/18 fetched (0 errors)", AoAgentStatus::Done, 6_400, 3_800, 24_000),
                AoTask::new("pr-2", "Extract unit shipment tables with quotes", "cite", "Extractor", "94 figures extracted", AoAgentStatus::Done, 10_300, 2_800, 20_000),
            ],
            context: vec![AoContextItem::new("Extracted Quotes.json", "94 verbatim citations", ProIcon::Code)],
        },
        AoAgent {
            id: "competitor-scan".into(),
            parent: Some("researcher".into()),
            name: "Competitor scan".into(),
            role: "Research".into(),
            task: "Compare price, range and channels across nine makers".into(),
            model: "GPT-6 Mini".into(),
            maker: "openai".into(),
            tools: vec!["web".into(), "code".into()],
            status: AoAgentStatus::Done,
            start_ms: Some(3_000),
            end_ms: Some(39_000),
            tokens_in: 34_800,
            tokens_out: 7_200,
            tokens_cached: 14_000,
            cost: 0.05,
            ask: None,
            error: None,
            retry_of: None,
            prompt_input: "Map commuter, trekking, and cargo specs plus DTC vs dealer share across 9 leading brands.".into(),
            tool_payload: "{\n  \"tool\": \"compare_matrix\",\n  \"makers\": [\"Riese & Müller\", \"Cube\", \"Gazelle\", \"Canyon\", \"VanMoof\", \"Cowboy\", \"Kalkhoff\", \"Moustache\", \"Specialized\"]\n}".into(),
            output_preview: "Dealer networks still drive 74% of €3,200+ trekking volume; DTC leads urban lightweight.".into(),
            tasks: vec![
                AoTask::new("cs-1", "Map dealer vs DTC channel split", "web", "Web search", "9 brands profiled", AoAgentStatus::Done, 3_500, 16_000, 21_400),
                AoTask::new("cs-2", "Benchmark battery Wh & mid-drive motors", "code", "Python", "Bosch 68% share in €3k+ tier", AoAgentStatus::Done, 21_000, 17_200, 20_600),
            ],
            context: vec![AoContextItem::new("Channel Split Matrix", "9 OEMs · DTC vs IBD", ProIcon::Database)],
        },
        AoAgent {
            id: "pricing-scraper".into(),
            parent: Some("competitor-scan".into()),
            name: "Pricing scraper".into(),
            role: "Research".into(),
            task: "Collect list prices for 64 models from the makers' shops".into(),
            model: "Gemini 3.8 Flash".into(),
            maker: "gemini".into(),
            tools: vec!["fetch".into(), "code".into()],
            status: AoAgentStatus::Done,
            start_ms: Some(5_200),
            end_ms: Some(22_200),
            tokens_in: 29_400,
            tokens_out: 4_600,
            tokens_cached: 11_200,
            cost: 0.03,
            ask: None,
            error: None,
            retry_of: None,
            prompt_input: "Scrape current EUR MSRP and promo discounts across 64 flagship 2025 e-bike SKUs.".into(),
            tool_payload: "{\n  \"tool\": \"scrape_catalog\",\n  \"models\": 64,\n  \"region\": \"DE/NL/FR\"\n}".into(),
            output_preview: "Median MSRP €3,490 (-4.2% YoY); cargo tier median €5,190.".into(),
            tasks: vec![
                AoTask::new("ps-1", "Crawl 64 product pages in DE/NL stores", "fetch", "Page reader", "64 SKUs parsed", AoAgentStatus::Done, 5_600, 10_400, 19_800),
                AoTask::new("ps-2", "Compute MSRP quartiles by frame category", "code", "Python", "4 tier medians exported", AoAgentStatus::Done, 16_200, 5_600, 14_200),
            ],
            context: vec![AoContextItem::new("64_Model_Prices.csv", "EUR MSRP + promo delta", ProIcon::Database)],
        },
        AoAgent {
            id: "data-analyst".into(),
            parent: Some("orchestrator".into()),
            name: "Data analyst".into(),
            role: "Analysis".into(),
            task: "Size the market and model its growth up to 2028".into(),
            model: "GPT-6 Sol".into(),
            maker: "openai".into(),
            tools: vec!["sql".into(), "code".into(), "chart".into()],
            status: AoAgentStatus::Done,
            start_ms: Some(12_800),
            end_ms: Some(71_800),
            tokens_in: 64_200,
            tokens_out: 13_800,
            tokens_cached: 31_000,
            cost: 0.31,
            ask: None,
            error: None,
            retry_of: None,
            prompt_input: "Reconcile top-down country registrations with bottom-up maker filings and project 2025–2028 CAGR.".into(),
            tool_payload: "{\n  \"tool\": \"python_sandbox\",\n  \"script\": \"forecast_eu_ebike_2028.py\"\n}".into(),
            output_preview: "2025 EU27+UK market sized at €11.8B (5.42M units), 7.6% CAGR through 2028 (€14.7B).".into(),
            tasks: vec![
                AoTask::new("da-1", "Coordinate Filings reader & Market sizing", "delegate", "Handoff", "7 annual reports + 14 countries", AoAgentStatus::Done, 13_200, 18_000, 26_000),
                AoTask::new("da-2", "Run 2025–2028 base/bull/bear Monte Carlo", "code", "Python", "7.6% base CAGR (R²=0.94)", AoAgentStatus::Done, 48_000, 14_000, 31_400),
                AoTask::new("da-3", "Validate margin recovery after inventory clear", "sql", "DuckDB", "Gross margin +210 bps in H2", AoAgentStatus::Done, 62_500, 8_800, 20_600),
            ],
            context: vec![
                AoContextItem::new("EU27_Forecast_2028.parquet", "3 scenarios · 14 countries", ProIcon::Database),
                AoContextItem::new("6 Publication Charts", "SVG vector series", ProIcon::UsageChart),
            ],
        },
        AoAgent {
            id: "filings-reader".into(),
            parent: Some("data-analyst".into()),
            name: "Filings reader".into(),
            role: "Analysis".into(),
            task: "Pull revenue and margins from seven annual reports".into(),
            model: "Sonnet 5.5".into(),
            maker: "claude".into(),
            tools: vec!["doc".into(), "sql".into()],
            status: AoAgentStatus::Done,
            start_ms: Some(13_500),
            end_ms: Some(33_500),
            tokens_in: 52_000,
            tokens_out: 8_400,
            tokens_cached: 29_500,
            cost: 0.19,
            ask: None,
            error: None,
            retry_of: None,
            prompt_input: "Parse FY24/H1-25 IFRS segment tables from Accell, Pon.Bike, Pierer Mobility, Shimano EU, Bosch eBike, Giant EU, and Merida.".into(),
            tool_payload: "{\n  \"tool\": \"pdf_tables\",\n  \"filings\": 7,\n  \"metrics\": [\"revenue_eur\", \"ebitda_margin\", \"inventory_days\"]\n}".into(),
            output_preview: "Extracted IFRS segment revenue and inventory-days trend across all 7 public filings.".into(),
            tasks: vec![
                AoTask::new("fr-1", "Parse 7 IFRS annual & interim PDFs", "doc", "PDF parser", "31 financial tables", AoAgentStatus::Done, 14_000, 11_200, 38_400),
                AoTask::new("fr-2", "Standardize EUR segment margins", "sql", "DuckDB", "7/7 issuers normalized", AoAgentStatus::Done, 25_600, 7_500, 22_000),
            ],
            context: vec![AoContextItem::new("IFRS_Segment_Margins.sql", "7 issuers · FY22–FY25", ProIcon::Database)],
        },
        AoAgent {
            id: "market-sizing".into(),
            parent: Some("data-analyst".into()),
            name: "Market sizing".into(),
            role: "Analysis".into(),
            task: "Estimate the 2025 market size for each country".into(),
            model: "GPT-6 Mini".into(),
            maker: "openai".into(),
            tools: vec!["sql".into(), "code".into()],
            status: AoAgentStatus::Done,
            start_ms: Some(34_000),
            end_ms: Some(59_000),
            tokens_in: 26_500,
            tokens_out: 5_500,
            tokens_cached: 12_000,
            cost: 0.04,
            ask: None,
            error: None,
            retry_of: None,
            prompt_input: "Combine national cycling federation counts with ASP tiers for DE, NL, FR, BE, AT, IT, CH, ES, DK, SE, PL, UK.".into(),
            tool_payload: "{\n  \"tool\": \"python_sandbox\",\n  \"countries\": 12,\n  \"output\": \"country_tam_2025.json\"\n}".into(),
            output_preview: "DE leads at 2.05M units (€5.1B), followed by NL (0.71M) and FR (0.68M).".into(),
            tasks: vec![
                AoTask::new("ms-1", "Join federation unit series with country ASP", "sql", "DuckDB", "12 country rows built", AoAgentStatus::Done, 34_600, 13_400, 17_200),
                AoTask::new("ms-2", "Cross-check import/export Eurostat CN 871160", "code", "Python", "Variance < 1.8%", AoAgentStatus::Done, 48_500, 10_100, 14_800),
            ],
            context: vec![AoContextItem::new("country_tam_2025.json", "12 European markets", ProIcon::Code)],
        },
        AoAgent {
            id: "chart-builder".into(),
            parent: Some("data-analyst".into()),
            name: "Chart builder".into(),
            role: "Analysis".into(),
            task: "Draw the six charts for the report".into(),
            model: "Sonnet 5.5".into(),
            maker: "claude".into(),
            tools: vec!["chart".into(), "code".into()],
            status: AoAgentStatus::Done,
            start_ms: Some(59_300),
            end_ms: Some(70_300),
            tokens_in: 19_200,
            tokens_out: 4_800,
            tokens_cached: 7_400,
            cost: 0.08,
            ask: None,
            error: None,
            retry_of: None,
            prompt_input: "Render 6 publication-grade charts: country TAM, OEM share, price vs battery Wh, channel mix, inventory days, and 2028 forecast.".into(),
            tool_payload: "{\n  \"tool\": \"render_charts\",\n  \"count\": 6,\n  \"format\": \"svg\"\n}".into(),
            output_preview: "Rendered Fig 1–Fig 6 with consistent monochrome + accent palette.".into(),
            tasks: vec![
                AoTask::new("cb-1", "Generate Fig 1–3 (TAM, OEM share, pricing)", "chart", "Charting", "3 SVG figures exported", AoAgentStatus::Done, 59_600, 5_200, 12_100),
                AoTask::new("cb-2", "Generate Fig 4–6 (channels, inventory, 2028)", "chart", "Charting", "3 SVG figures exported", AoAgentStatus::Done, 65_000, 5_000, 11_900),
            ],
            context: vec![AoContextItem::new("Figures 1–6 Bundle", "6 vector charts", ProIcon::UsageChart)],
        },
        AoAgent {
            id: "writer".into(),
            parent: Some("orchestrator".into()),
            name: "Writer".into(),
            role: "Writing".into(),
            task: "Draft the report from the findings and the charts".into(),
            model: "Opus 5.5".into(),
            maker: "claude".into(),
            tools: vec!["doc".into(), "cite".into(), "delegate".into()],
            status: AoAgentStatus::Waiting,
            start_ms: Some(44_000),
            end_ms: None,
            tokens_in: 46_800,
            tokens_out: 11_200,
            tokens_cached: 22_000,
            cost: 0.34,
            ask: Some("Publish draft to shared Research folder and notify #product-strategy?".into()),
            error: None,
            retry_of: None,
            prompt_input: "Synthesize executive summary and 6 analytical chapters with inline source citations and embedded Fig 1–6.".into(),
            tool_payload: "{\n  \"tool\": \"request_approval\",\n  \"scope\": \"research/Q4-EU-Ebike-Report.md\",\n  \"notify\": \"#product-strategy\"\n}".into(),
            output_preview: "Executive summary & section structure ready · waiting for permission to write shared report file.".into(),
            tasks: vec![
                AoTask::new("w-1", "Generate six-section report architecture", "delegate", "Outline", "6 sections + exec summary", AoAgentStatus::Done, 44_500, 7_700, 14_600),
                AoTask::new("w-2", "Draft Executive Summary & key takeaways", "doc", "Document", "1,120 words · 19 citations", AoAgentStatus::Done, 53_000, 21_000, 28_400),
                AoTask::new("w-3", "Confirm shared folder write permission", "cite", "Approval", "Awaiting operator decision", AoAgentStatus::Waiting, 75_000, 20_000, 15_000),
            ],
            context: vec![
                AoContextItem::new("Executive Summary Draft.md", "1,120 words", ProIcon::PageDoc),
                AoContextItem::new("Citation Ledger", "94 source anchors", ProIcon::Note),
            ],
        },
        AoAgent {
            id: "outline".into(),
            parent: Some("writer".into()),
            name: "Outline".into(),
            role: "Writing".into(),
            task: "Outline six sections and the executive summary".into(),
            model: "Sonnet 5.5".into(),
            maker: "claude".into(),
            tools: vec!["doc".into()],
            status: AoAgentStatus::Done,
            start_ms: Some(44_500),
            end_ms: Some(52_200),
            tokens_in: 12_400,
            tokens_out: 2_600,
            tokens_cached: 6_200,
            cost: 0.05,
            ask: None,
            error: None,
            retry_of: None,
            prompt_input: "Structure a 6-part institutional market report mapping every chart and table to a thesis claim.".into(),
            tool_payload: "{\n  \"tool\": \"write_outline\",\n  \"sections\": 6\n}".into(),
            output_preview: "1. Demand Reset · 2. Maker Share · 3. Pricing Tiers · 4. Retail vs DTC · 5. Margin Recovery · 6. 2028 Outlook.".into(),
            tasks: vec![AoTask::new("ol-1", "Structure 6 chapters & chart placements", "doc", "Outliner", "Outline approved by Writer", AoAgentStatus::Done, 44_800, 7_200, 15_000)],
            context: vec![AoContextItem::new("Report_Outline_v1.md", "6 sections · 6 figures", ProIcon::PageDoc)],
        },
        AoAgent {
            id: "section-drafter".into(),
            parent: Some("writer".into()),
            name: "Section drafter".into(),
            role: "Writing".into(),
            task: "Write sections 1–6 with a citation for every number".into(),
            model: "Sonnet 5.5".into(),
            maker: "claude".into(),
            tools: vec!["doc".into(), "cite".into()],
            status: AoAgentStatus::Queued,
            start_ms: None,
            end_ms: None,
            tokens_in: 0,
            tokens_out: 0,
            tokens_cached: 0,
            cost: 0.0,
            ask: None,
            error: None,
            retry_of: None,
            prompt_input: "Queued behind Writer's folder permission check to write full sections 1–6.".into(),
            tool_payload: "{\n  \"status\": \"queued\",\n  \"blocked_by\": \"writer\"\n}".into(),
            output_preview: "Waiting for Writer approval before streaming sections 1–6.".into(),
            tasks: vec![AoTask::new("sd-1", "Draft chapters 1–6 with inline citations", "doc", "Document", "Queued on Writer", AoAgentStatus::Queued, 0, 0, 0)],
            context: vec![AoContextItem::new("Report_Outline_v1.md", "Upstream blueprint", ProIcon::PageDoc)],
        },
        AoAgent {
            id: "fact-checker".into(),
            parent: Some("orchestrator".into()),
            name: "Fact-checker".into(),
            role: "Review".into(),
            task: "Check every number and quote in the findings".into(),
            model: "Sonnet 5.5".into(),
            maker: "claude".into(),
            tools: vec!["fetch".into(), "cite".into(), "delegate".into()],
            status: AoAgentStatus::Running,
            start_ms: Some(72_000),
            end_ms: None,
            tokens_in: 14_200,
            tokens_out: 3_400,
            tokens_cached: 6_800,
            cost: 0.09,
            ask: None,
            error: None,
            retry_of: None,
            prompt_input: "Cross-verify all 58 quantitative claims and direct quotes against primary PDFs and trade releases.".into(),
            tool_payload: "{\n  \"tool\": \"verify_claims\",\n  \"total_claims\": 58,\n  \"verified_so_far\": 49\n}".into(),
            output_preview: "49/58 claims verified; retrying remaining 9 after upstream rate limit.".into(),
            tasks: vec![
                AoTask::new("fc-1", "Extract 58 numerical assertions from draft", "cite", "Extractor", "58 claims indexed", AoAgentStatus::Done, 72_400, 4_200, 7_800),
                AoTask::new("fc-2", "Supervise source-check batch & backoff retry", "delegate", "Retry supervisor", "49/58 verified", AoAgentStatus::Running, 77_000, 18_000, 9_800),
            ],
            context: vec![AoContextItem::new("Claim_Audit_Log.json", "49/58 verified", ProIcon::Shield)],
        },
        AoAgent {
            id: "claim-check".into(),
            parent: Some("fact-checker".into()),
            name: "Claim check".into(),
            role: "Review".into(),
            task: "Check 58 claims against their sources".into(),
            model: "GPT-6 Mini".into(),
            maker: "openai".into(),
            tools: vec!["fetch".into(), "cite".into()],
            status: AoAgentStatus::Failed,
            start_ms: Some(73_000),
            end_ms: Some(81_500),
            tokens_in: 6_400,
            tokens_out: 1_100,
            tokens_cached: 2_000,
            cost: 0.03,
            ask: None,
            error: Some("HTTP 429 rate limit on ZIV archive endpoint after verifying 41 of 58 claims.".into()),
            retry_of: None,
            prompt_input: "Verify claims 1..58 against cached source snapshots and live archive endpoints.".into(),
            tool_payload: "{\n  \"tool\": \"verify_source_batch\",\n  \"batch_size\": 58,\n  \"failed_at\": 42\n}".into(),
            output_preview: "41 claims confirmed; aborted on claim #42 with HTTP 429 Too Many Requests.".into(),
            tasks: vec![
                AoTask::new("cc-1", "Verify claims 1–41 against primary filings", "cite", "Verifier", "41 matched verbatim", AoAgentStatus::Done, 73_200, 5_600, 5_400),
                AoTask::new("cc-2", "Fetch ZIV archive table for claims 42–58", "fetch", "Page reader", "429 Too Many Requests", AoAgentStatus::Failed, 79_000, 2_500, 2_100),
            ],
            context: vec![AoContextItem::new("Failed Batch #42–58", "17 claims remaining", ProIcon::AlertTriangle)],
        },
        AoAgent {
            id: "claim-check-retry".into(),
            parent: Some("fact-checker".into()),
            name: "Claim check · retry".into(),
            role: "Review · retry".into(),
            task: "Check the 17 claims that are left, with backoff".into(),
            model: "GPT-6 Mini".into(),
            maker: "openai".into(),
            tools: vec!["fetch".into(), "cite".into(), "code".into()],
            status: AoAgentStatus::Running,
            start_ms: Some(83_000),
            end_ms: None,
            tokens_in: 2_200,
            tokens_out: 500,
            tokens_cached: 1_100,
            cost: 0.01,
            ask: None,
            error: None,
            retry_of: Some("claim-check".into()),
            prompt_input: "Verify remaining claims 42..58 with 600ms exponential backoff and local PDF cache fallback.".into(),
            tool_payload: "{\n  \"tool\": \"verify_source_batch\",\n  \"claims\": \"42..58\",\n  \"backoff_ms\": 600\n}".into(),
            output_preview: "8 of 17 remaining claims verified (0 discrepancies found so far).".into(),
            tasks: vec![
                AoTask::new("ccr-1", "Verify claims 42–49 from cached PDFs", "cite", "Verifier", "8/17 verified", AoAgentStatus::Done, 83_400, 5_800, 1_600),
                AoTask::new("ccr-2", "Verify claims 50–58 with 600ms backoff", "fetch", "Page reader", "Checking claim 51/58…", AoAgentStatus::Running, 89_400, 5_600, 1_100),
            ],
            context: vec![AoContextItem::new("Retry Policy", "Exponential backoff · 600ms", ProIcon::Refresh)],
        },
        AoAgent {
            id: "reviewer".into(),
            parent: Some("orchestrator".into()),
            name: "Reviewer".into(),
            role: "Review".into(),
            task: "Read the draft end to end and sign it off".into(),
            model: "Opus 5.5".into(),
            maker: "claude".into(),
            tools: vec!["doc".into()],
            status: AoAgentStatus::Queued,
            start_ms: None,
            end_ms: None,
            tokens_in: 0,
            tokens_out: 0,
            tokens_cached: 0,
            cost: 0.0,
            ask: None,
            error: None,
            retry_of: None,
            prompt_input: "Perform final editorial and numerical consistency sign-off once Writer and Fact-checker complete.".into(),
            tool_payload: "{\n  \"status\": \"queued\",\n  \"depends_on\": [\"writer\", \"fact-checker\"]\n}".into(),
            output_preview: "Queued until all six report sections and 58 claim checks complete.".into(),
            tasks: vec![AoTask::new("rv-1", "End-to-end coherence & executive sign-off", "doc", "Editorial review", "Queued", AoAgentStatus::Queued, 0, 0, 0)],
            context: vec![AoContextItem::new("Sign-off Rubric.md", "Accuracy · tone · citations", ProIcon::Shield)],
        },
    ]
}

fn default_market_research_events() -> Vec<AoEvent> {
    vec![
        AoEvent::new(
            "ev-1",
            0,
            "orchestrator",
            "Orchestrator",
            AoEventKind::Start,
            "Started run_8f3a2c on Market research",
        ),
        AoEvent::new(
            "ev-2",
            1_200,
            "researcher",
            "Researcher",
            AoEventKind::Handoff,
            "Orchestrator delegated European OEM sales & pricing scan",
        ),
        AoEvent::new(
            "ev-3",
            12_800,
            "data-analyst",
            "Data analyst",
            AoEventKind::Handoff,
            "Orchestrator delegated 2025–2028 market sizing & IFRS margin model",
        ),
        AoEvent::new(
            "ev-4",
            13_300,
            "page-reader",
            "Page reader",
            AoEventKind::Done,
            "Extracted 94 unit figures from 18 primary trade bulletins",
        ),
        AoEvent::new(
            "ev-5",
            33_500,
            "filings-reader",
            "Filings reader",
            AoEventKind::Tool,
            "Parsed segment revenue & inventory days across 7 annual reports",
        ),
        AoEvent::new(
            "ev-6",
            42_200,
            "researcher",
            "Researcher",
            AoEventKind::Done,
            "Completed 12-maker unit matrix and 64-model retail price catalog",
        ),
        AoEvent::new(
            "ev-7",
            44_000,
            "writer",
            "Writer",
            AoEventKind::Handoff,
            "Orchestrator handed merged tables & 6 charts to Writer",
        ),
        AoEvent::new(
            "ev-8",
            70_300,
            "chart-builder",
            "Chart builder",
            AoEventKind::Done,
            "Rendered Figures 1–6 in publication vector format",
        ),
        AoEvent::new(
            "ev-9",
            71_800,
            "data-analyst",
            "Data analyst",
            AoEventKind::Done,
            "Sized 2025 EU market at €11.8B (7.6% CAGR through 2028)",
        ),
        AoEvent::new(
            "ev-10",
            75_000,
            "writer",
            "Writer",
            AoEventKind::Approval,
            "Waiting for approval: Publish draft to shared Research folder?",
        ),
        AoEvent::new(
            "ev-11",
            81_500,
            "claim-check",
            "Claim check",
            AoEventKind::Error,
            "HTTP 429 rate limit on ZIV archive endpoint at claim 42/58",
        ),
        AoEvent::new(
            "ev-12",
            83_000,
            "claim-check-retry",
            "Claim check · retry",
            AoEventKind::Retry,
            "Fact-checker spawned backoff retry for remaining 17 claims",
        ),
    ]
}

/// Interactive multi-agent orchestration canvas and execution trace screen.
pub struct AgentOrchestrationScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    focus: FocusHandle,
    standalone: bool,
    project: SharedString,
    run_id: SharedString,
    agents: Vec<AoAgent>,
    events: Vec<AoEvent>,
    base_elapsed_ms: u64,
    live: bool,
    paused: bool,
    stopped: bool,
    loading: bool,
    empty: bool,
    run_tab: AoRunTab,
    run_panel_folded: bool,
    run_panel_height: f32,
    inspector_tab: AoInspectorTab,
    selected_agent: Option<SharedString>,
    last_selected_agent: Option<SharedString>,
    focused_agent_idx: usize,
    expanded_trace_agents: HashSet<SharedString>,
    project_menu_open: bool,
    run_menu_open: bool,
    zoom: f32,
    pan_x: f32,
    pan_y: f32,
    pan_initialized: bool,
    dragging: Option<DragKind>,
    toast: Option<(SharedString, Instant)>,
    copied_key: Option<(SharedString, Instant)>,
    boot_instant: Instant,
    tick_origin: Option<Instant>,
    measured_width: Rc<Cell<f32>>,
    measured_height: Rc<Cell<f32>>,
}

impl AgentOrchestrationScreen {
    /// Construct an [`AgentOrchestrationScreen`] preloaded with the 17-agent market research run.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let focus = cx.focus_handle();
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let groups = vec![
                ShellNavGroup::primary(vec![
                    ShellNavItem::new(
                        "orchestrations",
                        tr(shell_cx, "Orchestrations", "编排流程"),
                        ProIcon::GitBranch,
                    ),
                    ShellNavItem::new("runs", tr(shell_cx, "Runs", "运行记录"), ProIcon::Play)
                        .with_count(3),
                    ShellNavItem::new("agents", tr(shell_cx, "Agents", "智能体"), ProIcon::Users),
                    ShellNavItem::new(
                        "traces",
                        tr(shell_cx, "Traces", "调用追踪"),
                        ProIcon::UsageChart,
                    ),
                    ShellNavItem::new(
                        "datasets",
                        tr(shell_cx, "Datasets", "数据集"),
                        ProIcon::Database,
                    ),
                ]),
                ShellNavGroup::named(
                    tr(shell_cx, "Workspace", "工作区"),
                    vec![
                        ShellNavItem::new(
                            "knowledge",
                            tr(shell_cx, "Knowledge", "知识库"),
                            ProIcon::PageDoc,
                        ),
                        ShellNavItem::new(
                            "settings",
                            tr(shell_cx, "Settings", "设置"),
                            ProIcon::Settings,
                        ),
                    ],
                ),
            ];
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_workspace("Northwind")
                .with_user("Alex Morgan", "alex@northwind.ai")
                .with_side_cards(false)
                .with_groups(groups)
                .with_active("orchestrations")
                .with_work_area(false, false);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s
        });

        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| {
            if let AppShellEvent::Navigate(label) = ev {
                cx.emit(AgentOrchestrationEvent::Navigate(label.clone()));
            }
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            shell,
            focus,
            standalone: false,
            project: "Market research".into(),
            run_id: "run_8f3a2c".into(),
            agents: default_market_research_agents(),
            events: default_market_research_events(),
            base_elapsed_ms: 95_000,
            live: true,
            paused: false,
            stopped: false,
            loading: false,
            empty: false,
            run_tab: AoRunTab::Trace,
            run_panel_folded: false,
            run_panel_height: 224.0,
            inspector_tab: AoInspectorTab::Tasks,
            selected_agent: None,
            last_selected_agent: None,
            focused_agent_idx: 0,
            expanded_trace_agents: HashSet::new(),
            project_menu_open: false,
            run_menu_open: false,
            zoom: 0.70,
            pan_x: 24.0,
            pan_y: 42.0,
            pan_initialized: false,
            dragging: None,
            toast: None,
            copied_key: None,
            boot_instant: Instant::now(),
            tick_origin: None,
            measured_width: Rc::new(Cell::new(920.)),
            measured_height: Rc::new(Cell::new(620.)),
        }
    }

    /// Render without the surrounding [`AppShell`] frame.
    pub fn standalone(mut self) -> Self {
        self.standalone = true;
        self
    }

    /// Set the project breadcrumb label.
    pub fn with_project(mut self, project: impl Into<SharedString>) -> Self {
        self.project = project.into();
        self
    }

    /// Set the active run identifier in the path bar.
    pub fn with_run_id(mut self, run_id: impl Into<SharedString>) -> Self {
        self.run_id = run_id.into();
        self
    }

    /// Supply a custom list of orchestrated agents.
    pub fn with_agents(mut self, agents: Vec<AoAgent>) -> Self {
        self.agents = agents;
        self.focused_agent_idx = 0;
        self.selected_agent = None;
        self.last_selected_agent = None;
        self.pan_initialized = false;
        self
    }

    /// Supply custom orchestration events.
    pub fn with_events(mut self, events: Vec<AoEvent>) -> Self {
        self.events = events;
        self
    }

    /// Set initial elapsed run time in milliseconds.
    pub fn with_elapsed_ms(mut self, elapsed_ms: u64) -> Self {
        self.base_elapsed_ms = elapsed_ms;
        self
    }

    /// Set whether the clock opens running (`true`) or paused (`false`).
    pub fn with_live(mut self, live: bool) -> Self {
        self.live = live;
        self.paused = !live;
        self
    }

    /// Choose the initial bottom panel tab (`Trace`, `TokensAndCost`, or `Events`).
    pub fn with_default_tab(mut self, tab: AoRunTab) -> Self {
        self.run_tab = tab;
        self
    }

    /// Open a specific agent's right-hand inspector initially.
    pub fn with_selected_agent(mut self, agent_id: Option<SharedString>) -> Self {
        if agent_id.is_some() {
            self.last_selected_agent = agent_id.clone();
        }
        self.selected_agent = agent_id;
        self
    }

    /// Configure skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure empty state (`Start the run`).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Toggle skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        cx.notify();
    }

    /// Return the currently open agent id in the right inspector, if any.
    pub fn selected_agent_id(&self) -> Option<&SharedString> {
        self.selected_agent.as_ref()
    }

    fn is_flowing(&self) -> bool {
        self.live && !self.paused && !self.stopped && !self.loading && !self.empty
    }

    fn current_elapsed_ms(&self) -> u64 {
        if self.empty {
            return 0;
        }
        if let Some(origin) = self.tick_origin
            && self.is_flowing()
        {
            return self
                .base_elapsed_ms
                .saturating_add(origin.elapsed().as_millis() as u64);
        }
        self.base_elapsed_ms
    }

    fn push_toast(&mut self, msg: impl Into<SharedString>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    fn open_agent(&mut self, agent_id: SharedString, cx: &mut Context<Self>) {
        let layout = layout_agent_tree(&self.agents).0;
        if let Some(pos_idx) = layout
            .iter()
            .position(|p| self.agents[p.index].id == agent_id)
        {
            self.focused_agent_idx = pos_idx;
            // Ensure selected card stays clear of the 388px right inspector
            let node = &layout[pos_idx];
            let vw = self.measured_width.get();
            let visible_w = if vw >= 640. {
                (vw - 408.0).max(240.0)
            } else {
                vw
            };
            let card_right = self.pan_x + (node.x + CARD_W) * self.zoom;
            let card_left = self.pan_x + node.x * self.zoom;
            if card_right > visible_w - 16.0 {
                self.pan_x -= card_right - (visible_w - 28.0);
            }
            if card_left < 16.0 {
                self.pan_x += 24.0 - card_left;
            }
        }
        self.selected_agent = Some(agent_id.clone());
        self.last_selected_agent = Some(agent_id.clone());
        cx.emit(AgentOrchestrationEvent::SelectAgent(Some(agent_id)));
        cx.notify();
    }

    fn close_agent(&mut self, cx: &mut Context<Self>) {
        if self.selected_agent.take().is_some() {
            cx.emit(AgentOrchestrationEvent::SelectAgent(None));
            cx.notify();
        }
    }

    fn resolve_waiting_agent(&mut self, agent_id: &str, approve: bool, cx: &mut Context<Self>) {
        let now_ms = self.current_elapsed_ms();
        let mut agent_name = SharedString::from("Agent");
        for a in &mut self.agents {
            if a.id.as_ref() == agent_id {
                agent_name = a.name.clone();
                if approve {
                    a.status = AoAgentStatus::Running;
                    a.ask = None;
                    for t in &mut a.tasks {
                        if t.status == AoAgentStatus::Waiting {
                            t.status = AoAgentStatus::Done;
                            t.result = "Approved by operator".into();
                        }
                    }
                } else {
                    a.status = AoAgentStatus::Stopped;
                    a.end_ms = Some(now_ms);
                    a.ask = None;
                }
            } else if a.parent.as_deref() == Some(agent_id) && a.status == AoAgentStatus::Queued {
                if approve {
                    a.status = AoAgentStatus::Running;
                    a.start_ms = Some(now_ms);
                } else {
                    a.status = AoAgentStatus::Stopped;
                }
            }
        }
        let text = if approve {
            format!("Approved {agent_name} — resumed downstream drafter")
        } else {
            format!("Rejected {agent_name} — stopped branch")
        };
        self.events.push(AoEvent::new(
            format!("ev-res-{}", self.events.len() + 1),
            now_ms,
            agent_id.to_string(),
            agent_name,
            if approve {
                AoEventKind::Handoff
            } else {
                AoEventKind::Done
            },
            text.clone(),
        ));
        self.push_toast(text);
        cx.emit(AgentOrchestrationEvent::Resolve {
            agent_id: agent_id.to_string().into(),
            approved: approve,
        });
        cx.notify();
    }

    fn handle_run_action(&mut self, action: AoRunAction, cx: &mut Context<Self>) {
        match action {
            AoRunAction::Start => {
                self.empty = false;
                self.stopped = false;
                self.paused = false;
                self.live = true;
                self.base_elapsed_ms = 95_000;
                self.tick_origin = Some(Instant::now());
            }
            AoRunAction::Pause => {
                self.base_elapsed_ms = self.current_elapsed_ms();
                self.tick_origin = None;
                self.paused = true;
            }
            AoRunAction::Resume => {
                self.paused = false;
                self.live = true;
                self.tick_origin = Some(Instant::now());
            }
            AoRunAction::Stop => {
                let now = self.current_elapsed_ms();
                self.base_elapsed_ms = now;
                self.tick_origin = None;
                self.stopped = true;
                self.live = false;
                for a in &mut self.agents {
                    if matches!(
                        a.status,
                        AoAgentStatus::Running | AoAgentStatus::Waiting | AoAgentStatus::Queued
                    ) {
                        a.status = AoAgentStatus::Stopped;
                        if a.start_ms.is_some() && a.end_ms.is_none() {
                            a.end_ms = Some(now);
                        }
                    }
                }
                self.push_toast(tr(cx, "Run stopped", "已停止运行"));
            }
            AoRunAction::Restart => {
                self.agents = default_market_research_agents();
                self.events = default_market_research_events();
                self.stopped = false;
                self.paused = false;
                self.live = true;
                self.base_elapsed_ms = 95_000;
                self.tick_origin = Some(Instant::now());
                self.push_toast(tr(cx, "Replaying orchestration", "正在重新运行编排"));
            }
        }
        cx.emit(AgentOrchestrationEvent::RunControl(action));
        cx.notify();
    }

    fn fit_to_view(&mut self, cx: &mut Context<Self>) {
        let (_, world_w, world_h) = layout_agent_tree(&self.agents);
        let vw = self.measured_width.get();
        let panel_offset = if self.selected_agent.is_some() && vw >= 640. {
            398.0
        } else {
            0.0
        };
        let avail_w = (vw - panel_offset - 48.0).max(220.0);
        let run_h = if self.run_panel_folded {
            44.0
        } else {
            self.run_panel_height
        };
        let avail_h = (self.measured_height.get() - run_h - 88.0).max(180.0);
        let fit_z = (avail_w / world_w.max(1.0))
            .min(avail_h / world_h.max(1.0))
            .clamp(0.45, 1.0);
        self.zoom = (fit_z * 20.0).round() / 20.0;
        self.pan_x = ((avail_w - world_w * self.zoom) * 0.5 + 20.0).max(16.0);
        self.pan_y = 44.0;
        cx.notify();
    }

    fn jump_next_running(&mut self, cx: &mut Context<Self>) {
        let (ordered, _, _) = layout_agent_tree(&self.agents);
        if ordered.is_empty() {
            return;
        }
        let len = ordered.len();
        for offset in 1..=len {
            let cand = (self.focused_agent_idx + offset) % len;
            let ag = &self.agents[ordered[cand].index];
            if matches!(ag.status, AoAgentStatus::Running | AoAgentStatus::Waiting) {
                self.focused_agent_idx = cand;
                let node = &ordered[cand];
                let vw = self.measured_width.get();
                let run_h = if self.run_panel_folded {
                    44.0
                } else {
                    self.run_panel_height
                };
                let vh = (self.measured_height.get() - run_h).max(220.0);
                self.pan_x = vw * 0.5 - (node.x + CARD_W * 0.5) * self.zoom;
                self.pan_y = (vh * 0.45 - (node.y + CARD_H * 0.5) * self.zoom).max(36.0);
                self.open_agent(ag.id.clone(), cx);
                return;
            }
        }
    }

    fn render_maker_mark(
        maker: &str,
        seed: &str,
        side: f32,
        working: bool,
        anim_ms: f32,
        p: &ProPalette,
        cx: &App,
    ) -> AnyElement {
        let radius = (side * 0.28).round().clamp(5., 8.);
        let icon_side = (side * 0.56).round().clamp(11., 16.);
        if let Some(art) = pro_app_artwork(maker, icon_side, cx) {
            return div()
                .size(px(side))
                .flex_none()
                .rounded(px(radius))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_center()
                .child(art)
                .into_any_element();
        }
        pro_agent_avatar(seed, p.blue, side, working, anim_ms, cx)
    }

    fn render_status_chip(
        status: AoAgentStatus,
        flowing: bool,
        anim_ms: f32,
        scale: f32,
        p: &ProPalette,
        cx: &App,
    ) -> Div {
        let (bg, fg, word): (Hsla, Hsla, &str) = match status {
            AoAgentStatus::Running => (
                if p.dark {
                    hsla(212. / 360., 1.0, 0.50, 0.23)
                } else {
                    hsla(205. / 360., 1.0, 0.48, 0.10)
                },
                p.blue,
                tr(cx, "Running", "运行中"),
            ),
            AoAgentStatus::Waiting => (p.amber_soft, p.amber_ink, tr(cx, "Needs you", "待确认")),
            AoAgentStatus::Done => (p.green_soft, p.green_ink, tr(cx, "Done", "已完成")),
            AoAgentStatus::Failed => (p.red_soft, p.red_ink, tr(cx, "Failed", "失败")),
            AoAgentStatus::Queued => (p.chip, p.muted, tr(cx, "Queued", "排队中")),
            AoAgentStatus::Stopped => (p.chip, p.muted, tr(cx, "Stopped", "已停止")),
        };

        let h = (20.0 * scale).clamp(15.0, 20.0);
        let fs = (11.0 * scale).clamp(9.0, 11.0);
        let ic_side = (11.0 * scale).clamp(8.5, 11.0);

        let glyph: AnyElement = match status {
            AoAgentStatus::Running => {
                let spin_pct = if flowing {
                    0.28 + 0.45 * ((anim_ms / 900.0).sin().abs())
                } else {
                    0.35
                };
                pro_progress_ring(spin_pct, ic_side, 1.6, fg, fg.opacity(0.22))
            }
            AoAgentStatus::Waiting => pro_icon(ProIcon::Clock, ic_side, fg),
            AoAgentStatus::Done => pro_icon(ProIcon::Check, ic_side, fg),
            AoAgentStatus::Failed => pro_icon(ProIcon::AlertTriangle, ic_side, fg),
            AoAgentStatus::Queued => pro_svg("0 0 24 24", DASHED_RING_SVG, ic_side, fg),
            AoAgentStatus::Stopped => pro_svg("0 0 24 24", STOP_SQUARE_SVG, ic_side, fg),
        };

        div()
            .h(px(h))
            .px(px((5.5 * scale).clamp(4.0, 6.0)))
            .flex_none()
            .rounded(px((5.0 * scale).clamp(4.0, 5.0)))
            .bg(bg)
            .text_color(fg)
            .text_size(px(fs))
            .font_weight(FontWeight::MEDIUM)
            .flex()
            .items_center()
            .gap(px((4.0 * scale).clamp(2.5, 4.0)))
            .whitespace_nowrap()
            .child(glyph)
            .child(word)
    }

    fn is_in_selected_lineage(&self, agent_idx: usize) -> bool {
        let Some(ref sel_id) = self.selected_agent else {
            return true;
        };
        let target = &self.agents[agent_idx];
        if &target.id == sel_id {
            return true;
        }
        // Check if `target` is an ancestor of `sel_id` or a descendant of `sel_id`
        let mut cur = target.parent.as_deref();
        while let Some(pid) = cur {
            if pid == sel_id.as_ref() {
                return true;
            }
            cur = self
                .agents
                .iter()
                .find(|a| a.id.as_ref() == pid)
                .and_then(|a| a.parent.as_deref());
        }
        let mut sel_anc = self
            .agents
            .iter()
            .find(|a| &a.id == sel_id)
            .and_then(|a| a.parent.as_deref());
        while let Some(pid) = sel_anc {
            if pid == target.id.as_ref() {
                return true;
            }
            sel_anc = self
                .agents
                .iter()
                .find(|a| a.id.as_ref() == pid)
                .and_then(|a| a.parent.as_deref());
        }
        false
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn render_canvas_area(
        &mut self,
        ordered: &[PositionedAgent],
        world_w: f32,
        now_ms: u64,
        anim_ms: f32,
        skel_phase: f32,
        narrow: bool,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let reduced = Theme::global(cx).reduced_motion;
        let flowing = self.is_flowing();
        let panel_open = self.selected_agent.is_some() && !self.loading && !self.empty;

        // Center initial pan once container width is measured
        let vw = self.measured_width.get();
        if !self.pan_initialized && vw > 100.0 && !ordered.is_empty() {
            self.pan_x = ((vw - world_w * self.zoom) * 0.5).max(18.0);
            self.pan_y = 42.0;
            self.pan_initialized = true;
        }

        // Build connector polylines + junction dots in world space
        let mut links: Vec<ConnectorLink> = Vec::new();
        let mut dots: Vec<(f32, f32, bool)> = Vec::new();

        for parent_pos in ordered {
            let parent_ag = &self.agents[parent_pos.index];
            let child_positions: Vec<&PositionedAgent> = ordered
                .iter()
                .filter(|c| self.agents[c.index].parent.as_ref() == Some(&parent_ag.id))
                .collect();
            if child_positions.is_empty() {
                continue;
            }

            if parent_pos.depth == 0 {
                let sx = parent_pos.x + CARD_W * 0.5;
                let sy = parent_pos.y + CARD_H;
                let mid_y = 131.0;
                dots.push((sx, sy, !self.is_in_selected_lineage(parent_pos.index)));
                for ch_pos in child_positions {
                    let ch_ag = &self.agents[ch_pos.index];
                    let tx = ch_pos.x + CARD_W * 0.5;
                    let ty = ch_pos.y;
                    let muted =
                        matches!(ch_ag.status, AoAgentStatus::Queued | AoAgentStatus::Stopped);
                    let is_flow = ch_ag.status == AoAgentStatus::Running && flowing && !reduced;
                    let faded = panel_open && !self.is_in_selected_lineage(ch_pos.index);
                    links.push(ConnectorLink {
                        points: vec![(sx, sy), (sx, mid_y), (tx, mid_y), (tx, ty)],
                        muted,
                        flowing: is_flow,
                        faded,
                    });
                }
            } else {
                let sx = parent_pos.x + 18.0;
                let sy = parent_pos.y + CARD_H;
                dots.push((sx, sy, !self.is_in_selected_lineage(parent_pos.index)));
                for ch_pos in child_positions {
                    let ch_ag = &self.agents[ch_pos.index];
                    let tx = ch_pos.x;
                    let ty = ch_pos.y + 24.0;
                    let muted =
                        matches!(ch_ag.status, AoAgentStatus::Queued | AoAgentStatus::Stopped);
                    let is_flow = ch_ag.status == AoAgentStatus::Running && flowing && !reduced;
                    let faded = panel_open && !self.is_in_selected_lineage(ch_pos.index);
                    links.push(ConnectorLink {
                        points: vec![(sx, sy), (sx, ty), (tx, ty)],
                        muted,
                        flowing: is_flow,
                        faded,
                    });
                }
            }
        }

        let zoom = self.zoom;
        let pan_x = self.pan_x;
        let pan_y = self.pan_y;
        let dot_color = if p.dark {
            hsla(0., 0., 1., 0.11)
        } else {
            hsla(0., 0., 0., 0.14)
        };
        let link_rest_c = if p.dark {
            hsla(0., 0., 1., 0.17)
        } else {
            hsla(0., 0., 0., 0.16)
        };
        let link_flow_base_c: Hsla = if p.dark {
            p.blue.opacity(0.42)
        } else {
            rgb(0x83c7fe).into()
        };
        let link_muted_c = if p.dark {
            hsla(0., 0., 1., 0.09)
        } else {
            hsla(0., 0., 0., 0.09)
        };
        let flow_c: Hsla = if p.dark {
            rgb(0x90cdfe).into()
        } else {
            rgb(0x0070d6).into()
        };
        let card_fill = p.card;
        let is_loading = self.loading;
        let is_empty = self.empty;
        let flow_phase = (anim_ms / 1350.0).fract();

        let backdrop_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let left = f32::from(bounds.left());
                let top = f32::from(bounds.top());
                let w = f32::from(bounds.size.width);
                let h = f32::from(bounds.size.height);
                if w <= 4. || h <= 4. {
                    return;
                }

                // 1. Paint panned/zoomed dot-grid background
                let step = (20.0 * zoom).clamp(12.0, 32.0);
                let off_x = pan_x.rem_euclid(step);
                let off_y = pan_y.rem_euclid(step);
                let mut gx = off_x;
                while gx < w {
                    let mut gy = off_y;
                    while gy < h {
                        window.paint_quad(fill(
                            Bounds::new(point(px(left + gx), px(top + gy)), size(px(1.5), px(1.5))),
                            dot_color,
                        ));
                        gy += step;
                    }
                    gx += step;
                }

                if is_empty {
                    return;
                }

                let to_screen = |wx: f32, wy: f32| -> Point<Pixels> {
                    point(px(left + pan_x + wx * zoom), px(top + pan_y + wy * zoom))
                };

                // 2. Paint rounded orthogonal connector links
                let corner_r = 9.0 * zoom;
                for lnk in &links {
                    let stroke_c = if is_loading || lnk.muted {
                        link_muted_c
                    } else if lnk.flowing {
                        link_flow_base_c
                    } else {
                        link_rest_c
                    };
                    let final_c = if lnk.faded && !is_loading {
                        stroke_c.opacity(stroke_c.a * 0.35)
                    } else {
                        stroke_c
                    };

                    let pts: Vec<(f32, f32)> = lnk
                        .points
                        .iter()
                        .map(|&(wx, wy)| (left + pan_x + wx * zoom, top + pan_y + wy * zoom))
                        .collect();
                    if pts.len() < 2 {
                        continue;
                    }

                    let mut pb = PathBuilder::stroke(px(1.5));
                    pb.move_to(point(px(pts[0].0), px(pts[0].1)));
                    for win in pts.windows(3) {
                        let [(px0, py0), (px1, py1), (px2, py2)] = [win[0], win[1], win[2]];
                        let d1 = ((px1 - px0).hypot(py1 - py0)).max(0.001);
                        let d2 = ((px2 - px1).hypot(py2 - py1)).max(0.001);
                        let r = corner_r.min(d1 * 0.45).min(d2 * 0.45);
                        if r > 0.8 {
                            let in_x = px1 - (px1 - px0) / d1 * r;
                            let in_y = py1 - (py1 - py0) / d1 * r;
                            let out_x = px1 + (px2 - px1) / d2 * r;
                            let out_y = py1 + (py2 - py1) / d2 * r;
                            pb.line_to(point(px(in_x), px(in_y)));
                            pb.curve_to(point(px(out_x), px(out_y)), point(px(px1), px(py1)));
                        } else {
                            pb.line_to(point(px(px1), px(py1)));
                        }
                    }
                    if let Some(&(lx, ly)) = pts.last() {
                        pb.line_to(point(px(lx), px(ly)));
                    }
                    if let Ok(path) = pb.build() {
                        window.paint_path(path, final_c);
                    }

                    // Animated flow pulse traveling toward running agents
                    if lnk.flowing && !is_loading {
                        let mut seg_lens = Vec::with_capacity(pts.len() - 1);
                        let mut total_len = 0.0_f32;
                        for w_pair in pts.windows(2) {
                            let l = (w_pair[1].0 - w_pair[0].0).hypot(w_pair[1].1 - w_pair[0].1);
                            seg_lens.push(l);
                            total_len += l;
                        }
                        if total_len > 6.0 {
                            let sample_at = |dist: f32| -> Point<Pixels> {
                                let mut rem = dist.clamp(0.0, total_len);
                                for (si, &sl) in seg_lens.iter().enumerate() {
                                    if rem <= sl || si + 1 == seg_lens.len() {
                                        let t = if sl > 0.001 {
                                            (rem / sl).clamp(0.0, 1.0)
                                        } else {
                                            0.0
                                        };
                                        let x = pts[si].0 + (pts[si + 1].0 - pts[si].0) * t;
                                        let y = pts[si].1 + (pts[si + 1].1 - pts[si].1) * t;
                                        return point(px(x), px(y));
                                    }
                                    rem -= sl;
                                }
                                point(px(pts[0].0), px(pts[0].1))
                            };
                            let pulse_len = (total_len * 0.34).clamp(22.0, 72.0);
                            let head_d =
                                -pulse_len * 0.2 + (total_len + pulse_len * 1.2) * flow_phase;
                            let tail_d = head_d - pulse_len;
                            let slices = 6usize;
                            for k in 0..slices {
                                let d0 = tail_d + (k as f32 / slices as f32) * pulse_len;
                                let d1 = tail_d + ((k + 1) as f32 / slices as f32) * pulse_len;
                                if d1 <= 0.0 || d0 >= total_len {
                                    continue;
                                }
                                let alpha = ((k + 1) as f32 / slices as f32).powi(2) * 0.92;
                                let mut fpb = PathBuilder::stroke(px(2.2));
                                fpb.move_to(sample_at(d0));
                                fpb.line_to(sample_at(d1));
                                if let Ok(fpath) = fpb.build() {
                                    window.paint_path(fpath, flow_c.opacity(alpha));
                                }
                            }
                        }
                    }
                }

                // 3. Paint junction circles
                let dot_r = (3.5 * zoom).clamp(2.5, 4.0);
                let inner_r = (dot_r - 1.5).max(1.0);
                for &(dx, dy, faded) in &dots {
                    let center = to_screen(dx, dy);
                    let stroke_c = if is_loading {
                        link_muted_c
                    } else if faded {
                        link_rest_c.opacity(link_rest_c.a * 0.35)
                    } else {
                        link_rest_c
                    };
                    window.paint_quad(
                        fill(
                            Bounds::new(
                                point(center.x - px(dot_r), center.y - px(dot_r)),
                                size(px(dot_r * 2.0), px(dot_r * 2.0)),
                            ),
                            stroke_c,
                        )
                        .corner_radii(px(dot_r)),
                    );
                    window.paint_quad(
                        fill(
                            Bounds::new(
                                point(center.x - px(inner_r), center.y - px(inner_r)),
                                size(px(inner_r * 2.0), px(inner_r * 2.0)),
                            ),
                            card_fill,
                        )
                        .corner_radii(px(inner_r)),
                    );
                }
            },
        )
        .size_full();

        let is_panning = matches!(self.dragging, Some(DragKind::CanvasPan { .. }));
        let mut viewport = div()
            .id(pro_eid(&self.id, "canvas-vp"))
            .relative()
            .size_full()
            .bg(p.panel)
            .overflow_hidden()
            .cursor(if is_panning {
                CursorStyle::ClosedHand
            } else {
                CursorStyle::OpenHand
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, _, cx| {
                    this.project_menu_open = false;
                    this.run_menu_open = false;
                    this.dragging = Some(DragKind::CanvasPan {
                        start_mouse_x: f32::from(ev.position.x) as i32,
                        start_mouse_y: f32::from(ev.position.y) as i32,
                        orig_pan_x: this.pan_x,
                        orig_pan_y: this.pan_y,
                    });
                    cx.notify();
                }),
            )
            .on_scroll(cx.listener(|this, ev: &ScrollEvent, _, cx| {
                let delta = ev.delta.pixel_delta(px(18.));
                if ev.modifiers.platform || ev.modifiers.control {
                    let dy = f32::from(delta.y);
                    let next_z = (this.zoom + dy * 0.002).clamp(0.45, 1.25);
                    this.zoom = (next_z * 100.0).round() / 100.0;
                } else {
                    this.pan_x += f32::from(delta.x);
                    this.pan_y += f32::from(delta.y);
                }
                cx.notify();
            }))
            .child(backdrop_canvas);

        // Render empty state or positioned agent cards
        if self.empty && !self.loading {
            let start_lbl: SharedString = tr(cx, "Start run", "启动运行").into();
            viewport = viewport.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .p(px(24.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(12.))
                            .child(
                                div()
                                    .size(px(40.))
                                    .rounded(px(10.))
                                    .bg(p.chip)
                                    .border_1()
                                    .border_color(p.line)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(pro_icon(ProIcon::GitBranch, 18., p.muted)),
                            )
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(tr(cx, "Start the run", "启动多智能体编排")),
                            )
                            .child(
                                div()
                                    .max_w(px(300.))
                                    .text_size(px(12.5))
                                    .line_height(px(18.))
                                    .text_color(p.muted)
                                    .text_center()
                                    .child(tr(
                                        cx,
                                        "Watch the orchestrator delegate research, analysis, writing, and fact-checking live.",
                                        "实时观察主控智能体分发调研、建模分析、报告撰写与事实核查任务。",
                                    )),
                            )
                            .child(pro_btn(
                                pro_eid(&self.id, "empty-start-btn"),
                                start_lbl,
                                Some(ProIcon::Play),
                                ProBtnStyle::Primary,
                                true,
                                cx,
                                |this, _, cx| this.handle_run_action(AoRunAction::Start, cx),
                            )),
                    ),
            );
        } else {
            let well_bg = if p.dark {
                hsla(0., 0., 1., 0.04)
            } else {
                hsla(0., 0., 0., 0.028)
            };

            for (ord_i, pos) in ordered.iter().enumerate() {
                let ag = &self.agents[pos.index];
                let sx = pan_x + pos.x * zoom;
                let sy = pan_y + pos.y * zoom;
                let cw = CARD_W * zoom;
                let ch = CARD_H * zoom;
                let role_h = (16.0 * zoom).clamp(13.0, 16.0);
                let role_gap = (5.0 * zoom).clamp(3.0, 6.0);

                if self.loading {
                    viewport = viewport.child(
                        div()
                            .absolute()
                            .left(px(sx))
                            .top(px(sy - role_h - role_gap))
                            .w(px(cw))
                            .flex()
                            .flex_col()
                            .gap(px(role_gap))
                            .child(pro_skeleton_bar(
                                (54.0 * zoom).max(36.0),
                                role_h * 0.78,
                                4.,
                                skel_phase,
                                cx,
                            ))
                            .child(
                                div()
                                    .w(px(cw))
                                    .h(px(ch))
                                    .p(px((10.0 * zoom).clamp(7.0, 12.0)))
                                    .rounded(px((10.0 * zoom).clamp(7.0, 10.0)))
                                    .bg(p.card)
                                    .border_1()
                                    .border_color(p.line)
                                    .flex()
                                    .flex_col()
                                    .justify_between()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.0 * zoom))
                                            .child(pro_skeleton_bar(
                                                (22.0 * zoom).max(15.0),
                                                (22.0 * zoom).max(15.0),
                                                5.,
                                                skel_phase,
                                                cx,
                                            ))
                                            .child(pro_skeleton_bar(
                                                (86.0 * zoom).max(52.0),
                                                (11.0 * zoom).max(8.0),
                                                4.,
                                                skel_phase,
                                                cx,
                                            )),
                                    )
                                    .child(pro_skeleton_bar(
                                        (160.0 * zoom).max(96.0),
                                        (9.0 * zoom).max(7.0),
                                        3.,
                                        skel_phase,
                                        cx,
                                    ))
                                    .child(pro_skeleton_bar(
                                        cw - (20.0 * zoom).clamp(14.0, 24.0),
                                        (28.0 * zoom).max(19.0),
                                        6.,
                                        skel_phase,
                                        cx,
                                    )),
                            ),
                    );
                    continue;
                }

                let is_selected = self.selected_agent.as_ref() == Some(&ag.id);
                let in_lineage = self.is_in_selected_lineage(pos.index);
                let card_alpha = if panel_open && !in_lineage {
                    if matches!(ag.status, AoAgentStatus::Queued | AoAgentStatus::Stopped) {
                        0.28
                    } else {
                        0.38
                    }
                } else if matches!(ag.status, AoAgentStatus::Queued | AoAgentStatus::Stopped) {
                    0.62
                } else {
                    1.0
                };

                let b_cost = branch_cost(&self.agents, &ag.id);
                let dur_opt = ag.start_ms.map(|st| {
                    let end = ag.end_ms.unwrap_or(now_ms).max(st);
                    format_duration_ms(end - st)
                });

                // Tool tiles stack
                let max_tools = 3usize;
                let mut tool_row = div().flex().items_center();
                for (ti, tid) in ag.tools.iter().take(max_tools).enumerate() {
                    let t_side = (19.0 * zoom).clamp(14.0, 20.0);
                    tool_row = tool_row.child(
                        div()
                            .size(px(t_side))
                            .when(ti > 0, |d| d.ml(px(-4.0 * zoom)))
                            .rounded(px((5.0 * zoom).clamp(3.5, 5.0)))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_icon(
                                tool_icon(tid),
                                (10.5 * zoom).clamp(8.0, 11.0),
                                p.text,
                            )),
                    );
                }

                let cost_time_el = if b_cost > 0.001 {
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(3.0 * zoom))
                        .text_size(px((11.5 * zoom).clamp(9.0, 12.0)))
                        .whitespace_nowrap()
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(format!("${b_cost:.2}")),
                        )
                        .when_some(dur_opt, |d, ds| {
                            d.child(div().text_color(p.muted).child(format!("· {ds}")))
                        })
                } else {
                    div().flex_none()
                };

                let ag_id = ag.id.clone();
                let aria = format!("{}, {}, {:?}", ag.name, ag.role, ag.status);
                let border_c = if is_selected {
                    p.blue
                } else if ord_i == self.focused_agent_idx {
                    p.line_strong
                } else {
                    p.line
                };

                let card_inner = activate(
                    div()
                        .id(pro_eid(&self.id, format!("card-{}", ag.id)))
                        .role(Role::Button)
                        .aria_label(aria)
                        .tab_index(if ord_i == 0 { 0 } else { -1 })
                        .w(px(cw))
                        .h(px(ch))
                        .px(px((11.0 * zoom).clamp(8.0, 12.0)))
                        .pt(px((10.0 * zoom).clamp(7.0, 11.0)))
                        .pb(px((5.0 * zoom).clamp(3.5, 5.0)))
                        .rounded(px((10.0 * zoom).clamp(7.5, 10.0)))
                        .bg(p.card)
                        .border_1()
                        .border_color(border_c)
                        .hover(move |s| s.border_color(p.line_strong))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .flex_col()
                        .justify_between()
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(
                            div()
                                .w_full()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px((7.0 * zoom).clamp(4.5, 8.0)))
                                .child(Self::render_maker_mark(
                                    &ag.maker,
                                    &ag.id,
                                    (23.0 * zoom).clamp(16.0, 24.0),
                                    ag.status == AoAgentStatus::Running && flowing,
                                    anim_ms,
                                    p,
                                    cx,
                                ))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_size(px((13.0 * zoom).clamp(9.5, 13.0)))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .truncate()
                                        .child(ag.name.clone()),
                                )
                                .child(Self::render_status_chip(
                                    ag.status, flowing, anim_ms, zoom, p, cx,
                                )),
                        )
                        .child(
                            div()
                                .w_full()
                                .min_w_0()
                                .text_size(px((12.0 * zoom).clamp(9.0, 12.5)))
                                .text_color(p.text)
                                .truncate()
                                .child(ag.task.clone()),
                        )
                        .child(
                            div()
                                .w_full()
                                .h(px((33.0 * zoom).clamp(22.0, 35.0)))
                                .px(px((7.0 * zoom).clamp(5.0, 8.0)))
                                .rounded(px((7.0 * zoom).clamp(5.0, 7.0)))
                                .bg(well_bg)
                                .flex()
                                .items_center()
                                .gap(px((6.0 * zoom).clamp(4.0, 8.0)))
                                .child(tool_row)
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_size(px((11.5 * zoom).clamp(8.5, 12.0)))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(ag.model.clone()),
                                )
                                .child(cost_time_el),
                        ),
                    cx,
                    move |this, _, cx| {
                        this.open_agent(ag_id.clone(), cx);
                    },
                );

                let role_pill = div()
                    .h(px(role_h))
                    .px(px((5.5 * zoom).clamp(4.0, 6.0)))
                    .rounded(px(4.5))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .text_size(px((9.5 * zoom).clamp(8.0, 10.0)))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .whitespace_nowrap()
                    .child(ag.role.to_ascii_uppercase());

                viewport = viewport.child(
                    div()
                        .absolute()
                        .left(px(sx))
                        .top(px(sy - role_h - role_gap))
                        .opacity(card_alpha)
                        .flex()
                        .flex_col()
                        .items_start()
                        .gap(px(role_gap))
                        .child(role_pill)
                        .child(card_inner),
                );
            }
        }

        // Top floating path bar & run controls
        let top_right_inset = if panel_open && !narrow { 406.0 } else { 10.0 };
        let env_badge = if self.empty {
            tr(cx, "READY", "就绪")
        } else if self.stopped {
            tr(cx, "STOPPED", "已停止")
        } else if self.paused {
            tr(cx, "PAUSED", "已暂停")
        } else {
            tr(cx, "LIVE", "实时")
        };

        let path_bar = div()
            .h(px(28.))
            .rounded(px(7.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "crumb-project"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .cursor_pointer()
                    .child(self.project.clone())
                    .child(pro_icon(ProIcon::ChevronDown, 12., p.faint)),
                cx,
                |this, _, cx| {
                    this.project_menu_open = !this.project_menu_open;
                    this.run_menu_open = false;
                    cx.notify();
                },
            ))
            .child(
                div()
                    .px(px(2.))
                    .text_size(px(13.))
                    .text_color(p.faint)
                    .child("/"),
            )
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "crumb-run"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .cursor_pointer()
                    .child(
                        div()
                            .font_family("Menlo")
                            .text_size(px(12.))
                            .text_color(p.ink)
                            .child(self.run_id.clone()),
                    )
                    .when(!narrow, |d| {
                        d.child(
                            div()
                                .h(px(16.))
                                .px(px(5.))
                                .rounded(px(4.))
                                .bg(p.chip)
                                .flex()
                                .items_center()
                                .text_size(px(10.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .child(env_badge),
                        )
                    })
                    .child(pro_icon(ProIcon::ChevronDown, 12., p.faint)),
                cx,
                |this, _, cx| {
                    this.run_menu_open = !this.run_menu_open;
                    this.project_menu_open = false;
                    cx.notify();
                },
            ));

        let controls = if self.empty {
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(pro_btn(
                    pro_eid(&self.id, "top-start"),
                    tr(cx, "Start run", "启动运行"),
                    Some(ProIcon::Play),
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    |this, _, cx| this.handle_run_action(AoRunAction::Start, cx),
                ))
        } else if self.stopped {
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(pro_btn(
                    pro_eid(&self.id, "top-restart"),
                    tr(cx, "Run again", "重新运行"),
                    Some(ProIcon::Refresh),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    |this, _, cx| this.handle_run_action(AoRunAction::Restart, cx),
                ))
        } else {
            let pause_lbl = if self.paused {
                tr(cx, "Resume", "继续")
            } else {
                tr(cx, "Pause run", "暂停运行")
            };
            let pause_ic = if self.paused {
                ProIcon::Play
            } else {
                ProIcon::Pause
            };
            let pause_act = if self.paused {
                AoRunAction::Resume
            } else {
                AoRunAction::Pause
            };
            let stop_lbl: SharedString = tr(cx, "Stop", "停止").into();
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "top-pause"))
                        .role(Role::Button)
                        .aria_label(pause_lbl)
                        .tab_index(0)
                        .h(px(28.))
                        .px(px(if narrow { 7. } else { 9. }))
                        .rounded(px(7.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .hover(move |s| s.bg(p.hover))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .cursor_pointer()
                        .child(pro_icon(pause_ic, 13., p.muted))
                        .when(!narrow, |d| d.child(pause_lbl)),
                    cx,
                    move |this, _, cx| this.handle_run_action(pause_act, cx),
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "top-stop"))
                        .role(Role::Button)
                        .aria_label(stop_lbl.clone())
                        .tab_index(0)
                        .h(px(28.))
                        .px(px(if narrow { 7. } else { 9. }))
                        .rounded(px(7.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .hover(move |s| s.bg(p.hover))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .cursor_pointer()
                        .child(pro_svg("0 0 24 24", STOP_SQUARE_SVG, 13., p.muted))
                        .when(!narrow, |d| d.child(stop_lbl)),
                    cx,
                    |this, _, cx| this.handle_run_action(AoRunAction::Stop, cx),
                ))
        };

        let top_bar = div()
            .absolute()
            .top(px(10.))
            .left(px(10.))
            .right(px(top_right_inset))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(path_bar)
            .child(controls);

        // Bottom floating canvas toolbar
        let zoom_pct = (self.zoom * 100.0).round() as u32;
        let tb_center_x = if panel_open && !narrow {
            ((vw - 398.0) * 0.5 - 88.0).max(12.0)
        } else {
            (vw * 0.5 - if narrow { 62.0 } else { 88.0 }).max(12.0)
        };

        let toolbar = div()
            .absolute()
            .bottom(px(14.))
            .left(px(tb_center_x))
            .p(px(4.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .items_center()
            .gap(px(2.))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "tb-next-running"))
                    .role(Role::Button)
                    .aria_label(tr(
                        cx,
                        "Go to the next running agent",
                        "跳转到下一个运行中的智能体",
                    ))
                    .tab_index(0)
                    .size(px(28.))
                    .rounded(px(6.))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(pro_svg("0 0 24 24", LOCATE_PIN_SVG, 15., p.muted)),
                cx,
                |this, _, cx| this.jump_next_running(cx),
            ))
            .child(div().mx(px(4.)).w(px(1.)).h(px(16.)).bg(p.line_strong))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "tb-fit"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Fit to view", "适应视图"))
                    .tab_index(0)
                    .size(px(28.))
                    .rounded(px(6.))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(pro_svg("0 0 24 24", FIT_VIEW_SVG, 15., p.muted)),
                cx,
                |this, _, cx| this.fit_to_view(cx),
            ))
            .child(pro_icon_btn(
                pro_eid(&self.id, "tb-zoom-out"),
                tr(cx, "Zoom out", "缩小"),
                ProIcon::Minus,
                28.,
                false,
                cx,
                |this, _, cx| {
                    this.zoom = (this.zoom - 0.10).clamp(0.45, 1.25);
                    cx.notify();
                },
            ))
            .when(!narrow, |d| {
                d.child(activate(
                    div()
                        .id(pro_eid(&self.id, "tb-zoom-pct"))
                        .role(Role::Button)
                        .aria_label(tr(cx, "Reset zoom to 100%", "重置缩放为 100%"))
                        .tab_index(0)
                        .w(px(44.))
                        .h(px(28.))
                        .rounded(px(6.))
                        .hover(move |s| s.bg(p.hover).text_color(p.ink))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .cursor_pointer()
                        .child(format!("{zoom_pct} %")),
                    cx,
                    |this, _, cx| {
                        this.zoom = if (this.zoom - 1.0).abs() < 0.03 {
                            0.70
                        } else {
                            1.0
                        };
                        cx.notify();
                    },
                ))
            })
            .child(pro_icon_btn(
                pro_eid(&self.id, "tb-zoom-in"),
                tr(cx, "Zoom in", "放大"),
                ProIcon::Plus,
                28.,
                false,
                cx,
                |this, _, cx| {
                    this.zoom = (this.zoom + 0.10).clamp(0.45, 1.25);
                    cx.notify();
                },
            ));

        let mut area = div()
            .relative()
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_hidden()
            .child(viewport)
            .child(top_bar)
            .when(!self.empty, |d| d.child(toolbar));

        // Project dropdown popover
        {
            let projects = [
                "Market research",
                "Competitor pricing Q4",
                "Customer onboarding",
            ];
            let proj_card = div()
                .w(px(220.))
                .p(px(4.))
                .rounded(px(11.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .flex_col()
                .gap(px(1.))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .children(projects.into_iter().enumerate().map(|(pi, proj)| {
                    let active = self.project.as_ref() == proj;
                    let ps: SharedString = proj.into();
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("proj-opt-{pi}")))
                            .role(Role::MenuItem)
                            .tab_index(0)
                            .h(px(30.))
                            .px(px(10.))
                            .rounded(px(7.))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(13.))
                            .text_color(if active { p.ink } else { p.text })
                            .font_weight(if active {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .cursor_pointer()
                            .child(proj)
                            .when(active, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink))),
                        cx,
                        move |this, _, cx| {
                            this.project = ps.clone();
                            this.project_menu_open = false;
                            cx.emit(AgentOrchestrationEvent::SwitchProject(ps.clone()));
                            cx.notify();
                        },
                    )
                }));
            area = area.child(pro_presence(
                pro_eid(&self.id, "proj-menu-pres"),
                self.project_menu_open,
                0.16,
                0.12,
                move |amount| {
                    div()
                        .absolute()
                        .top(px(42. - 6. * (1. - amount)))
                        .left(px(10.))
                        .opacity(amount)
                        .child(proj_card)
                        .into_any_element()
                },
            ));
        }

        // Run menu popover
        {
            let runs = [
                ("run_8f3a2c", "Live · $2.17"),
                ("run_7d19b0", "14m ago · $1.84"),
                ("run_6c04e1", "2h ago · $2.40"),
            ];
            let run_id_for_copy = self.run_id.clone();
            let run_card = div()
                .w(px(244.))
                .p(px(4.))
                .rounded(px(11.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .flex_col()
                .gap(px(1.))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .children(runs.into_iter().enumerate().map(|(ri, (rid, sub))| {
                    let active = self.run_id.as_ref() == rid;
                    let rs: SharedString = rid.into();
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("run-opt-{ri}")))
                            .role(Role::MenuItem)
                            .tab_index(0)
                            .h(px(32.))
                            .px(px(10.))
                            .rounded(px(7.))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .cursor_pointer()
                            .child(
                                div()
                                    .font_family("Menlo")
                                    .text_size(px(12.))
                                    .text_color(p.ink)
                                    .child(rid),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .child(sub)
                                    .when(active, |d| {
                                        d.child(pro_icon(ProIcon::Check, 13., p.ink))
                                    }),
                            ),
                        cx,
                        move |this, _, cx| {
                            this.run_id = rs.clone();
                            this.run_menu_open = false;
                            cx.emit(AgentOrchestrationEvent::SwitchRun(rs.clone()));
                            cx.notify();
                        },
                    )
                }))
                .child(div().my(px(3.)).mx(px(6.)).h(px(1.)).bg(p.line))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "run-copy-id"))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(30.))
                        .px(px(10.))
                        .rounded(px(7.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(13.))
                        .text_color(p.text)
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Copy, 14., p.muted))
                        .child(tr(cx, "Copy run ID", "复制运行 ID")),
                    cx,
                    move |this, _, cx| {
                        clipboard::write(run_id_for_copy.as_ref(), cx).detach();
                        this.run_menu_open = false;
                        this.push_toast(format!("Copied {}", run_id_for_copy));
                        cx.notify();
                    },
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "run-download"))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(30.))
                        .px(px(10.))
                        .rounded(px(7.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(13.))
                        .text_color(p.text)
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Download, 14., p.muted))
                        .child(tr(cx, "Download trace", "下载追踪日志")),
                    cx,
                    |this, _, cx| {
                        this.run_menu_open = false;
                        this.push_toast(tr(cx, "Exported run_8f3a2c.json", "已导出追踪文件"));
                        cx.notify();
                    },
                ));
            area = area.child(pro_presence(
                pro_eid(&self.id, "run-menu-pres"),
                self.run_menu_open,
                0.16,
                0.12,
                move |amount| {
                    div()
                        .absolute()
                        .top(px(42. - 6. * (1. - amount)))
                        .left(px(124.))
                        .opacity(amount)
                        .child(run_card)
                        .into_any_element()
                },
            ));
        }

        // Floating right agent/span inspector card (or bottom sheet on narrow)
        let active_or_closing_id = self
            .selected_agent
            .clone()
            .or_else(|| self.last_selected_agent.clone());
        if let Some(ref aid) = active_or_closing_id
            && let Some(panel_card) = self.render_agent_inspector(aid, now_ms, anim_ms, p, cx)
        {
            let scrim_id = pro_eid(&self.id, "sheet-scrim");
            let on_scrim_close = cx.listener(|this, _, _, cx| this.close_agent(cx));
            let p_dark = p.dark;
            area = area.child(pro_presence(
                pro_eid(&self.id, "ao-insp-pres"),
                panel_open,
                0.22,
                0.16,
                move |amount| {
                    if narrow {
                        div()
                            .absolute()
                            .inset_0()
                            .child(
                                div()
                                    .id(scrim_id)
                                    .absolute()
                                    .inset_0()
                                    .bg(hsla(
                                        0.,
                                        0.,
                                        0.,
                                        (if p_dark { 0.45 } else { 0.16 }) * amount,
                                    ))
                                    .on_mouse_down(MouseButton::Left, on_scrim_close),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .left(px(8.))
                                    .right(px(8.))
                                    .bottom(px(8. - 14. * (1. - amount)))
                                    .h(px(360.))
                                    .opacity(amount)
                                    .child(panel_card),
                            )
                            .into_any_element()
                    } else {
                        div()
                            .absolute()
                            .top(px(10.))
                            .right(px(10. - 14. * (1. - amount)))
                            .bottom(px(10.))
                            .w(px(388.))
                            .opacity(amount)
                            .child(panel_card)
                            .into_any_element()
                    }
                },
            ));
        }

        // Toast banner
        area = area.child(pro_toast_banner(self.toast.clone()));

        area
    }

    #[allow(clippy::too_many_lines)]
    fn render_agent_inspector(
        &mut self,
        sel_id: &SharedString,
        now_ms: u64,
        anim_ms: f32,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        let ag = self.agents.iter().find(|a| &a.id == sel_id)?.clone();
        let flowing = self.is_flowing();
        let b_cost = branch_cost(&self.agents, &ag.id);

        let dur_str = match ag.start_ms {
            Some(st) => format_duration_ms(ag.end_ms.unwrap_or(now_ms).saturating_sub(st)),
            None => "–".to_string(),
        };

        let parent_ag = ag
            .parent
            .as_ref()
            .and_then(|pid| self.agents.iter().find(|a| &a.id == pid))
            .cloned();
        let child_ags: Vec<AoAgent> = self
            .agents
            .iter()
            .filter(|c| c.parent.as_ref() == Some(&ag.id))
            .cloned()
            .collect();
        let retry_target = self
            .agents
            .iter()
            .find(|c| c.retry_of.as_ref() == Some(&ag.id))
            .cloned();

        let tab_defs = [
            (
                AoInspectorTab::Tasks,
                tr(cx, "Tasks", "任务"),
                ag.tasks.len(),
            ),
            (
                AoInspectorTab::Context,
                tr(cx, "Context", "上下文"),
                ag.context.len() + child_ags.len() + usize::from(parent_ag.is_some()),
            ),
            (
                AoInspectorTab::Payloads,
                tr(cx, "Tools & I/O", "工具与载荷"),
                ag.tools.len(),
            ),
        ];

        let insp_tab_sensor = pro_selection_sensor();
        let mut tabs_row = div()
            .relative()
            .px(px(10.))
            .pb(px(8.))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "insp-tab-ind"),
                format!("{:?}", self.inspector_tab),
                insp_tab_sensor.clone(),
                6.,
                p.chip,
                None,
                false,
            ));
        for (ti, (tk, t_lbl, cnt)) in tab_defs.into_iter().enumerate() {
            let active = self.inspector_tab == tk;
            tabs_row = tabs_row.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("insp-tab-{ti}")))
                    .role(Role::Tab)
                    .tab_index(0)
                    .relative()
                    .h(px(28.))
                    .px(px(9.))
                    .rounded(px(6.))
                    .when(!active, |d| {
                        d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                    })
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.5))
                    .font_weight(if active {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if active { p.ink } else { p.muted })
                    .cursor_pointer()
                    .child(t_lbl)
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(p.faint)
                            .child(cnt.to_string()),
                    )
                    .when(active, |d| d.child(pro_mark_selected(&insp_tab_sensor))),
                cx,
                move |this, _, cx| {
                    this.inspector_tab = tk;
                    cx.notify();
                },
            ));
        }

        let well_bg: Hsla = if p.dark {
            rgb(0x151518).into()
        } else {
            rgb(0xf6f6f7).into()
        };

        let body_content: Div = match self.inspector_tab {
            AoInspectorTab::Tasks => {
                let mut col = div().flex().flex_col().gap(px(14.)).child(
                    div()
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.ink)
                        .child(ag.task.clone()),
                );

                // Waiting approval banner
                if ag.status == AoAgentStatus::Waiting
                    && let Some(ref question) = ag.ask
                {
                    let aid_rej = ag.id.clone();
                    let aid_app = ag.id.clone();
                    col = col.child(
                        div()
                            .p(px(12.))
                            .rounded(px(8.))
                            .bg(p.amber_soft)
                            .border_1()
                            .border_color(p.amber.opacity(0.30))
                            .flex()
                            .flex_col()
                            .gap(px(10.))
                            .child(
                                div()
                                    .flex()
                                    .items_start()
                                    .gap(px(8.))
                                    .text_size(px(12.5))
                                    .line_height(px(19.))
                                    .text_color(p.ink)
                                    .child(pro_icon(ProIcon::Clock, 15., p.amber_ink))
                                    .child(div().flex_1().min_w_0().child(question.clone())),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_end()
                                    .gap(px(6.))
                                    .child(pro_btn(
                                        pro_eid(&self.id, "btn-reject"),
                                        tr(cx, "Reject", "拒绝"),
                                        None,
                                        ProBtnStyle::Secondary,
                                        true,
                                        cx,
                                        move |this, _, cx| {
                                            this.resolve_waiting_agent(&aid_rej, false, cx);
                                        },
                                    ))
                                    .child(pro_btn(
                                        pro_eid(&self.id, "btn-approve"),
                                        tr(cx, "Approve", "批准继续"),
                                        Some(ProIcon::Check),
                                        ProBtnStyle::Primary,
                                        true,
                                        cx,
                                        move |this, _, cx| {
                                            this.resolve_waiting_agent(&aid_app, true, cx);
                                        },
                                    )),
                            ),
                    );
                }

                // Error banner + jump to retry
                if ag.status == AoAgentStatus::Failed
                    && let Some(ref err_txt) = ag.error
                {
                    col = col.child(
                        div()
                            .p(px(11.))
                            .rounded(px(8.))
                            .bg(p.red_soft)
                            .border_1()
                            .border_color(p.red.opacity(0.30))
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex()
                                    .items_start()
                                    .gap(px(8.))
                                    .text_size(px(12.5))
                                    .line_height(px(18.))
                                    .text_color(p.red_ink)
                                    .child(pro_icon(ProIcon::AlertTriangle, 15., p.red_ink))
                                    .child(div().flex_1().min_w_0().child(err_txt.clone())),
                            )
                            .when_some(retry_target, |d, rt| {
                                let rt_id = rt.id.clone();
                                let rt_name = rt.name.clone();
                                d.child(activate(
                                    div()
                                        .id(pro_eid(&self.id, "jump-retry"))
                                        .role(Role::Button)
                                        .tab_index(0)
                                        .h(px(24.))
                                        .px(px(8.))
                                        .rounded(px(6.))
                                        .hover(move |s| s.bg(p.hover))
                                        .flex()
                                        .items_center()
                                        .gap(px(5.))
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .cursor_pointer()
                                        .child(format!("{} {rt_name}", tr(cx, "Open", "查看")))
                                        .child(pro_icon(ProIcon::ArrowRight, 12., p.muted)),
                                    cx,
                                    move |this, _, cx| this.open_agent(rt_id.clone(), cx),
                                ))
                            }),
                    );
                }

                // 2x2 Stats grid
                let stats_box = div()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .border_b_1()
                            .border_color(p.line)
                            .child(
                                div()
                                    .flex_1()
                                    .px(px(12.))
                                    .py(px(9.))
                                    .border_r_1()
                                    .border_color(p.line)
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.muted)
                                            .child(tr(cx, "DURATION", "耗时")),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(15.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(dur_str),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .px(px(12.))
                                    .py(px(9.))
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.muted)
                                            .child(tr(cx, "TOKENS", "TOKEN")),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(15.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(format_tokens_k(ag.total_tokens())),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .child(
                                div()
                                    .flex_1()
                                    .px(px(12.))
                                    .py(px(9.))
                                    .border_r_1()
                                    .border_color(p.line)
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.muted)
                                            .child(tr(cx, "OWN COST", "自身费用")),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(15.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(if ag.cost > 0.001 {
                                                format!("${:.2}", ag.cost)
                                            } else {
                                                "–".to_string()
                                            }),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .px(px(12.))
                                    .py(px(9.))
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.muted)
                                            .child(tr(cx, "BRANCH COST", "分支总计")),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(15.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(if b_cost > 0.001 {
                                                format!("${b_cost:.2}")
                                            } else {
                                                "–".to_string()
                                            }),
                                    ),
                            ),
                    );
                col = col.child(stats_box);

                // Task checklist
                let mut todo_list = div().flex().flex_col();
                let task_len = ag.tasks.len();
                for (ti, t) in ag.tasks.iter().enumerate() {
                    let check_el: AnyElement = match t.status {
                        AoAgentStatus::Done => div()
                            .size(px(16.))
                            .rounded_full()
                            .bg(p.muted)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_icon(ProIcon::Check, 10., p.card))
                            .into_any_element(),
                        AoAgentStatus::Running => {
                            pro_progress_ring(0.38, 16., 1.8, p.blue, p.line_strong)
                        }
                        AoAgentStatus::Failed => div()
                            .size(px(16.))
                            .rounded_full()
                            .bg(p.red)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_icon(ProIcon::Close, 10., rgb(0xffffff).into()))
                            .into_any_element(),
                        AoAgentStatus::Waiting => div()
                            .size(px(16.))
                            .rounded_full()
                            .bg(p.amber_soft)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_icon(ProIcon::Clock, 11., p.amber_ink))
                            .into_any_element(),
                        _ => div()
                            .size(px(16.))
                            .rounded_full()
                            .border_1()
                            .border_color(p.line_strong)
                            .into_any_element(),
                    };

                    todo_list = todo_list.child(
                        div()
                            .py(px(9.))
                            .when(ti + 1 < task_len, |d| d.border_b_1().border_color(p.line))
                            .flex()
                            .items_start()
                            .gap(px(10.))
                            .child(div().pt(px(2.)).flex_none().child(check_el))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .text_color(p.ink)
                                            .truncate()
                                            .child(t.title.clone()),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(5.))
                                            .text_size(px(12.))
                                            .text_color(if t.status == AoAgentStatus::Failed {
                                                p.red_ink
                                            } else {
                                                p.muted
                                            })
                                            .child(pro_icon(tool_icon(&t.tool_id), 11., p.muted))
                                            .child(t.tool_label.clone())
                                            .child("·")
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .truncate()
                                                    .child(t.result.clone()),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .flex()
                                    .flex_col()
                                    .items_end()
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .child(if t.duration_ms > 0 {
                                        format_duration_ms(t.duration_ms)
                                    } else {
                                        "–".to_string()
                                    })
                                    .child(
                                        div().text_color(p.faint).child(format_tokens_k(t.tokens)),
                                    ),
                            ),
                    );
                }

                let done_task_cnt = ag
                    .tasks
                    .iter()
                    .filter(|t| t.status == AoAgentStatus::Done)
                    .count();
                let task_prog_ratio = if ag.tasks.is_empty() {
                    0.0
                } else {
                    done_task_cnt as f32 / ag.tasks.len() as f32
                };
                col.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div().flex().items_center().justify_between().child(
                                div()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(format!(
                                        "{} · {} / {}",
                                        tr(cx, "TASKS", "执行步骤"),
                                        done_task_cnt,
                                        ag.tasks.len()
                                    )),
                            ),
                        )
                        .when(!ag.tasks.is_empty(), |d| {
                            d.child(pro_spring_bar(
                                pro_eid(&self.id, format!("insp-task-prog-{}", ag.id)),
                                task_prog_ratio,
                                4.,
                                2.,
                                p.chip,
                                p.green,
                            ))
                        })
                        .child(todo_list),
                )
            }
            AoInspectorTab::Context => {
                let mut col = div().flex().flex_col().gap(px(14.));
                if let Some(parent) = parent_ag {
                    let pid = parent.id.clone();
                    col = col.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "STARTED BY", "上级智能体")),
                            )
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, "ctx-parent"))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .h(px(34.))
                                    .px(px(8.))
                                    .rounded(px(7.))
                                    .bg(p.chip)
                                    .hover(move |s| s.bg(p.hover))
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .cursor_pointer()
                                    .child(Self::render_maker_mark(
                                        &parent.maker,
                                        &parent.id,
                                        20.,
                                        false,
                                        anim_ms,
                                        p,
                                        cx,
                                    ))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .text_size(px(13.))
                                            .text_color(p.ink)
                                            .truncate()
                                            .child(parent.name),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.muted)
                                            .child(parent.model),
                                    ),
                                cx,
                                move |this, _, cx| this.open_agent(pid.clone(), cx),
                            )),
                    );
                }

                if !child_ags.is_empty() {
                    let mut list = div().flex().flex_col().gap(px(3.));
                    for (ci, ch) in child_ags.into_iter().enumerate() {
                        let cid = ch.id.clone();
                        list = list.child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("ctx-child-{ci}")))
                                .role(Role::Button)
                                .tab_index(0)
                                .h(px(34.))
                                .px(px(8.))
                                .rounded(px(7.))
                                .hover(move |s| s.bg(p.hover))
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .cursor_pointer()
                                .child(Self::render_maker_mark(
                                    &ch.maker,
                                    &ch.id,
                                    20.,
                                    ch.status == AoAgentStatus::Running && flowing,
                                    anim_ms,
                                    p,
                                    cx,
                                ))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_size(px(13.))
                                        .text_color(p.ink)
                                        .truncate()
                                        .child(ch.name),
                                )
                                .child(Self::render_status_chip(
                                    ch.status, flowing, anim_ms, 0.92, p, cx,
                                )),
                            cx,
                            move |this, _, cx| this.open_agent(cid.clone(), cx),
                        ));
                    }
                    col = col.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "SUB-AGENTS SPAWNED", "派生的子智能体")),
                            )
                            .child(list),
                    );
                }

                let mut ctx_rows = div().flex().flex_col().gap(px(4.));
                for item in &ag.context {
                    ctx_rows = ctx_rows.child(
                        div()
                            .h(px(34.))
                            .px(px(8.))
                            .rounded(px(7.))
                            .bg(well_bg)
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(pro_icon(item.icon, 14., p.muted))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(12.5))
                                    .text_color(p.ink)
                                    .truncate()
                                    .child(item.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .child(item.meta.clone()),
                            ),
                    );
                }
                col.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(
                            div()
                                .text_size(px(10.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .child(tr(cx, "CONTEXT & ARTIFACTS", "上下文与产物")),
                        )
                        .child(ctx_rows),
                )
            }
            AoInspectorTab::Payloads => {
                let copied_now: Option<SharedString> =
                    self.copied_key.as_ref().and_then(|(k, ts)| {
                        if ts.elapsed().as_millis() < 1800 {
                            Some(k.clone())
                        } else {
                            None
                        }
                    });

                let mut tool_chips = div().flex().flex_wrap().gap(px(6.));
                for tid in &ag.tools {
                    tool_chips = tool_chips.child(
                        div()
                            .h(px(24.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .text_size(px(12.))
                            .text_color(p.ink)
                            .child(pro_icon(tool_icon(tid), 12., p.muted))
                            .child(tid.clone()),
                    );
                }

                let render_payload_box =
                    |key_id: &str, heading: &str, body: SharedString, cx: &Context<Self>| {
                        let full_key = SharedString::from(format!("{}:{key_id}", ag.id));
                        let is_cp = copied_now.as_ref() == Some(&full_key);
                        let copy_txt = body.clone();
                        let fk_clone = full_key;
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.muted)
                                            .child(heading.to_string()),
                                    )
                                    .child(activate(
                                        div()
                                            .id(pro_eid(&self.id, format!("cp-{key_id}")))
                                            .role(Role::Button)
                                            .tab_index(0)
                                            .h(px(20.))
                                            .px(px(6.))
                                            .rounded(px(5.))
                                            .hover(move |s| s.bg(p.chip))
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .text_size(px(11.5))
                                            .text_color(if is_cp { p.green_ink } else { p.muted })
                                            .cursor_pointer()
                                            .child(pro_icon(
                                                if is_cp { ProIcon::Check } else { ProIcon::Copy },
                                                11.,
                                                if is_cp { p.green_ink } else { p.muted },
                                            ))
                                            .child(if is_cp {
                                                tr(cx, "Copied", "已复制")
                                            } else {
                                                tr(cx, "Copy", "复制")
                                            }),
                                        cx,
                                        move |this, _, cx| {
                                            clipboard::write(copy_txt.as_ref(), cx).detach();
                                            this.copied_key =
                                                Some((fk_clone.clone(), Instant::now()));
                                            cx.notify();
                                        },
                                    )),
                            )
                            .child(
                                div()
                                    .px(px(12.))
                                    .py(px(10.))
                                    .rounded(px(8.))
                                    .bg(well_bg)
                                    .font_family("Menlo")
                                    .text_size(px(11.5))
                                    .line_height(px(17.5))
                                    .text_color(p.text)
                                    .child(body),
                            )
                    };

                let h_in = tr(cx, "PROMPT / INPUT", "提示词输入");
                let h_tool = tr(cx, "TOOL CALL PAYLOAD", "工具调用载荷");
                let h_out = tr(cx, "OUTPUT", "输出摘要");
                let p_in = render_payload_box("prompt", h_in, ag.prompt_input.clone(), cx);
                let p_tool = render_payload_box("tool", h_tool, ag.tool_payload.clone(), cx);
                let p_out = render_payload_box("out", h_out, ag.output_preview.clone(), cx);

                div()
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "BOUND TOOLS", "关联工具")),
                            )
                            .child(tool_chips),
                    )
                    .child(p_in)
                    .child(p_tool)
                    .child(p_out)
            }
        };

        let vtx_key = format!("{}-{:?}", ag.id, self.inspector_tab);
        let body_el = pro_view_transition(pro_eid(&self.id, "ao-insp-vtx"), vtx_key, body_content);

        Some(
            div()
                .size_full()
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .overflow_hidden()
                .flex()
                .flex_col()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .h(px(56.))
                        .pl(px(12.))
                        .pr(px(8.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(Self::render_maker_mark(
                            &ag.maker,
                            &ag.id,
                            30.,
                            ag.status == AoAgentStatus::Running && flowing,
                            anim_ms,
                            p,
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
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .truncate()
                                        .child(ag.name),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(format!(
                                            "{} · {} · {} cached",
                                            ag.role,
                                            ag.model,
                                            format_tokens_k(ag.tokens_cached)
                                        )),
                                ),
                        )
                        .child(Self::render_status_chip(
                            ag.status, flowing, anim_ms, 1.0, p, cx,
                        ))
                        .child(pro_icon_btn(
                            pro_eid(&self.id, "insp-close"),
                            tr(cx, "Close agent panel", "关闭详情面板"),
                            ProIcon::Close,
                            28.,
                            false,
                            cx,
                            |this, _, cx| this.close_agent(cx),
                        )),
                )
                .child(tabs_row)
                .child(
                    div()
                        .id(pro_eid(&self.id, "insp-scroll"))
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .p(px(14.))
                        .child(body_el),
                ),
        )
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn render_run_panel(
        &mut self,
        ordered: &[PositionedAgent],
        now_ms: u64,
        anim_ms: f32,
        skel_phase: f32,
        narrow: bool,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let reduced = Theme::global(cx).reduced_motion;
        let flowing = self.is_flowing();
        let total_agents = self.agents.len();
        let done_agents = self
            .agents
            .iter()
            .filter(|a| a.status == AoAgentStatus::Done)
            .count();
        let total_tokens: u64 = self.agents.iter().map(AoAgent::total_tokens).sum();
        let total_cost: f64 = self.agents.iter().map(|a| a.cost).sum();
        let vis_events: Vec<AoEvent> = self
            .events
            .iter()
            .filter(|e| !self.empty && e.at_ms <= now_ms)
            .cloned()
            .collect();
        let error_count = vis_events
            .iter()
            .filter(|e| e.kind == AoEventKind::Error)
            .count();

        let body_h = (self.run_panel_height - 44.0).max(80.0);

        // Top resize handle
        let orig_h = self.run_panel_height;
        let grip = div()
            .id(pro_eid(&self.id, "run-grip"))
            .w_full()
            .h(px(5.))
            .cursor(CursorStyle::ResizeUpDown)
            .hover(move |s| s.bg(p.line_strong))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                    if ev.click_count >= 2 {
                        this.run_panel_folded = false;
                        this.run_panel_height = 224.0;
                    } else {
                        this.run_panel_folded = false;
                        this.dragging = Some(DragKind::ResizeRunPanel {
                            start_mouse_y: f32::from(ev.position.y) as i32,
                            orig_h,
                        });
                    }
                    cx.notify();
                }),
            );

        // Run panel header tabs + summary + fold toggle
        let tab_specs = [
            (AoRunTab::Trace, tr(cx, "Trace", "调用追踪"), 0usize),
            (
                AoRunTab::TokensAndCost,
                tr(cx, "Tokens & cost", "用量与费用"),
                0usize,
            ),
            (AoRunTab::Events, tr(cx, "Events", "事件流"), error_count),
        ];

        let run_tab_sensor = pro_selection_sensor();
        let mut tabs_bar =
            div()
                .relative()
                .flex()
                .items_center()
                .gap(px(2.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "run-tab-ind"),
                    format!("{:?}", self.run_tab),
                    run_tab_sensor.clone(),
                    6.,
                    p.chip,
                    None,
                    false,
                ));
        for (ti, (tab_k, lbl, badge_n)) in tab_specs.into_iter().enumerate() {
            let active = self.run_tab == tab_k;
            let lead_el: AnyElement = match tab_k {
                AoRunTab::Trace => pro_svg(
                    "0 0 24 24",
                    LAYERS_SVG,
                    14.,
                    if active { p.ink } else { p.muted },
                ),
                AoRunTab::TokensAndCost => {
                    pro_icon(ProIcon::Coins, 14., if active { p.ink } else { p.muted })
                }
                AoRunTab::Events => {
                    pro_icon(ProIcon::Tasks, 14., if active { p.ink } else { p.muted })
                }
            };
            tabs_bar = tabs_bar.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("run-tab-{ti}")))
                    .role(Role::Tab)
                    .tab_index(0)
                    .relative()
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .when(!active, |d| {
                        d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                    })
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .text_color(if active { p.ink } else { p.muted })
                    .cursor_pointer()
                    .child(lead_el)
                    .when(!narrow, |d| d.child(lbl))
                    .when(badge_n > 0, |d| {
                        d.child(
                            div()
                                .min_w(px(16.))
                                .h(px(16.))
                                .px(px(4.))
                                .rounded(px(4.))
                                .bg(p.red_soft)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(11.))
                                .text_color(p.red_ink)
                                .child(badge_n.to_string()),
                        )
                    })
                    .when(active, |d| d.child(pro_mark_selected(&run_tab_sensor))),
                cx,
                move |this, _, cx| {
                    this.run_tab = tab_k;
                    this.run_panel_folded = false;
                    cx.notify();
                },
            ));
        }

        let done_frac = if total_agents > 0 && !self.empty {
            done_agents as f32 / total_agents as f32
        } else {
            0.0
        };

        let summary_bar = if self.loading {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(14.))
                .child(pro_skeleton_bar(48., 12., 4., skel_phase, cx))
                .child(pro_skeleton_bar(46., 12., 4., skel_phase, cx))
                .child(pro_skeleton_bar(52., 12., 4., skel_phase, cx))
                .child(pro_skeleton_bar(38., 12., 4., skel_phase, cx))
        } else {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(if narrow { 10. } else { 16. }))
                .text_size(px(12.))
                .text_color(p.muted)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(pro_progress_ring(
                            done_frac,
                            13.,
                            2.2,
                            p.muted,
                            p.line_strong,
                        ))
                        .child(if self.empty {
                            format!("0 / {total_agents}")
                        } else {
                            format!("{done_agents} / {total_agents}")
                        }),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .when(!narrow, |d| d.child(pro_icon(ProIcon::Clock, 13., p.faint)))
                        .child(format_clock_ms(now_ms)),
                )
                .when(!narrow, |d| {
                    d.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .child(pro_icon(ProIcon::Sparkles, 13., p.faint))
                            .child(if self.empty {
                                "–".to_string()
                            } else {
                                format_tokens_k(total_tokens)
                            }),
                    )
                })
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(if self.empty {
                            "$0.00".to_string()
                        } else {
                            format!("${total_cost:.2}")
                        }),
                )
        };

        let head = div()
            .h(px(39.))
            .pl(px(10.))
            .pr(px(8.))
            .flex_none()
            .when(!self.run_panel_folded, |d| {
                d.border_b_1().border_color(p.line)
            })
            .flex()
            .items_center()
            .gap(px(8.))
            .child(tabs_bar)
            .child(summary_bar)
            .child(pro_icon_btn(
                pro_eid(&self.id, "run-fold"),
                if self.run_panel_folded {
                    tr(cx, "Expand run panel", "展开运行面板")
                } else {
                    tr(cx, "Fold run panel", "折叠运行面板")
                },
                if self.run_panel_folded {
                    ProIcon::ChevronUp
                } else {
                    ProIcon::ChevronDown
                },
                28.,
                false,
                cx,
                |this, _, cx| {
                    this.run_panel_folded = !this.run_panel_folded;
                    cx.notify();
                },
            ));

        // Body content per selected tab
        let body: AnyElement = if self.empty && !self.loading {
            pro_empty_state(
                ProIcon::Activity,
                tr(cx, "Run hasn't started", "运行尚未开始"),
                tr(
                    cx,
                    "Click Start run to watch spans, token usage, and handoff events stream in.",
                    "点击“启动运行”即可实时查看调用时序图、Token 明细与事件流。",
                ),
                None,
                cx,
            )
            .py(px(18.))
            .into_any_element()
        } else if self.loading {
            let mut skel_rows = div().p(px(10.)).flex().flex_col().gap(px(8.));
            for i in 0..5usize {
                skel_rows = skel_rows.child(
                    div()
                        .h(px(22.))
                        .flex()
                        .items_center()
                        .gap(px(14.))
                        .child(pro_skeleton_bar(
                            130. + (i as f32 * 14.),
                            12.,
                            4.,
                            skel_phase,
                            cx,
                        ))
                        .child(div().flex_1().child(pro_skeleton_bar(
                            180. + ((i % 3) as f32 * 56.),
                            10.,
                            4.,
                            skel_phase,
                            cx,
                        )))
                        .child(pro_skeleton_bar(42., 12., 4., skel_phase, cx)),
                );
            }
            skel_rows.into_any_element()
        } else {
            match self.run_tab {
                AoRunTab::Trace => self
                    .render_trace_tab(ordered, now_ms, anim_ms, reduced, flowing, narrow, p, cx)
                    .into_any_element(),
                AoRunTab::TokensAndCost => self
                    .render_cost_tab(ordered, anim_ms, flowing, total_cost, narrow, p, cx)
                    .into_any_element(),
                AoRunTab::Events => self
                    .render_events_tab(&vis_events, narrow, p, cx)
                    .into_any_element(),
            }
        };
        let body = if self.loading {
            body
        } else {
            pro_view_transition(
                pro_eid(&self.id, "ao-run-tab-vtx"),
                format!("{:?}-{}", self.run_tab, self.empty),
                body,
            )
        };
        let scroll_el = div()
            .id(pro_eid(&self.id, "run-body-scroll"))
            .w_full()
            .h(px(body_h))
            .overflow_y_scroll()
            .child(body)
            .into_any_element();
        let folded_body = pro_fold(
            pro_eid(&self.id, "run-panel-fold"),
            !self.run_panel_folded,
            body_h,
            0.,
            move || scroll_el,
        );

        div()
            .w_full()
            .flex_none()
            .bg(p.panel)
            .border_t_1()
            .border_color(p.line_strong)
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(grip)
            .child(head)
            .child(folded_body)
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn render_trace_tab(
        &self,
        ordered: &[PositionedAgent],
        now_ms: u64,
        anim_ms: f32,
        reduced: bool,
        flowing: bool,
        narrow: bool,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let range_ms = (now_ms.saturating_add(25_000)).max(120_000);
        let step_ms = if range_ms <= 150_000 { 15_000 } else { 30_000 };
        let tick_n = (range_ms / step_ms) as usize;
        let name_w = if narrow { 136.0 } else { 248.0 };
        let len_w = if narrow { 48.0 } else { 62.0 };
        let col_gap = if narrow { 8.0 } else { 14.0 };

        // Sticky axis
        let mut ticks_el = div().relative().flex_1().h_full().min_w_0();
        for i in 0..=tick_n {
            let v_ms = (i as u64) * step_ms;
            if v_ms > range_ms {
                break;
            }
            let frac = (v_ms as f32 / range_ms as f32).clamp(0., 1.);
            let lbl = if v_ms == 0 {
                "0s".to_string()
            } else {
                format_duration_ms(v_ms)
            };
            if i < tick_n {
                ticks_el = ticks_el.child(
                    div()
                        .absolute()
                        .top(px(6.))
                        .left(relative(frac))
                        .pl(px(4.))
                        .text_size(px(11.))
                        .text_color(p.muted)
                        .whitespace_nowrap()
                        .child(lbl),
                );
            }
        }

        let axis = div()
            .w_full()
            .h(px(28.))
            .px(px(10.))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(col_gap))
            .child(
                div()
                    .w(px(name_w))
                    .flex_none()
                    .pl(px(6.))
                    .text_size(px(10.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(tr(cx, "AGENT · CALL", "智能体 · 调用")),
            )
            .child(ticks_el)
            .child(div().w(px(len_w)).flex_none());

        // Grid hairlines + now cursor
        let mut grid_track = div().relative().flex_1().h_full().min_w_0();
        for i in 0..=tick_n {
            let frac = ((i as u64 * step_ms) as f32 / range_ms as f32).clamp(0., 1.);
            grid_track = grid_track.child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(relative(frac))
                    .w(px(1.))
                    .bg(p.ink.opacity(0.045)),
            );
        }
        let now_frac = (now_ms as f32 / range_ms as f32).clamp(0., 1.);
        grid_track = grid_track.child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(relative(now_frac))
                .w(px(1.))
                .bg(p.blue.opacity(0.55)),
        );

        let grid_overlay = div()
            .absolute()
            .inset_0()
            .px(px(10.))
            .flex()
            .gap(px(col_gap))
            .child(div().w(px(name_w)).flex_none())
            .child(grid_track)
            .child(div().w(px(len_w)).flex_none());

        let mut rows_wrap = div()
            .relative()
            .w_full()
            .pt(px(3.))
            .pb(px(8.))
            .flex()
            .flex_col()
            .child(grid_overlay);

        let sheen_frac = ((anim_ms / 1500.0).fract()) * 0.85;

        for pos in ordered {
            let ag = &self.agents[pos.index];
            let is_open = self.selected_agent.as_ref() == Some(&ag.id);
            let is_expanded = self.expanded_trace_agents.contains(&ag.id);
            let has_spans = !ag.tasks.is_empty() && ag.start_ms.is_some();

            let indent = (pos.depth.min(if narrow { 2 } else { 5 }) as f32)
                * if narrow { 8.0 } else { 14.0 };

            let dur_label = match ag.start_ms {
                Some(st) => format_duration_ms(ag.end_ms.unwrap_or(now_ms).saturating_sub(st)),
                None => match ag.status {
                    AoAgentStatus::Queued => tr(cx, "Queued", "排队中").to_string(),
                    AoAgentStatus::Stopped => tr(cx, "Stopped", "已停止").to_string(),
                    _ => "–".to_string(),
                },
            };

            // Agent waterfall bar + embedded purple/cyan call sub-bars
            let track_el = if let Some(st) = ag.start_ms {
                let end = ag.end_ms.unwrap_or(now_ms).max(st + 200);
                let s_frac = (st as f32 / range_ms as f32).clamp(0., 0.99);
                let d_frac =
                    ((end - st) as f32 / range_ms as f32).clamp(0.006, (1.0 - s_frac).max(0.006));
                let ag_dur = (end - st).max(1) as f32;

                let (bar_bg, bar_edge) = if ag.status == AoAgentStatus::Failed {
                    (p.red_soft, p.red.opacity(0.40))
                } else if ag.status == AoAgentStatus::Waiting {
                    (p.amber_soft, p.amber.opacity(0.36))
                } else {
                    (
                        p.ink.opacity(if p.dark { 0.08 } else { 0.05 }),
                        p.ink.opacity(0.14),
                    )
                };

                let mut bar = div()
                    .absolute()
                    .top(px(9.))
                    .left(relative(s_frac))
                    .w(relative(d_frac))
                    .min_w(px(3.))
                    .h(px(10.))
                    .rounded(px(3.5))
                    .overflow_hidden()
                    .bg(bar_bg)
                    .border_1()
                    .border_color(if is_open { p.blue } else { bar_edge });

                // Embedded LLM (purple) + Tool (cyan) call segments inside agent bar
                for task in &ag.tasks {
                    if task.duration_ms == 0 || task.start_ms < st {
                        continue;
                    }
                    let rel_s = ((task.start_ms - st) as f32 / ag_dur).clamp(0., 0.96);
                    let rel_w = (task.duration_ms as f32 / ag_dur).clamp(0.03, 1.0 - rel_s);
                    let llm_w = rel_w * 0.42;
                    let tool_w = rel_w * 0.52;
                    bar = bar
                        .child(
                            div()
                                .absolute()
                                .top(px(2.))
                                .bottom(px(2.))
                                .left(relative(rel_s))
                                .w(relative(llm_w))
                                .rounded(px(1.5))
                                .bg(p.purple.opacity(0.55)),
                        )
                        .child(
                            div()
                                .absolute()
                                .top(px(2.))
                                .bottom(px(2.))
                                .left(relative((rel_s + llm_w + 0.02).min(0.98)))
                                .w(relative(tool_w.min(0.98 - rel_s)))
                                .rounded(px(1.5))
                                .bg(if task.status == AoAgentStatus::Failed {
                                    p.red.opacity(0.65)
                                } else {
                                    p.cyan.opacity(0.55)
                                }),
                        );
                }

                if ag.status == AoAgentStatus::Running && flowing && !reduced {
                    bar = bar.child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(relative(sheen_frac))
                            .w(px(24.))
                            .bg(p.card.opacity(0.40)),
                    );
                }

                div().relative().flex_1().h_full().min_w_0().child(bar)
            } else {
                div().relative().flex_1().h_full().min_w_0()
            };

            let ag_id_chev = ag.id.clone();
            let chev_btn = if has_spans {
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("tr-chev-{}", ag.id)))
                        .role(Role::Button)
                        .tab_index(-1)
                        .w(px(16.))
                        .h(px(18.))
                        .rounded(px(4.))
                        .hover(move |s| s.bg(p.chip))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(pro_icon(
                            if is_expanded {
                                ProIcon::ChevronDown
                            } else {
                                ProIcon::ChevronRight
                            },
                            11.,
                            p.faint,
                        )),
                    cx,
                    move |this, _, cx| {
                        if !this.expanded_trace_agents.remove(&ag_id_chev) {
                            this.expanded_trace_agents.insert(ag_id_chev.clone());
                        }
                        cx.notify();
                    },
                )
                .into_any_element()
            } else {
                div().w(px(16.)).h(px(18.)).into_any_element()
            };

            let state_dot: Option<AnyElement> = match ag.status {
                AoAgentStatus::Running => Some(pro_progress_ring(
                    0.35,
                    11.,
                    1.6,
                    p.blue,
                    p.blue.opacity(0.22),
                )),
                AoAgentStatus::Waiting => Some(pro_icon(ProIcon::Clock, 11., p.amber_ink)),
                AoAgentStatus::Failed => Some(pro_icon(ProIcon::AlertTriangle, 11., p.red_ink)),
                _ => None,
            };

            let ag_id_open = ag.id.clone();
            rows_wrap = rows_wrap.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("tr-row-{}", ag.id)))
                    .role(Role::Row)
                    .tab_index(0)
                    .w_full()
                    .h(px(28.))
                    .px(px(10.))
                    .when(is_open, |d| d.bg(p.blue.opacity(0.08)))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .gap(px(col_gap))
                    .cursor_pointer()
                    .child(
                        div()
                            .w(px(name_w))
                            .flex_none()
                            .min_w_0()
                            .pl(px(indent))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(chev_btn)
                            .when(!narrow, |d| {
                                d.child(Self::render_maker_mark(
                                    &ag.maker,
                                    &ag.id,
                                    18.,
                                    ag.status == AoAgentStatus::Running && flowing,
                                    anim_ms,
                                    p,
                                    cx,
                                ))
                            })
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .truncate()
                                    .child(ag.name.clone()),
                            )
                            .when_some(state_dot, |d, ic| d.child(ic)),
                    )
                    .child(track_el)
                    .child(
                        div()
                            .w(px(len_w))
                            .flex_none()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .text_right()
                            .child(dur_label),
                    ),
                cx,
                move |this, _, cx| this.open_agent(ag_id_open.clone(), cx),
            ));

            // Expanded child LLM + Tool call spans
            let non_zero_tasks = ag.tasks.iter().filter(|t| t.duration_ms > 0).count();
            if non_zero_tasks > 0 {
                let mut sub_rows = div().w_full().flex().flex_col();
                for task in &ag.tasks {
                    if task.duration_ms == 0 {
                        continue;
                    }
                    let llm_dur = (task.duration_ms * 42) / 100;
                    let tool_dur = task.duration_ms.saturating_sub(llm_dur);
                    let sub_spans = [
                        (
                            format!("{}-llm", task.id),
                            AoSpanKind::Llm,
                            ag.model.clone(),
                            task.title.clone(),
                            task.start_ms,
                            llm_dur,
                            AoAgentStatus::Done,
                        ),
                        (
                            format!("{}-tool", task.id),
                            AoSpanKind::Tool,
                            task.tool_label.clone(),
                            task.result.clone(),
                            task.start_ms + llm_dur,
                            tool_dur,
                            task.status,
                        ),
                    ];
                    for (sp_id, sp_kind, sp_title, sp_detail, sp_s, sp_d, sp_st) in sub_spans {
                        let (sw_c, bg_c, ed_c) = if sp_st == AoAgentStatus::Failed {
                            (p.red, p.red_soft, p.red.opacity(0.40))
                        } else if sp_kind == AoSpanKind::Llm {
                            (p.purple, p.purple.opacity(0.14), p.purple.opacity(0.36))
                        } else {
                            (p.cyan, p.cyan.opacity(0.12), p.cyan.opacity(0.34))
                        };
                        let sf = (sp_s as f32 / range_ms as f32).clamp(0., 0.99);
                        let df = (sp_d as f32 / range_ms as f32).clamp(0.005, 1.0 - sf);
                        let ag_open = ag.id.clone();

                        sub_rows = sub_rows.child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("sp-{sp_id}")))
                                .role(Role::Row)
                                .tab_index(0)
                                .w_full()
                                .h(px(26.))
                                .px(px(10.))
                                .hover(move |s| s.bg(p.hover))
                                .flex()
                                .items_center()
                                .gap(px(col_gap))
                                .cursor_pointer()
                                .child(
                                    div()
                                        .w(px(name_w))
                                        .flex_none()
                                        .min_w_0()
                                        .pl(px(indent + 22.0))
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .child(
                                            div().size(px(8.)).flex_none().rounded(px(2.)).bg(sw_c),
                                        )
                                        .child(
                                            div()
                                                .flex_none()
                                                .text_size(px(12.5))
                                                .text_color(p.ink)
                                                .child(sp_title),
                                        )
                                        .when(!narrow, |d| {
                                            d.child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .text_size(px(11.5))
                                                    .text_color(p.muted)
                                                    .truncate()
                                                    .child(sp_detail),
                                            )
                                        }),
                                )
                                .child(
                                    div().relative().flex_1().h_full().min_w_0().child(
                                        div()
                                            .absolute()
                                            .top(px(6.))
                                            .left(relative(sf))
                                            .w(relative(df))
                                            .min_w(px(3.))
                                            .h(px(14.))
                                            .rounded(px(4.))
                                            .bg(bg_c)
                                            .border_1()
                                            .border_color(ed_c),
                                    ),
                                )
                                .child(
                                    div()
                                        .w(px(len_w))
                                        .flex_none()
                                        .text_size(px(11.5))
                                        .text_color(p.muted)
                                        .text_right()
                                        .child(format_duration_ms(sp_d)),
                                ),
                            cx,
                            move |this, _, cx| {
                                this.inspector_tab = AoInspectorTab::Payloads;
                                this.open_agent(ag_open.clone(), cx);
                            },
                        ));
                    }
                }
                let est_h = (non_zero_tasks * 2) as f32 * 26.0;
                let sub_el = sub_rows.into_any_element();
                rows_wrap = rows_wrap.child(pro_fold(
                    pro_eid(&self.id, format!("tr-fold-{}", ag.id)),
                    is_expanded,
                    est_h,
                    0.,
                    move || sub_el,
                ));
            }
        }

        div()
            .w_full()
            .flex()
            .flex_col()
            .child(axis)
            .child(rows_wrap)
    }

    #[allow(clippy::too_many_arguments)]
    fn render_cost_tab(
        &self,
        ordered: &[PositionedAgent],
        anim_ms: f32,
        flowing: bool,
        total_cost: f64,
        narrow: bool,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let header = div()
            .h(px(28.))
            .px(px(14.))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(8.))
            .text_size(px(10.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(p.muted)
            .child(div().flex_1().min_w_0().child(tr(cx, "AGENT", "智能体")))
            .when(!narrow, |d| {
                d.child(div().w(px(112.)).flex_none().child(tr(cx, "MODEL", "模型")))
            })
            .child(
                div()
                    .w(px(64.))
                    .flex_none()
                    .text_right()
                    .child(tr(cx, "IN", "输入")),
            )
            .child(
                div()
                    .w(px(64.))
                    .flex_none()
                    .text_right()
                    .child(tr(cx, "OUT", "输出")),
            )
            .when(!narrow, |d| {
                d.child(
                    div()
                        .w(px(68.))
                        .flex_none()
                        .text_right()
                        .child(tr(cx, "CACHED", "缓存")),
                )
            })
            .child(
                div()
                    .w(px(64.))
                    .flex_none()
                    .text_right()
                    .child(tr(cx, "COST", "自身费用")),
            )
            .when(!narrow, |d| {
                d.child(div().w(px(86.)).flex_none().child(tr(cx, "SHARE", "占比")))
                    .child(div().w(px(68.)).flex_none().text_right().child(tr(
                        cx,
                        "BRANCH",
                        "分支总计",
                    )))
            });

        let mut rows = div().flex().flex_col();
        let mut sum_in = 0u64;
        let mut sum_out = 0u64;
        let mut sum_cached = 0u64;

        for pos in ordered {
            let ag = &self.agents[pos.index];
            sum_in += ag.tokens_in;
            sum_out += ag.tokens_out;
            sum_cached += ag.tokens_cached;

            let has_children = self
                .agents
                .iter()
                .any(|c| c.parent.as_ref() == Some(&ag.id));
            let b_cost = branch_cost(&self.agents, &ag.id);
            let share = if total_cost > 0.001 {
                (ag.cost / total_cost) as f32
            } else {
                0.0
            };
            let indent = (pos.depth.min(if narrow { 2 } else { 4 }) as f32) * 14.0;
            let aid = ag.id.clone();

            rows = rows.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("cost-row-{}", ag.id)))
                    .role(Role::Row)
                    .tab_index(0)
                    .h(px(30.))
                    .px(px(14.))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.5))
                    .cursor_pointer()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .pl(px(indent))
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .child(Self::render_maker_mark(
                                &ag.maker,
                                &ag.id,
                                18.,
                                ag.status == AoAgentStatus::Running && flowing,
                                anim_ms,
                                p,
                                cx,
                            ))
                            .child(
                                div()
                                    .min_w_0()
                                    .text_color(p.ink)
                                    .truncate()
                                    .child(ag.name.clone()),
                            ),
                    )
                    .when(!narrow, |d| {
                        d.child(
                            div()
                                .w(px(112.))
                                .flex_none()
                                .text_color(p.muted)
                                .truncate()
                                .child(ag.model.clone()),
                        )
                    })
                    .child(
                        div()
                            .w(px(64.))
                            .flex_none()
                            .text_right()
                            .text_color(p.text)
                            .child(format_tokens_k(ag.tokens_in)),
                    )
                    .child(
                        div()
                            .w(px(64.))
                            .flex_none()
                            .text_right()
                            .text_color(p.text)
                            .child(format_tokens_k(ag.tokens_out)),
                    )
                    .when(!narrow, |d| {
                        d.child(
                            div()
                                .w(px(68.))
                                .flex_none()
                                .text_right()
                                .text_color(p.text)
                                .child(format_tokens_k(ag.tokens_cached)),
                        )
                    })
                    .child(
                        div()
                            .w(px(64.))
                            .flex_none()
                            .text_right()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(if ag.cost > 0.001 {
                                format!("${:.2}", ag.cost)
                            } else {
                                "–".to_string()
                            }),
                    )
                    .when(!narrow, |d| {
                        d.child(
                            div()
                                .w(px(86.))
                                .flex_none()
                                .px(px(4.))
                                .child(pro_spring_bar(
                                    pro_eid(&self.id, format!("cost-share-{}", ag.id)),
                                    if share > 0.005 {
                                        share.clamp(0.03, 1.0)
                                    } else {
                                        0.0
                                    },
                                    6.,
                                    3.,
                                    p.chip,
                                    p.muted,
                                )),
                        )
                        .child(
                            div()
                                .w(px(68.))
                                .flex_none()
                                .text_right()
                                .text_color(p.muted)
                                .child(if has_children && b_cost > 0.001 {
                                    format!("${b_cost:.2}")
                                } else {
                                    "–".to_string()
                                }),
                        )
                    }),
                cx,
                move |this, _, cx| this.open_agent(aid.clone(), cx),
            ));
        }

        let footer = div()
            .h(px(32.))
            .px(px(14.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(8.))
            .text_size(px(12.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(p.ink)
            .child(div().flex_1().child(tr(cx, "Total", "合计")))
            .when(!narrow, |d| d.child(div().w(px(112.)).flex_none()))
            .child(
                div()
                    .w(px(64.))
                    .flex_none()
                    .text_right()
                    .child(format_tokens_k(sum_in)),
            )
            .child(
                div()
                    .w(px(64.))
                    .flex_none()
                    .text_right()
                    .child(format_tokens_k(sum_out)),
            )
            .when(!narrow, |d| {
                d.child(
                    div()
                        .w(px(68.))
                        .flex_none()
                        .text_right()
                        .child(format_tokens_k(sum_cached)),
                )
            })
            .child(
                div()
                    .w(px(64.))
                    .flex_none()
                    .text_right()
                    .child(format!("${total_cost:.2}")),
            )
            .when(!narrow, |d| {
                d.child(div().w(px(86.)).flex_none())
                    .child(div().w(px(68.)).flex_none())
            });

        div()
            .w_full()
            .flex()
            .flex_col()
            .child(header)
            .child(rows)
            .child(footer)
    }

    fn render_events_tab(
        &self,
        vis_events: &[AoEvent],
        narrow: bool,
        p: &ProPalette,
        cx: &Context<Self>,
    ) -> Div {
        let mut list = div().px(px(6.)).py(px(4.)).flex().flex_col().gap(px(1.));
        for (ei, ev) in vis_events.iter().rev().enumerate() {
            let (ic, tag_lbl, tag_bg, tag_fg) = match ev.kind {
                AoEventKind::Error => (
                    ProIcon::AlertTriangle,
                    tr(cx, "error", "错误"),
                    p.red_soft,
                    p.red_ink,
                ),
                AoEventKind::Approval => (
                    ProIcon::Clock,
                    tr(cx, "approval", "待确认"),
                    p.amber_soft,
                    p.amber_ink,
                ),
                AoEventKind::Retry => (ProIcon::Refresh, tr(cx, "retry", "重试"), p.chip, p.muted),
                AoEventKind::Handoff => (
                    ProIcon::GitBranch,
                    tr(cx, "handoff", "委派"),
                    p.chip,
                    p.muted,
                ),
                AoEventKind::Tool => (ProIcon::Bolt, tr(cx, "tool", "工具"), p.chip, p.muted),
                AoEventKind::Done => (ProIcon::Check, tr(cx, "done", "完成"), p.chip, p.muted),
                AoEventKind::Start => (ProIcon::Play, tr(cx, "start", "启动"), p.chip, p.muted),
            };
            let aid = ev.agent_id.clone();

            list = list.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("ev-row-{ei}")))
                    .role(Role::Button)
                    .tab_index(0)
                    .w_full()
                    .h(px(32.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .text_size(px(13.))
                    .cursor_pointer()
                    .child(
                        div()
                            .w(px(42.))
                            .flex_none()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(format_clock_ms(ev.at_ms)),
                    )
                    .child(pro_icon(ic, 14., tag_fg))
                    .when(!narrow, |d| {
                        d.child(
                            div()
                                .w(px(136.))
                                .flex_none()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .truncate()
                                .child(ev.agent_name.clone()),
                        )
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(p.text)
                            .truncate()
                            .child(ev.text.clone()),
                    )
                    .when(!narrow, |d| {
                        d.child(
                            div()
                                .h(px(18.))
                                .px(px(6.))
                                .flex_none()
                                .rounded(px(4.))
                                .bg(tag_bg)
                                .flex()
                                .items_center()
                                .text_size(px(11.))
                                .text_color(tag_fg)
                                .child(tag_lbl),
                        )
                    }),
                cx,
                move |this, _, cx| this.open_agent(aid.clone(), cx),
            ));
        }
        list
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let anim_ms = self.boot_instant.elapsed().as_secs_f32() * 1000.0;
        let skel_phase = (anim_ms / 1600.0).fract();

        if self.is_flowing() && self.tick_origin.is_none() {
            self.tick_origin = Some(Instant::now());
        }
        if (self.is_flowing() || self.loading) && !reduced {
            w.request_animation_frame();
        }

        let now_ms = self.current_elapsed_ms();
        let (ordered, world_w, _) = layout_agent_tree(&self.agents);
        let narrow = self.measured_width.get() < 640.0;

        let canvas_area = self.render_canvas_area(
            &ordered, world_w, now_ms, anim_ms, skel_phase, narrow, &p, cx,
        );
        let run_panel =
            self.render_run_panel(&ordered, now_ms, anim_ms, skel_phase, narrow, &p, cx);

        let w_cell = self.measured_width.clone();
        let h_cell = self.measured_height.clone();

        div()
            .id(pro_eid(&self.id, "root"))
            .track_focus(&self.focus)
            .key_context("AgentOrchestrationScreen")
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| {
                if let Some(drag) = this.dragging {
                    match drag {
                        DragKind::CanvasPan {
                            start_mouse_x,
                            start_mouse_y,
                            orig_pan_x,
                            orig_pan_y,
                        } => {
                            let dx = f32::from(ev.position.x) as i32 - start_mouse_x;
                            let dy = f32::from(ev.position.y) as i32 - start_mouse_y;
                            this.pan_x = orig_pan_x + dx as f32;
                            this.pan_y = orig_pan_y + dy as f32;
                        }
                        DragKind::ResizeRunPanel {
                            start_mouse_y,
                            orig_h,
                        } => {
                            let dy = f32::from(ev.position.y) as i32 - start_mouse_y;
                            this.run_panel_height = (orig_h - dy as f32).clamp(132.0, 420.0);
                        }
                    }
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.dragging.take().is_some() {
                        cx.notify();
                    }
                }),
            )
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                let key = ev.keystroke.key.as_str();
                if ev.keystroke.modifiers.alt {
                    let step = if ev.keystroke.modifiers.shift {
                        72.0
                    } else {
                        28.0
                    };
                    match key {
                        "left" => this.pan_x += step,
                        "right" => this.pan_x -= step,
                        "up" => this.pan_y += step,
                        "down" => this.pan_y -= step,
                        _ => return,
                    }
                    cx.notify();
                    return;
                }

                let (ord, _, _) = layout_agent_tree(&this.agents);
                if ord.is_empty() {
                    return;
                }
                match key {
                    "escape" => {
                        if this.project_menu_open || this.run_menu_open {
                            this.project_menu_open = false;
                            this.run_menu_open = false;
                            cx.notify();
                        } else {
                            this.close_agent(cx);
                        }
                    }
                    "down" | "j" => {
                        let next = (this.focused_agent_idx + 1).min(ord.len() - 1);
                        this.focused_agent_idx = next;
                        if this.selected_agent.is_some() {
                            let id = this.agents[ord[next].index].id.clone();
                            this.open_agent(id, cx);
                        } else {
                            cx.notify();
                        }
                    }
                    "up" | "k" => {
                        let prev = this.focused_agent_idx.saturating_sub(1);
                        this.focused_agent_idx = prev;
                        if this.selected_agent.is_some() {
                            let id = this.agents[ord[prev].index].id.clone();
                            this.open_agent(id, cx);
                        } else {
                            cx.notify();
                        }
                    }
                    "home" => {
                        this.focused_agent_idx = 0;
                        cx.notify();
                    }
                    "end" => {
                        this.focused_agent_idx = ord.len() - 1;
                        cx.notify();
                    }
                    "enter" | "space" => {
                        let id = this.agents[ord[this.focused_agent_idx].index].id.clone();
                        this.open_agent(id, cx);
                    }
                    "+" | "=" => {
                        this.zoom = (this.zoom + 0.10).clamp(0.45, 1.25);
                        cx.notify();
                    }
                    "-" => {
                        this.zoom = (this.zoom - 0.10).clamp(0.45, 1.25);
                        cx.notify();
                    }
                    "1" if ev.keystroke.modifiers.shift => {
                        this.fit_to_view(cx);
                    }
                    _ => {}
                }
            }))
            .relative()
            .size_full()
            .min_h_0()
            .bg(p.panel)
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                canvas(
                    move |b, window, _| {
                        let nw = f32::from(b.size.width);
                        let nh = f32::from(b.size.height);
                        let mut changed = false;
                        if nw > 10. && (w_cell.get() - nw).abs() > 1.0 {
                            w_cell.set(nw);
                            changed = true;
                        }
                        if nh > 10. && (h_cell.get() - nh).abs() > 1.0 {
                            h_cell.set(nh);
                            changed = true;
                        }
                        if changed {
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .size_0(),
            )
            .child(canvas_area)
            .child(run_panel)
    }
}

impl Render for AgentOrchestrationScreen {
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
        AoAgent, AoAgentStatus, branch_cost, default_market_research_agents, format_clock_ms,
        format_duration_ms, format_tokens_k, layout_agent_tree,
    };

    #[test]
    fn default_orchestration_hierarchy_and_costs_reconcile() {
        let agents = default_market_research_agents();
        assert_eq!(agents.len(), 17);

        let done_count = agents
            .iter()
            .filter(|a| a.status == AoAgentStatus::Done)
            .count();
        assert_eq!(done_count, 10);

        let total_tokens: u64 = agents.iter().map(AoAgent::total_tokens).sum();
        assert_eq!(format_tokens_k(total_tokens), "579.2K");

        let root_branch = branch_cost(&agents, "orchestrator");
        assert_eq!(format!("${root_branch:.2}"), "$2.17");
        assert_eq!(
            format!("${:.2}", branch_cost(&agents, "researcher")),
            "$0.41"
        );
        assert_eq!(
            format!("${:.2}", branch_cost(&agents, "data-analyst")),
            "$0.62"
        );
        assert_eq!(format!("${:.2}", branch_cost(&agents, "writer")), "$0.39");
        assert_eq!(
            format!("${:.2}", branch_cost(&agents, "fact-checker")),
            "$0.13"
        );
    }

    #[test]
    fn tree_layout_places_root_centered_over_five_columns() {
        let agents = default_market_research_agents();
        let (ordered, world_w, world_h) = layout_agent_tree(&agents);
        assert_eq!(ordered.len(), 17);
        assert_eq!(ordered[0].depth, 0);
        assert_eq!(ordered[0].x, 590.0);
        assert_eq!(ordered[0].y, 0.0);
        assert_eq!(world_w, 1420.0);
        assert_eq!(world_h, 892.0);

        assert_eq!(format_clock_ms(95_000), "01:35");
        assert_eq!(format_duration_ms(7_100), "7.1s");
        assert_eq!(format_duration_ms(95_000), "1m 35s");
    }
}
