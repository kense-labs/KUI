//! Persistent multi-agent crew tree with live handoff pulses and steering controls.
//!
//! Renders a lead agent and its sub-agents as a permanent live tree of cards on soft trays,
//! animating handoffs along rounded connectors with directional arrival glow and per-agent steering.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_connector_pulse,
        pro_ease, pro_eid, pro_empty_state, pro_modal_appear, pro_shimmer_text, pro_skeleton_bar,
        pro_spring_bar, pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        family::disabled_motion,
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

const CORNER_HANDOFF_SVG: &str = r#"<path d="M5 5V10.5C5 12.433 6.567 14 8.5 14H19M19 14L15 10M19 14L15 18" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8"/>"#;
const SPIN_ARC_SVG: &str = r#"<circle cx="12" cy="12" r="8.5" stroke="currentColor" stroke-opacity="0.25" stroke-width="2.5"/><path d="M12 3.5A8.5 8.5 0 0 1 20.5 12" stroke="currentColor" stroke-linecap="round" stroke-width="2.5"/>"#;

/// Runtime lifecycle state of an agent inside [`AgentCrew`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CrewStatus {
    /// Actively working; pixel mark twinkles and activity line shimmers.
    Active,
    /// Resting between delegated tasks.
    #[default]
    Idle,
    /// Temporarily suspended by operator steering.
    Paused,
    /// Waiting on human sign-off or clarification.
    Waiting,
    /// Completed its assigned task cleanly.
    Done,
    /// Stopped due to an execution or tool error.
    Failed,
}

impl CrewStatus {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Active => tr(cx, "Working", "工作中"),
            Self::Idle => tr(cx, "Idle", "空闲"),
            Self::Paused => tr(cx, "Paused", "已暂停"),
            Self::Waiting => tr(cx, "Needs you", "待确认"),
            Self::Done => tr(cx, "Done", "已完成"),
            Self::Failed => tr(cx, "Failed", "失败"),
        }
    }
}

/// Most recent instruction or artifact handed to a [`CrewAgent`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrewLastHandoff {
    /// Sender agent or human name (for example `"Lead"`).
    pub from: SharedString,
    /// Concise handoff summary shown in the tray footer.
    pub text: SharedString,
}

impl CrewLastHandoff {
    /// Create a [`CrewLastHandoff`] record from `from` and `text`.
    pub fn new(from: impl Into<SharedString>, text: impl Into<SharedString>) -> Self {
        Self {
            from: from.into(),
            text: text.into(),
        }
    }
}

/// One node in the persistent [`AgentCrew`] hierarchy.
#[derive(Clone, Debug, PartialEq)]
pub struct CrewAgent {
    /// Stable identifier used for handoff routing, focus, and steering.
    pub id: SharedString,
    /// Display name shown on the card header.
    pub name: SharedString,
    /// Model label shown beneath `name` (for example `"Claude Opus"`).
    pub model: SharedString,
    /// Current runtime status of this agent.
    pub status: CrewStatus,
    /// Present-tense one-line activity or resting state.
    pub activity: SharedString,
    /// Context window utilization ratio (`0.0..=1.0`, clamped safely).
    pub context: f32,
    /// Maximum context capacity in tokens (for example `200_000`).
    pub context_limit: u32,
    /// Connected tool or integration names shown as overlapping tray tiles.
    pub tools: Vec<SharedString>,
    /// Most recent handoff received by this agent.
    pub last: Option<CrewLastHandoff>,
    /// Detailed prompt or objective inspected inside the steering dialog.
    pub task: SharedString,
    /// Accumulated USD spend for this agent (`NaN` is omitted safely).
    pub cost: f64,
    /// Base elapsed runtime in seconds prior to live ticking.
    pub elapsed_secs: u64,
    /// Child sub-agents coordinated directly by this agent.
    pub children: Vec<CrewAgent>,
}

impl CrewAgent {
    /// Construct a [`CrewAgent`] with `id`, `name`, `status`, and `activity`.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        status: CrewStatus,
        activity: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            model: "Claude Sonnet".into(),
            status,
            activity: activity.into(),
            context: 0.18,
            context_limit: 200_000,
            tools: Vec::new(),
            last: None,
            task: SharedString::default(),
            cost: 0.12,
            elapsed_secs: 140,
            children: Vec::new(),
        }
    }

    /// Set the model name displayed under the agent title.
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = model.into();
        self
    }

    /// Set context window ratio (`0.0..=1.0`) and token limit.
    pub fn with_context(mut self, ratio: f32, limit: u32) -> Self {
        self.context = sanitize_ratio(ratio);
        self.context_limit = limit.max(1_000);
        self
    }

    /// Attach tool names rendered as small overlapping tiles on the tray.
    pub fn with_tools(mut self, tools: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.tools = tools.into_iter().map(Into::into).collect();
        self
    }

    /// Set the most recent handoff shown on the card tray.
    pub fn with_last(
        mut self,
        from: impl Into<SharedString>,
        text: impl Into<SharedString>,
    ) -> Self {
        self.last = Some(CrewLastHandoff::new(from, text));
        self
    }

    /// Set the detailed assignment shown when opening the steering dialog.
    pub fn with_task(mut self, task: impl Into<SharedString>) -> Self {
        self.task = task.into();
        self
    }

    /// Set the accumulated cost in USD and base elapsed seconds.
    pub fn with_usage(mut self, cost: f64, elapsed_secs: u64) -> Self {
        self.cost = cost;
        self.elapsed_secs = elapsed_secs;
        self
    }

    /// Attach child sub-agents below this agent.
    pub fn with_children(mut self, children: Vec<CrewAgent>) -> Self {
        self.children = children;
        self
    }
}

/// Directed work handoff animated across [`AgentCrew`] connectors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrewHandoff {
    /// Unique identifier ensuring each handoff animates once.
    pub id: SharedString,
    /// Source agent id.
    pub from: SharedString,
    /// Destination agent id.
    pub to: SharedString,
    /// Optional message handed to the destination agent.
    pub label: Option<SharedString>,
}

impl CrewHandoff {
    /// Construct a [`CrewHandoff`] between `from` and `to`.
    pub fn new(
        id: impl Into<SharedString>,
        from: impl Into<SharedString>,
        to: impl Into<SharedString>,
        label: Option<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            label,
        }
    }
}

/// Project entry selectable from the top-left crew switcher.
#[derive(Clone, Debug, PartialEq)]
pub struct CrewProject {
    /// Stable project identifier.
    pub id: SharedString,
    /// Project display title shown in the header and dropdown menu.
    pub name: SharedString,
    /// Lead agent root for this project's crew.
    pub root: CrewAgent,
}

impl CrewProject {
    /// Construct a [`CrewProject`] with `id`, `name`, and `root` crew tree.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        root: CrewAgent,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            root,
        }
    }
}

/// Operator steering action triggered from an [`AgentCrew`] inspector dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrewSteerAction {
    /// Pause an actively working agent.
    Pause,
    /// Resume a paused or idle agent.
    Resume,
    /// Stop execution and return the agent to rest.
    Stop,
    /// Approve a waiting gate so the agent continues.
    Approve,
    /// Decline a waiting gate and pause the agent.
    Decline,
    /// Retry a failed agent run.
    Retry,
    /// Deliver a direct text instruction to the agent.
    Message,
}

/// Typed events emitted by [`AgentCrew`].
#[derive(Clone, Debug, PartialEq)]
pub enum AgentCrewEvent {
    /// User clicked or activated an agent card to inspect it.
    Selected {
        /// Identifier of the selected agent.
        agent_id: SharedString,
    },
    /// User switched the active project from the header listbox.
    ProjectChanged {
        /// Newly selected project identifier.
        project_id: SharedString,
    },
    /// A handoff pulse started traveling across the crew connectors.
    HandoffTriggered {
        /// Handoff specification.
        handoff: CrewHandoff,
    },
    /// Operator steered an agent or sent a direct instruction.
    Steered {
        /// Target agent identifier.
        agent_id: SharedString,
        /// Steering action requested.
        action: CrewSteerAction,
        /// Optional instruction text when `action == CrewSteerAction::Message`.
        message: Option<SharedString>,
    },
}

