//! Unified multi-agent mission control block with event-peak recap curve, hierarchy tree, and action inbox.
//!
//! Combines a `"Since you looked"` event-peak curve (`7:08 … 7:53`), a hierarchical `"Agents"`
//! fleet card with live `pro_connector_pulse` links, and a `"NEEDS YOU"` action queue governed
//! by one status filter (`All | Running | Needs you | Failed`).

use crate::{
    components::pro_chrome::{
        ProPalette, pro_agent_avatar, pro_app_tile, pro_card_shadow, pro_check_draw,
        pro_connector_pulse, pro_eid, pro_empty_state, pro_fold, pro_mark_selected, pro_mix_color,
        pro_pill_indicator, pro_progress_ring, pro_rolling_number, pro_selection_sensor,
        pro_skeleton_bar, pro_spring_bar, pro_spring_f32, pro_swap_label, pro_tactile,
        pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const RECAP_NEEDS_SVG: &str = r#"<circle cx="8" cy="5.25" r="2.5" stroke="currentColor" stroke-width="1.4"></circle><path d="M3.25 13.25c.6-2.35 2.45-3.75 4.75-3.75s4.15 1.4 4.75 3.75" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"></path>"#;
const RECAP_FAIL_SVG: &str = r#"<path d="M7.13 2.75a1 1 0 0 1 1.74 0l5.2 9.25a1 1 0 0 1-.87 1.5H2.8a1 1 0 0 1-.87-1.5z" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"></path><path d="M8 6.5v2.75M8 11.25v.01" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"></path>"#;

/// Unified status filter across the recap curve, agent hierarchy, and inbox in [`MissionControlBlock`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MissionFilter {
    /// Show all agents and recap events.
    #[default]
    All,
    /// Show actively running agents.
    Running,
    /// Show agents waiting on user input plus failed agents requiring intervention.
    NeedsYou,
    /// Show failed agents only.
    Failed,
}

impl MissionFilter {
    const ALL: [Self; 4] = [Self::All, Self::Running, Self::NeedsYou, Self::Failed];

    fn slug(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Running => "running",
            Self::NeedsYou => "needs",
            Self::Failed => "failed",
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::All => tr(cx, "All", "全部"),
            Self::Running => tr(cx, "Running", "运行中"),
            Self::NeedsYou => tr(cx, "Needs you", "待处理"),
            Self::Failed => tr(cx, "Failed", "失败"),
        }
    }

    fn next(self) -> Self {
        match self {
            Self::All => Self::Running,
            Self::Running => Self::NeedsYou,
            Self::NeedsYou => Self::Failed,
            Self::Failed => Self::All,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::All => Self::Failed,
            Self::Running => Self::All,
            Self::NeedsYou => Self::Running,
            Self::Failed => Self::NeedsYou,
        }
    }
}

/// Live operational status of a [`MissionAgent`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MissionAgentStatus {
    /// Actively executing tools or coordinating sub-agents.
    #[default]
    Running,
    /// Blocked waiting for user approval or an answer.
    Waiting,
    /// Halted due to an upstream error.
    Failed,
    /// Completed cleanly.
    Done,
}

impl MissionAgentStatus {
    /// Check whether this status matches `filter` (`NeedsYou` includes both `Waiting` and `Failed`).
    pub fn matches(self, filter: MissionFilter) -> bool {
        match filter {
            MissionFilter::All => true,
            MissionFilter::Running => self == Self::Running,
            MissionFilter::NeedsYou => matches!(self, Self::Waiting | Self::Failed),
            MissionFilter::Failed => self == Self::Failed,
        }
    }
}

/// Prompt or approval request surfaced when a [`MissionAgent`] enters [`MissionAgentStatus::Waiting`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionAgentAsk {
    /// Short title shown in the `"NEEDS YOU"` card.
    pub title: SharedString,
    /// Plain-language context explaining why human input is needed.
    pub detail: SharedString,
    /// Optional quick-answer options (if empty, renders `Reject` / `Approve`).
    pub choices: Vec<SharedString>,
    /// Label for the primary approval action (default `"Approve"`).
    pub approve_label: SharedString,
    /// Label for the secondary rejection action (default `"Reject"`).
    pub reject_label: SharedString,
}

impl MissionAgentAsk {
    /// Construct an approval prompt (`Reject` + `Approve`).
    pub fn approval(title: impl Into<SharedString>, detail: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            detail: detail.into(),
            choices: Vec::new(),
            approve_label: "Approve".into(),
            reject_label: "Reject".into(),
        }
    }

    /// Construct a multi-choice question prompt (`Answer` options).
    pub fn question(
        title: impl Into<SharedString>,
        detail: impl Into<SharedString>,
        choices: Vec<SharedString>,
    ) -> Self {
        Self {
            title: title.into(),
            detail: detail.into(),
            choices,
            approve_label: "Answer".into(),
            reject_label: "Dismiss".into(),
        }
    }
}

/// Lead or sub-agent node in [`MissionControlBlock`].
#[derive(Clone, Debug, PartialEq)]
pub struct MissionAgent {
    /// Stable agent identifier.
    pub id: SharedString,
    /// Display name (such as `"Launch lead"` or `"Writer"`).
    pub name: SharedString,
    /// Model label (such as `"Opus 5.5"` or `"Sonnet 5.5"`).
    pub model: SharedString,
    /// Current operational state.
    pub status: MissionAgentStatus,
    /// Assigned goal or task summary.
    pub task: SharedString,
    /// Optional waiting prompt when `status == Waiting`.
    pub ask: Option<MissionAgentAsk>,
    /// Optional failure reason when `status == Failed`.
    pub error: Option<SharedString>,
    /// Accumulated run spend in USD.
    pub cost_usd: f64,
    /// Formatted elapsed duration (`"26m 31s"`).
    pub elapsed: SharedString,
    /// Connected tool/app names (`"Linear"`, `"Notion"`, `"Stripe"`).
    pub tools: Vec<SharedString>,
    /// Sub-agents supervised by this agent.
    pub children: Vec<MissionAgent>,
}

impl MissionAgent {
    /// Check whether this agent or any of its sub-agents match `filter`.
    pub fn matches_tree(&self, filter: MissionFilter) -> bool {
        self.status.matches(filter) || self.children.iter().any(|c| c.matches_tree(filter))
    }
}

/// Kind of milestone plotted along the `"Since you looked"` curve.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MissionEventKind {
    /// Completed step or batch.
    #[default]
    Done,
    /// Step waiting for user approval or answer.
    NeedsYou,
    /// Step that encountered an error.
    Failed,
}

/// Event plotted on the `"Since you looked"` curve in [`MissionControlBlock`].
#[derive(Clone, Debug, PartialEq)]
pub struct MissionRecapEvent {
    /// Stable event identifier.
    pub id: SharedString,
    /// Owning agent identifier.
    pub agent_id: SharedString,
    /// Clock timestamp (`"7:39"`).
    pub at_label: SharedString,
    /// Normalized position along `[0.0, 1.0]` between `since` and `now`.
    pub position: f32,
    /// Peak amplitude (`0.35..=1.0`).
    pub weight: f32,
    /// Event category (`Done`, `NeedsYou`, `Failed`).
    pub kind: MissionEventKind,
    /// Short headline shown in the hover tooltip.
    pub title: SharedString,
}

/// Events emitted by [`MissionControlBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MissionControlEvent {
    /// User switched the unified status filter.
    FilterChanged(MissionFilter),
    /// User selected a lead agent in the fleet hierarchy.
    LeadSelected(SharedString),
    /// User approved a waiting agent (`agent_id`).
    Approved(SharedString),
    /// User rejected a waiting agent (`agent_id`).
    Rejected(SharedString),
    /// User answered a waiting agent's multi-choice prompt.
    Answered {
        /// Agent identifier.
        agent_id: SharedString,
        /// Picked answer option.
        choice: SharedString,
    },
    /// User retried a failed agent (`agent_id`).
    Retried(SharedString),
    /// User dismissed a failed agent (`agent_id`).
    Dismissed(SharedString),
    /// User hovered or cleared an event dot on the recap curve.
    EventHovered(Option<SharedString>),
}

fn flatten_agents<'a>(roots: &'a [MissionAgent], out: &mut Vec<&'a MissionAgent>) {
    for ag in roots {
        out.push(ag);
        flatten_agents(&ag.children, out);
    }
}

/// Count agents matching `filter` across all leads and sub-agents.
pub fn count_mission_agents(roots: &[MissionAgent], filter: MissionFilter) -> usize {
    let mut all = Vec::new();
    flatten_agents(roots, &mut all);
    all.into_iter().filter(|a| a.status.matches(filter)).count()
}

