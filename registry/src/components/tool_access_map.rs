//! Multi-agent radial/hub tool access map with live call beams, failure badges, and per-call replay inspector.
//!
//! Displays agent tabs above a hub diagram placing the selected agent in the center and every run
//! tool on flanking rails—distinguishing `Active now` (pulsing beam), `Used`, `Failed` (red),
//! `Not used yet` (idle), and `No access` (dimmed dashed wire). Clicking any accessible tool opens
//! its call log where payloads can be edited and replayed inline.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_app_tile, pro_btn, pro_card_shadow, pro_check_draw,
        pro_connector_pulse, pro_eid, pro_empty_state, pro_icon_btn, pro_mark_selected,
        pro_mix_color, pro_pill_indicator, pro_presence, pro_rolling_number, pro_selection_sensor,
        pro_shimmer_text, pro_skeleton_bar, pro_spring_f32, pro_tactile, pro_toast_banner,
        pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::{UiLanguage, activate},
        editor::{EditEvent, Editor, EditorShortcutEvent, InputMode},
        family::disabled_motion,
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

const HUB_MARK_SVG: &str = r#"<path d="M8.4 12H4.2M15.6 12h4.2M12 8.4V4.5M12 15.6v3.9" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" opacity="0.5"/><rect x="7.8" y="7.8" width="8.4" height="8.4" rx="2.2" fill="currentColor"/><circle cx="3.2" cy="12" r="2" fill="currentColor"/><circle cx="20.8" cy="12" r="2" fill="currentColor"/>"#;

fn spinning_ring(side: f32, stroke: f32, color: Hsla, elapsed_s: f32, active: bool) -> AnyElement {
    let angle = if active {
        (elapsed_s / 0.85) * std::f32::consts::TAU
    } else {
        -std::f32::consts::FRAC_PI_2
    };
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let c = bounds.center();
            let r = (f32::from(bounds.size.width).min(f32::from(bounds.size.height)) - stroke)
                * 0.5_f32;
            if r <= 1.0 {
                return;
            }
            let mut ring = PathBuilder::stroke(px(stroke));
            for i in 0..=20usize {
                let a = (i as f32 / 20.0) * std::f32::consts::TAU;
                let pt = c + point(px(r * a.cos()), px(r * a.sin()));
                if i == 0 {
                    ring.move_to(pt);
                } else {
                    ring.line_to(pt);
                }
            }
            if let Ok(path) = ring.build() {
                window.paint_path(path, color.opacity(0.25));
            }
            let mut arc = PathBuilder::stroke(px(stroke));
            for i in 0..=7usize {
                let a = angle + (i as f32 / 7.0) * std::f32::consts::FRAC_PI_2;
                let pt = c + point(px(r * a.cos()), px(r * a.sin()));
                if i == 0 {
                    arc.move_to(pt);
                } else {
                    arc.line_to(pt);
                }
            }
            if let Ok(path) = arc.build() {
                window.paint_path(path, color);
            }
        },
    )
    .size(px(side))
    .flex_none()
    .into_any_element()
}

fn paint_dashed_elbow(
    p0: Point<Pixels>,
    p1: Point<Pixels>,
    color: Hsla,
    stroke_w: f32,
    w: &mut Window,
) {
    let x0 = f32::from(p0.x);
    let y0 = f32::from(p0.y);
    let x1 = f32::from(p1.x);
    let y1 = f32::from(p1.y);
    let xm = (x0 + x1) * 0.5_f32;
    let segs = [
        (point(px(x0), px(y0)), point(px(xm), px(y0))),
        (point(px(xm), px(y0)), point(px(xm), px(y1))),
        (point(px(xm), px(y1)), point(px(x1), px(y1))),
    ];
    for (a, b) in segs {
        let ax = f32::from(a.x);
        let ay = f32::from(a.y);
        let bx = f32::from(b.x);
        let by = f32::from(b.y);
        let dx = bx - ax;
        let dy = by - ay;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1.5 {
            continue;
        }
        let step = 7.0_f32;
        let n = (len / step).ceil() as usize;
        for i in 0..n {
            let t0 = (i as f32 * step) / len;
            let t1 = ((i as f32 * step + 3.2) / len).min(1.0);
            if t1 <= t0 {
                continue;
            }
            let mut pb = PathBuilder::stroke(px(stroke_w));
            pb.move_to(point(px(ax + dx * t0), px(ay + dy * t0)));
            pb.line_to(point(px(ax + dx * t1), px(ay + dy * t1)));
            if let Ok(path) = pb.build() {
                w.paint_path(path, color);
            }
        }
    }
}

fn paint_joint(pt: Point<Pixels>, stroke_col: Hsla, card_bg: Hsla, w: &mut Window) {
    let r = 2.8_f32;
    let rect = Bounds::new(
        point(pt.x - px(r), pt.y - px(r)),
        size(px(r * 2.0), px(r * 2.0)),
    );
    w.paint_quad(quad(
        rect,
        Corners::all(px(r)),
        card_bg,
        Edges::all(px(1.5)),
        stroke_col,
        BorderStyle::Solid,
    ));
}

fn tool_glyph(id: &str, name: &str, side: f32, fg: Hsla, cx: &App) -> AnyElement {
    if let Some(mark) = pro_app_artwork(name, side, cx) {
        return mark;
    }
    let ic = match id.to_ascii_lowercase().as_str() {
        "terminal" | "shell" => ProIcon::Terminal,
        "database" | "db" | "sql" => ProIcon::Database,
        "web" | "web-search" | "search" => ProIcon::Globe,
        "code" => ProIcon::Code,
        "mail" | "email" => ProIcon::Mail,
        _ => ProIcon::Bolt,
    };
    pro_icon(ic, side, fg)
}

/// Execution state of a [`MapAgent`] or [`ToolCallRecord`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AccessRunStatus {
    /// Queued and not yet started.
    Queued,
    /// Actively executing right now.
    Running,
    /// Paused awaiting human input (`"Needs you"`).
    Waiting,
    /// Completed cleanly.
    #[default]
    Done,
    /// Terminated or returned an error.
    Failed,
}

/// Global catalog definition of a tool in [`ToolAccessMap`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessTool {
    /// Stable tool identifier (`"github"`, `"terminal"`, `"stripe"`).
    pub id: SharedString,
    /// Display name rendered on the card and inspector header.
    pub name: SharedString,
    /// Short one-line capability description shown in the call drawer header.
    pub description: SharedString,
}

impl AccessTool {
    /// Construct an [`AccessTool`] with `id`, `name`, and `description`.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        description: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
        }
    }
}

/// Individual invocation record inside an [`AgentToolAccess`] list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCallRecord {
    /// Stable call identifier.
    pub id: SharedString,
    /// Status of this invocation (`Done`, `Running`, `Failed`).
    pub status: AccessRunStatus,
    /// JSON or CLI input payload.
    pub input: SharedString,
    /// Returned output payload when `Done`, or error message when `Failed`.
    pub output: SharedString,
    /// Formatted timestamp (`"0:42"`).
    pub at: SharedString,
    /// Duration in milliseconds, if finished.
    pub duration_ms: Option<u32>,
    /// Whether this call was triggered via the inspector's `Replay` button.
    pub replay: bool,
}

impl ToolCallRecord {
    /// Construct a completed [`ToolCallRecord`].
    pub fn done(
        id: impl Into<SharedString>,
        input: impl Into<SharedString>,
        output: impl Into<SharedString>,
        at: impl Into<SharedString>,
        duration_ms: u32,
    ) -> Self {
        Self {
            id: id.into(),
            status: AccessRunStatus::Done,
            input: input.into(),
            output: output.into(),
            at: at.into(),
            duration_ms: Some(duration_ms),
            replay: false,
        }
    }

    /// Construct an actively running [`ToolCallRecord`].
    pub fn running(
        id: impl Into<SharedString>,
        input: impl Into<SharedString>,
        at: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            status: AccessRunStatus::Running,
            input: input.into(),
            output: SharedString::default(),
            at: at.into(),
            duration_ms: None,
            replay: false,
        }
    }

    /// Construct a failed [`ToolCallRecord`] with an `error` message.
    pub fn failed(
        id: impl Into<SharedString>,
        input: impl Into<SharedString>,
        error: impl Into<SharedString>,
        at: impl Into<SharedString>,
        duration_ms: u32,
    ) -> Self {
        Self {
            id: id.into(),
            status: AccessRunStatus::Failed,
            input: input.into(),
            output: error.into(),
            at: at.into(),
            duration_ms: Some(duration_ms),
            replay: false,
        }
    }

    /// Mark whether this call originated from an interactive replay.
    pub fn with_replay(mut self, replay: bool) -> Self {
        self.replay = replay;
        self
    }
}