impl EventEmitter<AgentCrewEvent> for AgentCrew {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GlowSide {
    Top,
    Left,
}

#[derive(Clone, Debug)]
struct ActiveFlight {
    hops: Vec<(SharedString, SharedString)>,
    to_id: SharedString,
    from_name: SharedString,
    label: Option<SharedString>,
    started: Instant,
    hop_secs: f32,
    applied: bool,
}

/// Persistent live tree of a lead agent and its sub-agents with animated handoffs and steering.
///
/// # Examples
///
/// ```ignore
/// use kui::components::agent_crew::AgentCrew;
///
/// let crew = cx.new(|cx| AgentCrew::new("checkout-crew", cx));
/// ```
pub struct AgentCrew {
    id: ElementId,
    focus: FocusHandle,
    title: Option<SharedString>,
    projects: Vec<CrewProject>,
    custom_data: bool,
    active_project_idx: usize,
    project_menu_open: bool,
    focused_card_idx: usize,
    open_agent_id: Option<SharedString>,
    seen_handoffs: Vec<SharedString>,
    active_flight: Option<ActiveFlight>,
    card_glows: HashMap<SharedString, (Instant, GlowSide)>,
    auto_handoffs: bool,
    demo_step: usize,
    next_demo_at: Instant,
    steer_editor: Option<Entity<Editor>>,
    sr_status: SharedString,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
    boot: Instant,
    measured_width: Rc<Cell<f32>>,
}

impl AgentCrew {
    /// Create a live [`AgentCrew`] initialized with three switchable multi-agent projects.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let now = Instant::now();
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: None,
            projects: default_crew_projects(),
            custom_data: false,
            active_project_idx: 0,
            project_menu_open: false,
            focused_card_idx: 0,
            open_agent_id: None,
            seen_handoffs: Vec::new(),
            active_flight: Some(ActiveFlight {
                hops: vec![
                    ("lead".into(), "research".into()),
                    ("research".into(), "web-search".into()),
                ],
                to_id: "web-search".into(),
                from_name: "Researcher".into(),
                label: Some("Known empty-session issues?".into()),
                started: now,
                hop_secs: 0.62,
                applied: false,
            }),
            card_glows: HashMap::new(),
            auto_handoffs: true,
            demo_step: 1,
            next_demo_at: now,
            steer_editor: None,
            sr_status: "Researcher handed to Web search: Known empty-session issues?".into(),
            toast: None,
            loading: false,
            empty: false,
            boot: now,
            measured_width: Rc::new(Cell::new(820.0)),
        }
    }

    /// Override the single-project header title (hides the multi-project dropdown unless `with_projects` is also called).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        let t = title.into();
        self.title = Some(t.clone());
        if !self.custom_data
            && let Some(first) = self.projects.first_mut()
        {
            first.name = t;
            self.projects.truncate(1);
        }
        self
    }

    /// Replace the crew hierarchy with a single custom root [`CrewAgent`].
    pub fn with_root(mut self, root: CrewAgent) -> Self {
        let name = self.title.clone().unwrap_or_else(|| "Agent crew".into());
        self.projects = vec![CrewProject::new("default", name, root)];
        self.active_project_idx = 0;
        self.custom_data = true;
        self.active_flight = None;
        self.auto_handoffs = false;
        self
    }

    /// Provide multiple switchable [`CrewProject`] trees for the header dropdown.
    pub fn with_projects(mut self, projects: Vec<CrewProject>) -> Self {
        if projects.is_empty() {
            self.projects.clear();
            self.empty = true;
        } else {
            self.projects = projects;
            self.active_project_idx = self
                .active_project_idx
                .min(self.projects.len().saturating_sub(1));
        }
        self.custom_data = true;
        self.active_flight = None;
        self
    }

    /// Select which project starts active by its `id`.
    pub fn with_active_project(mut self, project_id: impl Into<SharedString>) -> Self {
        let target = project_id.into();
        if let Some(idx) = self.projects.iter().position(|p| p.id == target) {
            self.active_project_idx = idx;
        }
        self
    }

    /// Seed initial handoffs; the final new handoff launches across the connectors.
    pub fn with_handoffs(mut self, handoffs: Vec<CrewHandoff>) -> Self {
        self.active_flight = None;
        for h in handoffs {
            if !self.seen_handoffs.contains(&h.id) {
                self.seen_handoffs.push(h.id.clone());
                self.start_handoff_internal(&h, false);
            }
        }
        self
    }

    /// Enable or disable periodic background demo handoffs.
    pub fn with_auto_handoffs(mut self, enabled: bool) -> Self {
        self.auto_handoffs = enabled;
        if !enabled {
            self.active_flight = None;
        }
        self
    }

    /// Render loading skeleton placeholders when `loading` is `true`.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the empty crew placeholder state when `empty` is `true`.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the currently selected project identifier, if any.
    pub fn active_project_id(&self) -> Option<&SharedString> {
        self.projects.get(self.active_project_idx).map(|p| &p.id)
    }

    /// Return the agent identifier whose steering dialog is currently open.
    pub fn selected_agent_id(&self) -> Option<&SharedString> {
        self.open_agent_id.as_ref()
    }

    /// Count total agents in the active project tree.
    pub fn agent_count(&self) -> usize {
        self.current_root().map(count_tree_all).unwrap_or(0)
    }

    /// Count actively working agents in the active project tree.
    pub fn working_count(&self) -> usize {
        self.current_root()
            .map(|r| count_tree_by(r, |a| a.status == CrewStatus::Active))
            .unwrap_or(0)
    }

    /// Toggle skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Return whether skeleton loading is active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle the empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    /// Return whether the crew is currently in its empty state.
    pub fn is_empty(&self) -> bool {
        self.empty || self.projects.is_empty()
    }

    /// Switch to the project matching `project_id`.
    pub fn select_project(&mut self, project_id: impl Into<SharedString>, cx: &mut Context<Self>) {
        let target = project_id.into();
        if let Some(idx) = self.projects.iter().position(|p| p.id == target) {
            self.project_menu_open = false;
            if self.active_project_idx != idx {
                self.active_project_idx = idx;
                self.focused_card_idx = 0;
                self.open_agent_id = None;
                self.active_flight = None;
                self.card_glows.clear();
                cx.emit(AgentCrewEvent::ProjectChanged { project_id: target });
            }
            cx.notify();
        }
    }

    /// Trigger a new [`CrewHandoff`] along the connectors if its `id` has not run yet.
    pub fn push_handoff(&mut self, handoff: CrewHandoff, cx: &mut Context<Self>) {
        if self.seen_handoffs.contains(&handoff.id) {
            return;
        }
        self.seen_handoffs.push(handoff.id.clone());
        let reduced = disabled_motion(cx);
        self.start_handoff_internal(&handoff, reduced);
        cx.emit(AgentCrewEvent::HandoffTriggered { handoff });
        cx.notify();
    }

    /// Trigger the next sample handoff across the current project tree.
    pub fn trigger_sample_handoff(&mut self, cx: &mut Context<Self>) {
        let Some(root) = self.current_root().cloned() else {
            return;
        };
        let ids = flatten_agent_ids(&root);
        if ids.len() < 2 {
            return;
        }
        let samples: &[(&str, &str, &str)] = match self.active_project_idx {
            0 => &[
                ("lead", "coder", "Guard empty carts before coupon calc"),
                ("coder", "linter", "Check strict null types on cart.ts"),
                ("coder", "test-runner", "Run checkout empty-session suite"),
                ("lead", "reviewer", "Verify regression diff & release note"),
                (
                    "reviewer",
                    "docs-writer",
                    "Document session fallback behavior",
                ),
                ("lead", "research", "Confirm session store TTL edge cases"),
            ],
            1 => &[
                ("p-lead", "p-billing", "Sync annual Stripe price IDs"),
                ("p-lead", "p-copy", "Polish enterprise FAQ comparison"),
                ("p-copy", "p-seo", "Validate canonical metadata tags"),
            ],
            _ => &[
                ("a-lead", "a-rotator", "Re-run legacy JWT verifier check"),
                ("a-lead", "a-audit", "Export token rotation audit trail"),
            ],
        };
        let pick = samples[self.demo_step % samples.len()];
        self.demo_step = self.demo_step.wrapping_add(1);
        let (from, to, text) = if ids.iter().any(|x| x.as_ref() == pick.0)
            && ids.iter().any(|x| x.as_ref() == pick.1)
        {
            (pick.0.to_string(), pick.1.to_string(), pick.2.to_string())
        } else {
            (
                ids[0].to_string(),
                ids[1 % ids.len()].to_string(),
                "Sync latest subtask context".to_string(),
            )
        };
        let hid = format!("hof-{}-{}", self.demo_step, self.seen_handoffs.len());
        self.push_handoff(CrewHandoff::new(hid, from, to, Some(text.into())), cx);
    }

    /// Apply a [`CrewSteerAction`] to `agent_id` and emit [`AgentCrewEvent::Steered`].
    pub fn steer_agent(
        &mut self,
        agent_id: impl Into<SharedString>,
        action: CrewSteerAction,
        message: Option<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let target_id = agent_id.into();
        let mut agent_name = target_id.clone();
        if let Some(proj) = self.projects.get_mut(self.active_project_idx)
            && let Some(node) = find_agent_mut(&mut proj.root, &target_id)
        {
            agent_name = node.name.clone();
            match action {
                CrewSteerAction::Pause => {
                    node.status = CrewStatus::Paused;
                    node.activity = tr(cx, "Paused by you", "已由你暂停").into();
                }
                CrewSteerAction::Resume => {
                    node.status = CrewStatus::Active;
                    node.activity = tr(cx, "Resuming assigned task", "正在恢复任务").into();
                }
                CrewSteerAction::Stop => {
                    node.status = CrewStatus::Idle;
                    node.activity = tr(cx, "Stopped", "已停止").into();
                }
                CrewSteerAction::Approve => {
                    node.status = CrewStatus::Active;
                    node.activity =
                        tr(cx, "Approved — applying changes", "已批准，正在执行").into();
                }
                CrewSteerAction::Decline => {
                    node.status = CrewStatus::Paused;
                    node.activity =
                        tr(cx, "Declined — awaiting guidance", "已拒绝，等待新指令").into();
                }
                CrewSteerAction::Retry => {
                    node.status = CrewStatus::Active;
                    node.context = (node.context * 0.65).clamp(0.12, 0.72);
                    node.activity = tr(cx, "Retrying clean run", "正在重新执行").into();
                }
                CrewSteerAction::Message => {
                    if let Some(msg) = &message {
                        node.last = Some(CrewLastHandoff::new(tr(cx, "You", "你"), msg.clone()));
                        if matches!(
                            node.status,
                            CrewStatus::Idle | CrewStatus::Paused | CrewStatus::Waiting
                        ) {
                            node.status = CrewStatus::Active;
                        }
                        node.activity = msg.clone();
                    }
                }
            }
        }
        let side = self
            .current_root()
            .and_then(|r| glow_side_for_target(r, &target_id))
            .unwrap_or(GlowSide::Top);
        if !disabled_motion(cx) {
            self.card_glows
                .insert(target_id.clone(), (Instant::now(), side));
        }
        let summary = match action {
            CrewSteerAction::Pause => format!("{} {agent_name}", tr(cx, "Paused", "已暂停")),
            CrewSteerAction::Resume => format!("{} {agent_name}", tr(cx, "Resumed", "已恢复")),
            CrewSteerAction::Stop => format!("{} {agent_name}", tr(cx, "Stopped", "已停止")),
            CrewSteerAction::Approve => format!("{} {agent_name}", tr(cx, "Approved", "已批准")),
            CrewSteerAction::Decline => format!("{} {agent_name}", tr(cx, "Declined", "已拒绝")),
            CrewSteerAction::Retry => format!("{} {agent_name}", tr(cx, "Retried", "已重试")),
            CrewSteerAction::Message => format!(
                "{} {agent_name}",
                tr(cx, "Sent instruction to", "已发送指令至")
            ),
        };
        self.sr_status = summary.clone().into();
        self.toast = Some((summary.into(), Instant::now()));
        cx.emit(AgentCrewEvent::Steered {
            agent_id: target_id,
            action,
            message,
        });
        cx.notify();
    }

    fn current_root(&self) -> Option<&CrewAgent> {
        self.projects.get(self.active_project_idx).map(|p| &p.root)
    }

    fn start_handoff_internal(&mut self, handoff: &CrewHandoff, immediate: bool) {
        let Some(proj) = self.projects.get_mut(self.active_project_idx) else {
            return;
        };
        let from_name = find_agent(&proj.root, &handoff.from)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| handoff.from.clone());
        let to_name = find_agent(&proj.root, &handoff.to)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| handoff.to.clone());
        let hops = build_hop_chain(&proj.root, &handoff.from, &handoff.to);
        if let Some(lbl) = &handoff.label {
            self.sr_status = format!("{from_name} handed to {to_name}: {lbl}").into();
        } else {
            self.sr_status = format!("{from_name} handed work to {to_name}").into();
        }
        if immediate || hops.is_empty() {
            if let Some(target) = find_agent_mut(&mut proj.root, &handoff.to) {
                if let Some(lbl) = &handoff.label {
                    target.last = Some(CrewLastHandoff::new(from_name, lbl.clone()));
                }
                if target.status == CrewStatus::Idle {
                    target.status = CrewStatus::Active;
                }
            }
            self.active_flight = None;
        } else {
            self.active_flight = Some(ActiveFlight {
                hops,
                to_id: handoff.to.clone(),
                from_name,
                label: handoff.label.clone(),
                started: Instant::now(),
                hop_secs: 0.58,
                applied: false,
            });
        }
    }

    fn advance_animations(&mut self, reduced: bool) -> bool {
        let mut needs_frame = false;
        let now = Instant::now();

        if let Some(flight) = &mut self.active_flight {
            let total_dur = flight.hop_secs * (flight.hops.len().max(1) as f32);
            let elapsed = now.duration_since(flight.started).as_secs_f32();
            if elapsed >= total_dur {
                if !flight.applied {
                    flight.applied = true;
                    let to_id = flight.to_id.clone();
                    let from_name = flight.from_name.clone();
                    let label = flight.label.clone();
                    let side = self
                        .current_root()
                        .and_then(|r| glow_side_for_target(r, &to_id))
                        .unwrap_or(GlowSide::Top);
                    if let Some(proj) = self.projects.get_mut(self.active_project_idx)
                        && let Some(target) = find_agent_mut(&mut proj.root, &to_id)
                        && let Some(lbl) = label
                    {
                        target.last = Some(CrewLastHandoff::new(from_name, lbl.clone()));
                        if target.status == CrewStatus::Idle {
                            target.status = CrewStatus::Active;
                            target.activity = lbl;
                        }
                    }
                    if !reduced {
                        self.card_glows.insert(to_id, (now, side));
                        needs_frame = true;
                    }
                }
                self.active_flight = None;
                self.next_demo_at = now;
            } else if !reduced {
                needs_frame = true;
            }
        } else if self.auto_handoffs
            && !reduced
            && self.open_agent_id.is_none()
            && now.duration_since(self.next_demo_at).as_secs_f32() >= 4.2
            && let Some(root) = self.current_root().cloned()
        {
            let ids = flatten_agent_ids(&root);
            if ids.len() >= 2 {
                let samples: &[(&str, &str, &str)] = match self.active_project_idx {
                    0 => &[
                        ("lead", "coder", "Guard empty carts"),
                        ("coder", "linter", "Check cart.ts types"),
                        ("lead", "research", "Verify session null check"),
                        ("research", "web-search", "Known empty-session issues?"),
                    ],
                    1 => &[
                        ("p-lead", "p-billing", "Verify Stripe annual coupons"),
                        ("p-copy", "p-seo", "Audit pricing meta descriptions"),
                    ],
                    _ => &[("a-lead", "a-audit", "Capture rotation diff summary")],
                };
                let pick = samples[self.demo_step % samples.len()];
                self.demo_step = self.demo_step.wrapping_add(1);
                if ids.iter().any(|x| x.as_ref() == pick.0)
                    && ids.iter().any(|x| x.as_ref() == pick.1)
                {
                    let h = CrewHandoff::new(
                        format!("auto-{}", self.demo_step),
                        pick.0,
                        pick.1,
                        Some(pick.2.into()),
                    );
                    self.start_handoff_internal(&h, false);
                    needs_frame = true;
                } else {
                    self.next_demo_at = now;
                }
            }
        }

        self.card_glows
            .retain(|_, (started, _)| now.duration_since(*started).as_secs_f32() < 0.95);
        if !self.card_glows.is_empty() && !reduced {
            needs_frame = true;
        }
        if self.working_count() > 0 && !reduced {
            needs_frame = true;
        }
        needs_frame
    }

    fn ensure_steer_editor(&mut self, agent_name: &str, w: &mut Window, cx: &mut Context<Self>) {
        let ph: SharedString = format!("{} {agent_name}…", tr(cx, "Message", "发送指令给")).into();
        if let Some(ed) = &self.steer_editor {
            ed.update(cx, |e, _| e.set_placeholder(ph));
            return;
        }
        let ed = cx.new(|cx| {
            let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
            e.set_placeholder(ph);
            e.font_size = 12.0;
            e
        });
        cx.subscribe_in(&ed, w, |this, _, ev: &EditEvent, w, cx| match ev {
            EditEvent::Submitted => {
                this.submit_steer_message(w, cx);
            }
            EditEvent::Escape => {
                this.open_agent_id = None;
                this.focus.focus(w, cx);
                cx.notify();
            }
            EditEvent::Changed => {
                cx.notify();
            }
            _ => {}
        })
        .detach();
        self.steer_editor = Some(ed);
    }

    fn submit_steer_message(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(ed) = self.steer_editor.clone() else {
            return;
        };
        let Some(target_id) = self.open_agent_id.clone() else {
            return;
        };
        let raw = ed.read(cx).value().trim().to_owned();
        if raw.is_empty() {
            return;
        }
        ed.update(cx, |e, cx| e.set_value("", w, cx));
        self.steer_agent(target_id, CrewSteerAction::Message, Some(raw.into()), cx);
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        if key == "escape" {
            if self.project_menu_open {
                self.project_menu_open = false;
                cx.notify();
                return;
            }
            if self.open_agent_id.is_some() {
                self.open_agent_id = None;
                self.focus.focus(w, cx);
                cx.notify();
                return;
            }
        }
        if self.open_agent_id.is_some() {
            return;
        }
        let Some(root) = self.current_root() else {
            return;
        };
        let ids = flatten_agent_ids(root);
        if ids.is_empty() {
            return;
        }
        match key {
            "right" | "down" => {
                self.focused_card_idx = (self.focused_card_idx + 1) % ids.len();
                cx.notify();
            }
            "left" | "up" => {
                self.focused_card_idx = if self.focused_card_idx == 0 {
                    ids.len() - 1
                } else {
                    self.focused_card_idx - 1
                };
                cx.notify();
            }
            "home" => {
                self.focused_card_idx = 0;
                cx.notify();
            }
            "end" => {
                self.focused_card_idx = ids.len() - 1;
                cx.notify();
            }
            "enter" | " " => {
                let id = ids[self.focused_card_idx.min(ids.len() - 1)].clone();
                self.open_agent_id = Some(id.clone());
                cx.emit(AgentCrewEvent::Selected { agent_id: id });
                cx.notify();
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_card(
        &self,
        agent: &CrewAgent,
        branch_color: Hsla,
        compact: bool,
        full_width: bool,
        sensor: Rc<Cell<Option<Bounds<Pixels>>>>,
        elapsed_ms: f32,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let aid = agent.id.clone();
        let is_open = self.open_agent_id.as_ref() == Some(&aid);
        let disp_name = if self.custom_data {
            agent.name.clone()
        } else {
            tr_agent_name(&agent.id, &agent.name, cx).into()
        };
        let disp_act = if self.custom_data {
            agent.activity.clone()
        } else {
            tr_activity(&agent.activity, cx).into()
        };
        let ratio = sanitize_ratio(agent.context);
        let pct_int = (ratio * 100.0).round() as u32;
        let used_k = ((ratio * (agent.context_limit as f32)) / 1000.0).round() as u32;
        let lim_k = (agent.context_limit / 1000).max(1);
        let ctx_label = format!("{used_k}k / {lim_k}k");

        let bar_color = if ratio >= 0.95 {
            p.red
        } else if ratio >= 0.80 {
            p.orange
        } else {
            branch_color
        };
        let ctx_num_color = if ratio >= 0.95 {
            p.red_ink
        } else if ratio >= 0.80 {
            p.amber_ink
        } else {
            p.muted
        };

        let (st_bg, st_fg) = match agent.status {
            CrewStatus::Active => (p.accent_soft, p.blue),
            CrewStatus::Waiting => (p.amber_soft, p.amber_ink),
            CrewStatus::Done => (p.green_soft, p.green_ink),
            CrewStatus::Failed => (p.red_soft, p.red_ink),
            CrewStatus::Paused | CrewStatus::Idle => (p.chip, p.text),
        };

        let spin_deg = ((elapsed_ms / 900.0).fract()) * 360.0;
        let status_pill = div()
            .h(px(18.))
            .px(px(6.))
            .flex_none()
            .rounded(px(4.))
            .bg(st_bg)
            .text_color(st_fg)
            .text_size(px(11.))
            .font_weight(FontWeight::MEDIUM)
            .flex()
            .items_center()
            .gap(px(4.))
            .when(agent.status == CrewStatus::Active, |d| {
                // Animated orbital dot/arc indicator
                let _ = spin_deg;
                d.child(pro_svg("0 0 24 24", SPIN_ARC_SVG, 11.5, st_fg))
            })
            .when(agent.status == CrewStatus::Waiting, |d| {
                d.child(div().size(px(5.5)).rounded_full().bg(p.amber))
            })
            .when(agent.status == CrewStatus::Done, |d| {
                d.child(pro_icon(ProIcon::Check, 11., st_fg))
            })
            .when(agent.status == CrewStatus::Failed, |d| {
                d.child(pro_icon(ProIcon::AlertTriangle, 11., st_fg))
            })
            .when(agent.status == CrewStatus::Paused, |d| {
                d.child(pro_icon(ProIcon::Pause, 10.5, st_fg))
            })
            .child(agent.status.label(cx));

        let act_color = match agent.status {
            CrewStatus::Waiting => p.amber_ink,
            CrewStatus::Failed => p.red_ink,
            _ => p.text,
        };
        let act_el = if agent.status == CrewStatus::Active {
            pro_shimmer_text(
                pro_eid(&self.id, format!("shim-{}", agent.id)),
                disp_act.clone(),
                p.text,
                p.strong,
                12.,
                FontWeight::NORMAL,
                true,
            )
        } else {
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .text_color(act_color)
                .truncate()
                .child(disp_act.clone())
                .into_any_element()
        };

        // Arrival directional gradient floor wash
        let glow_overlay = if let Some((started, side)) = self.card_glows.get(&agent.id) {
            let age = (started.elapsed().as_secs_f32() / 0.95).clamp(0., 1.);
            let alpha = (1.0 - pro_ease(age)) * (if p.dark { 0.30 } else { 0.22 });
            let angle = match side {
                GlowSide::Top => 180.,
                GlowSide::Left => 90.,
            };
            Some(
                div()
                    .absolute()
                    .inset_0()
                    .rounded(px(8.))
                    .bg(linear_gradient(
                        angle,
                        linear_color_stop(branch_color.opacity(alpha), 0.),
                        linear_color_stop(transparent_black(), 0.88),
                    )),
            )
        } else {
            None
        };

        let border_c = if is_open { p.line_strong } else { p.line };
        let hover_border = p.line_strong;
        let focus_c = p.focus;

        let mut card_inner = div()
            .w_full()
            .relative()
            .overflow_hidden()
            .rounded(px(8.))
            .bg(p.card)
            .border_1()
            .border_color(border_c)
            .p(px(if compact { 8.5 } else { 10. }))
            .flex()
            .flex_col()
            .gap(px(if compact { 5. } else { 6. }))
            .children(glow_overlay)
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .min_w_0()
                    .child(pro_agent_avatar(
                        &agent.id,
                        branch_color,
                        22.,
                        agent.status == CrewStatus::Active,
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
                                    .line_height(px(16.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .truncate()
                                    .child(disp_name.clone()),
                            )
                            .when(!compact && !agent.model.is_empty(), |d| {
                                d.child(
                                    div()
                                        .text_size(px(11.))
                                        .line_height(px(14.))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(agent.model.clone()),
                                )
                            }),
                    )
                    .child(status_pill),
            )
            .child(
                div()
                    .w_full()
                    .h(px(16.))
                    .min_w_0()
                    .overflow_hidden()
                    .child(act_el),
            )
            .child(
                div()
                    .w_full()
                    .h(px(14.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().flex_1().min_w_0().child(pro_spring_bar(
                        pro_eid(&self.id, format!("ctx-{}", agent.id)),
                        ratio,
                        3.0,
                        2.0,
                        p.chip,
                        bar_color,
                    )))
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(11.))
                            .line_height(px(14.))
                            .text_color(ctx_num_color)
                            .child(ctx_label),
                    ),
            );

        // Prepaint bounds sensor inside card_inner so connectors attach to the card box!
        let cell_clone = sensor;
        card_inner = card_inner.child(
            canvas(
                move |bounds, _, _| {
                    cell_clone.set(Some(bounds));
                },
                |_, _, _, _| (),
            )
            .absolute()
            .inset_0(),
        );

        let has_tray_foot = !agent.tools.is_empty() || agent.last.is_some();
        let mut tray_foot = div()
            .w_full()
            .h(px(21.))
            .px(px(7.))
            .flex()
            .items_center()
            .gap(px(6.))
            .min_w_0();

        if !agent.tools.is_empty() {
            let shown = if agent.tools.len() > 4 {
                3
            } else {
                agent.tools.len()
            };
            let mut tools_row = div().flex_none().flex().items_center();
            for (ti, tool) in agent.tools.iter().take(shown).enumerate() {
                tools_row = tools_row.child(
                    div()
                        .when(ti > 0, |d| d.ml(px(-4.)))
                        .child(pro_app_tile(tool, 16., cx)),
                );
            }
            if agent.tools.len() > shown {
                tools_row = tools_row.child(
                    div()
                        .ml(px(-3.))
                        .h(px(16.))
                        .px(px(4.))
                        .rounded(px(4.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .text_size(px(9.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(p.muted)
                        .flex()
                        .items_center()
                        .child(format!("+{}", agent.tools.len() - shown)),
                );
            }
            tray_foot = tray_foot.child(tools_row);
        }

        if let Some(last) = &agent.last {
            let from_txt = if self.custom_data {
                last.from.clone()
            } else {
                tr_agent_name(&last.from, &last.from, cx).into()
            };
            tray_foot = tray_foot.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .text_size(px(11.))
                    .line_height(px(14.))
                    .text_color(p.muted)
                    .child(pro_svg("0 0 24 24", CORNER_HANDOFF_SVG, 11., p.muted))
                    .child(
                        div()
                            .flex_none()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.text)
                            .child(from_txt),
                    )
                    .child(div().flex_1().min_w_0().truncate().child(last.text.clone())),
            );
        }

        let aria = format!(
            "{}: {}, {}, {} {}%",
            disp_name,
            agent.status.label(cx),
            disp_act,
            tr(cx, "context", "上下文"),
            pct_int
        );
        let click_id = agent.id.clone();

        let outer = activate(
            div()
                .id(pro_eid(&self.id, format!("node-{}", agent.id)))
                .role(Role::TreeItem)
                .aria_label(aria)
                .tab_index(0)
                .when(full_width, |d| d.w_full())
                .when(!full_width, |d| d.w(px(214.)))
                .rounded(px(10.))
                .bg(p.tray)
                .border_1()
                .border_color(if is_open { p.line_strong } else { p.line_soft })
                .p(px(2.5))
                .flex()
                .flex_col()
                .cursor_pointer()
                .hover(move |s| s.border_color(hover_border))
                .focus_visible(move |s| s.border_color(focus_c))
                .child(card_inner)
                .when(has_tray_foot, |d| d.child(tray_foot)),
            cx,
            move |this, _, cx| {
                let target = click_id.clone();
                this.project_menu_open = false;
                if this.open_agent_id.as_ref() == Some(&target) {
                    this.open_agent_id = None;
                } else {
                    this.open_agent_id = Some(target.clone());
                    cx.emit(AgentCrewEvent::Selected { agent_id: target });
                }
                cx.notify();
            },
        );
        outer.into_any_element()
    }

    fn render_steering_modal(
        &mut self,
        agent: &CrewAgent,
        branch_color: Hsla,
        elapsed_ms: f32,
        p: &ProPalette,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let disp_name = if self.custom_data {
            agent.name.clone()
        } else {
            tr_agent_name(&agent.id, &agent.name, cx).into()
        };
        self.ensure_steer_editor(&disp_name, w, cx);

        let aid = agent.id.clone();
        let live_secs = if agent.status == CrewStatus::Active {
            agent.elapsed_secs + (elapsed_ms / 1000.0) as u64
        } else {
            agent.elapsed_secs
        };
        let ratio = sanitize_ratio(agent.context);
        let used_k = ((ratio * (agent.context_limit as f32)) / 1000.0).round() as u32;
        let lim_k = (agent.context_limit / 1000).max(1);
        let cost_str = format_usd(agent.cost);
        let provider_key = infer_provider_key(&agent.model);

        let task_text = if agent.task.is_empty() {
            if self.custom_data {
                agent.activity.clone()
            } else {
                tr_activity(&agent.activity, cx).into()
            }
        } else {
            agent.task.clone()
        };

        // Steering action buttons based on agent status
        let mut actions_row = div().w_full().flex().items_center().gap(px(6.));
        match agent.status {
            CrewStatus::Active => {
                let a1 = aid.clone();
                let a2 = aid.clone();
                actions_row = actions_row
                    .child(div().flex_1().child(pro_btn(
                        pro_eid(&self.id, "steer-pause"),
                        tr(cx, "Pause", "暂停"),
                        Some(ProIcon::Pause),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.steer_agent(a1.clone(), CrewSteerAction::Pause, None, cx);
                        },
                    )))
                    .child(div().flex_1().child(pro_btn(
                        pro_eid(&self.id, "steer-stop"),
                        tr(cx, "Stop", "停止"),
                        None,
                        ProBtnStyle::Ghost,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.steer_agent(a2.clone(), CrewSteerAction::Stop, None, cx);
                        },
                    )));
            }
            CrewStatus::Paused | CrewStatus::Idle | CrewStatus::Done => {
                let a1 = aid.clone();
                let a2 = aid.clone();
                actions_row = actions_row
                    .child(div().flex_1().child(pro_btn(
                        pro_eid(&self.id, "steer-resume"),
                        tr(cx, "Resume", "恢复运行"),
                        Some(ProIcon::Play),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.steer_agent(a1.clone(), CrewSteerAction::Resume, None, cx);
                        },
                    )))
                    .child(div().flex_1().child(pro_btn(
                        pro_eid(&self.id, "steer-retry-idle"),
                        tr(cx, "Restart", "重新开始"),
                        Some(ProIcon::Refresh),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.steer_agent(a2.clone(), CrewSteerAction::Retry, None, cx);
                        },
                    )));
            }
            CrewStatus::Waiting => {
                let a1 = aid.clone();
                let a2 = aid.clone();
                actions_row = actions_row
                    .child(div().flex_1().child(pro_btn(
                        pro_eid(&self.id, "steer-approve"),
                        tr(cx, "Approve", "批准"),
                        Some(ProIcon::Check),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.steer_agent(a1.clone(), CrewSteerAction::Approve, None, cx);
                        },
                    )))
                    .child(div().flex_1().child(pro_btn(
                        pro_eid(&self.id, "steer-decline"),
                        tr(cx, "Decline", "拒绝"),
                        Some(ProIcon::Close),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.steer_agent(a2.clone(), CrewSteerAction::Decline, None, cx);
                        },
                    )));
            }
            CrewStatus::Failed => {
                let a1 = aid.clone();
                let a2 = aid.clone();
                actions_row = actions_row
                    .child(div().flex_1().child(pro_btn(
                        pro_eid(&self.id, "steer-retry"),
                        tr(cx, "Retry", "重试"),
                        Some(ProIcon::Refresh),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.steer_agent(a1.clone(), CrewSteerAction::Retry, None, cx);
                        },
                    )))
                    .child(div().flex_1().child(pro_btn(
                        pro_eid(&self.id, "steer-stop-fail"),
                        tr(cx, "Stop", "停止"),
                        None,
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.steer_agent(a2.clone(), CrewSteerAction::Stop, None, cx);
                        },
                    )));
            }
        }

        let hover_bg = p.chip_hover;
        let focus_c = p.focus;
        let close_btn = activate(
            div()
                .id(pro_eid(&self.id, "close-steer"))
                .role(Role::Button)
                .aria_label(tr(cx, "Close inspector", "关闭详情面板"))
                .tab_index(0)
                .size(px(24.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(move |s| s.bg(hover_bg))
                .focus_visible(move |s| s.border_1().border_color(focus_c))
                .child(pro_icon(ProIcon::Close, 13., p.muted)),
            cx,
            |this, w, cx| {
                this.open_agent_id = None;
                this.focus.focus(w, cx);
                cx.notify();
            },
        );

        let mut tools_wrap = div().w_full().flex().flex_wrap().gap(px(5.));
        for tool in &agent.tools {
            tools_wrap = tools_wrap.child(
                div()
                    .h(px(22.))
                    .pl(px(4.))
                    .pr(px(7.))
                    .rounded(px(6.))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(11.5))
                    .text_color(p.text)
                    .child(pro_app_tile(tool, 14., cx))
                    .child(tool.clone()),
            );
        }

        let editor_box = if let Some(ed) = &self.steer_editor {
            div()
                .w_full()
                .pt(px(2.))
                .flex()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h(px(28.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .bg(p.chip)
                        .border_1()
                        .border_color(p.line_strong)
                        .flex()
                        .items_center()
                        .text_size(px(12.))
                        .text_color(p.ink)
                        .child(ed.clone()),
                )
                .child(pro_btn(
                    pro_eid(&self.id, "steer-send"),
                    tr(cx, "Send", "发送"),
                    Some(ProIcon::ArrowUp),
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    |this, w, cx| {
                        this.submit_steer_message(w, cx);
                    },
                ))
        } else {
            div()
        };

        let dialog = div()
            .id(pro_eid(&self.id, "steer-dialog"))
            .role(Role::Dialog)
            .aria_label(disp_name.clone())
            .on_click(|_, _, cx| cx.stop_propagation())
            .w(px(340.))
            .max_w_full()
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .p(px(12.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(pro_agent_avatar(
                        &agent.id,
                        branch_color,
                        26.,
                        agent.status == CrewStatus::Active,
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
                                    .text_size(px(13.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .truncate()
                                    .child(disp_name),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .children(pro_app_artwork(provider_key, 12., cx))
                                    .child(agent.model.clone()),
                            ),
                    )
                    .child(close_btn),
            )
            .child(
                div()
                    .text_size(px(12.5))
                    .line_height(px(18.))
                    .text_color(p.text)
                    .child(task_text),
            )
            .child(
                div()
                    .w_full()
                    .py(px(8.))
                    .border_t_1()
                    .border_b_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(usage_cell(
                        tr(cx, "Time", "耗时"),
                        format_duration_secs(live_secs),
                        p,
                    ))
                    .when(!cost_str.is_empty(), |d| {
                        d.child(usage_cell(tr(cx, "Cost", "花费"), cost_str, p))
                    })
                    .child(usage_cell(
                        tr(cx, "Context", "上下文"),
                        format!("{used_k}k / {lim_k}k"),
                        p,
                    )),
            )
            .when(!agent.tools.is_empty(), |d| d.child(tools_wrap))
            .when_some(agent.last.as_ref(), |d, last| {
                d.child(
                    div()
                        .w_full()
                        .flex()
                        .items_start()
                        .gap(px(5.))
                        .text_size(px(11.5))
                        .line_height(px(16.))
                        .text_color(p.muted)
                        .child(pro_svg("0 0 24 24", CORNER_HANDOFF_SVG, 12., p.muted))
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.text)
                                .child(format!("{}:", last.from)),
                        )
                        .child(div().flex_1().min_w_0().child(last.text.clone())),
                )
            })
            .child(actions_row)
            .child(editor_box);

        let scrim = div()
            .id(pro_eid(&self.id, "steer-scrim"))
            .absolute()
            .inset_0()
            .on_click(cx.listener(|this, _, w, cx| {
                this.open_agent_id = None;
                this.focus.focus(w, cx);
                cx.notify();
            }))
            .child(pro_modal_appear(
                pro_eid(&self.id, "steer-modal"),
                aid,
                if p.dark { 0.48 } else { 0.22 },
                dialog,
            ));

        scrim.into_any_element()
    }
}

impl Render for AgentCrew {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        if self.advance_animations(reduced) {
            w.request_animation_frame();
        }
        let elapsed_ms = self.boot.elapsed().as_secs_f32() * 1000.0;

        let width_cell = self.measured_width.clone();
        let width_sensor = canvas(
            move |bounds, _, _| {
                let bw = f32::from(bounds.size.width);
                if bw > 20.0 {
                    width_cell.set(bw);
                }
            },
            |_, _, _, _| (),
        )
        .absolute()
        .left_0()
        .right_0()
        .top_0()
        .h(px(1.));

        if self.loading {
            let phase = (elapsed_ms / 1200.0).fract();
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(20.))
                .child(width_sensor)
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_between()
                        .child(pro_skeleton_bar(180., 20., 6., phase, cx))
                        .child(pro_skeleton_bar(120., 16., 5., phase, cx)),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_center()
                        .child(pro_skeleton_bar(214., 114., 10., phase, cx)),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_center()
                        .gap(px(16.))
                        .child(pro_skeleton_bar(214., 114., 10., phase, cx))
                        .child(pro_skeleton_bar(214., 114., 10., phase, cx))
                        .child(pro_skeleton_bar(214., 114., 10., phase, cx)),
                )
                .into_any_element();
        }

        if self.empty || self.projects.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(width_sensor)
                .child(pro_empty_state(
                    ProIcon::Users,
                    tr(cx, "No active agent crew", "暂无活跃的智能体协作组"),
                    tr(
                        cx,
                        "Assign a lead agent and sub-agents to monitor live handoffs and context windows.",
                        "配置主控智能体与子智能体后，即可在此实时查看上下文用量与交接脉动。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let container_w = self.measured_width.get();
        let active_proj = &self.projects[self.active_project_idx];
        let root_clone = active_proj.root.clone();
        let wide = container_w >= 720.0 && root_clone.children.len() <= 3;

        let total_agents = count_tree_all(&root_clone);
        let working_agents = count_tree_by(&root_clone, |a| a.status == CrewStatus::Active);
        let waiting_agents = count_tree_by(&root_clone, |a| a.status == CrewStatus::Waiting);

        let proj_title: SharedString = if self.custom_data {
            active_proj.name.clone()
        } else {
            tr_project_name(&active_proj.id, &active_proj.name, cx).into()
        };

        let menu_open = self.project_menu_open;
        let hover_fill = p.chip_hover;
        let open_fill = p.chip;
        let focus_c = p.focus;

        let header_left = if self.projects.len() > 1 {
            let btn = activate(
                div()
                    .id(pro_eid(&self.id, "proj-btn"))
                    .role(Role::Button)
                    .aria_label(proj_title.clone())
                    .tab_index(0)
                    .h(px(28.))
                    .pl(px(8.))
                    .pr(px(7.))
                    .rounded(px(6.))
                    .when(menu_open, |d| d.bg(open_fill))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .cursor_pointer()
                    .hover(move |s| s.bg(hover_fill))
                    .focus_visible(move |s| s.border_1().border_color(focus_c))
                    .child(pro_icon(ProIcon::Folder, 14., p.muted))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .truncate()
                            .child(proj_title.clone()),
                    )
                    .child(pro_icon(
                        if menu_open {
                            ProIcon::ChevronUp
                        } else {
                            ProIcon::ChevronDown
                        },
                        12.,
                        p.muted,
                    )),
                cx,
                |this, _, cx| {
                    this.project_menu_open = !this.project_menu_open;
                    cx.notify();
                },
            );
            btn.into_any_element()
        } else {
            div()
                .text_size(px(13.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.strong)
                .truncate()
                .child(proj_title.clone())
                .into_any_element()
        };

        let header = div()
            .w_full()
            .mb(px(16.))
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(header_left)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(10.))
                    .text_size(px(12.))
                    .child(pro_btn(
                        pro_eid(&self.id, "handoff-btn"),
                        tr(cx, "Send handoff", "模拟交接"),
                        Some(ProIcon::Bolt),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, _, cx| {
                            this.trigger_sample_handoff(cx);
                        },
                    ))
                    .when(working_agents > 0, |d| {
                        d.child(
                            div()
                                .text_color(p.blue)
                                .font_weight(FontWeight::MEDIUM)
                                .child(format!("{working_agents} {}", tr(cx, "working", "工作中"))),
                        )
                    })
                    .when(waiting_agents > 0, |d| {
                        d.child(
                            div()
                                .text_color(p.amber_ink)
                                .font_weight(FontWeight::MEDIUM)
                                .child(format!(
                                    "{waiting_agents} {}",
                                    tr(cx, "needs you", "待确认")
                                )),
                        )
                    })
                    .child(
                        div()
                            .text_color(p.muted)
                            .child(format!("{total_agents} {}", tr(cx, "agents", "个智能体"))),
                    ),
            );

        // Build bounds sensors for Lead + every sub-agent so our connector canvas paints exact links
        let branch_palette = [p.purple, p.cyan, p.green, p.orange, p.pink, p.teal];
        let mut sensors: HashMap<SharedString, Rc<Cell<Option<Bounds<Pixels>>>>> = HashMap::new();
        let mut color_map: HashMap<SharedString, Hsla> = HashMap::new();
        let mut edges: Vec<(SharedString, SharedString, Hsla, bool, bool)> = Vec::new();

        let lead_sensor = Rc::new(Cell::new(None));
        sensors.insert(root_clone.id.clone(), lead_sensor.clone());
        color_map.insert(root_clone.id.clone(), p.blue);

        for (bi, l1) in root_clone.children.iter().enumerate() {
            let bcol = branch_palette[bi % branch_palette.len()];
            sensors.insert(l1.id.clone(), Rc::new(Cell::new(None)));
            color_map.insert(l1.id.clone(), bcol);
            let rest = l1.status != CrewStatus::Active && l1.status != CrewStatus::Waiting;
            edges.push((root_clone.id.clone(), l1.id.clone(), p.blue, wide, rest));
            for l2 in &l1.children {
                sensors.insert(l2.id.clone(), Rc::new(Cell::new(None)));
                color_map.insert(l2.id.clone(), bcol);
                let l2_rest = l2.status != CrewStatus::Active && l2.status != CrewStatus::Waiting;
                edges.push((l1.id.clone(), l2.id.clone(), bcol, false, l2_rest));
            }
        }

        // Compute active hop progress if a handoff flight is traveling
        let active_hop_progress: Option<(SharedString, SharedString, f32)> =
            self.active_flight.as_ref().and_then(|fl| {
                if reduced || fl.hops.is_empty() {
                    return None;
                }
                let elapsed = fl.started.elapsed().as_secs_f32();
                let hop_i = (elapsed / fl.hop_secs).floor() as usize;
                if hop_i >= fl.hops.len() {
                    return None;
                }
                let local_t =
                    ((elapsed - (hop_i as f32) * fl.hop_secs) / fl.hop_secs).clamp(0., 1.);
                let (ref f, ref t) = fl.hops[hop_i];
                Some((f.clone(), t.clone(), pro_ease(local_t)))
            });

        let lead_card = self.render_card(
            &root_clone,
            p.blue,
            false,
            !wide,
            lead_sensor,
            elapsed_ms,
            &p,
            cx,
        );

        let mut l1_elements: Vec<AnyElement> = Vec::with_capacity(root_clone.children.len());
        for (bi, l1) in root_clone.children.iter().enumerate() {
            let bcol = branch_palette[bi % branch_palette.len()];
            let s1 = sensors
                .get(&l1.id)
                .cloned()
                .unwrap_or_else(|| Rc::new(Cell::new(None)));
            let c1 = self.render_card(l1, bcol, false, !wide, s1, elapsed_ms, &p, cx);

            let mut below = div().pl(px(28.)).flex().flex_col().gap(px(10.));
            for l2 in &l1.children {
                let s2 = sensors
                    .get(&l2.id)
                    .cloned()
                    .unwrap_or_else(|| Rc::new(Cell::new(None)));
                below =
                    below.child(self.render_card(l2, bcol, true, !wide, s2, elapsed_ms, &p, cx));
            }

            let col_w = if l1.children.is_empty() { 214. } else { 242. };
            let hang = div()
                .when(wide, |d| d.w(px(col_w)).flex_none())
                .when(!wide, |d| d.w_full())
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(c1)
                .when(!l1.children.is_empty(), |d| d.child(below));
            l1_elements.push(hang.into_any_element());
        }

        // Background connector + junction dots canvas
        let sensors_paint = sensors.clone();
        let edges_paint = edges;
        let hop_paint = active_hop_progress;
        let card_bg = p.card;

        let links_canvas = canvas(
            |_, _, _| (),
            move |_, _, window, _| {
                let mut parent_anchors: Vec<(Point<Pixels>, Hsla)> = Vec::new();
                for (from_id, to_id, col, s_bend_vertical, rest) in &edges_paint {
                    let Some(b0) = sensors_paint.get(from_id).and_then(|c| c.get()) else {
                        continue;
                    };
                    let Some(b1) = sensors_paint.get(to_id).and_then(|c| c.get()) else {
                        continue;
                    };
                    let prog = hop_paint.as_ref().and_then(|(hf, ht, pr)| {
                        if hf == from_id && ht == to_id {
                            Some(*pr)
                        } else {
                            None
                        }
                    });
                    let base_alpha = if prog.is_some() {
                        0.72
                    } else if *rest {
                        0.32
                    } else {
                        0.56
                    };
                    let base_c = col.opacity(base_alpha);

                    if *s_bend_vertical {
                        let p0 = point((b0.left() + b0.right()) * 0.5, b0.bottom() + px(2.5));
                        let p1 = point((b1.left() + b1.right()) * 0.5, b1.top() - px(2.5));
                        pro_connector_pulse(p0, p1, 8.0, true, base_c, *col, prog, 1.5, window);
                        parent_anchors.push((p0, *col));
                    } else {
                        // Indented L-elbow from parent bottom-left down to child center-left
                        let x0 = f32::from(b0.left()) + 14.0;
                        let y0 = f32::from(b0.bottom()) + 2.5;
                        let x1 = f32::from(b1.left()) - 2.5;
                        let y1 = (f32::from(b1.top()) + f32::from(b1.bottom())) * 0.5;
                        paint_l_elbow(x0, y0, x1, y1, 5.0, base_c, *col, prog, 1.5, window);
                        parent_anchors.push((point(px(x0), px(y0)), *col));
                    }
                }

                // Ringed junction dots at parent outgoing anchors
                for (pt, col) in parent_anchors {
                    paint_ring_dot(pt, 3.0, card_bg, col.opacity(0.82), 1.5, window);
                }
            },
        )
        .absolute()
        .inset_0();

        let tree_body = pro_view_transition(
            pro_eid(&self.id, "proj-vtx"),
            active_proj.id.clone(),
            div()
                .id(pro_eid(&self.id, "crew-tree"))
                .role(Role::Tree)
                .aria_label(proj_title.clone())
                .w_full()
                .relative()
                .flex()
                .flex_col()
                .when(wide, |d| d.items_center())
                .child(links_canvas)
                .child(
                    div()
                        .when(wide, |d| d.mr(px(28.)))
                        .when(!wide, |d| d.w_full())
                        .child(lead_card),
                )
                .when(!l1_elements.is_empty(), |d| {
                    d.child(
                        div()
                            .w_full()
                            .when(wide, |lv| {
                                lv.mt(px(38.))
                                    .flex()
                                    .items_start()
                                    .justify_center()
                                    .gap(px(10.))
                            })
                            .when(!wide, |lv| {
                                lv.mt(px(12.)).pl(px(28.)).flex().flex_col().gap(px(12.))
                            })
                            .children(l1_elements),
                    )
                }),
        );

        // Optional project dropdown menu
        let proj_menu =
            if self.project_menu_open && self.projects.len() > 1 {
                let mut list = div()
                    .id(pro_eid(&self.id, "proj-menu"))
                    .role(Role::ListBox)
                    .aria_label(tr(cx, "Select project crew", "切换项目协作组"))
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .absolute()
                    .top(px(40.))
                    .left(px(12.))
                    .w(px(300.))
                    .max_w(px((container_w - 24.).max(220.)))
                    .p(px(4.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .gap(px(2.));

                for (pi, proj) in self.projects.iter().enumerate() {
                    let selected = pi == self.active_project_idx;
                    let pid = proj.id.clone();
                    let pname: SharedString = if self.custom_data {
                        proj.name.clone()
                    } else {
                        tr_project_name(&proj.id, &proj.name, cx).into()
                    };
                    let w_cnt = count_tree_by(&proj.root, |a| a.status == CrewStatus::Active);
                    let u_cnt = count_tree_by(&proj.root, |a| a.status == CrewStatus::Waiting);
                    let row_hover = p.hover;
                    let row_sel = p.select;

                    list =
                        list.child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("proj-opt-{pi}")))
                                .role(Role::ListBoxOption)
                                .aria_label(pname.clone())
                                .h(px(32.))
                                .px(px(8.))
                                .rounded(px(7.))
                                .when(selected, |d| d.bg(row_sel))
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .cursor_pointer()
                                .hover(move |s| s.bg(row_hover))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_size(px(12.5))
                                        .font_weight(if selected {
                                            FontWeight::MEDIUM
                                        } else {
                                            FontWeight::NORMAL
                                        })
                                        .text_color(p.strong)
                                        .truncate()
                                        .child(pname),
                                )
                                .when(u_cnt > 0, |d| {
                                    d.child(div().text_size(px(11.)).text_color(p.amber_ink).child(
                                        format!("{u_cnt} {}", tr(cx, "needs you", "待确认")),
                                    ))
                                })
                                .when(u_cnt == 0 && w_cnt > 0, |d| {
                                    d.child(
                                        div().text_size(px(11.)).text_color(p.blue).child(format!(
                                            "{w_cnt} {}",
                                            tr(cx, "working", "工作中")
                                        )),
                                    )
                                })
                                .when(selected, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink))),
                            cx,
                            move |this, _, cx| {
                                this.select_project(pid.clone(), cx);
                            },
                        ));
                }
                Some(list)
            } else {
                None
            };

        let open_modal = self.open_agent_id.clone().and_then(|oid| {
            let found = find_agent(&root_clone, &oid)?.clone();
            let col = color_map.get(&oid).copied().unwrap_or(p.blue);
            Some(self.render_steering_modal(&found, col, elapsed_ms, &p, w, cx))
        });

        let sr_text = self.sr_status.clone();

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .role(Role::Region)
            .aria_label(proj_title)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                this.handle_key_down(ev, w, cx);
            }))
            .on_click(cx.listener(|this, _, _, cx| {
                if this.project_menu_open {
                    this.project_menu_open = false;
                    cx.notify();
                }
            }))
            .w_full()
            .p(px(12.))
            .relative()
            .flex()
            .flex_col()
            .child(width_sensor)
            .child(header)
            .child(tree_body)
            .children(proj_menu)
            .children(open_modal)
            .child(pro_toast_banner(self.toast.clone()))
            .child(
                div()
                    .id(pro_eid(&self.id, "sr-live"))
                    .role(Role::Status)
                    .aria_label(sr_text)
                    .absolute()
                    .size_0(),
            )
            .into_any_element()
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_l_elbow(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    radius: f32,
    base_color: Hsla,
    pulse_color: Hsla,
    progress: Option<f32>,
    stroke_w: f32,
    window: &mut Window,
) {
    let dy = y1 - y0;
    let dx = x1 - x0;
    let r = radius.min(dy.abs() * 0.5).min(dx.abs() * 0.5).max(0.);
    let mut pts = Vec::with_capacity(22);
    if r < 1.0 {
        pts.push(point(px(x0), px(y0)));
        pts.push(point(px(x0), px(y1)));
        pts.push(point(px(x1), px(y1)));
    } else {
        let sy = dy.signum();
        let sx = dx.signum();
        let b = (x0, y1 - sy * r);
        let c = (x0, y1);
        let d = (x0 + sx * r, y1);
        for k in 0..=8 {
            let u = k as f32 / 8.0;
            pts.push(point(px(x0), px(y0 + (b.1 - y0) * u)));
        }
        for k in 1..=6 {
            let u = k as f32 / 6.0;
            let inv = 1.0 - u;
            pts.push(point(
                px(inv * inv * b.0 + 2.0 * inv * u * c.0 + u * u * d.0),
                px(inv * inv * b.1 + 2.0 * inv * u * c.1 + u * u * d.1),
            ));
        }
        for k in 1..=6 {
            let u = k as f32 / 6.0;
            pts.push(point(px(d.0 + (x1 - d.0) * u), px(y1)));
        }
    }

    let mut base = PathBuilder::stroke(px(stroke_w));
    for (i, &pt) in pts.iter().enumerate() {
        if i == 0 {
            base.move_to(pt);
        } else {
            base.line_to(pt);
        }
    }
    if let Ok(path) = base.build() {
        window.paint_path(path, base_color);
    }

    if let Some(prog) = progress {
        let prog = prog.clamp(0., 1.);
        let total = (pts.len().saturating_sub(1)) as f32;
        if total > 0.5 {
            let head = prog * total;
            let span = (total * 0.30).max(2.5);
            for i in 0..(pts.len() - 1) {
                let mid = i as f32 + 0.5;
                let dist = (mid - head).abs();
                if dist <= span {
                    let falloff = (1.0 - dist / span).clamp(0., 1.);
                    let alpha = falloff * falloff;
                    let mut seg = PathBuilder::stroke(px(stroke_w + 0.7 * alpha));
                    seg.move_to(pts[i]);
                    seg.line_to(pts[i + 1]);
                    if let Ok(sp) = seg.build() {
                        window.paint_path(sp, pulse_color.opacity(alpha));
                    }
                }
            }
        }
    }
}