fn find_agent_status(roots: &[MissionAgent], agent_id: &str) -> Option<MissionAgentStatus> {
    for ag in roots {
        if ag.id.as_ref() == agent_id {
            return Some(ag.status);
        }
        if let Some(st) = find_agent_status(&ag.children, agent_id) {
            return Some(st);
        }
    }
    None
}

fn find_lead_for_agent(roots: &[MissionAgent], agent_id: &str) -> Option<SharedString> {
    for lead in roots {
        if lead.id.as_ref() == agent_id || lead.children.iter().any(|c| c.id.as_ref() == agent_id) {
            return Some(lead.id.clone());
        }
    }
    None
}

fn mutate_agent(
    roots: &mut [MissionAgent],
    agent_id: &str,
    f: impl FnOnce(&mut MissionAgent),
) -> bool {
    for ag in roots {
        if ag.id.as_ref() == agent_id {
            f(ag);
            return true;
        }
        for ch in &mut ag.children {
            if ch.id.as_ref() == agent_id {
                f(ch);
                return true;
            }
        }
    }
    false
}

fn agent_hue(id: &str, p: &ProPalette) -> Hsla {
    match id {
        "launch" => p.blue,
        "research" => p.purple,
        "writer" => p.teal,
        "release" => p.cyan,
        "support" => p.green,
        "billing" => p.red,
        "ops" => p.orange,
        _ => p.blue,
    }
}

fn default_mission_agents() -> Vec<MissionAgent> {
    // 7 agents total: 3 Running (Launch lead, Support lead, Ops), 1 Done (Researcher),
    // 2 Waiting (Writer, Release), 1 Failed (Billing) => All 7 | Running 3 | Needs you 3 | Failed 1
    vec![
        MissionAgent {
            id: "launch".into(),
            name: "Launch lead".into(),
            model: "Opus 5.5".into(),
            status: MissionAgentStatus::Running,
            task: "Launch the new pricing page by Friday".into(),
            ask: None,
            error: None,
            cost_usd: 2.15,
            elapsed: "26m 31s".into(),
            tools: vec!["Linear".into(), "Notion".into()],
            children: vec![
                MissionAgent {
                    id: "research".into(),
                    name: "Researcher".into(),
                    model: "Sonnet 5.5".into(),
                    status: MissionAgentStatus::Done,
                    task: "Compare Linear, Notion & Figma seat pricing".into(),
                    ask: None,
                    error: None,
                    cost_usd: 0.48,
                    elapsed: "8m 12s".into(),
                    tools: vec!["Notion".into(), "Figma".into()],
                    children: Vec::new(),
                },
                MissionAgent {
                    id: "writer".into(),
                    name: "Writer".into(),
                    model: "Opus 5.5".into(),
                    status: MissionAgentStatus::Waiting,
                    task: "Draft the copy for Free, Pro and Team".into(),
                    ask: Some(MissionAgentAsk::question(
                        "Pick the Team plan seat price",
                        "Copy for Free ($0) and Pro ($12) is ready · choose Team pricing",
                        vec!["$20 / seat".into(), "$24 / seat".into()],
                    )),
                    error: None,
                    cost_usd: 0.92,
                    elapsed: "14m 05s".into(),
                    tools: vec!["Notion".into()],
                    children: Vec::new(),
                },
                MissionAgent {
                    id: "release".into(),
                    name: "Release".into(),
                    model: "GPT-5.4".into(),
                    status: MissionAgentStatus::Waiting,
                    task: "Merge & deploy pricing page PR #482".into(),
                    ask: Some(MissionAgentAsk::approval(
                        "Deploy pricing page PR #482",
                        "All 212 staging checks passed · waiting on production sign-off",
                    )),
                    error: None,
                    cost_usd: 0.75,
                    elapsed: "4m 14s".into(),
                    tools: vec!["GitHub".into(), "Vercel".into()],
                    children: Vec::new(),
                },
            ],
        },
        MissionAgent {
            id: "support".into(),
            name: "Support lead".into(),
            model: "Sonnet 5.5".into(),
            status: MissionAgentStatus::Running,
            task: "Answer the open tickets in the queue".into(),
            ask: None,
            error: None,
            cost_usd: 1.02,
            elapsed: "29m 31s".into(),
            tools: vec!["Slack".into(), "Gmail".into()],
            children: vec![MissionAgent {
                id: "billing".into(),
                name: "Billing".into(),
                model: "Haiku 4.5".into(),
                status: MissionAgentStatus::Failed,
                task: "Refund $240 to Acme Corp".into(),
                ask: None,
                error: Some("Stripe live refund token expired (HTTP 401)".into()),
                cost_usd: 0.19,
                elapsed: "3m 08s".into(),
                tools: vec!["Stripe".into()],
                children: Vec::new(),
            }],
        },
        MissionAgent {
            id: "ops".into(),
            name: "Ops".into(),
            model: "Gemini 3.0 Flash".into(),
            status: MissionAgentStatus::Running,
            task: "Sync CRM contacts & rotate webhook keys".into(),
            ask: None,
            error: None,
            cost_usd: 0.64,
            elapsed: "18m 10s".into(),
            tools: vec!["HubSpot".into(), "GitHub".into()],
            children: Vec::new(),
        },
    ]
}

fn default_recap_events() -> Vec<MissionRecapEvent> {
    vec![
        MissionRecapEvent {
            id: "ev-1".into(),
            agent_id: "research".into(),
            at_label: "7:12".into(),
            position: 0.10,
            weight: 0.62,
            kind: MissionEventKind::Done,
            title: "Compared seat prices across Linear, Notion & Figma".into(),
        },
        MissionRecapEvent {
            id: "ev-2".into(),
            agent_id: "ops".into(),
            at_label: "7:18".into(),
            position: 0.22,
            weight: 0.46,
            kind: MissionEventKind::Done,
            title: "Synced 1,240 CRM contacts to HubSpot".into(),
        },
        MissionRecapEvent {
            id: "ev-3".into(),
            agent_id: "support".into(),
            at_label: "7:29".into(),
            position: 0.46,
            weight: 0.84,
            kind: MissionEventKind::Done,
            title: "Answered 24 support tickets in #help".into(),
        },
        MissionRecapEvent {
            id: "ev-4".into(),
            agent_id: "billing".into(),
            at_label: "7:36".into(),
            position: 0.61,
            weight: 0.56,
            kind: MissionEventKind::Failed,
            title: "Refund $240 to Acme Corp failed (token expired)".into(),
        },
        MissionRecapEvent {
            id: "ev-5".into(),
            agent_id: "writer".into(),
            at_label: "7:40".into(),
            position: 0.70,
            weight: 0.58,
            kind: MissionEventKind::NeedsYou,
            title: "Writer asks: Team plan $20 or $24 per seat?".into(),
        },
        MissionRecapEvent {
            id: "ev-6".into(),
            agent_id: "release".into(),
            at_label: "7:45".into(),
            position: 0.79,
            weight: 0.64,
            kind: MissionEventKind::NeedsYou,
            title: "Release asks: Approve PR #482 deploy".into(),
        },
        MissionRecapEvent {
            id: "ev-7".into(),
            agent_id: "launch".into(),
            at_label: "7:49".into(),
            position: 0.88,
            weight: 0.44,
            kind: MissionEventKind::Done,
            title: "Verified Stripe webhook signatures on staging".into(),
        },
    ]
}

fn eval_peak_curve(u: f32, peaks: &[(f32, f32)]) -> f32 {
    let mut acc = 0.0_f32;
    for &(pos, weight) in peaks {
        let d = (u - pos) / 0.048;
        acc += weight * (-d * d).exp();
    }
    acc.clamp(0.0, 0.92)
}

/// Multi-agent command center block uniting an event-peak recap curve, agent hierarchy, and action queue.
pub struct MissionControlBlock {
    id: ElementId,
    focus: FocusHandle,
    title: SharedString,
    since_label: SharedString,
    now_label: SharedString,
    filter: MissionFilter,
    agents: Vec<MissionAgent>,
    events: Vec<MissionRecapEvent>,
    open_lead: SharedString,
    hovered_event: Option<SharedString>,
    loading: bool,
    empty: bool,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
    boot_at: Instant,
}

impl EventEmitter<MissionControlEvent> for MissionControlBlock {}