/// Per-agent grant and call log for one [`AccessTool`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentToolAccess {
    /// Matching [`AccessTool::id`].
    pub tool_id: SharedString,
    /// Chronological call records (oldest first; empty means `"Not used yet"`).
    pub calls: Vec<ToolCallRecord>,
}

impl AgentToolAccess {
    /// Grant access to `tool_id` with `calls`.
    pub fn new(tool_id: impl Into<SharedString>, calls: Vec<ToolCallRecord>) -> Self {
        Self {
            tool_id: tool_id.into(),
            calls,
        }
    }
}

/// One agent tab and hub configuration in [`ToolAccessMap`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapAgent {
    /// Stable agent identifier.
    pub id: SharedString,
    /// Display name (`"Coder"`, `"Researcher"`).
    pub name: SharedString,
    /// Model name (`"GPT-5.4"`, `"Claude Sonnet 4.5"`, `"Gemini 2.5 Pro"`).
    pub model: SharedString,
    /// Current execution status badge.
    pub status: AccessRunStatus,
    /// Current assigned task summary shown on the hub card.
    pub task: SharedString,
    /// Tools this agent is permitted to call (any global tool omitted here renders as `"No access"`).
    pub tools: Vec<AgentToolAccess>,
}

impl MapAgent {
    /// Construct a [`MapAgent`] specification.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        model: impl Into<SharedString>,
        status: AccessRunStatus,
        task: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            model: model.into(),
            status,
            task: task.into(),
            tools: Vec::new(),
        }
    }

    /// Set the granted tools and their call histories for this agent.
    pub fn with_tools(mut self, tools: Vec<AgentToolAccess>) -> Self {
        self.tools = tools;
        self
    }
}

/// Typed events emitted by [`ToolAccessMap`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolAccessMapEvent {
    /// Active agent tab changed.
    AgentChanged(SharedString),
    /// A tool's call inspector opened (`Some(tool_id)`) or closed (`None`).
    ToolOpened(Option<SharedString>),
    /// Selected call inside the open inspector changed.
    CallSelected {
        tool_id: SharedString,
        call_id: SharedString,
    },
    /// A call was replayed with original or modified input.
    Replayed {
        agent_id: SharedString,
        tool_id: SharedString,
        source_call_id: SharedString,
        new_call_id: SharedString,
        input: SharedString,
    },
}

fn default_tools() -> Vec<AccessTool> {
    vec![
        AccessTool::new(
            "github",
            "GitHub",
            "Branches, pull requests, and repository commits",
        ),
        AccessTool::new("terminal", "Terminal", "Sandboxed workspace shell commands"),
        AccessTool::new(
            "stripe",
            "Stripe",
            "Prices, products, and checkout test sessions",
        ),
        AccessTool::new("figma", "Figma", "Design tokens and component frame specs"),
        AccessTool::new("database", "Database", "Read-only staging SQL queries"),
        AccessTool::new(
            "web",
            "Web search",
            "Live documentation and package lookups",
        ),
        AccessTool::new("notion", "Notion", "Product specs and launch checklists"),
        AccessTool::new("slack", "Slack", "Channel updates and review pings"),
        AccessTool::new(
            "hubspot",
            "HubSpot",
            "Enterprise pipeline and account records",
        ),
    ]
}

fn default_agents() -> Vec<MapAgent> {
    vec![
        MapAgent::new(
            "researcher",
            "Researcher",
            "Claude Opus 4.6",
            AccessRunStatus::Done,
            "Audit checkout docs and collect Stripe Billing webhook edge cases",
        )
        .with_tools(vec![
            AgentToolAccess::new(
                "web",
                vec![
                    ToolCallRecord::done(
                        "r1",
                        "{\n  \"query\": \"stripe checkout session subscription_data trial_period_days\"\n}",
                        "{\n  \"hits\": 6,\n  \"top\": \"docs.stripe.com/api/checkout/sessions/create\"\n}",
                        "0:08",
                        410,
                    ),
                    ToolCallRecord::done(
                        "r2",
                        "{\n  \"query\": \"idempotency keys webhook retry backoff\"\n}",
                        "{\n  \"hits\": 4,\n  \"top\": \"docs.stripe.com/webhooks/best-practices\"\n}",
                        "0:19",
                        360,
                    ),
                ],
            ),
            AgentToolAccess::new(
                "notion",
                vec![ToolCallRecord::done(
                    "r3",
                    "{\n  \"page\": \"Q4 Pricing Launch Spec\"\n}",
                    "{\n  \"sections\": [\"Tiers\", \"Annual discount\", \"Trial banner\"]\n}",
                    "0:27",
                    290,
                )],
            ),
            AgentToolAccess::new("github", Vec::new()),
            AgentToolAccess::new("figma", Vec::new()),
        ]),
        MapAgent::new(
            "coder",
            "Coder",
            "GPT-5.4",
            AccessRunStatus::Running,
            "Build the pricing page and wire up checkout",
        )
        .with_tools(vec![
            AgentToolAccess::new(
                "github",
                vec![
                    ToolCallRecord::done(
                        "c1",
                        "{\n  \"action\": \"create_branch\",\n  \"branch\": \"feat/pricing-checkout\"\n}",
                        "{\n  \"ref\": \"refs/heads/feat/pricing-checkout\",\n  \"sha\": \"9c4b1e2\"\n}",
                        "0:12",
                        450,
                    ),
                    ToolCallRecord::done(
                        "c2",
                        "{\n  \"action\": \"commit_files\",\n  \"files\": [\"app/pricing/page.tsx\", \"lib/stripe.ts\"]\n}",
                        "{\n  \"commit\": \"a81d04f\",\n  \"changed\": 2\n}",
                        "0:34",
                        520,
                    ),
                ],
            ),
            AgentToolAccess::new(
                "terminal",
                vec![
                    ToolCallRecord::done(
                        "c3",
                        "{\n  \"cmd\": \"pnpm typecheck\"\n}",
                        "{\n  \"exit_code\": 0,\n  \"stdout\": \"Checked 84 files in 1.4s\"\n}",
                        "0:41",
                        1420,
                    ),
                    ToolCallRecord::running(
                        "c4",
                        "{\n  \"cmd\": \"pnpm test --filter checkout\"\n}",
                        "0:52",
                    ),
                ],
            ),
            AgentToolAccess::new(
                "stripe",
                vec![ToolCallRecord::done(
                    "c5",
                    "{\n  \"action\": \"list_prices\",\n  \"product\": \"prod_team_tier\"\n}",
                    "{\n  \"monthly\": \"price_1Q9m24\",\n  \"annual\": \"price_1Q9mYr\"\n}",
                    "0:24",
                    730,
                )],
            ),
            AgentToolAccess::new(
                "figma",
                vec![ToolCallRecord::done(
                    "c6",
                    "{\n  \"node_id\": \"418:2091\",\n  \"frame\": \"Pricing / Desktop\"\n}",
                    "{\n  \"columns\": 3,\n  \"radius\": 8,\n  \"gap\": 16\n}",
                    "0:06",
                    1200,
                )],
            ),
            AgentToolAccess::new(
                "database",
                vec![ToolCallRecord::failed(
                    "c7",
                    "{\n  \"sql\": \"ALTER TABLE subscriptions ADD COLUMN seat_cap INT\"\n}",
                    "ERROR 42501: permission denied — staging connection is read-only",
                    "0:46",
                    280,
                )],
            ),
            AgentToolAccess::new("web", Vec::new()),
        ]),
        MapAgent::new(
            "writer",
            "Writer",
            "Claude Sonnet 4.5",
            AccessRunStatus::Waiting,
            "Draft changelog and customer announcement for #launch-review",
        )
        .with_tools(vec![
            AgentToolAccess::new(
                "notion",
                vec![ToolCallRecord::done(
                    "w1",
                    "{\n  \"action\": \"create_draft\",\n  \"title\": \"Pricing 2.0 Changelog\"\n}",
                    "{\n  \"doc_id\": \"ntn_892a\",\n  \"words\": 410\n}",
                    "1:04",
                    610,
                )],
            ),
            AgentToolAccess::new(
                "slack",
                vec![ToolCallRecord::done(
                    "w2",
                    "{\n  \"channel\": \"#launch-review\",\n  \"preview_only\": true\n}",
                    "{\n  \"queued_for_approval\": true\n}",
                    "1:12",
                    340,
                )],
            ),
            AgentToolAccess::new("figma", Vec::new()),
        ]),
        MapAgent::new(
            "analyst",
            "Analyst",
            "Gemini 2.5 Pro",
            AccessRunStatus::Failed,
            "Join Stripe trial conversions with HubSpot enterprise deals",
        )
        .with_tools(vec![
            AgentToolAccess::new(
                "database",
                vec![ToolCallRecord::done(
                    "a1",
                    "{\n  \"sql\": \"SELECT plan, COUNT(*) FROM trials GROUP BY 1\"\n}",
                    "{\n  \"rows\": [[\"pro\", 312], [\"team\", 94]]\n}",
                    "0:18",
                    310,
                )],
            ),
            AgentToolAccess::new(
                "hubspot",
                vec![ToolCallRecord::failed(
                    "a2",
                    "{\n  \"endpoint\": \"/crm/v3/objects/deals/search\"\n}",
                    "HTTP 401: OAuth token expired for workspace lumen-prod",
                    "0:29",
                    490,
                )],
            ),
            AgentToolAccess::new("stripe", Vec::new()),
        ]),
    ]
}