fn paint_ring_dot(
    center: Point<Pixels>,
    r: f32,
    fill_color: Hsla,
    stroke_color: Hsla,
    stroke_w: f32,
    window: &mut Window,
) {
    let outer = r + stroke_w * 0.5;
    window.paint_quad(
        fill(
            Bounds::new(
                point(center.x - px(outer), center.y - px(outer)),
                size(px(outer * 2.0), px(outer * 2.0)),
            ),
            stroke_color,
        )
        .corner_radii(px(outer)),
    );
    let inner = (outer - stroke_w).max(1.0);
    window.paint_quad(
        fill(
            Bounds::new(
                point(center.x - px(inner), center.y - px(inner)),
                size(px(inner * 2.0), px(inner * 2.0)),
            ),
            fill_color,
        )
        .corner_radii(px(inner)),
    );
}

fn usage_cell(label: &str, value: String, p: &ProPalette) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(
            div()
                .text_size(px(11.))
                .line_height(px(14.))
                .text_color(p.muted)
                .child(label.to_string()),
        )
        .child(
            div()
                .text_size(px(12.5))
                .line_height(px(16.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink)
                .truncate()
                .child(value),
        )
}

fn sanitize_ratio(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn format_usd(amount: f64) -> String {
    if !amount.is_finite() || amount < 0.0 {
        return String::new();
    }
    if amount > 0.0 && amount < 0.01 {
        "<$0.01".to_string()
    } else {
        format!("${amount:.2}")
    }
}

fn format_duration_secs(secs: u64) -> String {
    if secs >= 86_400 {
        let days = secs / 86_400;
        let hrs = (secs % 86_400) / 3_600;
        format!("{days}d {hrs}h")
    } else if secs >= 3_600 {
        let hrs = secs / 3_600;
        let mins = (secs % 3_600) / 60;
        format!("{hrs}h {mins:02}m")
    } else if secs >= 60 {
        let mins = secs / 60;
        let rem = secs % 60;
        format!("{mins}m {rem:02}s")
    } else {
        format!("{secs}s")
    }
}

fn infer_provider_key(model: &str) -> &'static str {
    let lower = model.to_ascii_lowercase();
    if lower.contains("claude")
        || lower.contains("opus")
        || lower.contains("sonnet")
        || lower.contains("haiku")
    {
        "claude"
    } else if lower.contains("gemini") {
        "gemini"
    } else {
        "openai"
    }
}