impl MissionControlBlock {
    /// Create a new [`MissionControlBlock`] with 7 active/waiting/failed agents and recap peaks.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: "Mission control".into(),
            since_label: "7:08".into(),
            now_label: "7:53".into(),
            filter: MissionFilter::All,
            agents: default_mission_agents(),
            events: default_recap_events(),
            open_lead: "launch".into(),
            hovered_event: None,
            loading: false,
            empty: false,
            measured_width: Rc::new(Cell::new(894.)),
            toast: None,
            boot_at: Instant::now(),
        }
    }

    /// Set the top-left heading (default `"Mission control"`).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the initial [`MissionFilter`].
    pub fn with_filter(mut self, filter: MissionFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Alias matching `defaultFilter` in the reference specification.
    pub fn with_default_filter(self, filter: MissionFilter) -> Self {
        self.with_filter(filter)
    }

    /// Supply custom lead and sub-agent trees.
    pub fn with_agents(mut self, agents: Vec<MissionAgent>) -> Self {
        if let Some(first) = agents.first() {
            self.open_lead = first.id.clone();
        }
        self.agents = agents;
        self
    }

    /// Supply custom `"Since you looked"` recap events.
    pub fn with_events(mut self, events: Vec<MissionRecapEvent>) -> Self {
        self.events = events;
        self
    }

    /// Customize the start and end clock labels (`"7:08"` … `"7:53"`).
    pub fn with_window_labels(
        mut self,
        since_label: impl Into<SharedString>,
        now_label: impl Into<SharedString>,
    ) -> Self {
        self.since_label = since_label.into();
        self.now_label = now_label.into();
        self
    }

    /// Toggle loading skeleton placeholders.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the active [`MissionFilter`].
    pub fn filter(&self) -> MissionFilter {
        self.filter
    }

    /// Set the unified [`MissionFilter`] and emit [`MissionControlEvent::FilterChanged`].
    pub fn set_filter(&mut self, filter: MissionFilter, cx: &mut Context<Self>) {
        if self.filter != filter {
            self.filter = filter;
            // Ensure an applicable lead is open when filtering
            if !self
                .agents
                .iter()
                .any(|a| a.id == self.open_lead && a.matches_tree(filter))
                && let Some(next_lead) = self.agents.iter().find(|a| a.matches_tree(filter))
            {
                self.open_lead = next_lead.id.clone();
            }
            cx.emit(MissionControlEvent::FilterChanged(filter));
            cx.notify();
        }
    }

    fn resolve_approve(
        &mut self,
        agent_id: &str,
        choice: Option<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let changed = mutate_agent(&mut self.agents, agent_id, |ag| {
            ag.status = MissionAgentStatus::Running;
            ag.ask = None;
            ag.error = None;
        });
        if changed {
            for ev in &mut self.events {
                if ev.agent_id.as_ref() == agent_id && ev.kind == MissionEventKind::NeedsYou {
                    ev.kind = MissionEventKind::Done;
                }
            }
            let id_shared = SharedString::from(agent_id.to_owned());
            if let Some(ch) = choice {
                self.toast = Some((
                    format!("{}: {ch}", tr(cx, "Sent answer", "已发送回复")).into(),
                    Instant::now(),
                ));
                cx.emit(MissionControlEvent::Answered {
                    agent_id: id_shared,
                    choice: ch,
                });
            } else {
                self.toast = Some((
                    tr(cx, "Approved agent action", "已批准智能体操作").into(),
                    Instant::now(),
                ));
                cx.emit(MissionControlEvent::Approved(id_shared));
            }
            cx.notify();
        }
    }

    fn resolve_reject_or_dismiss(
        &mut self,
        agent_id: &str,
        was_failed: bool,
        cx: &mut Context<Self>,
    ) {
        let changed = mutate_agent(&mut self.agents, agent_id, |ag| {
            ag.status = MissionAgentStatus::Done;
            ag.ask = None;
            ag.error = None;
        });
        if changed {
            for ev in &mut self.events {
                if ev.agent_id.as_ref() == agent_id {
                    ev.kind = MissionEventKind::Done;
                }
            }
            let id_shared = SharedString::from(agent_id.to_owned());
            if was_failed {
                self.toast = Some((
                    tr(cx, "Dismissed failed run", "已忽略失败任务").into(),
                    Instant::now(),
                ));
                cx.emit(MissionControlEvent::Dismissed(id_shared));
            } else {
                self.toast = Some((
                    tr(cx, "Rejected pending request", "已拒绝待审批请求").into(),
                    Instant::now(),
                ));
                cx.emit(MissionControlEvent::Rejected(id_shared));
            }
            cx.notify();
        }
    }

    fn resolve_retry(&mut self, agent_id: &str, cx: &mut Context<Self>) {
        let changed = mutate_agent(&mut self.agents, agent_id, |ag| {
            ag.status = MissionAgentStatus::Running;
            ag.error = None;
        });
        if changed {
            for ev in &mut self.events {
                if ev.agent_id.as_ref() == agent_id && ev.kind == MissionEventKind::Failed {
                    ev.kind = MissionEventKind::Done;
                }
            }
            self.toast = Some((
                tr(cx, "Retrying agent step", "正在重试失败步骤").into(),
                Instant::now(),
            ));
            cx.emit(MissionControlEvent::Retried(SharedString::from(
                agent_id.to_owned(),
            )));
            cx.notify();
        }
    }

    fn highlighted_agent_from_hover(&self) -> Option<&SharedString> {
        let hid = self.hovered_event.as_ref()?;
        self.events
            .iter()
            .find(|e| &e.id == hid)
            .map(|e| &e.agent_id)
    }

    fn visible_events(&self) -> Vec<&MissionRecapEvent> {
        self.events
            .iter()
            .filter(|ev| match self.filter {
                MissionFilter::All => true,
                other => find_agent_status(&self.agents, &ev.agent_id)
                    .is_some_and(|st| st.matches(other)),
            })
            .collect()
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if ev.keystroke.modifiers.platform || ev.keystroke.modifiers.control {
            return;
        }
        match ev.keystroke.key.as_str() {
            "left" => {
                self.set_filter(self.filter.prev(), cx);
                cx.stop_propagation();
            }
            "right" => {
                self.set_filter(self.filter.next(), cx);
                cx.stop_propagation();
            }
            "home" => {
                self.set_filter(MissionFilter::All, cx);
                cx.stop_propagation();
            }
            "end" => {
                self.set_filter(MissionFilter::Failed, cx);
                cx.stop_propagation();
            }
            "escape" if self.hovered_event.take().is_some() => {
                cx.emit(MissionControlEvent::EventHovered(None));
                cx.stop_propagation();
                cx.notify();
            }
            _ => {}
        }
    }

    fn render_recap_section(&self, w: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let vis = self.visible_events();
        let done_n = vis
            .iter()
            .filter(|e| e.kind == MissionEventKind::Done)
            .count();
        let needs_n = vis
            .iter()
            .filter(|e| e.kind == MissionEventKind::NeedsYou)
            .count();
        let fail_n = vis
            .iter()
            .filter(|e| e.kind == MissionEventKind::Failed)
            .count();

        // Spring-morphing peak weights across ALL events so filtering rises/settles smoothly
        let mut peaks: Vec<(f32, f32)> = Vec::with_capacity(self.events.len());
        let mut dots_meta: Vec<(SharedString, f32, MissionEventKind, f32, f32)> =
            Vec::with_capacity(self.events.len());
        for ev in &self.events {
            let is_vis = match self.filter {
                MissionFilter::All => true,
                other => find_agent_status(&self.agents, &ev.agent_id)
                    .is_some_and(|st| st.matches(other)),
            };
            let w_anim = pro_spring_f32(
                pro_eid(&self.id, format!("mc-pk-w-{}", ev.id)),
                if is_vis { ev.weight } else { 0.0 },
                0.28,
                w,
                cx,
            );
            let vis_alpha = pro_spring_f32(
                pro_eid(&self.id, format!("mc-pk-a-{}", ev.id)),
                if is_vis { 1.0 } else { 0.0 },
                0.24,
                w,
                cx,
            )
            .clamp(0.0, 1.0);
            let hot_amt = pro_spring_f32(
                pro_eid(&self.id, format!("mc-pk-h-{}", ev.id)),
                if is_vis && self.hovered_event.as_ref() == Some(&ev.id) {
                    1.0
                } else {
                    0.0
                },
                0.18,
                w,
                cx,
            )
            .clamp(0.0, 1.0);
            if w_anim > 0.004 {
                peaks.push((ev.position, w_anim));
            }
            if vis_alpha > 0.01 {
                dots_meta.push((ev.id.clone(), ev.position, ev.kind, vis_alpha, hot_amt));
            }
        }

        let hovered_info = self
            .hovered_event
            .as_ref()
            .and_then(|hid| vis.iter().copied().find(|e| &e.id == hid))
            .cloned();

        let scrub_st = w.use_keyed_state(pro_eid(&self.id, "mc-rcp-cur"), cx, |_, _| {
            (Slide::snappy(0.5), Slide::snappy(0.0), false)
        });
        let hover_pos = hovered_info.as_ref().map(|ev| ev.position);
        let (cur_u, cur_alpha) = scrub_st.update(cx, |s, cx| {
            if let Some(hp) = hover_pos {
                if !s.2 && s.1.value(cx) < 0.08 {
                    s.0.reset(hp);
                } else {
                    s.0.set(hp, cx);
                }
                s.1.set(1.0, cx);
                s.2 = true;
            } else {
                s.1.set(0.0, cx);
                s.2 = false;
            }
            (
                s.0.get(w, cx).clamp(0.0, 1.0),
                s.1.get(w, cx).clamp(0.0, 1.0),
            )
        });

        let stroke_col = p.ink.opacity(0.46);
        let fill_col = p.ink.opacity(0.08);
        let grid_col = p.line_soft;
        let card_bg = p.card;
        let done_dot = p.ink.opacity(0.62);
        let needs_dot = p.orange;
        let fail_dot = p.red;
        let cursor_col = p.line_strong;

        let curve_canvas = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 8. || bh <= 8. {
                    return;
                }
                let top_pad = 8.0_f32;
                let base_y = bh - 4.0_f32;
                let usable_h = (base_y - top_pad).max(12.0);

                // Subtle frame & vertical/horizontal gridlines
                for r in 0..=2usize {
                    let gy = bounds.top() + px((r as f32 / 2.0) * bh);
                    let mut gp = PathBuilder::stroke(px(1.));
                    gp.move_to(point(bounds.left(), gy));
                    gp.line_to(point(bounds.right(), gy));
                    if let Ok(path) = gp.build() {
                        window.paint_path(path, grid_col);
                    }
                }
                for c in 1..8usize {
                    let gx = bounds.left() + px((c as f32 / 8.0) * bw);
                    let mut vp = PathBuilder::stroke(px(1.));
                    vp.move_to(point(gx, bounds.top()));
                    vp.line_to(point(gx, bounds.bottom()));
                    if let Ok(path) = vp.build() {
                        window.paint_path(path, grid_col.opacity(grid_col.a * 0.65));
                    }
                }

                let samples = 112usize;
                let pt_at = |u: f32| -> Point<Pixels> {
                    let y_norm = eval_peak_curve(u, &peaks);
                    point(
                        bounds.left() + px(u * bw),
                        bounds.top() + px(base_y - y_norm * usable_h),
                    )
                };

                // Soft gradient area under peak curve
                if !peaks.is_empty() {
                    let mut area = PathBuilder::fill();
                    let floor = bounds.top() + px(base_y);
                    area.move_to(point(bounds.left(), floor));
                    for i in 0..=samples {
                        let u = i as f32 / samples as f32;
                        area.line_to(pt_at(u));
                    }
                    area.line_to(point(bounds.right(), floor));
                    area.close();
                    if let Ok(path) = area.build() {
                        window.paint_path(
                            path,
                            linear_gradient(
                                180.,
                                linear_color_stop(fill_col, 0.),
                                linear_color_stop(fill_col.opacity(0.0), 1.),
                            ),
                        );
                    }
                }

                // Smooth curve stroke
                let mut stroke = PathBuilder::stroke(px(1.5));
                for i in 0..=samples {
                    let u = i as f32 / samples as f32;
                    let pt = pt_at(u);
                    if i == 0 {
                        stroke.move_to(pt);
                    } else {
                        stroke.line_to(pt);
                    }
                }
                if let Ok(path) = stroke.build() {
                    window.paint_path(path, stroke_col);
                }

                if cur_alpha > 0.01 {
                    let gx = bounds.left() + px(cur_u * bw);
                    let mut cur = PathBuilder::stroke(px(1.));
                    cur.move_to(point(gx, bounds.top()));
                    cur.line_to(point(gx, bounds.bottom()));
                    if let Ok(path) = cur.build() {
                        window.paint_path(path, cursor_col.opacity(cursor_col.a * cur_alpha));
                    }
                }

                // Event dots perched on peaks
                for (_, pos, kind, vis_a, hot_t) in &dots_meta {
                    let pt = pt_at(*pos);
                    let dot_c = match kind {
                        MissionEventKind::Done => done_dot,
                        MissionEventKind::NeedsYou => needs_dot,
                        MissionEventKind::Failed => fail_dot,
                    };
                    let ri = px(2.8 + 1.5 * hot_t);
                    let ro = ri + px(1.6);
                    window.paint_quad(
                        fill(
                            Bounds::new(pt - point(ro, ro), size(ro * 2., ro * 2.)),
                            card_bg.opacity(*vis_a),
                        )
                        .corner_radii(ro),
                    );
                    window.paint_quad(
                        fill(
                            Bounds::new(pt - point(ri, ri), size(ri * 2., ri * 2.)),
                            dot_c.opacity(dot_c.a * *vis_a),
                        )
                        .corner_radii(ri),
                    );
                }
            },
        )
        .w_full()
        .h(px(78.));

        // Interactive hover & click hit targets positioned at each event peak
        let mut hit_layer = div().absolute().inset_0();
        for ev in &vis {
            let eid = ev.id.clone();
            let eid_click = ev.id.clone();
            let agent_id = ev.agent_id.clone();
            let left_frac = (ev.position - 0.045).clamp(0.0, 0.91);
            hit_layer = hit_layer.child(
                div()
                    .id(pro_eid(&self.id, format!("rcp-hit-{eid}")))
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(relative(left_frac))
                    .w(relative(0.09))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(ev_mut) = this.events.iter_mut().find(|e| e.id == eid_click) {
                            ev_mut.kind = match ev_mut.kind {
                                MissionEventKind::Done => MissionEventKind::NeedsYou,
                                _ => MissionEventKind::Done,
                            };
                            cx.notify();
                        }
                    }))
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        if *inside {
                            this.hovered_event = Some(eid.clone());
                            if let Some(lead_id) = find_lead_for_agent(&this.agents, &agent_id) {
                                this.open_lead = lead_id;
                            }
                            cx.emit(MissionControlEvent::EventHovered(Some(eid.clone())));
                        } else if this.hovered_event.as_ref() == Some(&eid) {
                            this.hovered_event = None;
                            cx.emit(MissionControlEvent::EventHovered(None));
                        }
                        cx.notify();
                    })),
            );
        }

        let tip_overlay = RecapScrubTooltip {
            id: pro_eid(&self.id, "mc-rcp-tip"),
            hovered: hovered_info,
            dark: p.dark,
        };

        div()
            .w_full()
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .shadow(pro_card_shadow(p.dark))
            .px(px(16.))
            .pt(px(14.))
            .pb(px(10.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_wrap()
                    .items_start()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "Since you looked", "自上次查看以来")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(format!(
                                        "{} {}",
                                        tr(cx, "Since", "始于"),
                                        self.since_label
                                    ))
                                    .child(
                                        div()
                                            .h(px(18.))
                                            .px(px(6.))
                                            .rounded(px(5.))
                                            .bg(p.chip)
                                            .flex()
                                            .items_center()
                                            .text_size(px(11.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.text)
                                            .child("45m"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .pt(px(2.))
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(12.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(pro_check_draw(
                                        pro_eid(&self.id, "mc-rcp-done-chk"),
                                        done_n > 0,
                                        13.,
                                        false,
                                        p.green,
                                    ))
                                    .child(pro_swap_label(
                                        pro_eid(&self.id, "mc-rcp-done-n"),
                                        done_n.to_string(),
                                        12.,
                                        FontWeight::MEDIUM,
                                        p.ink,
                                    ))
                                    .child(tr(cx, "done", "已完成")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(pro_svg("0 0 16 16", RECAP_NEEDS_SVG, 13., p.orange))
                                    .child(pro_swap_label(
                                        pro_eid(&self.id, "mc-rcp-needs-n"),
                                        needs_n.to_string(),
                                        12.,
                                        FontWeight::MEDIUM,
                                        p.ink,
                                    ))
                                    .child(tr(cx, "need you", "待你确认")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(pro_svg("0 0 16 16", RECAP_FAIL_SVG, 13., p.red))
                                    .child(pro_swap_label(
                                        pro_eid(&self.id, "mc-rcp-fail-n"),
                                        fail_n.to_string(),
                                        12.,
                                        FontWeight::MEDIUM,
                                        p.ink,
                                    ))
                                    .child(tr(cx, "failed", "失败")),
                            ),
                    ),
            )
            .child(
                div()
                    .relative()
                    .w_full()
                    .h(px(78.))
                    .child(curve_canvas)
                    .child(tip_overlay)
                    .child(hit_layer),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(10.5))
                    .text_color(p.faint)
                    .child(self.since_label.clone())
                    .child(self.now_label.clone()),
            )
    }

    fn render_status_pill(&self, agent_id: &str, status: MissionAgentStatus, cx: &App) -> Div {
        let p = ProPalette::get(cx);
        let (bg, fg, label) = match status {
            MissionAgentStatus::Running => (p.accent_soft, p.blue, tr(cx, "Running", "运行中")),
            MissionAgentStatus::Waiting => {
                (p.amber_soft, p.amber_ink, tr(cx, "Needs you", "待确认"))
            }
            MissionAgentStatus::Failed => (p.red_soft, p.red_ink, tr(cx, "Failed", "失败")),
            MissionAgentStatus::Done => (p.green_soft, p.green_ink, tr(cx, "Done", "已完成")),
        };
        let indicator: AnyElement = if status == MissionAgentStatus::Done {
            pro_check_draw(
                pro_eid(&self.id, format!("mc-st-chk-{agent_id}")),
                true,
                12.,
                false,
                fg,
            )
        } else {
            div().size(px(5.)).rounded_full().bg(fg).into_any_element()
        };
        div()
            .h(px(18.))
            .px(px(6.))
            .rounded(px(4.))
            .bg(bg)
            .flex_none()
            .flex()
            .items_center()
            .gap(px(4.))
            .child(indicator)
            .child(pro_swap_label(
                pro_eid(&self.id, format!("mc-st-lbl-{agent_id}")),
                label,
                11.,
                FontWeight::MEDIUM,
                fg,
            ))
    }

    fn render_fleet_card(
        &self,
        wide_tree: bool,
        elapsed_ms: f32,
        pulse_t: f32,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let highlighted_agent = self.highlighted_agent_from_hover().cloned();

        let mut flat = Vec::new();
        flatten_agents(&self.agents, &mut flat);
        let running_count = flat
            .iter()
            .filter(|a| a.status == MissionAgentStatus::Running)
            .count();
        let total_cost: f64 = flat.iter().map(|a| a.cost_usd).sum();

        let visible_leads: Vec<MissionAgent> = self
            .agents
            .iter()
            .filter(|lead| lead.matches_tree(self.filter))
            .cloned()
            .collect();

        type AnchorSlot = Rc<Cell<Option<Point<Pixels>>>>;
        let lead_anchor: AnchorSlot = Rc::new(Cell::new(None));
        let mut sub_anchors: Vec<(AnchorSlot, bool)> = Vec::new();

        let build_tools_strip = |tools: &[SharedString], cx: &App| -> Div {
            let mut strip = div().flex_none().flex().items_center().gap(px(3.));
            for tool in tools.iter().take(3) {
                strip = strip.child(pro_app_tile(tool, 16., cx));
            }
            strip
        };

        let mut leads_col = div()
            .when(wide_tree, |d| d.w(px(236.)).flex_none())
            .when(!wide_tree, |d| d.w_full())
            .flex()
            .flex_col()
            .gap(px(8.));

        let mut open_children: Vec<MissionAgent> = Vec::new();

        for lead in &visible_leads {
            let is_open = lead.id == self.open_lead && !lead.children.is_empty();
            let matching_children: Vec<MissionAgent> = lead
                .children
                .iter()
                .filter(|c| c.status.matches(self.filter))
                .cloned()
                .collect();
            if is_open {
                open_children = matching_children.clone();
            }

            let faded = highlighted_agent
                .as_ref()
                .is_some_and(|hid| &lead.id != hid && !lead.children.iter().any(|c| &c.id == hid));
            let lead_op = pro_spring_f32(
                pro_eid(&self.id, format!("mc-lead-op-{}", lead.id)),
                if faded { 0.42 } else { 1.0 },
                0.22,
                w,
                cx,
            );
            let sel_t = pro_spring_f32(
                pro_eid(&self.id, format!("mc-lead-sel-{}", lead.id)),
                if is_open { 1.0 } else { 0.0 },
                0.22,
                w,
                cx,
            );
            let rest_border = pro_mix_color(p.line, p.line_strong, sel_t);

            let child_summary = if lead.children.is_empty() {
                tr(cx, "Direct agent", "独立执行").to_owned()
            } else {
                let wait_c = lead
                    .children
                    .iter()
                    .filter(|c| c.status == MissionAgentStatus::Waiting)
                    .count();
                let fail_c = lead
                    .children
                    .iter()
                    .filter(|c| c.status == MissionAgentStatus::Failed)
                    .count();
                let done_c = lead
                    .children
                    .iter()
                    .filter(|c| c.status == MissionAgentStatus::Done)
                    .count();
                if fail_c > 0 {
                    format!("{fail_c} {}", tr(cx, "failed", "项失败"))
                } else if wait_c > 0 {
                    format!("{wait_c} {}", tr(cx, "need you", "项待确认"))
                } else {
                    format!(
                        "{done_c}/{} {}",
                        lead.children.len(),
                        tr(cx, "done", "已完成")
                    )
                }
            };

            let progress_frac = if lead.children.is_empty() {
                if lead.status == MissionAgentStatus::Done {
                    1.0
                } else {
                    0.65
                }
            } else {
                let done_c = lead
                    .children
                    .iter()
                    .filter(|c| c.status == MissionAgentStatus::Done)
                    .count();
                (done_c as f32 / lead.children.len() as f32).max(0.25)
            };

            let lead_id = lead.id.clone();
            let lead_seed = format!("{} {}", lead.name, lead.model);
            let col = agent_hue(&lead_id, &p);
            let anchor_cell = lead_anchor.clone();

            let lead_inner = div()
                .w_full()
                .p(px(10.))
                .opacity(lead_op)
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_agent_avatar(
                            &lead_seed,
                            col,
                            22.,
                            lead.status == MissionAgentStatus::Running,
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
                                        .text_size(px(12.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .truncate()
                                        .child(lead.name.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(lead.model.clone()),
                                ),
                        )
                        .child(self.render_status_pill(&lead_id, lead.status, cx)),
                )
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
                                .text_size(px(12.))
                                .text_color(p.text)
                                .truncate()
                                .child(lead.task.clone()),
                        )
                        .child(build_tools_strip(&lead.tools, cx)),
                )
                .child(pro_spring_bar(
                    pro_eid(&self.id, format!("mc-lead-bar-{lead_id}")),
                    progress_frac,
                    3.,
                    1.5,
                    p.chip,
                    col,
                ))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .text_size(px(11.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .text_color(p.muted)
                                .child(pro_progress_ring(
                                    progress_frac,
                                    11.,
                                    1.6,
                                    col,
                                    p.line_strong,
                                ))
                                .child(pro_swap_label(
                                    pro_eid(&self.id, format!("mc-lead-sum-{lead_id}")),
                                    child_summary,
                                    11.,
                                    FontWeight::NORMAL,
                                    p.muted,
                                )),
                        )
                        .child(
                            div()
                                .text_color(p.ink)
                                .font_weight(FontWeight::MEDIUM)
                                .child(format!("${:.2} · {}", lead.cost_usd, lead.elapsed)),
                        ),
                );

            let lead_card = pro_tactile(
                pro_eid(&self.id, format!("fleet-lead-{lead_id}")),
                lead.name.clone(),
                10.,
                p.card,
                pro_mix_color(p.card, p.hover, 0.55),
                rest_border,
                p.line_strong,
                is_open,
                lead_inner,
                cx,
                move |this, _, cx| {
                    this.open_lead = lead_id.clone();
                    cx.emit(MissionControlEvent::LeadSelected(lead_id.clone()));
                    cx.notify();
                },
            )
            .relative()
            .w_full()
            .when(is_open && wide_tree, |d| {
                d.child(
                    canvas(
                        move |b, _, _| {
                            anchor_cell.set(Some(point(b.right(), b.center().y)));
                        },
                        |_, _, _, _| (),
                    )
                    .absolute()
                    .inset_0(),
                )
            });

            leads_col = leads_col.child(lead_card);

            // On narrow screens (< 580px), smoothly fold open children directly under their lead
            if !wide_tree && !matching_children.is_empty() {
                let mut indent_col = div()
                    .pl(px(20.))
                    .ml(px(8.))
                    .border_l_1()
                    .border_color(p.blue.opacity(0.35))
                    .flex()
                    .flex_col()
                    .gap(px(6.));
                for sub in &matching_children {
                    indent_col = indent_col.child(self.render_sub_agent_card(
                        sub,
                        None,
                        &highlighted_agent,
                        elapsed_ms,
                        w,
                        cx,
                    ));
                }
                let indent_any = indent_col.into_any_element();
                leads_col = leads_col.child(pro_fold(
                    pro_eid(&self.id, format!("mc-narrow-fold-{}", lead.id)),
                    is_open,
                    180.,
                    8.,
                    move || indent_any,
                ));
            }
        }

        let body = if visible_leads.is_empty() {
            div()
                .w_full()
                .py(px(28.))
                .text_center()
                .text_size(px(12.5))
                .text_color(p.muted)
                .child(tr(
                    cx,
                    "No agents match this filter.",
                    "当前筛选下暂无智能体。",
                ))
        } else if wide_tree {
            let mut subs_col = div().flex_1().min_w_0().flex().flex_col().gap(px(8.));
            for sub in &open_children {
                let scell: Rc<Cell<Option<Point<Pixels>>>> = Rc::new(Cell::new(None));
                let live_branch = matches!(
                    sub.status,
                    MissionAgentStatus::Running | MissionAgentStatus::Waiting
                );
                sub_anchors.push((scell.clone(), live_branch));
                subs_col = subs_col.child(self.render_sub_agent_card(
                    sub,
                    Some(scell),
                    &highlighted_agent,
                    elapsed_ms,
                    w,
                    cx,
                ));
            }

            // Paint orthogonal rounded connectors with traveling light pulse via pro_connector_pulse
            let base_link = p.blue.opacity(0.36);
            let pulse_link = p.blue;
            let card_bg = p.card;
            let reduced = disabled_motion(cx);
            let connector_layer = canvas(
                |_, _, _| (),
                move |_, _, window, _| {
                    let Some(p0) = lead_anchor.get() else {
                        return;
                    };
                    for (idx, (cell, live)) in sub_anchors.iter().enumerate() {
                        let Some(p1) = cell.get() else {
                            continue;
                        };
                        let prog = if *live && !reduced {
                            Some((pulse_t + idx as f32 * 0.28).fract())
                        } else {
                            None
                        };
                        pro_connector_pulse(
                            p0, p1, 8.0, false, base_link, pulse_link, prog, 1.5, window,
                        );
                    }
                    if !sub_anchors.is_empty() {
                        let r_out = px(3.8);
                        let r_in = px(2.4);
                        window.paint_quad(
                            fill(
                                Bounds::new(p0 - point(r_out, r_out), size(r_out * 2., r_out * 2.)),
                                pulse_link,
                            )
                            .corner_radii(r_out),
                        );
                        window.paint_quad(
                            fill(
                                Bounds::new(p0 - point(r_in, r_in), size(r_in * 2., r_in * 2.)),
                                card_bg,
                            )
                            .corner_radii(r_in),
                        );
                    }
                },
            )
            .absolute()
            .inset_0();

            div()
                .relative()
                .w_full()
                .flex()
                .items_start()
                .gap(px(40.))
                .child(leads_col)
                .child(subs_col)
                .child(connector_layer)
        } else {
            leads_col
        };

        let lead_total = self.agents.len();
        let sub_total = flat.len().saturating_sub(lead_total);
        let waiting_count = flat
            .iter()
            .filter(|a| a.status == MissionAgentStatus::Waiting)
            .count();

        div()
            .w_full()
            .h_full()
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .shadow(pro_card_shadow(p.dark))
            .flex()
            .flex_col()
            .child(
                div()
                    .px(px(16.))
                    .py(px(12.))
                    .border_b_1()
                    .border_color(p.line)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(pro_icon(ProIcon::Sparkles, 14., p.blue))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "Agents", "智能体编队")),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(10.))
                            .text_size(px(11.5))
                            .text_color(p.muted)
                            .child(format!(
                                "{lead_total} {} / {sub_total} {}",
                                tr(cx, "agents", "主智能体"),
                                tr(cx, "sub-agents", "子智能体")
                            ))
                            .child(pro_swap_label(
                                pro_eid(&self.id, "mc-fleet-run-lbl"),
                                format!("{running_count} {}", tr(cx, "running", "运行中")),
                                11.5,
                                FontWeight::NORMAL,
                                p.muted,
                            ))
                            .when(waiting_count > 0, |d| {
                                d.child(format!(
                                    "{waiting_count} {}",
                                    tr(cx, "needs you", "待确认")
                                ))
                            })
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "mc-fleet-cost"),
                                total_cost,
                                2,
                                "$",
                                "",
                                12.,
                                FontWeight::MEDIUM,
                                p.ink,
                            )),
                    ),
            )
            .child(div().p(px(14.)).child(body))
    }

    fn render_sub_agent_card(
        &self,
        sub: &MissionAgent,
        anchor_cell: Option<Rc<Cell<Option<Point<Pixels>>>>>,
        highlighted_agent: &Option<SharedString>,
        elapsed_ms: f32,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let faded = highlighted_agent.as_ref().is_some_and(|hid| hid != &sub.id);
        let sub_op = pro_spring_f32(
            pro_eid(&self.id, format!("mc-sub-op-{}", sub.id)),
            if faded { 0.42 } else { 1.0 },
            0.22,
            w,
            cx,
        );
        let col = agent_hue(&sub.id, &p);
        let sub_id = sub.id.clone();
        let sub_seed = format!("{} {}", sub.name, sub.model);

        let quick_action = match sub.status {
            MissionAgentStatus::Waiting => {
                let aid = sub_id.clone();
                let first_choice = sub.ask.as_ref().and_then(|a| a.choices.first().cloned());
                let btn_label = if first_choice.is_some() {
                    tr(cx, "Answer", "回答")
                } else {
                    tr(cx, "Approve", "批准")
                };
                Some(
                    pro_tactile(
                        pro_eid(&self.id, format!("sub-act-{aid}")),
                        btn_label,
                        5.,
                        p.chip,
                        p.chip_hover,
                        p.line_strong,
                        p.line_strong,
                        false,
                        div()
                            .h(px(20.))
                            .px(px(7.))
                            .flex()
                            .items_center()
                            .whitespace_nowrap()
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(btn_label),
                        cx,
                        move |this, _, cx| {
                            this.resolve_approve(&aid, first_choice.clone(), cx);
                        },
                    )
                    .flex_none(),
                )
            }
            MissionAgentStatus::Failed => {
                let aid = sub_id.clone();
                let retry_lbl = tr(cx, "Retry", "重试");
                Some(
                    pro_tactile(
                        pro_eid(&self.id, format!("sub-retry-{aid}")),
                        retry_lbl,
                        5.,
                        p.chip,
                        p.chip_hover,
                        p.line_strong,
                        p.line_strong,
                        false,
                        div()
                            .h(px(20.))
                            .px(px(7.))
                            .flex()
                            .items_center()
                            .whitespace_nowrap()
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(retry_lbl),
                        cx,
                        move |this, _, cx| {
                            this.resolve_retry(&aid, cx);
                        },
                    )
                    .flex_none(),
                )
            }
            _ => None,
        };

        let lead_tool = sub.tools.first().map(|t| pro_app_tile(t, 15., cx));

        div()
            .relative()
            .w_full()
            .min_w_0()
            .rounded(px(9.))
            .bg(p.subtle)
            .border_1()
            .border_color(p.line)
            .p(px(9.))
            .opacity(sub_op)
            .flex()
            .flex_col()
            .gap(px(5.))
            .when_some(anchor_cell, |d, cell| {
                d.child(
                    canvas(
                        move |b, _, _| {
                            cell.set(Some(point(b.left(), b.center().y)));
                        },
                        |_, _, _, _| (),
                    )
                    .absolute()
                    .inset_0(),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .child(pro_agent_avatar(
                        &sub_seed,
                        col,
                        20.,
                        sub.status == MissionAgentStatus::Running,
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
                                    .text_size(px(12.))
                                    .line_height(px(15.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .truncate()
                                    .child(sub.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .line_height(px(13.))
                                    .text_color(p.muted)
                                    .truncate()
                                    .child(sub.model.clone()),
                            ),
                    )
                    .child(self.render_status_pill(&sub.id, sub.status, cx)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .min_w_0()
                    .when_some(lead_tool, |d, tile| d.child(tile))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(11.5))
                            .text_color(p.text)
                            .truncate()
                            .child(sub.task.clone()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(6.))
                    .text_size(px(11.))
                    .text_color(p.muted)
                    .child(format!("${:.2} · {}", sub.cost_usd, sub.elapsed))
                    .when_some(quick_action, |d, btn| d.child(btn)),
            )
    }

    fn render_inbox_queue(&self, w: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let highlighted_agent = self.highlighted_agent_from_hover().cloned();

        let mut flat = Vec::new();
        flatten_agents(&self.agents, &mut flat);
        let actionable: Vec<MissionAgent> = flat
            .into_iter()
            .filter(|a| {
                matches!(
                    a.status,
                    MissionAgentStatus::Waiting | MissionAgentStatus::Failed
                ) && a.status.matches(self.filter)
            })
            .cloned()
            .collect();

        let item_bg: Hsla = if p.dark {
            rgb(0x202024).into()
        } else {
            p.subtle
        };

        type InboxPillAction =
            Rc<dyn Fn(&mut MissionControlBlock, &mut Window, &mut Context<MissionControlBlock>)>;

        let quiet_pill = |id: ElementId,
                          label: SharedString,
                          emphasized: bool,
                          hov_t: f32,
                          cx: &mut Context<Self>,
                          on_press: InboxPillAction|
         -> Stateful<Div> {
            let rest_bg = if emphasized {
                pro_mix_color(p.chip, p.chip_hover, hov_t * 0.85)
            } else {
                transparent_black()
            };
            let hover_bg = if emphasized {
                pro_mix_color(p.chip_hover, p.ink.opacity(0.16), 0.5)
            } else {
                p.chip
            };
            let rest_bdr = if emphasized {
                pro_mix_color(p.line, p.line_strong, 0.5 + 0.5 * hov_t)
            } else {
                pro_mix_color(transparent_black(), p.line, hov_t)
            };
            let fg = if emphasized {
                pro_mix_color(p.text, p.strong, 0.55 + 0.45 * hov_t)
            } else {
                pro_mix_color(p.muted, p.text, hov_t)
            };
            pro_tactile(
                id,
                label.clone(),
                5.,
                rest_bg,
                hover_bg,
                rest_bdr,
                p.line_strong,
                false,
                div()
                    .h(px(20.))
                    .px(px(6.5))
                    .flex()
                    .items_center()
                    .whitespace_nowrap()
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(fg)
                    .child(label),
                cx,
                move |this, win, app_cx| {
                    (on_press)(this, win, app_cx);
                },
            )
            .flex_none()
        };

        let mut cards_col = div().w_full().flex().flex_col().gap(px(7.));
        if actionable.is_empty() {
            cards_col = cards_col.child(
                div()
                    .w_full()
                    .py(px(36.))
                    .px(px(12.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(8.))
                    .text_center()
                    .text_size(px(12.5))
                    .text_color(p.muted)
                    .child(pro_check_draw(
                        pro_eid(&self.id, "mc-inb-clear-chk"),
                        true,
                        18.,
                        false,
                        p.green,
                    ))
                    .child(tr(cx, "Nothing waiting on you", "暂无待你处理的请求")),
            );
        } else {
            for ag in &actionable {
                let aid = ag.id.clone();
                let faded = highlighted_agent.as_ref().is_some_and(|hid| hid != &aid);
                let inb_op = pro_spring_f32(
                    pro_eid(&self.id, format!("mc-inb-op-{aid}")),
                    if faded { 0.42 } else { 1.0 },
                    0.22,
                    w,
                    cx,
                );
                let hov_st = w.use_keyed_state(
                    pro_eid(&self.id, format!("mc-inb-hov-{aid}")),
                    cx,
                    |_, _| Slide::snappy(0.),
                );
                let hov_t = hov_st.update(cx, |s, cx| s.get(w, cx).clamp(0., 1.));
                let is_failed = ag.status == MissionAgentStatus::Failed;

                let (headline, desc) = if is_failed {
                    (
                        ag.task.clone(),
                        ag.error
                            .clone()
                            .unwrap_or_else(|| SharedString::from("Execution failed")),
                    )
                } else if let Some(ask) = &ag.ask {
                    (ask.title.clone(), ask.detail.clone())
                } else {
                    (ag.task.clone(), "Waiting on your confirmation".into())
                };

                let actions_row = if is_failed {
                    let aid_dis = aid.clone();
                    let aid_ret = aid.clone();
                    div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(px(4.))
                        .child(quiet_pill(
                            pro_eid(&self.id, format!("inb-dismiss-{aid}")),
                            tr(cx, "Dismiss", "忽略").into(),
                            false,
                            hov_t,
                            cx,
                            Rc::new(move |this, _, cx| {
                                this.resolve_reject_or_dismiss(&aid_dis, true, cx);
                            }),
                        ))
                        .child(quiet_pill(
                            pro_eid(&self.id, format!("inb-retry-{aid}")),
                            tr(cx, "Retry", "重试").into(),
                            true,
                            hov_t,
                            cx,
                            Rc::new(move |this, _, cx| {
                                this.resolve_retry(&aid_ret, cx);
                            }),
                        ))
                } else if let Some(ask) = &ag.ask
                    && !ask.choices.is_empty()
                {
                    let mut row = div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_end()
                        .gap(px(4.));
                    for (ci, choice) in ask.choices.iter().enumerate() {
                        let aid_ch = aid.clone();
                        let ch_val = choice.clone();
                        let is_primary = ci + 1 == ask.choices.len();
                        row = row.child(quiet_pill(
                            pro_eid(&self.id, format!("inb-ans-{aid}-{ci}")),
                            ch_val.clone(),
                            is_primary,
                            hov_t,
                            cx,
                            Rc::new(move |this, _, cx| {
                                this.resolve_approve(&aid_ch, Some(ch_val.clone()), cx);
                            }),
                        ));
                    }
                    row
                } else {
                    let aid_rej = aid.clone();
                    let aid_app = aid.clone();
                    div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(px(4.))
                        .child(quiet_pill(
                            pro_eid(&self.id, format!("inb-rej-{aid}")),
                            tr(cx, "Reject", "拒绝").into(),
                            false,
                            hov_t,
                            cx,
                            Rc::new(move |this, _, cx| {
                                this.resolve_reject_or_dismiss(&aid_rej, false, cx);
                            }),
                        ))
                        .child(quiet_pill(
                            pro_eid(&self.id, format!("inb-app-{aid}")),
                            tr(cx, "Approve", "批准").into(),
                            true,
                            hov_t,
                            cx,
                            Rc::new(move |this, _, cx| {
                                this.resolve_approve(&aid_app, None, cx);
                            }),
                        ))
                };

                let status_meta = if is_failed {
                    div()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(11.))
                        .child(div().size(px(6.)).rounded_full().bg(p.red))
                        .child(
                            div()
                                .truncate()
                                .text_color(p.red_ink)
                                .font_weight(FontWeight::MEDIUM)
                                .child(format!("{} · {}", tr(cx, "Failed", "失败"), ag.elapsed)),
                        )
                } else {
                    div()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(11.))
                        .child(pro_svg("0 0 16 16", RECAP_FAIL_SVG, 11., p.amber))
                        .child(div().truncate().text_color(p.muted).child(format!(
                            "{} · {}",
                            tr(cx, "Waiting for you", "等待你确认"),
                            ag.elapsed
                        )))
                };

                let card_border = pro_mix_color(p.line, p.line_strong, hov_t);
                cards_col = cards_col.child(
                    div()
                        .id(pro_eid(&self.id, format!("inb-card-{aid}")))
                        .w_full()
                        .rounded(px(9.))
                        .bg(item_bg)
                        .border_1()
                        .border_color(card_border)
                        .px(px(11.))
                        .py(px(9.))
                        .opacity(inb_op)
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .on_hover({
                            let hov_st = hov_st.clone();
                            move |inside, w, cx| {
                                hov_st.update(cx, |s, cx| {
                                    s.set(if *inside { 1. } else { 0. }, cx);
                                    cx.notify();
                                });
                                w.refresh();
                            }
                        })
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(pro_icon(ProIcon::Sparkles, 12., p.amber))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(12.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .child(headline),
                                ),
                        )
                        .child(
                            div()
                                .pl(px(18.))
                                .text_size(px(11.5))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(desc),
                        )
                        .child(
                            div()
                                .pl(px(18.))
                                .pt(px(1.))
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .justify_between()
                                .gap(px(6.))
                                .child(status_meta)
                                .child(actions_row),
                        ),
                );
            }
        }

        div()
            .w_full()
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .shadow(pro_card_shadow(p.dark))
            .p(px(10.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .h(px(24.))
                    .px(px(4.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_icon(ProIcon::Sparkles, 12.5, p.blue))
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "NEEDS YOU", "待你处理")),
                    )
                    .child(
                        div()
                            .min_w(px(18.))
                            .h(px(17.))
                            .px(px(5.))
                            .rounded(px(4.))
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_swap_label(
                                pro_eid(&self.id, "mc-inb-cnt"),
                                actionable.len().to_string(),
                                10.5,
                                FontWeight::MEDIUM,
                                p.text,
                            )),
                    ),
            )
            .child(cards_col)
    }
}