fn format_duration(ms: u32) -> String {
    if ms >= 1000 {
        format!("{:.1}s", ms as f32 / 1000.0)
    } else {
        format!("{ms}ms")
    }
}

/// Radial/hub map of an agent and every tool in the run, with call history inspection and inline replay.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::tool_access_map::{ToolAccessMap, ToolAccessMapEvent};
///
/// let map = cx.new(|cx| {
///     ToolAccessMap::new("tool-access-map", cx)
///         .with_default_agent("coder")
/// });
/// cx.subscribe(&map, |_, event: &ToolAccessMapEvent, _| {
///     let _ = event;
/// }).detach();
/// # }
/// ```
pub struct ToolAccessMap {
    id: ElementId,
    focus: FocusHandle,
    title: SharedString,
    tools: Vec<AccessTool>,
    agents: Vec<MapAgent>,
    active_agent_id: SharedString,
    hovered_tool_id: Option<SharedString>,
    hovered_agent_hub: bool,
    open_tool_id: Option<SharedString>,
    last_open_tool_id: Option<SharedString>,
    selected_call_id: Option<SharedString>,
    input_editor: Option<Entity<Editor>>,
    editor_seeded_key: Option<SharedString>,
    edited_input: Option<SharedString>,
    pending_replay: Option<(SharedString, SharedString, SharedString, Instant)>,
    replay_seq: usize,
    boot_at: Instant,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
}

impl ToolAccessMap {
    /// Create a [`ToolAccessMap`] with 4 agents (`Coder` active) and 9 tools showing `Used`, `Running`, `Failed`, `Not used yet`, and `No access`.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: "Tool access".into(),
            tools: default_tools(),
            agents: default_agents(),
            active_agent_id: "coder".into(),
            hovered_tool_id: None,
            hovered_agent_hub: false,
            open_tool_id: None,
            last_open_tool_id: None,
            selected_call_id: None,
            input_editor: None,
            editor_seeded_key: None,
            edited_input: None,
            pending_replay: None,
            replay_seq: 0,
            boot_at: Instant::now(),
            measured_width: Rc::new(Cell::new(680.0)),
            toast: None,
            loading: false,
            empty: false,
        }
    }

    /// Override the top bar heading.
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Supply the global ordered list of [`AccessTool`] definitions.
    pub fn with_tools(mut self, tools: Vec<AccessTool>) -> Self {
        self.tools = tools;
        self
    }

    /// Supply the [`MapAgent`] entries shown in the top tab strip.
    pub fn with_agents(mut self, agents: Vec<MapAgent>) -> Self {
        if let Some(first) = agents.first()
            && !agents.iter().any(|a| a.id == self.active_agent_id)
        {
            self.active_agent_id = first.id.clone();
        }
        self.agents = agents;
        self
    }

    /// Set which agent id starts selected on the map.
    pub fn with_default_agent(mut self, agent_id: impl Into<SharedString>) -> Self {
        self.active_agent_id = agent_id.into();
        self
    }

    /// Toggle skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Currently selected agent id.
    pub fn active_agent_id(&self) -> &SharedString {
        &self.active_agent_id
    }

    /// Currently open tool id in the call inspector, if any.
    pub fn open_tool_id(&self) -> Option<&SharedString> {
        self.open_tool_id.as_ref()
    }

    /// Switch the active agent tab.
    pub fn select_agent(&mut self, agent_id: impl Into<SharedString>, cx: &mut Context<Self>) {
        let aid: SharedString = agent_id.into();
        if self.active_agent_id != aid && self.agents.iter().any(|a| a.id == aid) {
            self.active_agent_id = aid.clone();
            self.open_tool_id = None;
            self.selected_call_id = None;
            self.editor_seeded_key = None;
            self.edited_input = None;
            cx.emit(ToolAccessMapEvent::AgentChanged(aid));
            cx.notify();
        }
    }

    /// Open a granted tool's call inspector (`None` closes it and returns to the hub map).
    pub fn open_tool(&mut self, tool_id: Option<SharedString>, cx: &mut Context<Self>) {
        if let Some(tid) = &tool_id {
            let allowed = self
                .current_agent()
                .is_some_and(|a| a.tools.iter().any(|t| &t.tool_id == tid));
            if !allowed {
                return;
            }
            self.last_open_tool_id = Some(tid.clone());
            self.selected_call_id = self
                .current_agent()
                .and_then(|a| a.tools.iter().find(|t| &t.tool_id == tid))
                .and_then(|t| t.calls.last().map(|c| c.id.clone()));
            self.editor_seeded_key = None;
            self.edited_input = None;
        }
        self.open_tool_id = tool_id.clone();
        cx.emit(ToolAccessMapEvent::ToolOpened(tool_id));
        cx.notify();
    }

    /// Select a specific call inside the currently open tool inspector.
    pub fn select_call(&mut self, call_id: impl Into<SharedString>, cx: &mut Context<Self>) {
        let cid: SharedString = call_id.into();
        if self.selected_call_id.as_ref() != Some(&cid) {
            self.selected_call_id = Some(cid.clone());
            self.editor_seeded_key = None;
            self.edited_input = None;
            if let Some(tid) = self.open_tool_id.clone() {
                cx.emit(ToolAccessMapEvent::CallSelected {
                    tool_id: tid,
                    call_id: cid,
                });
            }
            cx.notify();
        }
    }

    /// Replay the currently selected call (with optional overridden `input`) and append the new call record.
    pub fn replay_selected_call(
        &mut self,
        override_input: Option<SharedString>,
        cx: &mut Context<Self>,
    ) -> Option<SharedString> {
        let aid = self.active_agent_id.clone();
        let tid = self.open_tool_id.clone()?;
        let agent = self.agents.iter_mut().find(|a| a.id == aid)?;
        let grant = agent.tools.iter_mut().find(|t| t.tool_id == tid)?;

        let source_call = if let Some(cid) = &self.selected_call_id {
            grant.calls.iter().find(|c| &c.id == cid).cloned()
        } else {
            grant.calls.last().cloned()
        };

        let payload: SharedString = override_input
            .or_else(|| self.edited_input.clone())
            .or_else(|| source_call.as_ref().map(|c| c.input.clone()))
            .unwrap_or_else(|| "{\n  \"ping\": true\n}".into());

        let source_id: SharedString = source_call
            .as_ref()
            .map(|c| c.id.clone())
            .unwrap_or_else(|| "manual".into());

        self.replay_seq += 1;
        let new_id: SharedString = format!("replay-{}", self.replay_seq).into();
        let reduced = disabled_motion(cx);
        let new_record = if reduced {
            ToolCallRecord::done(
                new_id.clone(),
                payload.clone(),
                "{\n  \"replayed\": true,\n  \"ok\": true\n}",
                "now",
                310,
            )
            .with_replay(true)
        } else {
            ToolCallRecord::running(new_id.clone(), payload.clone(), "now").with_replay(true)
        };

        grant.calls.push(new_record);
        self.selected_call_id = Some(new_id.clone());
        self.editor_seeded_key = None;
        self.edited_input = None;
        if !reduced {
            self.pending_replay = Some((aid.clone(), tid.clone(), new_id.clone(), Instant::now()));
        }
        self.toast = Some((
            tr(cx, "Replayed tool call", "已重新执行该次工具调用").into(),
            Instant::now(),
        ));
        cx.emit(ToolAccessMapEvent::Replayed {
            agent_id: aid,
            tool_id: tid,
            source_call_id: source_id,
            new_call_id: new_id.clone(),
            input: payload,
        });
        cx.notify();
        Some(new_id)
    }

    fn current_agent(&self) -> Option<&MapAgent> {
        self.agents
            .iter()
            .find(|a| a.id == self.active_agent_id)
            .or_else(|| self.agents.first())
    }

    fn ensure_input_editor(
        &mut self,
        seed_key: &SharedString,
        seed_text: &str,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<Editor> {
        let ph: SharedString = tr(cx, "JSON or command input…", "输入调用载荷…").into();
        if self.input_editor.is_none() {
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Multi, w, cx);
                e.set_placeholder(ph);
                e.set_command_submit(true);
                e.font_size = 11.5;
                e
            });
            cx.subscribe_in(&ed, w, |this, ent, ev: &EditEvent, _, cx| {
                if matches!(ev, EditEvent::Changed) {
                    this.edited_input = Some(ent.read(cx).value().to_owned().into());
                    cx.notify();
                }
            })
            .detach();
            cx.subscribe_in(&ed, w, |this, ent, ev: &EditorShortcutEvent, _, cx| {
                let EditorShortcutEvent::CommandSubmit = ev;
                let val: SharedString = ent.read(cx).value().to_owned().into();
                this.replay_selected_call(Some(val), cx);
            })
            .detach();
            self.input_editor = Some(ed);
        }
        let ed = self
            .input_editor
            .as_ref()
            .expect("initialized above")
            .clone();
        if self.editor_seeded_key.as_ref() != Some(seed_key) {
            ed.update(cx, |e, cx| e.set_value(seed_text, w, cx));
            self.editor_seeded_key = Some(seed_key.clone());
        }
        ed
    }

    fn localize_str<'a>(&self, s: &'a str, cx: &App) -> &'a str {
        match s {
            "Tool access" => tr(cx, "Tool access", "工具访问图谱"),
            "Researcher" => tr(cx, "Researcher", "调研代理"),
            "Coder" => tr(cx, "Coder", "编码代理"),
            "Writer" => tr(cx, "Writer", "撰写代理"),
            "Analyst" => tr(cx, "Analyst", "分析代理"),
            "Web search" => tr(cx, "Web search", "网页检索"),
            "Database" => tr(cx, "Database", "数据库"),
            "Terminal" => tr(cx, "Terminal", "终端"),
            "Build the pricing page and wire up checkout" => tr(
                cx,
                "Build the pricing page and wire up checkout",
                "构建新定价页面并接入结账流程",
            ),
            "Audit checkout docs and collect Stripe Billing webhook edge cases" => tr(
                cx,
                "Audit checkout docs and collect Stripe Billing webhook edge cases",
                "梳理结账文档并汇总 Stripe 订阅回调边界情况",
            ),
            "Draft changelog and customer announcement for #launch-review" => tr(
                cx,
                "Draft changelog and customer announcement for #launch-review",
                "起草更新日志与客户发布公告并提交 #launch-review",
            ),
            "Join Stripe trial conversions with HubSpot enterprise deals" => tr(
                cx,
                "Join Stripe trial conversions with HubSpot enterprise deals",
                "关联分析 Stripe 试用转化与 HubSpot 企业商机",
            ),
            other => other,
        }
    }
}