fn count_tree_all(node: &CrewAgent) -> usize {
    1 + node.children.iter().map(count_tree_all).sum::<usize>()
}

fn count_tree_by(node: &CrewAgent, pred: impl Fn(&CrewAgent) -> bool + Copy) -> usize {
    usize::from(pred(node))
        + node
            .children
            .iter()
            .map(|c| count_tree_by(c, pred))
            .sum::<usize>()
}

fn flatten_agent_ids(node: &CrewAgent) -> Vec<SharedString> {
    let mut out = vec![node.id.clone()];
    for ch in &node.children {
        out.extend(flatten_agent_ids(ch));
    }
    out
}

fn find_agent<'a>(node: &'a CrewAgent, id: &SharedString) -> Option<&'a CrewAgent> {
    if &node.id == id {
        return Some(node);
    }
    for ch in &node.children {
        if let Some(found) = find_agent(ch, id) {
            return Some(found);
        }
    }
    None
}

fn find_agent_mut<'a>(node: &'a mut CrewAgent, id: &SharedString) -> Option<&'a mut CrewAgent> {
    if &node.id == id {
        return Some(node);
    }
    for ch in &mut node.children {
        if let Some(found) = find_agent_mut(ch, id) {
            return Some(found);
        }
    }
    None
}

fn path_from_root(node: &CrewAgent, target: &SharedString, acc: &mut Vec<SharedString>) -> bool {
    acc.push(node.id.clone());
    if &node.id == target {
        return true;
    }
    for ch in &node.children {
        if path_from_root(ch, target, acc) {
            return true;
        }
    }
    acc.pop();
    false
}