struct RecapTipState {
    last_ev: Option<MissionRecapEvent>,
    x_ratio: Slide,
    alpha: Slide,
    was_open: bool,
    host_w: Rc<Cell<f32>>,
}

#[derive(IntoElement)]
struct RecapScrubTooltip {
    id: ElementId,
    hovered: Option<MissionRecapEvent>,
    dark: bool,
}

impl RenderOnce for RecapScrubTooltip {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let st = w.use_keyed_state(pro_eid(&self.id, "rcp-tip-st"), cx, |_, _| RecapTipState {
            last_ev: None,
            x_ratio: Slide::snappy(0.5),
            alpha: Slide::snappy(0.),
            was_open: false,
            host_w: Rc::new(Cell::new(720.)),
        });
        let (shown_ev, rx, amt, host_cell) = st.update(cx, |s, cx| {
            if let Some(ev) = self.hovered {
                if !s.was_open && s.alpha.value(cx) < 0.08 {
                    s.x_ratio.reset(ev.position);
                } else {
                    s.x_ratio.set(ev.position, cx);
                }
                s.last_ev = Some(ev);
                s.alpha.set(1.0, cx);
                s.was_open = true;
            } else {
                s.alpha.set(0.0, cx);
                s.was_open = false;
            }
            (
                s.last_ev.clone(),
                s.x_ratio.get(w, cx).clamp(0.0, 1.0),
                s.alpha.get(w, cx).clamp(0.0, 1.0),
                s.host_w.clone(),
            )
        });