impl EventEmitter<ToolAccessMapEvent> for ToolAccessMap {}

impl Focusable for ToolAccessMap {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for ToolAccessMap {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);

        if self.loading {
            let phase = self.boot_at.elapsed().as_secs_f32() * 0.9;
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(pro_skeleton_bar(220., 18., 6., phase, cx))
                .child(pro_skeleton_bar(320., 28., 7., phase + 0.1, cx))
                .child(pro_skeleton_bar(660., 280., 10., phase + 0.2, cx).w_full())
                .into_any_element();
        }

        if self.empty || self.agents.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .rounded(px(12.))
                .bg(p.panel)
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::Shield,
                    tr(cx, "No agents or tools configured", "暂无智能体或工具映射"),
                    tr(
                        cx,
                        "Provide run agents and tools to inspect granted permissions and replay calls.",
                        "传入运行代理与工具清单即可查看权限状态并逐条重放调用。",
                    ),
                    Some((
                        pro_eid(&self.id, "restore-tam"),
                        tr(cx, "Load sample map", "加载示例图谱").into(),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            this.agents = default_agents();
                            this.tools = default_tools();
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        // Complete pending simulated replay after 0.85s
        if let Some((aid, tid, cid, started)) = self.pending_replay.clone() {
            if started.elapsed().as_secs_f32() >= 0.85 {
                if let Some(ag) = self.agents.iter_mut().find(|a| a.id == aid)
                    && let Some(gr) = ag.tools.iter_mut().find(|t| t.tool_id == tid)
                    && let Some(call) = gr.calls.iter_mut().find(|c| c.id == cid)
                {
                    call.status = AccessRunStatus::Done;
                    call.output =
                        "{\n  \"replayed\": true,\n  \"status\": 200,\n  \"verified\": true\n}"
                            .into();
                    call.duration_ms = Some(340);
                }
                self.pending_replay = None;
                cx.notify();
            } else if !reduced {
                w.request_animation_frame();
            }
        }

        let Some(agent) = self.current_agent().cloned() else {
            return div().id(self.id.clone()).into_any_element();
        };

        let accessible_count = self
            .tools
            .iter()
            .filter(|t| agent.tools.iter().any(|g| g.tool_id == t.id))
            .count();
        let total_tools = self.tools.len();
        let mut total_calls = 0usize;
        let mut running_calls = 0usize;
        for g in &agent.tools {
            total_calls += g.calls.len();
            running_calls += g
                .calls
                .iter()
                .filter(|c| c.status == AccessRunStatus::Running)
                .count();
        }

        if running_calls > 0 && !reduced {
            w.request_animation_frame();
        }

        let elapsed_s = self.boot_at.elapsed().as_secs_f32();
        let pulse_t = if reduced {
            None
        } else {
            Some(((elapsed_s / 1.30).fract()).clamp(0.0, 1.0))
        };

        let width_cell = self.measured_width.clone();
        let stage_w = width_cell.get().max(320.0);
        let narrow = stage_w < 540.0;

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let well_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.035)
        } else {
            hsla(0., 0., 0., 0.025)
        };
        let fill_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.07)
        } else {
            hsla(0., 0., 0., 0.05)
        };
        let wire_used: Hsla = if p.dark {
            rgb(0x0190fe).into()
        } else {
            rgb(0x83c7fe).into()
        };
        let wire_glint: Hsla = if p.dark {
            rgb(0xb9e0ff).into()
        } else {
            rgb(0x0190fe).into()
        };
        let wire_off: Hsla = if p.dark {
            rgb(0x3a3a3f).into()
        } else {
            rgb(0xd9d9dc).into()
        };
        let brand_bg: Hsla = if p.dark {
            rgba(0x0077ff3a).into()
        } else {
            rgba(0x008ff519).into()
        };
        let brand_fg: Hsla = if p.dark {
            rgb(0x70b8ff).into()
        } else {
            rgb(0x006dcb).into()
        };
        let focus_col = p.focus;
        let hover_border = p.line_strong;

        // 1. Header Bar with rolling tool & call counts
        let tools_sfx = if UiLanguage::current(cx) == UiLanguage::Chinese {
            format!(" / {total_tools} 个工具")
        } else {
            format!(" / {total_tools} tools")
        };
        let calls_sfx = if UiLanguage::current(cx) == UiLanguage::Chinese {
            " 次调用"
        } else if total_calls == 1 {
            " call"
        } else {
            " calls"
        };

        let head_bar = div()
            .w_full()
            .pb(px(10.))
            .flex()
            .items_center()
            .gap(px(8.))
            .child(pro_svg("0 0 24 24", HUB_MARK_SVG, 16., p.muted))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(self.localize_str(&self.title, cx).to_owned()),
            )
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .when(running_calls > 0, |d| {
                        d.child(
                            div()
                                .h(px(18.))
                                .pl(px(5.))
                                .pr(px(6.))
                                .rounded(px(4.))
                                .bg(brand_bg)
                                .text_color(brand_fg)
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .child(spinning_ring(9.5, 1.5, brand_fg, elapsed_s, !reduced))
                                .child(if narrow {
                                    running_calls.to_string()
                                } else if UiLanguage::current(cx) == UiLanguage::Chinese {
                                    format!("{running_calls} 项运行中")
                                } else {
                                    format!("{running_calls} running")
                                }),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .when(!narrow, |d| {
                                d.child(pro_rolling_number(
                                    pro_eid(&self.id, "tam-acc-cnt"),
                                    accessible_count as f64,
                                    0,
                                    "",
                                    &tools_sfx,
                                    12.,
                                    FontWeight::NORMAL,
                                    p.muted,
                                ))
                                .child(div().text_color(p.faint).child("·"))
                            })
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "tam-call-cnt"),
                                total_calls as f64,
                                0,
                                "",
                                calls_sfx,
                                12.,
                                FontWeight::NORMAL,
                                p.muted,
                            )),
                    ),
            );

        // 2. Agent Tabs Strip
        let tab_sensor = pro_selection_sensor();
        let mut agent_tabs = div()
            .id(pro_eid(&self.id, "agent-tabs"))
            .role(Role::TabList)
            .relative()
            .pb(px(6.))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(6.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "agent-tab-ind"),
                self.active_agent_id.clone(),
                tab_sensor.clone(),
                7.0,
                card_bg,
                Some(p.line_strong),
                true,
            ));

        for ag in &self.agents {
            let aid = ag.id.clone();
            let sel = aid == agent.id;
            let aid_click = aid.clone();
            let ag_name = self.localize_str(&ag.name, cx).to_owned();
            agent_tabs = agent_tabs.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("atab-{aid}")))
                    .role(Role::Tab)
                    .aria_selected(sel)
                    .tab_index(if sel { 0 } else { -1 })
                    .relative()
                    .h(px(28.))
                    .pl(px(5.))
                    .pr(px(10.))
                    .rounded(px(7.))
                    .bg(if sel { transparent_black() } else { fill_bg })
                    .focus_visible(move |s| s.border_2().border_color(focus_col))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .when(sel, |d| d.child(pro_mark_selected(&tab_sensor)))
                    .child(
                        div()
                            .size(px(18.))
                            .rounded(px(5.))
                            .bg(card_bg)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                pro_app_artwork(&ag.model, 11., cx)
                                    .unwrap_or_else(|| pro_icon(ProIcon::Sparkles, 10., p.text)),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if sel { p.strong } else { p.text })
                            .child(ag_name),
                    ),
                cx,
                move |this, _, cx| {
                    this.select_agent(aid_click.clone(), cx);
                },
            ));
        }

        // 3. Center Agent Hub Card
        let (st_label, st_bg, st_fg, st_spin) = match agent.status {
            AccessRunStatus::Running => (tr(cx, "Running", "运行中"), brand_bg, brand_fg, true),
            AccessRunStatus::Waiting => (
                tr(cx, "Needs you", "待确认"),
                p.amber_soft,
                p.amber_ink,
                false,
            ),
            AccessRunStatus::Done => (tr(cx, "Done", "已完成"), p.green_soft, p.green_ink, false),
            AccessRunStatus::Failed => (tr(cx, "Failed", "失败"), p.red_soft, p.red_ink, false),
            AccessRunStatus::Queued => (tr(cx, "Queued", "排队中"), fill_bg, p.text, false),
        };

        let agent_card_inner = pro_view_transition(
            pro_eid(&self.id, "hub-agent-vtx"),
            agent.id.clone(),
            div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .min_w_0()
                        .child(
                            div()
                                .size(px(24.))
                                .flex_none()
                                .rounded(px(6.))
                                .bg(card_bg)
                                .border_1()
                                .border_color(p.line)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    pro_app_artwork(&agent.model, 13., cx)
                                        .unwrap_or_else(|| pro_icon(ProIcon::Sparkles, 12., p.ink)),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(16.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .truncate()
                                        .child(self.localize_str(&agent.name, cx).to_owned()),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .line_height(px(14.))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(agent.model.clone()),
                                ),
                        )
                        .child(
                            div()
                                .h(px(18.))
                                .px(px(6.))
                                .self_start()
                                .rounded(px(4.))
                                .bg(st_bg)
                                .text_color(st_fg)
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .when(st_spin, |d| {
                                    d.child(spinning_ring(9.5, 1.5, st_fg, elapsed_s, !reduced))
                                })
                                .when(agent.status == AccessRunStatus::Done, |d| {
                                    d.child(pro_check_draw(
                                        pro_eid(&self.id, format!("hub-done-{}", agent.id)),
                                        true,
                                        12.0,
                                        false,
                                        st_fg,
                                    ))
                                })
                                .child(st_label),
                        ),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(p.text)
                        .line_clamp(2)
                        .child(self.localize_str(&agent.task, cx).to_owned()),
                ),
        );

        let agent_w = if narrow { stage_w } else { 206.0_f32 };
        let agent_hub_card = div()
            .id(pro_eid(&self.id, "hub-agent-card"))
            .w(px(agent_w))
            .p(px(10.))
            .rounded(px(10.))
            .bg(card_bg)
            .border_1()
            .border_color(if self.hovered_agent_hub {
                brand_fg.opacity(0.55)
            } else {
                p.line
            })
            .when(!p.dark, |d| d.shadow_xs())
            .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                if this.hovered_agent_hub != *inside {
                    this.hovered_agent_hub = *inside;
                    cx.notify();
                }
            }))
            .child(agent_card_inner);

        // Pre-sample per-tool spotlight & wire illumination springs!
        let focus_tid = self
            .hovered_tool_id
            .clone()
            .or_else(|| self.open_tool_id.clone());
        let mut tool_springs: HashMap<SharedString, (f32, f32)> =
            HashMap::with_capacity(self.tools.len());
        for tool in &self.tools {
            let allowed = agent.tools.iter().any(|g| g.tool_id == tool.id);
            let (target_vis, target_spt) = if let Some(ftid) = &focus_tid {
                if ftid == &tool.id {
                    (1.0_f32, 1.0_f32)
                } else if allowed {
                    (0.30_f32, 0.0_f32)
                } else {
                    (0.22_f32, 0.0_f32)
                }
            } else if self.hovered_agent_hub {
                if allowed {
                    (1.0_f32, 0.85_f32)
                } else {
                    (0.22_f32, 0.0_f32)
                }
            } else if allowed {
                (1.0_f32, 0.0_f32)
            } else {
                (0.45_f32, 0.0_f32)
            };
            let vis = pro_spring_f32(
                pro_eid(&self.id, format!("tmap-vis-{}", tool.id)),
                target_vis,
                0.22,
                w,
                cx,
            );
            let spt = pro_spring_f32(
                pro_eid(&self.id, format!("tmap-spt-{}", tool.id)),
                target_spt,
                0.20,
                w,
                cx,
            );
            tool_springs.insert(tool.id.clone(), (vis, spt));
        }

        // Classify each global tool for the active agent
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum ToolStand {
            ActiveNow,
            Failed,
            Used,
            Idle,
            NoAccess,
        }

        let build_tool_btn =
            |tool: &AccessTool, card_width: Option<f32>, cx: &mut Context<Self>| {
                let grant = agent.tools.iter().find(|g| g.tool_id == tool.id);
                let (stand, call_n, last_dur) = match grant {
                    None => (ToolStand::NoAccess, 0usize, None),
                    Some(g) if g.calls.is_empty() => (ToolStand::Idle, 0usize, None),
                    Some(g) => {
                        let n = g.calls.len();
                        let dur = g.calls.iter().rev().find_map(|c| c.duration_ms);
                        if g.calls.iter().any(|c| c.status == AccessRunStatus::Running) {
                            (ToolStand::ActiveNow, n, dur)
                        } else if g
                            .calls
                            .last()
                            .is_some_and(|c| c.status == AccessRunStatus::Failed)
                        {
                            (ToolStand::Failed, n, dur)
                        } else {
                            (ToolStand::Used, n, dur)
                        }
                    }
                };

                let blocked = stand == ToolStand::NoAccess;
                let tname = self.localize_str(&tool.name, cx).to_owned();
                let meta_el: AnyElement = match stand {
                    ToolStand::ActiveNow => div()
                        .h(px(16.))
                        .px(px(5.))
                        .rounded(px(4.))
                        .bg(brand_bg)
                        .text_color(brand_fg)
                        .text_size(px(10.5))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .child(spinning_ring(8.5, 1.4, brand_fg, elapsed_s, !reduced))
                        .child(tr(cx, "Running", "调用中"))
                        .into_any_element(),
                    ToolStand::Failed => div()
                        .h(px(16.))
                        .px(px(5.))
                        .rounded(px(4.))
                        .bg(p.red_soft)
                        .text_color(p.red_ink)
                        .text_size(px(10.5))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .child(tr(cx, "Failed", "失败"))
                        .into_any_element(),
                    ToolStand::Used => {
                        let txt = if let Some(ms) = last_dur {
                            format!("{} {}", tr(cx, "Last call", "最近"), format_duration(ms))
                        } else {
                            tr(cx, "Used", "已使用").to_owned()
                        };
                        div()
                            .text_size(px(11.))
                            .text_color(p.muted)
                            .truncate()
                            .child(txt)
                            .into_any_element()
                    }
                    ToolStand::Idle => div()
                        .text_size(px(11.))
                        .text_color(p.muted)
                        .child(tr(cx, "Not used yet", "尚未使用"))
                        .into_any_element(),
                    ToolStand::NoAccess => div()
                        .text_size(px(11.))
                        .text_color(p.muted)
                        .child(tr(cx, "No access", "无访问权限"))
                        .into_any_element(),
                };

                let bdr_col = match stand {
                    ToolStand::ActiveNow => brand_fg.opacity(0.55),
                    ToolStand::Failed => p.red.opacity(0.45),
                    _ => p.line,
                };
                let (t_vis, t_spt) = tool_springs.get(&tool.id).copied().unwrap_or((1.0, 0.0));
                let inner_content = div()
                    .when_some(card_width, |d, w| d.w(px(w)))
                    .when(card_width.is_none(), |d| d.w_full())
                    .h(px(46.))
                    .pl(px(8.))
                    .pr(px(10.))
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap(px(2.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .min_w_0()
                            .child(
                                div()
                                    .size(px(16.))
                                    .flex_none()
                                    .rounded(px(4.))
                                    .bg(card_bg)
                                    .border_1()
                                    .border_color(p.line)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(tool_glyph(&tool.id, &tool.name, 10., p.text, cx)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(tname.clone()),
                            )
                            .when(call_n > 0, |d| {
                                d.child(
                                    div()
                                        .flex_none()
                                        .text_size(px(11.))
                                        .text_color(p.muted)
                                        .child(format!("{call_n} ×")),
                                )
                            }),
                    )
                    .child(
                        div()
                            .h(px(16.))
                            .pl(px(22.))
                            .flex()
                            .items_center()
                            .min_w_0()
                            .child(meta_el),
                    );

                let el = if blocked {
                    div()
                        .id(pro_eid(&self.id, format!("tmap-{}", tool.id)))
                        .role(Role::Status)
                        .aria_label(format!("{tname}, {}", tr(cx, "no access", "无访问权限")))
                        .when_some(card_width, |d, w| d.w(px(w)))
                        .when(card_width.is_none(), |d| d.w_full())
                        .h(px(46.))
                        .rounded(px(9.))
                        .border_1()
                        .border_color(bdr_col)
                        .opacity(t_vis)
                        .child(inner_content)
                        .into_any_element()
                } else {
                    let tid_open = tool.id.clone();
                    let tid_hov = tool.id.clone();
                    let is_tool_sel = self.open_tool_id.as_ref() == Some(&tool.id);
                    pro_tactile(
                        pro_eid(&self.id, format!("tmap-{}", tool.id)),
                        format!("{tname}, {call_n} calls"),
                        9.0,
                        card_bg,
                        pro_mix_color(card_bg, brand_fg, if p.dark { 0.06 } else { 0.03 }),
                        bdr_col,
                        hover_border,
                        is_tool_sel,
                        inner_content,
                        cx,
                        move |this, _, cx| {
                            this.open_tool(Some(tid_open.clone()), cx);
                        },
                    )
                    .when_some(card_width, |d, w| d.w(px(w)))
                    .when(card_width.is_none(), |d| d.w_full())
                    .h(px(46.))
                    .when(!p.dark, |d| d.shadow_xs())
                    .opacity(t_vis)
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        let next = (*inside).then(|| tid_hov.clone());
                        if this.hovered_tool_id != next {
                            this.hovered_tool_id = next;
                            cx.notify();
                        }
                    }))
                    .into_any_element()
                };
                (el, stand, t_vis, t_spt)
            };

        let left_n = self.tools.len().div_ceil(2);
        let right_n = self.tools.len().saturating_sub(left_n);
        let rows_max = left_n.max(right_n).max(1);
        let row_pitch = 58.0_f32;
        let map_h = (rows_max as f32 * row_pitch + 16.0).max(296.0);

        let map_layer = if narrow {
            let mut grid = div().w_full().flex().flex_col().gap(px(8.));
            for tool in &self.tools {
                let (btn, _, _, _) = build_tool_btn(tool, None, cx);
                grid = grid.child(btn);
            }
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(agent_hub_card)
                .child(grid)
                .into_any_element()
        } else {
            let eff_w = stage_w.min(720.0);
            let tool_w = 138.0_f32;
            let agent_x = (eff_w - agent_w) * 0.5_f32;
            let agent_y = (map_h - 86.0) * 0.5_f32;
            let cy_mid = map_h * 0.5_f32;

            let mut left_specs: Vec<(f32, f32, ToolStand, f32, f32)> = Vec::with_capacity(left_n);
            let mut right_specs: Vec<(f32, f32, ToolStand, f32, f32)> = Vec::with_capacity(right_n);
            let mut left_els: Vec<(f32, f32, AnyElement)> = Vec::with_capacity(left_n);
            let mut right_els: Vec<(f32, f32, AnyElement)> = Vec::with_capacity(right_n);

            let left_span = (left_n.saturating_sub(1)) as f32 * row_pitch;
            let left_top0 = cy_mid - left_span * 0.5_f32 - 23.0;
            for (i, tool) in self.tools.iter().take(left_n).enumerate() {
                let ty = left_top0 + i as f32 * row_pitch;
                let (el, stand, vis, spt) = build_tool_btn(tool, Some(tool_w), cx);
                left_specs.push((0.0, ty, stand, vis, spt));
                left_els.push((0.0, ty, el));
            }

            let right_span = (right_n.saturating_sub(1)) as f32 * row_pitch;
            let right_top0 = cy_mid - right_span * 0.5_f32 - 23.0;
            for (i, tool) in self.tools.iter().skip(left_n).enumerate() {
                let tx = eff_w - tool_w;
                let ty = right_top0 + i as f32 * row_pitch;
                let (el, stand, vis, spt) = build_tool_btn(tool, Some(tool_w), cx);
                right_specs.push((tx, ty, stand, vis, spt));
                right_els.push((tx, ty, el));
            }

            let red_wire = p.red;
            let wires = canvas(
                |_, _, _| (),
                move |surface, _, window, _| {
                    let ox = f32::from(surface.origin.x);
                    let oy = f32::from(surface.origin.y);
                    let port_left = point(px(ox + agent_x), px(oy + cy_mid));
                    let port_right = point(px(ox + agent_x + agent_w), px(oy + cy_mid));

                    let mut any_left_on = false;
                    for (tx, ty, stand, vis, spt) in &left_specs {
                        let p_tool = point(px(ox + *tx + tool_w), px(oy + *ty + 23.0));
                        match stand {
                            ToolStand::NoAccess => {
                                paint_dashed_elbow(
                                    port_left,
                                    p_tool,
                                    wire_off.opacity(*vis),
                                    1.5,
                                    window,
                                );
                            }
                            ToolStand::Failed => {
                                any_left_on = true;
                                pro_connector_pulse(
                                    port_left,
                                    p_tool,
                                    8.0,
                                    false,
                                    red_wire.opacity((0.68 + 0.25 * spt) * vis),
                                    red_wire.opacity(*vis),
                                    (*spt > 0.5).then_some(pulse_t).flatten(),
                                    1.5 + 0.35 * spt,
                                    window,
                                );
                            }
                            ToolStand::ActiveNow => {
                                any_left_on = true;
                                pro_connector_pulse(
                                    port_left,
                                    p_tool,
                                    8.0,
                                    false,
                                    wire_used.opacity(*vis),
                                    wire_glint.opacity(*vis),
                                    pulse_t,
                                    1.6 + 0.35 * spt,
                                    window,
                                );
                            }
                            ToolStand::Used | ToolStand::Idle => {
                                any_left_on = true;
                                pro_connector_pulse(
                                    port_left,
                                    p_tool,
                                    8.0,
                                    false,
                                    wire_used.opacity(*vis),
                                    wire_glint.opacity(*vis),
                                    (*spt > 0.5).then_some(pulse_t).flatten(),
                                    1.5 + 0.35 * spt,
                                    window,
                                );
                            }
                        }
                    }

                    let mut any_right_on = false;
                    for (tx, ty, stand, vis, spt) in &right_specs {
                        let p_tool = point(px(ox + *tx), px(oy + *ty + 23.0));
                        match stand {
                            ToolStand::NoAccess => {
                                paint_dashed_elbow(
                                    port_right,
                                    p_tool,
                                    wire_off.opacity(*vis),
                                    1.5,
                                    window,
                                );
                            }
                            ToolStand::Failed => {
                                any_right_on = true;
                                pro_connector_pulse(
                                    port_right,
                                    p_tool,
                                    8.0,
                                    false,
                                    red_wire.opacity((0.68 + 0.25 * spt) * vis),
                                    red_wire.opacity(*vis),
                                    (*spt > 0.5).then_some(pulse_t).flatten(),
                                    1.5 + 0.35 * spt,
                                    window,
                                );
                            }
                            ToolStand::ActiveNow => {
                                any_right_on = true;
                                pro_connector_pulse(
                                    port_right,
                                    p_tool,
                                    8.0,
                                    false,
                                    wire_used.opacity(*vis),
                                    wire_glint.opacity(*vis),
                                    pulse_t,
                                    1.6 + 0.35 * spt,
                                    window,
                                );
                            }
                            ToolStand::Used | ToolStand::Idle => {
                                any_right_on = true;
                                pro_connector_pulse(
                                    port_right,
                                    p_tool,
                                    8.0,
                                    false,
                                    wire_used.opacity(*vis),
                                    wire_glint.opacity(*vis),
                                    (*spt > 0.5).then_some(pulse_t).flatten(),
                                    1.5 + 0.35 * spt,
                                    window,
                                );
                            }
                        }
                    }

                    paint_joint(
                        port_left,
                        if any_left_on { wire_used } else { wire_off },
                        card_bg,
                        window,
                    );
                    paint_joint(
                        port_right,
                        if any_right_on { wire_used } else { wire_off },
                        card_bg,
                        window,
                    );
                },
            )
            .absolute()
            .inset_0();

            let mut box_map = div()
                .w(px(eff_w))
                .h(px(map_h))
                .mx_auto()
                .relative()
                .child(wires)
                .child(
                    div()
                        .absolute()
                        .left(px(agent_x))
                        .top(px(agent_y))
                        .child(agent_hub_card),
                );
            for (tx, ty, el) in left_els.into_iter().chain(right_els) {
                box_map = box_map.child(div().absolute().left(px(tx)).top(px(ty)).child(el));
            }
            box_map.into_any_element()
        };

        // 4. Call History & Replay Drawer Overlay (`pro_presence`)
        let drawer_tool_id = self
            .open_tool_id
            .clone()
            .or_else(|| self.last_open_tool_id.clone());
        let drawer_open = self.open_tool_id.is_some();

        let drawer_el = if let Some(tid) = drawer_tool_id
            && let Some(tool_def) = self.tools.iter().find(|t| t.id == tid).cloned()
            && let Some(grant) = agent.tools.iter().find(|g| g.tool_id == tid).cloned()
        {
            let selected_call = self
                .selected_call_id
                .as_ref()
                .and_then(|cid| grant.calls.iter().find(|c| &c.id == cid))
                .or_else(|| grant.calls.last())
                .cloned();

            let seed_key: SharedString = format!(
                "{}:{}:{}",
                agent.id,
                tid,
                selected_call
                    .as_ref()
                    .map(|c| c.id.as_ref())
                    .unwrap_or("empty")
            )
            .into();
            let seed_input = selected_call
                .as_ref()
                .map(|c| c.input.as_ref())
                .unwrap_or("{\n  \"ping\": true\n}");
            let input_ed = self.ensure_input_editor(&seed_key, seed_input, w, cx);
            let ed_for_replay = input_ed.clone();
            let ed_for_reset = input_ed.clone();
            let orig_input: SharedString = seed_input.to_owned().into();
            let is_dirty = self
                .edited_input
                .as_ref()
                .is_some_and(|cur| cur != &orig_input);

            let mut call_list = div()
                .id(pro_eid(&self.id, "calls-list"))
                .role(Role::ListBox)
                .max_h(px(if narrow { 112. } else { 228. }))
                .overflow_y_scroll()
                .p(px(2.))
                .flex()
                .flex_col()
                .gap(px(2.));

            if grant.calls.is_empty() {
                call_list = call_list.child(
                    div()
                        .py(px(18.))
                        .px(px(8.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(tr(
                            cx,
                            "Not called yet — enter an input on the right and press Replay.",
                            "尚未产生调用——可在右侧输入参数后点击“重放”直接试运行。",
                        )),
                );
            } else {
                for call in &grant.calls {
                    let cid = call.id.clone();
                    let active = selected_call.as_ref().is_some_and(|sc| sc.id == cid);
                    let cid_click = cid.clone();
                    let status_ic = match call.status {
                        AccessRunStatus::Running => {
                            spinning_ring(11., 1.5, brand_fg, elapsed_s, !reduced)
                        }
                        AccessRunStatus::Failed => pro_icon(ProIcon::AlertTriangle, 13., p.red_ink),
                        _ => pro_check_draw(
                            pro_eid(&self.id, format!("crow-chk-{cid}")),
                            true,
                            13.0,
                            false,
                            p.green_ink,
                        ),
                    };
                    let compact_in = call
                        .input
                        .lines()
                        .map(str::trim)
                        .collect::<Vec<_>>()
                        .join(" ");
                    let time_lbl = call
                        .duration_ms
                        .map(format_duration)
                        .unwrap_or_else(|| call.at.to_string());

                    call_list = call_list.child(
                        pro_tactile(
                            pro_eid(&self.id, format!("crow-{cid}")),
                            compact_in.clone(),
                            7.0,
                            if active { fill_bg } else { transparent_black() },
                            if active { fill_bg } else { well_bg },
                            transparent_black(),
                            transparent_black(),
                            active,
                            div()
                                .w_full()
                                .h(px(30.))
                                .pl(px(6.))
                                .pr(px(8.))
                                .flex()
                                .items_center()
                                .gap(px(7.))
                                .child(status_ic)
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .font_family(crate::components::pro_chrome::pro_mono_name())
                                        .text_size(px(11.5))
                                        .text_color(p.ink)
                                        .child(compact_in),
                                )
                                .when(call.replay, |d| {
                                    d.child(
                                        div()
                                            .h(px(16.))
                                            .px(px(5.))
                                            .rounded(px(4.))
                                            .bg(brand_bg)
                                            .text_color(brand_fg)
                                            .text_size(px(10.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .flex()
                                            .items_center()
                                            .child(tr(cx, "Replay", "重放")),
                                    )
                                })
                                .child(
                                    div()
                                        .flex_none()
                                        .text_size(px(11.))
                                        .text_color(p.muted)
                                        .child(time_lbl),
                                ),
                            cx,
                            move |this, _, cx| {
                                this.select_call(cid_click.clone(), cx);
                            },
                        )
                        .role(Role::ListBoxOption)
                        .aria_selected(active),
                    );
                }
            }

            let out_header = if selected_call
                .as_ref()
                .is_some_and(|c| c.status == AccessRunStatus::Failed)
            {
                tr(cx, "ERROR", "错误输出")
            } else {
                tr(cx, "OUTPUT", "返回结果")
            };

            let out_box = if let Some(sc) = &selected_call {
                if sc.status == AccessRunStatus::Running {
                    div()
                        .px(px(10.))
                        .py(px(8.))
                        .rounded(px(7.))
                        .bg(well_bg)
                        .border_1()
                        .border_color(p.line)
                        .child(pro_shimmer_text(
                            pro_eid(&self.id, "wait-out"),
                            tr(cx, "Waiting for tool response…", "正在等待工具响应…"),
                            p.muted,
                            p.ink,
                            12.,
                            FontWeight::NORMAL,
                            true,
                        ))
                        .into_any_element()
                } else {
                    div()
                        .max_h(px(78.))
                        .id(pro_eid(&self.id, "out-scroll"))
                        .overflow_y_scroll()
                        .px(px(10.))
                        .py(px(8.))
                        .rounded(px(7.))
                        .bg(well_bg)
                        .border_1()
                        .border_color(if sc.status == AccessRunStatus::Failed {
                            p.red.opacity(0.35)
                        } else {
                            p.line
                        })
                        .font_family(crate::components::pro_chrome::pro_mono_name())
                        .text_size(px(11.5))
                        .line_height(px(17.))
                        .text_color(if sc.status == AccessRunStatus::Failed {
                            p.red_ink
                        } else {
                            p.ink
                        })
                        .child(sc.output.clone())
                        .into_any_element()
                }
            } else {
                div()
                    .px(px(10.))
                    .py(px(8.))
                    .rounded(px(7.))
                    .bg(well_bg)
                    .border_1()
                    .border_color(p.line)
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(tr(cx, "No output yet", "暂无输出"))
                    .into_any_element()
            };

            let detail_col = div()
                .flex_1()
                .min_w_0()
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
                                .text_size(px(10.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .child(tr(cx, "INPUT (EDITABLE)", "输入参数（可编辑）")),
                        )
                        .when_some(selected_call.as_ref(), |d, sc| {
                            d.child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(p.faint)
                                    .child(format!("@ {}", sc.at)),
                            )
                        }),
                )
                .child(
                    div()
                        .h(px(74.))
                        .px(px(10.))
                        .py(px(7.))
                        .rounded(px(7.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line_strong)
                        .font_family(crate::components::pro_chrome::pro_mono_name())
                        .child(input_ed),
                )
                .child(
                    div()
                        .text_size(px(10.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(out_header),
                )
                .child(pro_view_transition(
                    pro_eid(&self.id, "out-box-vtx"),
                    format!(
                        "{}:{:?}",
                        seed_key,
                        selected_call.as_ref().map(|c| c.status)
                    ),
                    out_box,
                ))
                .child(
                    div()
                        .pt(px(2.))
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(px(6.))
                        .when(is_dirty, |d| {
                            let orig_c = orig_input.clone();
                            d.child(pro_btn(
                                pro_eid(&self.id, "reset-in"),
                                tr(cx, "Reset", "重置"),
                                Some(ProIcon::Undo),
                                ProBtnStyle::Ghost,
                                true,
                                cx,
                                move |this, w, cx| {
                                    ed_for_reset.update(cx, |e, cx| {
                                        e.set_value(orig_c.as_ref(), w, cx);
                                    });
                                    this.edited_input = None;
                                    cx.notify();
                                },
                            ))
                        })
                        .child(pro_btn(
                            pro_eid(&self.id, "replay-btn"),
                            if is_dirty {
                                tr(cx, "Run modified", "按修改运行")
                            } else {
                                tr(cx, "Replay", "重放调用")
                            },
                            Some(ProIcon::Play),
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            move |this, _, cx| {
                                let cur: SharedString =
                                    ed_for_replay.read(cx).value().to_owned().into();
                                this.replay_selected_call(Some(cur), cx);
                            },
                        )),
                );

            let panel_card = div()
                .id(pro_eid(&self.id, "calls-drawer"))
                .role(Role::Group)
                .w_full()
                .p(px(12.))
                .pt(px(8.))
                .rounded(px(12.))
                .bg(card_bg)
                .border_1()
                .border_color(p.line_strong)
                .shadow(pro_card_shadow(p.dark))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .h(px(34.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .border_b_1()
                        .border_color(p.line)
                        .child(pro_icon_btn(
                            pro_eid(&self.id, "drawer-back"),
                            tr(cx, "Back to map", "返回图谱"),
                            ProIcon::ArrowLeft,
                            26.,
                            false,
                            cx,
                            |this, _, cx| this.open_tool(None, cx),
                        ))
                        .child(pro_app_tile(&tool_def.name, 20., cx))
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(self.localize_str(&tool_def.name, cx).to_owned()),
                        )
                        .when(!narrow, |d| {
                            d.child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(tool_def.description.clone()),
                            )
                        })
                        .child(div().ml_auto().child(pro_rolling_number(
                            pro_eid(&self.id, "drawer-call-cnt"),
                            grant.calls.len() as f64,
                            0,
                            "",
                            " ×",
                            12.,
                            FontWeight::NORMAL,
                            p.muted,
                        ))),
                )
                .child(if narrow {
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .child(call_list)
                        .child(detail_col)
                } else {
                    div()
                        .w_full()
                        .flex()
                        .items_start()
                        .gap(px(12.))
                        .child(div().w(px(236.)).flex_none().child(call_list))
                        .child(detail_col)
                });

            Some(pro_presence(
                pro_eid(&self.id, "drawer-pres"),
                drawer_open,
                0.22,
                0.16,
                move |amt| {
                    div()
                        .w_full()
                        .relative()
                        .top(px(6.0 * (1.0 - amt)))
                        .opacity(amt)
                        .child(panel_card)
                        .into_any_element()
                },
            ))
        } else {
            None
        };

        div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(self.localize_str(&self.title, cx).to_owned())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                match ev.keystroke.key.as_str() {
                    "escape" if this.open_tool_id.is_some() => {
                        this.open_tool(None, cx);
                        this.focus.focus(w, cx);
                    }
                    "left" if this.open_tool_id.is_none() => {
                        if let Some(pos) = this
                            .agents
                            .iter()
                            .position(|a| a.id == this.active_agent_id)
                            && !this.agents.is_empty()
                        {
                            let next = (pos + this.agents.len() - 1) % this.agents.len();
                            let nid = this.agents[next].id.clone();
                            this.select_agent(nid, cx);
                        }
                    }
                    "right" if this.open_tool_id.is_none() => {
                        if let Some(pos) = this
                            .agents
                            .iter()
                            .position(|a| a.id == this.active_agent_id)
                            && !this.agents.is_empty()
                        {
                            let next = (pos + 1) % this.agents.len();
                            let nid = this.agents[next].id.clone();
                            this.select_agent(nid, cx);
                        }
                    }
                    _ => {}
                }
            }))
            .w_full()
            .relative()
            .flex()
            .flex_col()
            .child(
                canvas(
                    move |bounds, _, _| {
                        let bw = f32::from(bounds.size.width);
                        if bw > 1.0 {
                            width_cell.set(bw);
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .left_0()
                .right_0()
                .top_0()
                .h(px(1.)),
            )
            .child(head_bar)
            .child(agent_tabs)
            .child(if drawer_open {
                div()
                    .w_full()
                    .when_some(drawer_el, |d, dr| d.child(dr))
                    .into_any_element()
            } else {
                div()
                    .w_full()
                    .relative()
                    .child(map_layer)
                    .when_some(drawer_el, |d, dr| {
                        d.child(div().absolute().inset_0().child(dr))
                    })
                    .into_any_element()
            })
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::ToolAccessMap;
    use gpui::{AppContext, TestAppContext};

    #[gpui::test]
    fn tool_access_map_enforces_grants_and_replays_calls(cx: &mut TestAppContext) {
        let view = cx.new(|cx| ToolAccessMap::new("test-tool-access-map", cx));

        view.update(cx, |tam, cx| {
            assert_eq!(tam.active_agent_id(), "coder");

            // Notion is "No access" for Coder, so opening it must be ignored.
            tam.open_tool(Some("notion".into()), cx);
            assert_eq!(tam.open_tool_id(), None);

            // GitHub is granted to Coder (2 calls); opening succeeds and Replay appends a 3rd call!
            tam.open_tool(Some("github".into()), cx);
            assert_eq!(tam.open_tool_id(), Some(&"github".into()));

            let replayed_id =
                tam.replay_selected_call(Some("{\n  \"action\": \"list_prs\"\n}".into()), cx);
            assert!(replayed_id.is_some());

            let gh_calls = tam
                .current_agent()
                .and_then(|a| a.tools.iter().find(|t| t.tool_id == "github"))
                .map(|t| t.calls.len())
                .unwrap_or(0);
            assert_eq!(gh_calls, 3);

            // Switching agents closes the inspector and updates grants.
            tam.select_agent("writer", cx);
            assert_eq!(tam.active_agent_id(), "writer");
            assert_eq!(tam.open_tool_id(), None);
            // Notion IS granted to Writer!
            tam.open_tool(Some("notion".into()), cx);
            assert_eq!(tam.open_tool_id(), Some(&"notion".into()));
        });
    }
}