fn build_hop_chain(
    root: &CrewAgent,
    from: &SharedString,
    to: &SharedString,
) -> Vec<(SharedString, SharedString)> {
    let mut p_from = Vec::new();
    let mut p_to = Vec::new();
    if !path_from_root(root, from, &mut p_from) || !path_from_root(root, to, &mut p_to) {
        return Vec::new();
    }
    let mut common = 0usize;
    while common < p_from.len() && common < p_to.len() && p_from[common] == p_to[common] {
        common += 1;
    }
    let lca = common.saturating_sub(1);
    let mut chain: Vec<SharedString> = Vec::new();
    for idx in (lca..p_from.len()).rev() {
        chain.push(p_from[idx].clone());
    }
    for item in p_to.iter().skip(lca + 1) {
        chain.push(item.clone());
    }
    let mut hops = Vec::new();
    for w in chain.windows(2) {
        hops.push((w[0].clone(), w[1].clone()));
    }
    hops
}

fn glow_side_for_target(root: &CrewAgent, target: &SharedString) -> Option<GlowSide> {
    if &root.id == target {
        return Some(GlowSide::Top);
    }
    for l1 in &root.children {
        if &l1.id == target {
            return Some(GlowSide::Top);
        }
        if l1.children.iter().any(|c| &c.id == target) {
            return Some(GlowSide::Left);
        }
    }
    Some(GlowSide::Top)
}