        let cw = host_cell.get().max(260.);
        let measure_cell = host_cell.clone();
        let sensor = canvas(
            move |b, window, _| {
                let nw = f32::from(b.size.width);
                if nw > 12. && (measure_cell.get() - nw).abs() > 1.0 {
                    measure_cell.set(nw);
                    window.request_animation_frame();
                }
            },
            |_, _, _, _| (),
        )
        .w_full()
        .h(px(0.));

        let Some(ev) = shown_ev.filter(|_| amt > 0.01) else {
            return div()
                .absolute()
                .left_0()
                .right_0()
                .top_0()
                .child(sensor)
                .into_any_element();
        };

        let tip_bg: Hsla = if self.dark {
            rgb(0xf4f4f5).into()
        } else {
            rgb(0x18181b).into()
        };
        let tip_fg: Hsla = if self.dark {
            rgb(0x111113).into()
        } else {
            rgb(0xfafafa).into()
        };
        let pill_w = 248.0_f32;
        let left_px = (rx * cw - pill_w * 0.5).clamp(6., (cw - pill_w - 6.).max(6.));

        div()
            .absolute()
            .left_0()
            .right_0()
            .top_0()
            .child(sensor)
            .child(
                div()
                    .absolute()
                    .left(px(left_px))
                    .top(px(4. + 4. * (1. - amt)))
                    .max_w(px(280.))
                    .px(px(8.))
                    .py(px(3.))
                    .rounded(px(6.))
                    .bg(tip_bg)
                    .text_color(tip_fg)
                    .text_size(px(11.5))
                    .shadow_sm()
                    .opacity(amt)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(div().opacity(0.68).child(ev.at_label))
                    .child(div().min_w_0().truncate().child(ev.title)),
            )
            .into_any_element()
    }
}