fn tr_project_name<'a>(id: &str, fallback: &'a str, cx: &App) -> &'a str {
    match id {
        "checkout-crash" => tr(cx, "Fix the checkout crash", "修复结账崩溃故障"),
        "pricing-rollout" => tr(cx, "Q4 pricing rollout", "Q4 定价页上线"),
        "auth-migration" => tr(cx, "Auth token migration", "鉴权令牌迁移"),
        _ => fallback,
    }
}

fn tr_agent_name<'a>(id: &str, fallback: &'a str, cx: &App) -> &'a str {
    match id {
        "lead" | "p-lead" | "a-lead" | "Lead" => tr(cx, "Lead", "主控智能体"),
        "research" | "Researcher" => tr(cx, "Researcher", "调研员"),
        "web-search" | "Web search" => tr(cx, "Web search", "网页检索"),
        "coder" | "Coder" => tr(cx, "Coder", "编码员"),
        "linter" | "Linter" => tr(cx, "Linter", "静态检查"),
        "test-runner" | "Test runner" => tr(cx, "Test runner", "测试执行"),
        "reviewer" | "Reviewer" => tr(cx, "Reviewer", "代码审查"),
        "docs-writer" | "Docs writer" => tr(cx, "Docs writer", "文档撰写"),
        "p-billing" => tr(cx, "Billing sync", "计费同步"),
        "p-copy" => tr(cx, "Copywriter", "文案撰写"),
        "p-seo" => tr(cx, "SEO auditor", "SEO 审计"),
        "p-qa" => tr(cx, "Stage verifier", "预发验证"),
        "a-rotator" => tr(cx, "Key rotator", "密钥轮换"),
        "a-audit" => tr(cx, "Audit logger", "审计记录"),
        _ => fallback,
    }
}

fn tr_activity<'a>(en: &'a str, cx: &App) -> &'a str {
    match en {
        "Delegating" => tr(cx, "Delegating", "正在分发任务"),
        "Tracing cart.ts" => tr(cx, "Tracing cart.ts", "正在追踪 cart.ts"),
        "Searching docs" => tr(cx, "Searching docs", "正在检索文档"),
        "Waiting for work" => tr(cx, "Waiting for work", "等待分配任务"),
        "Idle" => tr(cx, "Idle", "空闲待命"),
        "Guard empty carts" => tr(cx, "Guard empty carts", "拦截空购物车边界"),
        "Approve Stripe live publish?" => tr(
            cx,
            "Approve Stripe live publish?",
            "确认发布至 Stripe 生产环境？",
        ),
        "Syncing annual tiers" => tr(cx, "Syncing annual tiers", "正在同步年付档位"),
        "Drafted FAQ table" => tr(cx, "Drafted FAQ table", "已完成常见问答表格"),
        "Checking canonical tags" => tr(cx, "Checking canonical tags", "正在校验规范链接标签"),
        "Legacy JWT signature mismatch" => {
            tr(cx, "Legacy JWT signature mismatch", "旧版 JWT 签名校验失败")
        }
        _ => en,
    }
}

fn default_crew_projects() -> Vec<CrewProject> {
    let checkout_root = CrewAgent::new("lead", "Lead", CrewStatus::Active, "Delegating")
        .with_model("Claude Opus")
        .with_context(0.22, 200_000)
        .with_tools(["Linear", "GitHub"])
        .with_task("Investigate the empty-session checkout TypeError and coordinate the patch, tests, and release notes.")
        .with_usage(0.48, 312)
        .with_children(vec![
            CrewAgent::new("research", "Researcher", CrewStatus::Active, "Tracing cart.ts")
                .with_model("Claude Sonnet")
                .with_context(0.31, 200_000)
                .with_tools(["GitHub", "Notion"])
                .with_last("Lead", "Trace null session in cart.ts")
                .with_task("Locate where guest checkout drops session.cart before coupon calculation.")
                .with_usage(0.24, 245)
                .with_children(vec![
                    CrewAgent::new("web-search", "Web search", CrewStatus::Active, "Searching docs")
                        .with_model("Claude Haiku")
                        .with_context(0.22, 200_000)
                        .with_tools(["Slack"])
                        .with_last("Researcher", "Known empty-session issues?")
                        .with_usage(0.06, 98),
                ]),
            CrewAgent::new("coder", "Coder", CrewStatus::Idle, "Waiting for work")
                .with_model("Claude Sonnet")
                .with_context(0.06, 200_000)
                .with_tools(["GitHub", "Linear"])
                .with_task("Implement early empty-cart guard in cart.ts and preserve guest promo state.")
                .with_usage(0.19, 180)
                .with_children(vec![
                    CrewAgent::new("linter", "Linter", CrewStatus::Idle, "Idle")
                        .with_model("GPT-5 Mini")
                        .with_context(0.03, 200_000)
                        .with_tools(["GitHub"])
                        .with_usage(0.02, 42),
                    CrewAgent::new("test-runner", "Test runner", CrewStatus::Idle, "Idle")
                        .with_model("GPT-5 Mini")
                        .with_context(0.05, 200_000)
                        .with_tools(["GitHub"])
                        .with_usage(0.04, 65),
                ]),
            CrewAgent::new("reviewer", "Reviewer", CrewStatus::Idle, "Waiting for work")
                .with_model("Claude Opus")
                .with_context(0.03, 200_000)
                .with_tools(["GitHub"])
                .with_task("Audit diff safety, edge cases, and backwards compatibility before merge.")
                .with_usage(0.11, 90)
                .with_children(vec![
                    CrewAgent::new("docs-writer", "Docs writer", CrewStatus::Idle, "Idle")
                        .with_model("Claude Haiku")
                        .with_context(0.05, 200_000)
                        .with_tools(["Notion"])
                        .with_usage(0.03, 54),
                ]),
        ]);

    let pricing_root = CrewAgent::new("p-lead", "Lead", CrewStatus::Active, "Syncing annual tiers")
        .with_model("Claude Opus")
        .with_context(0.46, 200_000)
        .with_tools(["Stripe", "Figma", "Slack"])
        .with_task("Coordinate Q4 self-serve & enterprise pricing rollout across Stripe and marketing pages.")
        .with_usage(0.64, 520)
        .with_children(vec![
            CrewAgent::new(
                "p-billing",
                "Billing sync",
                CrewStatus::Waiting,
                "Approve Stripe live publish?",
            )
            .with_model("Claude Sonnet")
            .with_context(0.84, 200_000)
            .with_tools(["Stripe", "GitHub"])
            .with_last("Lead", "Stage annual coupon IDs")
            .with_task("Create live annual Stripe prices ($24/mo billed yearly) after operator approval.")
            .with_usage(0.34, 410),
            CrewAgent::new("p-copy", "Copywriter", CrewStatus::Done, "Drafted FAQ table")
                .with_model("GPT-5")
                .with_context(0.58, 128_000)
                .with_tools(["Notion", "Figma"])
                .with_usage(0.22, 290)
                .with_children(vec![
                    CrewAgent::new("p-seo", "SEO auditor", CrewStatus::Active, "Checking canonical tags")
                        .with_model("Gemini 2.5 Pro")
                        .with_context(0.32, 200_000)
                        .with_tools(["Google Drive"])
                        .with_usage(0.08, 115),
                ]),
            CrewAgent::new("p-qa", "Stage verifier", CrewStatus::Idle, "Waiting for work")
                .with_model("Claude Haiku")
                .with_context(0.14, 200_000)
                .with_tools(["GitHub", "Slack"])
                .with_usage(0.05, 70),
        ]);

    let auth_root = CrewAgent::new("a-lead", "Lead", CrewStatus::Active, "Delegating")
        .with_model("Claude Opus")
        .with_context(0.62, 200_000)
        .with_tools(["GitHub", "Slack"])
        .with_task("Rotate signing keys for session tokens with zero downtime across edge workers.")
        .with_usage(0.71, 680)
        .with_children(vec![
            CrewAgent::new(
                "a-rotator",
                "Key rotator",
                CrewStatus::Failed,
                "Legacy JWT signature mismatch",
            )
            .with_model("Claude Sonnet")
            .with_context(0.96, 200_000)
            .with_tools(["GitHub", "Linear"])
            .with_last("Lead", "Verify dual-key window")
            .with_task("Validate dual JWKS verification against v1 mobile refresh tokens.")
            .with_usage(0.41, 495),
            CrewAgent::new(
                "a-audit",
                "Audit logger",
                CrewStatus::Active,
                "Tracing cart.ts",
            )
            .with_model("Gemini 2.5 Pro")
            .with_context(0.29, 200_000)
            .with_tools(["Notion", "Slack"])
            .with_usage(0.16, 210),
        ]);

    vec![
        CrewProject::new("checkout-crash", "Fix the checkout crash", checkout_root),
        CrewProject::new("pricing-rollout", "Q4 pricing rollout", pricing_root),
        CrewProject::new("auth-migration", "Auth token migration", auth_root),
    ]
}