impl Render for MissionControlBlock {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let has_running = count_mission_agents(&self.agents, MissionFilter::Running) > 0;
        if (has_running || self.loading) && !reduced {
            w.request_animation_frame();
        }

        let elapsed_secs = self.boot_at.elapsed().as_secs_f32();
        let elapsed_ms = elapsed_secs * 1000.0;
        let pulse_t = (elapsed_secs / 2.2).fract();

        let width = self.measured_width.get();
        let stack_columns = width < 720.;
        let wide_fleet_tree = if stack_columns {
            width >= 540.
        } else {
            width >= 720.
        };
        let compact = width < 480.;
        let measure_cell = self.measured_width.clone();

        // Top Bar: "Mission control" + Unified Filter (All 7 | Running 3 | Needs you 3 | Failed 1)
        let heading = if self.title.as_ref() == "Mission control" {
            tr(cx, "Mission control", "任务指挥台").to_owned()
        } else {
            self.title.to_string()
        };

        let filter_sensor = pro_selection_sensor();
        let thumb_bg: Hsla = if p.dark {
            rgb(0x2a2a2e).into()
        } else {
            rgb(0xffffff).into()
        };
        let mut filter_bar = div()
            .relative()
            .p(px(2.))
            .rounded(px(9.))
            .bg(p.chip)
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "mc-filter-pill"),
                self.filter.slug(),
                filter_sensor.clone(),
                7.,
                thumb_bg,
                Some(p.line),
                true,
            ));

        for f in MissionFilter::ALL {
            let sel = self.filter == f;
            let count = count_mission_agents(&self.agents, f);
            let f_lbl = f.label(cx);
            filter_bar = filter_bar.child(
                pro_tactile(
                    pro_eid(&self.id, format!("mc-filter-{}", f.slug())),
                    format!("{f_lbl} {count}"),
                    7.,
                    transparent_black(),
                    if sel { transparent_black() } else { p.hover },
                    transparent_black(),
                    transparent_black(),
                    sel,
                    div()
                        .h(px(26.))
                        .px(px(9.))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .whitespace_nowrap()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if sel { p.strong } else { p.text })
                        .child(f_lbl)
                        .child(pro_swap_label(
                            pro_eid(&self.id, format!("mc-fcnt-{}", f.slug())),
                            count.to_string(),
                            12.5,
                            FontWeight::MEDIUM,
                            p.muted,
                        )),
                    cx,
                    move |this, _, cx| {
                        this.set_filter(f, cx);
                    },
                )
                .role(Role::RadioButton)
                .relative()
                .flex_none()
                .when(sel, |d| d.child(pro_mark_selected(&filter_sensor))),
            );
        }

        let top_bar = div()
            .w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(heading),
            )
            .child(filter_bar);

        if self.loading {
            let phase = (elapsed_secs * 0.9).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(top_bar)
                .child(
                    div()
                        .w_full()
                        .h(px(148.))
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .shadow(pro_card_shadow(p.dark))
                        .p(px(16.))
                        .child(pro_skeleton_bar(240., 84., 8., phase, cx)),
                )
                .child(
                    div()
                        .w_full()
                        .h(px(280.))
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .shadow(pro_card_shadow(p.dark))
                        .p(px(16.))
                        .child(pro_skeleton_bar(260., 180., 8., phase + 0.2, cx)),
                );
        }

        if self.empty || self.agents.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(top_bar)
                .child(
                    div()
                        .w_full()
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .shadow(pro_card_shadow(p.dark))
                        .child(pro_empty_state(
                            ProIcon::Sparkles,
                            tr(cx, "All quiet in Mission Control", "任务指挥台当前空闲"),
                            tr(
                                cx,
                                "Active agents, sub-agent trees, recap peaks, and approval requests appear here.",
                                "启动智能体后，此处将汇总事件曲线、主从编队树与待审批事项。",
                            ),
                            None,
                            cx,
                        )),
                );
        }

        let recap_card = self.render_recap_section(w, cx);
        let fleet_card = self.render_fleet_card(wide_fleet_tree, elapsed_ms, pulse_t, w, cx);
        let inbox_queue = self.render_inbox_queue(w, cx);

        let lower_grid = div()
            .w_full()
            .flex()
            .when(stack_columns, |d| d.flex_col())
            .items_start()
            .gap(px(14.))
            .child(div().flex_1().min_w_0().w_full().child(fleet_card))
            .child(
                div()
                    .when(!stack_columns, |d| d.w(px(304.)).flex_none())
                    .when(stack_columns, |d| d.w_full())
                    .child(inbox_queue),
            );

        div()
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(self.title.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .relative()
            .w_full()
            .min_w_0()
            .px(px(if compact { 12. } else { 18. }))
            .py(px(if compact { 16. } else { 22. }))
            .flex()
            .flex_col()
            .gap(px(14.))
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
                .w_full()
                .h(px(0.)),
            )
            .child(top_bar)
            .child(pro_view_transition(
                pro_eid(&self.id, "mc-body-vtx"),
                SharedString::from("mc-ready"),
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(recap_card)
                    .child(lower_grid),
            ))
            .child(pro_toast_banner(self.toast.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MissionAgentStatus, MissionFilter, count_mission_agents, default_mission_agents,
        default_recap_events, eval_peak_curve, mutate_agent,
    };

    #[test]
    fn default_fleet_counts_match_reference_7_3_3_1_distribution() {
        let agents = default_mission_agents();
        let events = default_recap_events();

        assert_eq!(count_mission_agents(&agents, MissionFilter::All), 7);
        assert_eq!(count_mission_agents(&agents, MissionFilter::Running), 3);
        assert_eq!(count_mission_agents(&agents, MissionFilter::NeedsYou), 3);
        assert_eq!(count_mission_agents(&agents, MissionFilter::Failed), 1);
        assert_eq!(events.len(), 7);
    }

    #[test]
    fn approving_and_retrying_updates_unified_filter_counts() {
        let mut agents = default_mission_agents();
        assert!(mutate_agent(&mut agents, "release", |ag| {
            ag.status = MissionAgentStatus::Running;
            ag.ask = None;
        }));
        assert_eq!(count_mission_agents(&agents, MissionFilter::Running), 4);
        assert_eq!(count_mission_agents(&agents, MissionFilter::NeedsYou), 2);

        assert!(mutate_agent(&mut agents, "billing", |ag| {
            ag.status = MissionAgentStatus::Running;
            ag.error = None;
        }));
        assert_eq!(count_mission_agents(&agents, MissionFilter::Failed), 0);
        assert_eq!(count_mission_agents(&agents, MissionFilter::NeedsYou), 1);

        let peak = eval_peak_curve(0.50, &[(0.50, 0.80)]);
        let far = eval_peak_curve(0.05, &[(0.50, 0.80)]);
        assert!(peak > 0.75);
        assert!(far < 0.01);
    }
}