#[cfg(test)]
mod tests {
    use gpui::AppContext;

    use super::{
        AgentCrew, CrewAgent, CrewHandoff, CrewStatus, CrewSteerAction, build_hop_chain,
        default_crew_projects, format_duration_secs, format_usd, sanitize_ratio,
    };
    use gpui::TestAppContext;

    #[test]
    fn default_checkout_project_matches_eight_agent_topology() {
        let projects = default_crew_projects();
        assert_eq!(projects.len(), 3);
        assert_eq!(projects[0].id.as_ref(), "checkout-crash");
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let crew = app.new(|cx| AgentCrew::new("crew-test", cx));
        crew.read_with(&app, |c, _| {
            assert_eq!(c.agent_count(), 8);
            assert_eq!(c.working_count(), 3);
        });
    }

    #[test]
    fn multi_hop_handoff_and_steering_update_agent_state_cleanly() {
        let projects = default_crew_projects();
        let hops = build_hop_chain(&projects[0].root, &"research".into(), &"linter".into());
        assert_eq!(hops.len(), 3);

        let mut app = TestAppContext::single();
        app.update(crate::init);
        let crew = app.new(|cx| AgentCrew::new("crew-steer", cx));
        crew.update(&mut app, |c, cx| {
            c.steer_agent("lead", CrewSteerAction::Pause, None, cx);
            assert_eq!(c.working_count(), 2);
            c.push_handoff(
                CrewHandoff::new("h1", "lead", "coder", Some("Ship patch".into())),
                cx,
            );
            // Duplicate handoff id must be ignored idempotently
            c.push_handoff(
                CrewHandoff::new("h1", "lead", "coder", Some("Duplicate".into())),
                cx,
            );
            c.select_project("pricing-rollout", cx);
            assert_eq!(
                c.active_project_id().map(|s| s.as_ref()),
                Some("pricing-rollout")
            );
        });
    }

    #[test]
    fn defensive_ratio_cost_and_duration_formatting_hold_invariants() {
        assert_eq!(sanitize_ratio(f32::NAN), 0.0);
        assert_eq!(sanitize_ratio(-0.4), 0.0);
        assert_eq!(sanitize_ratio(1.8), 1.0);
        assert_eq!(format_usd(f64::NAN), "");
        assert_eq!(format_usd(0.004), "<$0.01");
        assert_eq!(format_usd(0.52), "$0.52");
        assert_eq!(format_duration_secs(259_200), "3d 0h");
        let custom =
            CrewAgent::new("solo", "Solo", CrewStatus::Done, "Finished").with_context(1.5, 100);
        assert_eq!(custom.context, 1.0);
        assert_eq!(custom.context_limit, 1_000);
    }
}
