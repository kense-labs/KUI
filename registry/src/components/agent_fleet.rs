//! Side-by-side fleet of lead agents whose sub-agent trees unfold to the right on selection.
//!
//! Summarizes multiple parallel orchestrators with rolled-up branch costs, progress pie rings,
//! and status badges; clicking a lead card unfolds its sub-agent tree to the right while folding the last.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_card_shadow,
        pro_check_draw, pro_connector_pulse, pro_ease, pro_eid, pro_empty_state, pro_fold,
        pro_rolling_number, pro_shimmer_text, pro_skeleton_bar, pro_spring_bar, pro_spring_f32,
        pro_swap_label, pro_tactile, pro_toast_banner, tr,
    },
    foundation::Theme,
    primitives::{
        family::{PhaseClock, Slide},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, rc::Rc};
use web_time::Instant;

const SPIN_ARC_SVG: &str = r#"<circle cx="12" cy="12" r="8.5" stroke="currentColor" stroke-opacity="0.25" stroke-width="2.5"/><path d="M12 3.5A8.5 8.5 0 0 1 20.5 12" stroke="currentColor" stroke-linecap="round" stroke-width="2.5"/>"#;

/// Execution status of a lead agent, sub-agent, or task step inside [`AgentFleet`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FleetStatus {
    /// Queued and waiting for an execution slot.
    Queued,
    /// Actively executing; elapsed clock ticks and active step shimmers.
    #[default]
    Running,
    /// Blocked on human approval or clarification (`"Needs you"`).
    Waiting,
    /// Finished successfully.
    Done,
    /// Stopped due to a build, tool, or runtime error.
    Failed,
    /// Halted or skipped before completion.
    Stopped,
}

impl FleetStatus {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Queued => tr(cx, "Queued", "排队中"),
            Self::Running => tr(cx, "Running", "运行中"),
            Self::Waiting => tr(cx, "Needs you", "待确认"),
            Self::Done => tr(cx, "Done", "已完成"),
            Self::Failed => tr(cx, "Failed", "失败"),
            Self::Stopped => tr(cx, "Stopped", "已停止"),
        }
    }

    fn is_terminal(self) -> bool {
        matches!(self, Self::Done | Self::Failed | Self::Stopped)
    }

    fn is_muted(self) -> bool {
        matches!(self, Self::Queued | Self::Stopped)
    }
}

/// Individual work step inside a sub-agent's task breakdown in [`AgentFleet`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FleetStep {
    /// Short step description.
    pub title: SharedString,
    /// Current status of this step.
    pub status: FleetStatus,
    /// Optional tool name displayed as a mini tile before `title`.
    pub tool: Option<SharedString>,
    /// Optional duration string (for example `"2m 14s"`) shown when completed.
    pub took: Option<SharedString>,
}

impl FleetStep {
    /// Construct a completed [`FleetStep`] with `title` and `took` duration.
    pub fn done(
        title: impl Into<SharedString>,
        tool: Option<&str>,
        took: impl Into<SharedString>,
    ) -> Self {
        Self {
            title: title.into(),
            status: FleetStatus::Done,
            tool: tool.map(Into::into),
            took: Some(took.into()),
        }
    }

    /// Construct a [`FleetStep`] with an explicit [`FleetStatus`].
    pub fn with_status(
        title: impl Into<SharedString>,
        status: FleetStatus,
        tool: Option<&str>,
    ) -> Self {
        Self {
            title: title.into(),
            status,
            tool: tool.map(Into::into),
            took: None,
        }
    }
}

/// Lead orchestrator or nested sub-agent specification in [`AgentFleet`].
#[derive(Clone, Debug, PartialEq)]
pub struct FleetAgent {
    /// Stable identifier used for unfolding, focus, and resolution callbacks.
    pub id: SharedString,
    /// Display name shown on the card header.
    pub name: SharedString,
    /// Model name shown beneath `name` (for example `"Opus 5.5"`).
    pub model: SharedString,
    /// Single-line assignment summary.
    pub task: SharedString,
    /// Current runtime status.
    pub status: FleetStatus,
    /// Connected tool names rendered as overlapping `16x16` tiles before `task`.
    pub tools: Vec<SharedString>,
    /// Own spend in USD (lead cards automatically sum `cost` across all descendants).
    pub cost: f64,
    /// Base elapsed time in seconds.
    pub elapsed_secs: u64,
    /// Ordered task steps revealed when inspecting a sub-agent card.
    pub steps: Vec<FleetStep>,
    /// Question requiring operator approval when `status == FleetStatus::Waiting`.
    pub ask: Option<SharedString>,
    /// Error summary requiring resolution when `status == FleetStatus::Failed`.
    pub error: Option<SharedString>,
    /// Sub-agents coordinated by this agent.
    pub children: Vec<FleetAgent>,
}

impl FleetAgent {
    /// Construct a [`FleetAgent`] with `id`, `name`, `task`, and `status`.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        task: impl Into<SharedString>,
        status: FleetStatus,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            model: SharedString::default(),
            task: task.into(),
            status,
            tools: Vec::new(),
            cost: 0.15,
            elapsed_secs: 240,
            steps: Vec::new(),
            ask: None,
            error: None,
            children: Vec::new(),
        }
    }

    /// Set the model label displayed beneath `name`.
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = model.into();
        self
    }

    /// Attach overlapping tool tiles shown ahead of `task`.
    pub fn with_tools(mut self, tools: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.tools = tools.into_iter().map(Into::into).collect();
        self
    }

    /// Set this agent's own USD spend and elapsed seconds.
    pub fn with_usage(mut self, cost: f64, elapsed_secs: u64) -> Self {
        self.cost = if cost.is_finite() && cost >= 0.0 {
            cost
        } else {
            0.0
        };
        self.elapsed_secs = elapsed_secs;
        self
    }

    /// Attach task breakdown steps inspectable on click.
    pub fn with_steps(mut self, steps: Vec<FleetStep>) -> Self {
        self.steps = steps;
        self
    }

    /// Set the approval prompt shown when `status == FleetStatus::Waiting`.
    pub fn with_ask(mut self, ask: impl Into<SharedString>) -> Self {
        self.ask = Some(ask.into());
        self
    }

    /// Set the failure detail shown when `status == FleetStatus::Failed`.
    pub fn with_error(mut self, error: impl Into<SharedString>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Attach child sub-agents below this agent.
    pub fn with_children(mut self, children: Vec<FleetAgent>) -> Self {
        self.children = children;
        self
    }
}

/// Resolution decision for a waiting or failed sub-agent in [`AgentFleet`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FleetResolveAction {
    /// Approve a waiting sub-agent's question and mark it done/running.
    Approve,
    /// Reject a waiting sub-agent's proposal and stop it.
    Reject,
    /// Retry a failed sub-agent.
    Retry,
    /// Dismiss a failed sub-agent error and mark it stopped.
    Dismiss,
}

/// Typed events emitted by [`AgentFleet`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentFleetEvent {
    /// A lead agent's sub-agent tree was unfolded or folded (`None`).
    OpenChanged {
        /// Identifier of the newly open lead agent, or `None` when all are folded.
        open_id: Option<SharedString>,
    },
    /// User clicked a sub-agent card to inspect its steps or decision prompt.
    SubAgentSelected {
        /// Selected sub-agent identifier.
        agent_id: SharedString,
    },
    /// Operator resolved a waiting or failed sub-agent.
    Resolved {
        /// Resolved sub-agent identifier.
        agent_id: SharedString,
        /// Chosen resolution action.
        action: FleetResolveAction,
    },
}

impl EventEmitter<AgentFleetEvent> for AgentFleet {}

/// Overview of multiple parallel lead agents whose sub-agent trees unfold to the right.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use kui::components::agent_fleet::AgentFleet;
///
/// let fleet = cx.new(|cx| AgentFleet::new("agent-fleet", cx));
/// # }
/// ```
pub struct AgentFleet {
    id: ElementId,
    focus: FocusHandle,
    title: Option<SharedString>,
    agents: Vec<FleetAgent>,
    custom_data: bool,
    open_lead_id: Option<SharedString>,
    open_sub_id: Option<SharedString>,
    focused_lead_idx: usize,
    hovered_lead_id: Option<SharedString>,
    hovered_sub_id: Option<SharedString>,
    dim_slides: Vec<Slide>,
    branch_clock: PhaseClock,
    toast: Option<(SharedString, Instant)>,
    loading: bool,
    empty: bool,
    boot: Instant,
    measured_width: Rc<Cell<f32>>,
}

impl AgentFleet {
    /// Create a live [`AgentFleet`] with four parallel lead agents and `"launch"` unfolded.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let agents = default_fleet_agents();
        let open_lead_id = agents.first().map(|a| a.id.clone());
        let dim_slides = agents
            .iter()
            .enumerate()
            .map(|(i, _)| Slide::new(if i == 0 { 1.0 } else { 0.50 }))
            .collect();
        let mut branch_clock = PhaseClock::default();
        branch_clock.restart(cx);
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            title: None,
            agents,
            custom_data: false,
            open_lead_id,
            open_sub_id: None,
            focused_lead_idx: 0,
            hovered_lead_id: None,
            hovered_sub_id: None,
            dim_slides,
            branch_clock,
            toast: None,
            loading: false,
            empty: false,
            boot: Instant::now(),
            measured_width: Rc::new(Cell::new(780.0)),
        }
    }

    /// Override the fleet header title (defaults to `"Fleet"` / `"智能体舰队"`).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Replace the fleet with custom lead [`FleetAgent`] trees.
    pub fn with_agents(mut self, agents: Vec<FleetAgent>) -> Self {
        self.custom_data = true;
        if agents.is_empty() {
            self.agents.clear();
            self.open_lead_id = None;
            self.dim_slides.clear();
            self.empty = true;
        } else {
            let open = self
                .open_lead_id
                .clone()
                .filter(|id| agents.iter().any(|a| &a.id == id))
                .or_else(|| agents.first().map(|a| a.id.clone()));
            self.dim_slides = agents
                .iter()
                .map(|a| {
                    let target = match &open {
                        Some(oid) if oid == &a.id => 1.0,
                        Some(_) => 0.50,
                        None => 1.0,
                    };
                    Slide::new(target)
                })
                .collect();
            self.open_lead_id = open;
            self.agents = agents;
        }
        self
    }

    /// Set which lead agent starts unfolded (`None` folds all lead agents).
    pub fn with_default_open(mut self, open_id: Option<SharedString>) -> Self {
        self.open_lead_id = open_id.filter(|id| self.agents.iter().any(|a| &a.id == id));
        for (i, a) in self.agents.iter().enumerate() {
            if let Some(sl) = self.dim_slides.get_mut(i) {
                let target = match &self.open_lead_id {
                    Some(oid) if oid == &a.id => 1.0,
                    Some(_) => 0.50,
                    None => 1.0,
                };
                sl.reset(target);
            }
        }
        self
    }

    /// Render skeleton placeholders when `loading` is `true`.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Force the empty state when `empty` is `true`.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the currently unfolded lead agent identifier, if any.
    pub fn open_id(&self) -> Option<&SharedString> {
        self.open_lead_id.as_ref()
    }

    /// Compute the total rolled-up USD spend across the entire fleet.
    pub fn total_fleet_cost(&self) -> f64 {
        self.agents.iter().map(branch_cost).sum()
    }

    /// Open a specific lead agent's tree (or pass `None` to fold all trees).
    pub fn set_open(&mut self, next: Option<SharedString>, cx: &mut Context<Self>) {
        let valid = next.filter(|id| self.agents.iter().any(|a| &a.id == id));
        if self.open_lead_id == valid {
            return;
        }
        self.open_lead_id = valid.clone();
        self.open_sub_id = None;
        self.hovered_sub_id = None;
        self.branch_clock.restart(cx);
        self.retarget_dim_slides(cx);
        cx.emit(AgentFleetEvent::OpenChanged { open_id: valid });
        cx.notify();
    }

    /// Toggle a lead agent open, or fold it if already open.
    pub fn toggle_lead(&mut self, lead_id: SharedString, cx: &mut Context<Self>) {
        if self.open_lead_id.as_ref() == Some(&lead_id) {
            self.set_open(None, cx);
        } else {
            if let Some(pos) = self.agents.iter().position(|a| a.id == lead_id) {
                self.focused_lead_idx = pos;
            }
            self.set_open(Some(lead_id), cx);
        }
    }

    /// Resolve a waiting or failed sub-agent and emit [`AgentFleetEvent::Resolved`].
    pub fn resolve_sub_agent(
        &mut self,
        sub_id: impl Into<SharedString>,
        action: FleetResolveAction,
        cx: &mut Context<Self>,
    ) {
        let target = sub_id.into();
        let mut resolved_name = target.clone();
        for lead in &mut self.agents {
            if let Some(sub) = find_fleet_mut(lead, &target) {
                resolved_name = sub.name.clone();
                match action {
                    FleetResolveAction::Approve => {
                        sub.status = FleetStatus::Done;
                        sub.ask = None;
                        sub.cost += 0.04;
                        for step in &mut sub.steps {
                            if step.status == FleetStatus::Waiting {
                                step.status = FleetStatus::Done;
                                step.took = Some("0m 18s".into());
                            }
                        }
                    }
                    FleetResolveAction::Reject => {
                        sub.status = FleetStatus::Stopped;
                        sub.ask = None;
                    }
                    FleetResolveAction::Retry => {
                        sub.status = FleetStatus::Running;
                        sub.error = None;
                        sub.cost += 0.03;
                        for step in &mut sub.steps {
                            if step.status == FleetStatus::Failed {
                                step.status = FleetStatus::Running;
                            }
                        }
                    }
                    FleetResolveAction::Dismiss => {
                        sub.status = FleetStatus::Stopped;
                        sub.error = None;
                    }
                }
                break;
            }
        }
        self.open_sub_id = None;
        let msg = match action {
            FleetResolveAction::Approve => {
                format!("{} {resolved_name}", tr(cx, "Approved", "已批准"))
            }
            FleetResolveAction::Reject => {
                format!("{} {resolved_name}", tr(cx, "Rejected", "已拒绝"))
            }
            FleetResolveAction::Retry => {
                format!("{} {resolved_name}", tr(cx, "Retrying", "正在重试"))
            }
            FleetResolveAction::Dismiss => {
                format!("{} {resolved_name}", tr(cx, "Dismissed", "已忽略"))
            }
        };
        self.toast = Some((msg.into(), Instant::now()));
        cx.emit(AgentFleetEvent::Resolved {
            agent_id: target,
            action,
        });
        cx.notify();
    }

    /// Toggle skeleton loading placeholders at runtime.
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

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    /// Return whether the fleet is empty.
    pub fn is_empty(&self) -> bool {
        self.empty || self.agents.is_empty()
    }

    fn retarget_dim_slides(&mut self, cx: &mut Context<Self>) {
        if self.dim_slides.len() != self.agents.len() {
            self.dim_slides = self.agents.iter().map(|_| Slide::new(1.0)).collect();
        }
        for (i, a) in self.agents.iter().enumerate() {
            let is_open = self.open_lead_id.as_ref() == Some(&a.id);
            let is_hov = self.hovered_lead_id.as_ref() == Some(&a.id);
            let target = if self.open_lead_id.is_none() || is_open {
                1.0
            } else if is_hov {
                0.86
            } else {
                0.50
            };
            self.dim_slides[i].duration(target, 0.28, cx);
        }
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if self.agents.is_empty() {
            return;
        }
        match ev.keystroke.key.as_str() {
            "escape" => {
                if self.open_sub_id.is_some() {
                    self.open_sub_id = None;
                    self.focus.focus(w, cx);
                    cx.notify();
                } else if self.open_lead_id.is_some() {
                    self.set_open(None, cx);
                }
            }
            "down" => {
                self.focused_lead_idx = (self.focused_lead_idx + 1) % self.agents.len();
                cx.notify();
            }
            "up" => {
                self.focused_lead_idx = if self.focused_lead_idx == 0 {
                    self.agents.len() - 1
                } else {
                    self.focused_lead_idx - 1
                };
                cx.notify();
            }
            "home" => {
                self.focused_lead_idx = 0;
                cx.notify();
            }
            "end" => {
                self.focused_lead_idx = self.agents.len() - 1;
                cx.notify();
            }
            "enter" | " " => {
                let lid = self.agents[self.focused_lead_idx.min(self.agents.len() - 1)]
                    .id
                    .clone();
                self.toggle_lead(lid, cx);
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_lead_card(
        &self,
        idx: usize,
        lead: &FleetAgent,
        wide: bool,
        opacity: f32,
        sensor: Rc<Cell<Option<Bounds<Pixels>>>>,
        live_tick_secs: u64,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let lid = lead.id.clone();
        let is_open = self.open_lead_id.as_ref() == Some(&lid);
        let disp_name = if self.custom_data {
            lead.name.clone()
        } else {
            tr_fleet_name(&lead.id, &lead.name, cx).into()
        };
        let disp_task = if self.custom_data {
            lead.task.clone()
        } else {
            tr_fleet_task(&lead.task, cx).into()
        };

        let total_cost = branch_cost(lead);
        let elapsed = if lead.status == FleetStatus::Running {
            lead.elapsed_secs + live_tick_secs
        } else {
            lead.elapsed_secs
        };

        let (pie_ratio, summary_phrase, summary_tone) = summarize_sub_agents(lead, p, cx);
        let status_pill = render_status_pill(&self.id, &lead.id, lead.status, p, cx);
        let avatar_el = pro_agent_avatar(
            fleet_avatar_seed(&lead.model, &lead.name, &lead.id),
            p.blue,
            22.,
            lead.status == FleetStatus::Running,
            (live_tick_secs as f32) * 1000.0,
            cx,
        );

        let mut task_row = div()
            .w_full()
            .h(px(16.))
            .flex()
            .items_center()
            .gap(px(6.))
            .min_w_0();
        if !lead.tools.is_empty() {
            let shown = lead.tools.len().min(3);
            let mut tools_strip = div().flex_none().flex().items_center();
            for (ti, tool) in lead.tools.iter().take(shown).enumerate() {
                tools_strip = tools_strip.child(
                    div()
                        .when(ti > 0, |d| d.ml(px(-4.)))
                        .child(pro_app_tile(tool, 16., cx)),
                );
            }
            task_row = task_row.child(tools_strip);
        }
        task_row = task_row.child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(12.))
                .line_height(px(16.))
                .text_color(p.text)
                .truncate()
                .child(disp_task),
        );

        let border_c = if is_open { p.line_strong } else { p.line };
        let click_id = lid.clone();
        let hover_id = lid.clone();
        let aria = format!("{disp_name}: {}", lead.status.label(cx));

        let card_body = div()
            .w_full()
            .relative()
            .p(px(10.))
            .pb(px(8.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                canvas(
                    move |bounds, _, _| {
                        sensor.set(Some(bounds));
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .min_w_0()
                    .child(avatar_el)
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
                                    .child(disp_name),
                            )
                            .when(!lead.model.is_empty(), |d| {
                                d.child(
                                    div()
                                        .text_size(px(11.))
                                        .line_height(px(14.))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(lead.model.clone()),
                                )
                            }),
                    )
                    .child(status_pill),
            )
            .child(task_row)
            .child(pro_spring_bar(
                pro_eid(&self.id, format!("lprog-{}", lead.id)),
                pie_ratio,
                2.5,
                1.5,
                p.chip,
                if lead.status == FleetStatus::Done {
                    p.green
                } else {
                    p.blue
                },
            ))
            .child(
                div()
                    .w_full()
                    .h(px(20.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .min_w_0()
                            .child(render_pie_ring(
                                pro_eid(&self.id, format!("pie-{}", lead.id)),
                                pie_ratio,
                                summary_tone,
                            ))
                            .child(pro_swap_label(
                                pro_eid(&self.id, format!("lsum-{}", lead.id)),
                                summary_phrase,
                                11.,
                                FontWeight::MEDIUM,
                                summary_tone,
                            )),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(3.))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, format!("lcost-{}", lead.id)),
                                total_cost,
                                2,
                                "$",
                                "",
                                11.,
                                FontWeight::MEDIUM,
                                p.ink,
                            ))
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(p.muted)
                                    .child(format!("· {}", format_duration_secs(elapsed))),
                            ),
                    ),
            );

        pro_tactile(
            pro_eid(&self.id, format!("lead-{idx}")),
            aria,
            10.,
            p.card,
            p.subtle,
            border_c,
            p.line_strong,
            is_open,
            card_body,
            cx,
            move |this, _, cx| {
                this.toggle_lead(click_id.clone(), cx);
            },
        )
        .when(wide, |d| d.w(px(288.)))
        .when(!wide, |d| d.w_full())
        .shadow(pro_card_shadow(p.dark))
        .relative()
        .opacity(opacity)
        .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
            let next = if *hovered {
                Some(hover_id.clone())
            } else if this.hovered_lead_id.as_ref() == Some(&hover_id) {
                None
            } else {
                this.hovered_lead_id.clone()
            };
            if this.hovered_lead_id != next {
                this.hovered_lead_id = next;
                this.retarget_dim_slides(cx);
                cx.notify();
            }
        }))
        .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_sub_card(
        &self,
        sub: &FleetAgent,
        wide: bool,
        enter_t: f32,
        sensor: Rc<Cell<Option<Bounds<Pixels>>>>,
        live_tick_secs: u64,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let sid = sub.id.clone();
        let is_popped = self.open_sub_id.as_ref() == Some(&sid);
        let disp_name = if self.custom_data {
            sub.name.clone()
        } else {
            tr_fleet_name(&sub.id, &sub.name, cx).into()
        };
        let disp_task = if self.custom_data {
            sub.task.clone()
        } else {
            tr_fleet_task(&sub.task, cx).into()
        };
        let elapsed = if sub.status == FleetStatus::Running {
            sub.elapsed_secs + live_tick_secs
        } else {
            sub.elapsed_secs
        };
        let muted = sub.status.is_muted();
        let status_pill = render_status_pill(&self.id, &sub.id, sub.status, p, cx);

        let needs_decision = sub.status == FleetStatus::Waiting && sub.ask.is_some();
        let needs_resolve = sub.status == FleetStatus::Failed && sub.error.is_some();
        let action_pill_label = if needs_decision {
            Some(tr(cx, "Answer", "处理"))
        } else if needs_resolve {
            Some(tr(cx, "Resolve", "解决"))
        } else if !sub.steps.is_empty() {
            Some(tr(cx, "Steps", "步骤"))
        } else {
            None
        };

        let border_c = if is_popped { p.line_strong } else { p.line };
        let click_id = sid.clone();
        let hover_id = sid.clone();

        let dx = if wide { -8.0 * (1.0 - enter_t) } else { 0.0 };
        let base_alpha = if muted { 0.62 } else { 1.0 } * enter_t.clamp(0.0, 1.0);

        let step_ratio = if sub.steps.is_empty() {
            None
        } else {
            let done_n = sub
                .steps
                .iter()
                .filter(|s| s.status == FleetStatus::Done)
                .count();
            Some((done_n as f32) / (sub.steps.len() as f32))
        };

        let sub_body = div()
            .w_full()
            .relative()
            .p(px(10.))
            .pb(px(8.))
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                canvas(
                    move |bounds, _, _| {
                        sensor.set(Some(bounds));
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .min_w_0()
                    .child(pro_agent_avatar(
                        fleet_avatar_seed(&sub.model, &sub.name, &sub.id),
                        if sub.status == FleetStatus::Waiting {
                            p.amber
                        } else if sub.status == FleetStatus::Failed {
                            p.red
                        } else {
                            p.blue
                        },
                        22.,
                        sub.status == FleetStatus::Running,
                        (live_tick_secs as f32) * 1000.0,
                        cx,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.5))
                            .line_height(px(16.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .truncate()
                            .child(disp_name.clone()),
                    )
                    .child(status_pill),
            )
            .child(
                div()
                    .w_full()
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(p.text)
                    .truncate()
                    .child(disp_task),
            )
            .when_some(step_ratio, |d, sr| {
                d.child(pro_spring_bar(
                    pro_eid(&self.id, format!("sprog-{}", sub.id)),
                    sr,
                    2.5,
                    1.5,
                    p.chip,
                    if sub.status == FleetStatus::Failed {
                        p.red
                    } else if sub.status == FleetStatus::Waiting {
                        p.amber
                    } else if sub.status == FleetStatus::Done {
                        p.green
                    } else {
                        p.blue
                    },
                ))
            })
            .child(
                div()
                    .w_full()
                    .h(px(20.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(6.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(3.))
                            .text_size(px(11.))
                            .child(pro_rolling_number(
                                pro_eid(&self.id, format!("scost-{}", sub.id)),
                                sub.cost,
                                2,
                                "$",
                                "",
                                11.,
                                FontWeight::MEDIUM,
                                p.ink,
                            ))
                            .child(
                                div()
                                    .text_color(p.muted)
                                    .child(format!("· {}", format_duration_secs(elapsed))),
                            ),
                    )
                    .when_some(action_pill_label, |d, lbl| {
                        d.child(
                            div()
                                .h(px(20.))
                                .px(px(7.))
                                .rounded(px(5.))
                                .bg(if is_popped { p.chip_hover } else { p.chip })
                                .border_1()
                                .border_color(p.line)
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .flex()
                                .items_center()
                                .child(lbl),
                        )
                    }),
            );

        let card_box = pro_tactile(
            pro_eid(&self.id, format!("sub-{}", sub.id)),
            format!("{disp_name}: {}", sub.status.label(cx)),
            8.,
            p.card,
            p.subtle,
            border_c,
            p.line_strong,
            is_popped,
            sub_body,
            cx,
            move |this, _, cx| {
                let target = click_id.clone();
                if this.open_sub_id.as_ref() == Some(&target) {
                    this.open_sub_id = None;
                } else {
                    this.open_sub_id = Some(target.clone());
                    cx.emit(AgentFleetEvent::SubAgentSelected { agent_id: target });
                }
                cx.notify();
            },
        )
        .role(Role::TreeItem)
        .shadow(pro_card_shadow(p.dark))
        .when(wide, |d| d.w(px(240.)))
        .when(!wide, |d| d.w_full())
        .relative()
        .left(px(dx))
        .opacity(base_alpha)
        .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
            let next = if *hovered {
                Some(hover_id.clone())
            } else if this.hovered_sub_id.as_ref() == Some(&hover_id) {
                None
            } else {
                this.hovered_sub_id.clone()
            };
            if this.hovered_sub_id != next {
                this.hovered_sub_id = next;
                cx.notify();
            }
        }));

        let drawer_el = render_sub_tasks_drawer(&self.id, sub, p, cx);
        let fold_panel = pro_fold(
            pro_eid(&self.id, format!("sub-pop-{}", sub.id)),
            is_popped,
            150.,
            0.,
            move || drawer_el,
        );

        div()
            .when(wide, |d| d.w(px(240.)))
            .when(!wide, |d| d.w_full())
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(card_box)
            .child(fold_panel)
            .into_any_element()
    }
}

impl Render for AgentFleet {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let elapsed_secs = self.boot.elapsed().as_secs();

        let any_running = self
            .agents
            .iter()
            .any(|a| count_fleet_by(a, |n| n.status == FleetStatus::Running) > 0);
        if any_running && !reduced {
            w.request_animation_frame();
        }

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
            let phase = (self.boot.elapsed().as_secs_f32() / 1.2).fract();
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(width_sensor)
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_between()
                        .child(pro_skeleton_bar(120., 18., 5., phase, cx))
                        .child(pro_skeleton_bar(210., 16., 5., phase, cx)),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .gap(px(48.))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(8.))
                                .child(pro_skeleton_bar(288., 96., 10., phase, cx))
                                .child(pro_skeleton_bar(288., 96., 10., phase, cx))
                                .child(pro_skeleton_bar(288., 96., 10., phase, cx)),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(10.))
                                .child(pro_skeleton_bar(240., 90., 8., phase, cx))
                                .child(pro_skeleton_bar(240., 90., 8., phase, cx)),
                        ),
                )
                .into_any_element();
        }

        if self.empty || self.agents.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .child(width_sensor)
                .child(pro_empty_state(
                    ProIcon::Sparkles,
                    tr(cx, "No agents in fleet", "舰队中暂无智能体"),
                    tr(
                        cx,
                        "Launch one or more orchestrator agents to compare progress, cost, and sub-agent trees.",
                        "启动一个或多个主控智能体，即可并排查看子智能体进度、待办审批与实时花费。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let container_w = self.measured_width.get();
        let wide = container_w >= 650.0;

        let lead_count = self.agents.len();
        let sub_count: usize = self
            .agents
            .iter()
            .map(|a| count_fleet_all(a).saturating_sub(1))
            .sum();
        let running_count: usize = self
            .agents
            .iter()
            .map(|a| count_fleet_by(a, |n| n.status == FleetStatus::Running))
            .sum();
        let waiting_count: usize = self
            .agents
            .iter()
            .map(|a| count_fleet_by(a, |n| n.status == FleetStatus::Waiting))
            .sum();
        let fleet_cost = self.total_fleet_cost();

        let title_str: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Fleet", "智能体舰队").into());

        let header = div()
            .w_full()
            .mb(px(16.))
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .child(pro_icon(ProIcon::Sparkles, 14., p.blue))
                    .child(
                        div()
                            .text_size(px(13.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(title_str.clone()),
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
                        "{lead_count} {} / {sub_count} {}",
                        tr(cx, "agents", "个主智能体"),
                        tr(cx, "sub-agents", "个子智能体")
                    ))
                    .when(running_count > 0, |d| {
                        d.child(format!("{running_count} {}", tr(cx, "running", "运行中")))
                    })
                    .when(waiting_count > 0, |d| {
                        d.child(
                            div()
                                .text_color(p.amber_ink)
                                .font_weight(FontWeight::MEDIUM)
                                .child(format!(
                                    "{waiting_count} {}",
                                    tr(cx, "needs you", "待确认")
                                )),
                        )
                    })
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "fleet-cost"),
                        fleet_cost,
                        2,
                        "$",
                        "",
                        12.,
                        FontWeight::MEDIUM,
                        p.ink,
                    )),
            );

        if self.dim_slides.len() != self.agents.len() {
            self.retarget_dim_slides(cx);
        }

        let branch_age = self.branch_clock.time(0.38, w, cx);
        let enter_base = if reduced {
            1.0
        } else {
            pro_ease((branch_age / 0.34).clamp(0.0, 1.0))
        };

        let mut lead_sensors: HashMap<SharedString, Rc<Cell<Option<Bounds<Pixels>>>>> =
            HashMap::new();
        let mut sub_sensors: HashMap<SharedString, Rc<Cell<Option<Bounds<Pixels>>>>> =
            HashMap::new();
        let mut connector_edges: Vec<(SharedString, SharedString, bool, bool, f32)> = Vec::new();

        let focus_sub = self
            .hovered_sub_id
            .clone()
            .or_else(|| self.open_sub_id.clone());
        let any_sub_spot = pro_spring_f32(
            pro_eid(&self.id, "f-any-spot"),
            if focus_sub.is_some() { 1.0 } else { 0.0 },
            0.22,
            w,
            cx,
        )
        .clamp(0., 1.);

        let agents_snapshot = self.agents.clone();
        let mut left_col = div()
            .when(wide, |d| d.w(px(288.)).flex_none())
            .when(!wide, |d| d.w_full())
            .flex()
            .flex_col()
            .gap(px(8.));

        for (idx, lead) in agents_snapshot.iter().enumerate() {
            let l_op = self.dim_slides[idx].get(w, cx).clamp(0.2, 1.0);
            let l_sensor = Rc::new(Cell::new(None));
            lead_sensors.insert(lead.id.clone(), l_sensor.clone());
            let card_el =
                self.render_lead_card(idx, lead, wide, l_op, l_sensor, elapsed_secs, &p, cx);

            if !wide && !lead.children.is_empty() {
                let is_mob_open = self.open_lead_id.as_ref() == Some(&lead.id);
                let mut mobile_branch = div()
                    .w_full()
                    .pl(px(24.))
                    .pt(px(4.))
                    .pb(px(4.))
                    .flex()
                    .flex_col()
                    .gap(px(8.));
                for sub in &lead.children {
                    let dummy = Rc::new(Cell::new(None));
                    mobile_branch = mobile_branch.child(self.render_sub_card(
                        sub,
                        false,
                        1.0,
                        dummy,
                        elapsed_secs,
                        &p,
                        cx,
                    ));
                    for grand in &sub.children {
                        let gdummy = Rc::new(Cell::new(None));
                        mobile_branch = mobile_branch.child(div().pl(px(24.)).child(
                            self.render_sub_card(grand, false, 1.0, gdummy, elapsed_secs, &p, cx),
                        ));
                    }
                }
                let mob_fold = pro_fold(
                    pro_eid(&self.id, format!("mob-branch-{}", lead.id)),
                    is_mob_open,
                    220.,
                    6.,
                    move || mobile_branch.into_any_element(),
                );
                left_col = left_col.child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(card_el)
                        .child(mob_fold),
                );
            } else {
                left_col = left_col.child(card_el);
            }
        }

        let right_tree = if wide {
            if let Some(open_id) = self.open_lead_id.clone()
                && let Some(open_lead) = agents_snapshot.iter().find(|a| a.id == open_id)
            {
                let mut col = div()
                    .id(pro_eid(&self.id, "open-branch"))
                    .role(Role::Tree)
                    .aria_label(format!(
                        "{} {}",
                        tr(cx, "Sub-agents of", "子智能体："),
                        open_lead.name
                    ))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(10.));

                if open_lead.children.is_empty() {
                    col = col.child(
                        div()
                            .py(px(12.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(tr(
                                cx,
                                "Working solo — no sub-agents spawned",
                                "独立执行中，未派生子智能体",
                            )),
                    );
                } else {
                    for (si, sub) in open_lead.children.iter().enumerate() {
                        let stagger = if reduced {
                            1.0
                        } else {
                            let delay = (si as f32) * 0.05;
                            pro_ease(((branch_age - delay) / 0.30).clamp(0.0, 1.0))
                        };
                        let s_sensor = Rc::new(Cell::new(None));
                        sub_sensors.insert(sub.id.clone(), s_sensor.clone());
                        let lit1 = focus_sub.as_ref().is_some_and(|fid| {
                            fid == &sub.id || sub.children.iter().any(|g| &g.id == fid)
                        }) || self.hovered_lead_id.as_ref() == Some(&open_lead.id);
                        let spot1 = pro_spring_f32(
                            pro_eid(&self.id, format!("f-edge-{}-{}", open_lead.id, sub.id)),
                            if lit1 { 1.0 } else { 0.0 },
                            0.22,
                            w,
                            cx,
                        )
                        .clamp(0., 1.);
                        connector_edges.push((
                            open_lead.id.clone(),
                            sub.id.clone(),
                            true,
                            sub.status.is_muted(),
                            spot1,
                        ));

                        let sub_el = self.render_sub_card(
                            sub,
                            true,
                            stagger,
                            s_sensor,
                            elapsed_secs,
                            &p,
                            cx,
                        );

                        if sub.children.is_empty() {
                            col = col.child(sub_el);
                        } else {
                            let mut grand_col = div().pl(px(32.)).flex().flex_col().gap(px(10.));
                            for grand in &sub.children {
                                let g_sensor = Rc::new(Cell::new(None));
                                sub_sensors.insert(grand.id.clone(), g_sensor.clone());
                                let lit2 = focus_sub
                                    .as_ref()
                                    .is_some_and(|fid| fid == &sub.id || fid == &grand.id);
                                let spot2 = pro_spring_f32(
                                    pro_eid(&self.id, format!("f-edge-{}-{}", sub.id, grand.id)),
                                    if lit2 { 1.0 } else { 0.0 },
                                    0.22,
                                    w,
                                    cx,
                                )
                                .clamp(0., 1.);
                                connector_edges.push((
                                    sub.id.clone(),
                                    grand.id.clone(),
                                    false,
                                    grand.status.is_muted(),
                                    spot2,
                                ));
                                grand_col = grand_col.child(self.render_sub_card(
                                    grand,
                                    true,
                                    stagger,
                                    g_sensor,
                                    elapsed_secs,
                                    &p,
                                    cx,
                                ));
                            }
                            col = col.child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(10.))
                                    .child(sub_el)
                                    .child(grand_col),
                            );
                        }
                    }
                }
                Some(col.into_any_element())
            } else {
                None
            }
        } else {
            None
        };

        let link_blue: Hsla = if p.dark {
            p.blue.opacity(0.75 * enter_base)
        } else {
            rgb(0x83c7fe).into()
        };
        let link_muted = p.line_strong.opacity(enter_base);
        let pulse_blue = p.blue;
        let card_bg = p.card;
        let pulse_t = if !reduced {
            Some((self.boot.elapsed().as_secs_f32() / 2.2).fract())
        } else {
            None
        };

        let links_canvas = canvas(
            |_, _, _| (),
            move |_, _, window, _| {
                let mut anchors: Vec<(Point<Pixels>, f32)> = Vec::new();
                for (from_id, to_id, horizontal_bridge, is_muted, spot) in &connector_edges {
                    let b0_opt = lead_sensors
                        .get(from_id)
                        .and_then(|c| c.get())
                        .or_else(|| sub_sensors.get(from_id).and_then(|c| c.get()));
                    let b1_opt = sub_sensors.get(to_id).and_then(|c| c.get());
                    let (Some(b0), Some(b1)) = (b0_opt, b1_opt) else {
                        continue;
                    };
                    let dim_factor = 1.0 - 0.36 * any_sub_spot * (1.0 - *spot);
                    let base_col = if *is_muted {
                        link_muted
                    } else {
                        link_blue.opacity(if p.dark { 0.72 } else { 0.92 } * enter_base)
                    };
                    let stroke_col = if *spot > 0.01 {
                        pulse_blue.opacity((0.55 + 0.43 * *spot) * enter_base)
                    } else {
                        base_col.opacity((base_col.a * dim_factor).clamp(0.16, 1.0))
                    };
                    let stroke_w = 1.5 + 0.85 * *spot;
                    if *horizontal_bridge {
                        let p0 = point(b0.right(), (b0.top() + b0.bottom()) * 0.5);
                        let p1 = point(b1.left(), (b1.top() + b1.bottom()) * 0.5);
                        let prog = if *is_muted && *spot < 0.1 {
                            None
                        } else {
                            pulse_t
                        };
                        pro_connector_pulse(
                            p0, p1, 8.0, false, stroke_col, pulse_blue, prog, stroke_w, window,
                        );
                        if let Some(existing) = anchors.iter_mut().find(|(a, _)| {
                            (a.x - p0.x).abs() < px(1.) && (a.y - p0.y).abs() < px(1.)
                        }) {
                            existing.1 = existing.1.max(*spot);
                        } else {
                            anchors.push((p0, *spot));
                        }
                    } else {
                        let x0 = f32::from(b0.left()) + 16.0;
                        let y0 = f32::from(b0.bottom());
                        let x1 = f32::from(b1.left());
                        let y1 = (f32::from(b1.top()) + f32::from(b1.bottom())) * 0.5;
                        paint_sub_elbow(x0, y0, x1, y1, 5.5, stroke_col, stroke_w, window);
                        anchors.push((point(px(x0), px(y0)), *spot));
                    }
                }
                for (pt, spot) in anchors {
                    paint_ring_dot(
                        pt,
                        3.25 + 0.5 * spot,
                        card_bg,
                        if spot > 0.05 { pulse_blue } else { link_blue },
                        1.5 + 0.4 * spot,
                        window,
                    );
                }
            },
        )
        .absolute()
        .inset_0();

        let sr_announce = format!(
            "{lead_count} agents, {running_count} running, {waiting_count} needs you, ${fleet_cost:.2}"
        );

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .role(Role::Region)
            .aria_label(title_str)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                this.handle_key_down(ev, w, cx);
            }))
            .w_full()
            .p(px(12.))
            .relative()
            .flex()
            .flex_col()
            .child(width_sensor)
            .child(header)
            .child(
                div()
                    .w_full()
                    .relative()
                    .flex()
                    .items_start()
                    .gap(px(56.))
                    .when(wide, |d| d.child(links_canvas))
                    .child(left_col)
                    .children(right_tree),
            )
            .child(pro_toast_banner(self.toast.clone()))
            .child(
                div()
                    .id(pro_eid(&self.id, "sr-fleet"))
                    .role(Role::Status)
                    .aria_label(sr_announce)
                    .absolute()
                    .size_0(),
            )
            .into_any_element()
    }
}

fn render_sub_tasks_drawer(
    root_id: &ElementId,
    sub: &FleetAgent,
    p: &ProPalette,
    cx: &mut Context<AgentFleet>,
) -> AnyElement {
    let sid = sub.id.clone();
    let mut box_el = div()
        .id(pro_eid(root_id, format!("drawer-{sid}")))
        .on_click(|_, _, cx| cx.stop_propagation())
        .w_full()
        .rounded(px(9.))
        .bg(p.panel)
        .border_1()
        .border_color(p.line_strong)
        .shadow_md()
        .p(px(10.))
        .flex()
        .flex_col()
        .gap(px(8.));

    if sub.status == FleetStatus::Waiting
        && let Some(ask) = &sub.ask
    {
        let s_rej = sid.clone();
        let s_app = sid.clone();
        box_el = box_el.child(
            div()
                .w_full()
                .pb(px(8.))
                .border_b_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(
                    div()
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.amber_ink)
                        .child(tr(cx, "Needs your decision", "需要你的确认")),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(17.))
                        .text_color(p.ink)
                        .child(ask.clone()),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_end()
                        .gap(px(6.))
                        .child(pro_btn(
                            pro_eid(root_id, format!("rej-{sid}")),
                            tr(cx, "Reject", "拒绝"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.resolve_sub_agent(
                                    s_rej.clone(),
                                    FleetResolveAction::Reject,
                                    cx,
                                );
                            },
                        ))
                        .child(pro_btn(
                            pro_eid(root_id, format!("app-{sid}")),
                            tr(cx, "Approve", "批准"),
                            Some(ProIcon::Check),
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.resolve_sub_agent(
                                    s_app.clone(),
                                    FleetResolveAction::Approve,
                                    cx,
                                );
                            },
                        )),
                ),
        );
    }

    if sub.status == FleetStatus::Failed
        && let Some(err) = &sub.error
    {
        let s_dis = sid.clone();
        let s_ret = sid.clone();
        box_el = box_el.child(
            div()
                .w_full()
                .pb(px(8.))
                .border_b_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(
                    div()
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.red_ink)
                        .child(tr(cx, "Execution failed", "执行失败")),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .line_height(px(17.))
                        .text_color(p.ink)
                        .child(err.clone()),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .justify_end()
                        .gap(px(6.))
                        .child(pro_btn(
                            pro_eid(root_id, format!("dis-{sid}")),
                            tr(cx, "Dismiss", "忽略"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.resolve_sub_agent(
                                    s_dis.clone(),
                                    FleetResolveAction::Dismiss,
                                    cx,
                                );
                            },
                        ))
                        .child(pro_btn(
                            pro_eid(root_id, format!("ret-{sid}")),
                            tr(cx, "Retry", "重试"),
                            Some(ProIcon::Refresh),
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.resolve_sub_agent(
                                    s_ret.clone(),
                                    FleetResolveAction::Retry,
                                    cx,
                                );
                            },
                        )),
                ),
        );
    }

    if !sub.steps.is_empty() {
        let done_steps = sub
            .steps
            .iter()
            .filter(|s| s.status == FleetStatus::Done)
            .count();
        let total_steps = sub.steps.len();
        box_el = box_el
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(11.5))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tr(cx, "Tasks", "任务步骤")),
                    )
                    .child(pro_rolling_number(
                        pro_eid(root_id, format!("drw-cnt-{sid}")),
                        done_steps as f64,
                        0,
                        "",
                        format!("/{total_steps}"),
                        11.5,
                        FontWeight::NORMAL,
                        p.muted,
                    )),
            )
            .child(pro_spring_bar(
                pro_eid(root_id, format!("drw-bar-{sid}")),
                (done_steps as f32) / (total_steps.max(1) as f32),
                3.0,
                1.5,
                p.chip,
                p.green,
            ));

        let mut list = div().w_full().flex().flex_col().gap(px(4.));
        for (idx, st) in sub.steps.iter().enumerate() {
            let right_el = match st.status {
                FleetStatus::Done => div()
                    .text_size(px(11.))
                    .text_color(p.muted)
                    .child(st.took.clone().unwrap_or_else(|| "Done".into()))
                    .into_any_element(),
                FleetStatus::Running => pro_shimmer_text(
                    pro_eid(root_id, format!("st-shim-{sid}-{idx}")),
                    tr(cx, "Running", "运行中"),
                    p.blue,
                    p.strong,
                    11.,
                    FontWeight::MEDIUM,
                    true,
                ),
                FleetStatus::Waiting => div()
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.amber_ink)
                    .child(tr(cx, "Needs you", "待确认"))
                    .into_any_element(),
                FleetStatus::Failed => div()
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.red_ink)
                    .child(tr(cx, "Failed", "失败"))
                    .into_any_element(),
                FleetStatus::Queued | FleetStatus::Stopped => div()
                    .text_size(px(11.))
                    .text_color(p.faint)
                    .child(tr(cx, "Queued", "排队中"))
                    .into_any_element(),
            };

            let title_col = if st.status.is_muted() {
                p.faint
            } else if matches!(
                st.status,
                FleetStatus::Running | FleetStatus::Waiting | FleetStatus::Failed
            ) {
                p.strong
            } else {
                p.text
            };

            list = list.child(
                div()
                    .w_full()
                    .py(px(2.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(pro_check_draw(
                                pro_eid(root_id, format!("st-chk-{sid}-{idx}")),
                                st.status == FleetStatus::Done,
                                13.,
                                false,
                                p.green,
                            ))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(12.))
                                    .text_color(title_col)
                                    .truncate()
                                    .child(st.title.clone()),
                            ),
                    )
                    .child(right_el),
            );
        }
        box_el = box_el.child(list);
    }

    box_el.into_any_element()
}

fn render_status_pill(
    root_id: &ElementId,
    agent_id: &str,
    status: FleetStatus,
    p: &ProPalette,
    cx: &App,
) -> Div {
    let (bg, fg) = match status {
        FleetStatus::Running => (p.accent_soft, p.blue),
        FleetStatus::Waiting => (p.amber_soft, p.amber_ink),
        FleetStatus::Done => (p.green_soft, p.green_ink),
        FleetStatus::Failed => (p.red_soft, p.red_ink),
        FleetStatus::Queued | FleetStatus::Stopped => (p.chip, p.muted),
    };
    div()
        .h(px(18.))
        .px(px(6.))
        .flex_none()
        .rounded(px(4.))
        .bg(bg)
        .text_color(fg)
        .text_size(px(11.))
        .font_weight(FontWeight::MEDIUM)
        .flex()
        .items_center()
        .gap(px(4.))
        .when(status == FleetStatus::Running, |d| {
            d.child(pro_svg("0 0 24 24", SPIN_ARC_SVG, 11.5, fg))
        })
        .when(status == FleetStatus::Done, |d| {
            d.child(pro_check_draw(
                pro_eid(root_id, format!("f-done-{agent_id}")),
                true,
                12.,
                false,
                p.green,
            ))
        })
        .when(status == FleetStatus::Waiting, |d| {
            d.child(pro_icon(ProIcon::Clock, 11., fg))
        })
        .when(status == FleetStatus::Failed, |d| {
            d.child(pro_icon(ProIcon::AlertTriangle, 11., fg))
        })
        .child(pro_swap_label(
            pro_eid(root_id, format!("f-stlbl-{agent_id}")),
            status.label(cx),
            11.,
            FontWeight::MEDIUM,
            fg,
        ))
}

fn render_pie_ring(id: ElementId, ratio: f32, color: Hsla) -> AnyElement {
    let r_target = ratio.clamp(0.0, 1.0);
    canvas(
        |_, _, _| (),
        move |bounds, _, window, cx| {
            let r_clamped = pro_spring_f32(id.clone(), r_target, 0.32, window, cx).clamp(0.0, 1.0);
            let cx_pt = (bounds.left() + bounds.right()) * 0.5;
            let cy_pt = (bounds.top() + bounds.bottom()) * 0.5;
            let x0 = f32::from(cx_pt);
            let y0 = f32::from(cy_pt);

            // Outer ring (r = 4.5)
            let mut ring = PathBuilder::stroke(px(1.35));
            for i in 0..=24 {
                let ang = (i as f32 / 24.0) * std::f32::consts::TAU;
                let pt = point(px(x0 + 4.4 * ang.cos()), px(y0 + 4.4 * ang.sin()));
                if i == 0 {
                    ring.move_to(pt);
                } else {
                    ring.line_to(pt);
                }
            }
            if let Ok(path) = ring.build() {
                window.paint_path(path, color.opacity(0.85));
            }

            // Inner pie wedge (r = 2.7) starting at -90 deg
            if r_clamped > 0.02 {
                let mut wedge = PathBuilder::fill();
                let steps = ((r_clamped * 20.0).ceil() as usize).max(2);
                wedge.move_to(point(px(x0), px(y0)));
                for i in 0..=steps {
                    let frac = (i as f32 / steps as f32) * r_clamped;
                    let ang = -std::f32::consts::FRAC_PI_2 + frac * std::f32::consts::TAU;
                    wedge.line_to(point(px(x0 + 2.7 * ang.cos()), px(y0 + 2.7 * ang.sin())));
                }
                wedge.close();
                if let Ok(path) = wedge.build() {
                    window.paint_path(path, color);
                }
            }
        },
    )
    .size(px(11.))
    .flex_none()
    .into_any_element()
}

fn summarize_sub_agents(lead: &FleetAgent, p: &ProPalette, cx: &App) -> (f32, SharedString, Hsla) {
    let subs_total = count_fleet_all(lead).saturating_sub(1);
    if subs_total == 0 {
        return (
            if lead.status.is_terminal() { 1.0 } else { 0.35 },
            tr(cx, "Solo agent", "独立运行").into(),
            p.muted,
        );
    }
    let finished = count_descendants_by(lead, |n| n.status.is_terminal());
    let waiting = count_descendants_by(lead, |n| n.status == FleetStatus::Waiting);
    let failed = count_descendants_by(lead, |n| n.status == FleetStatus::Failed);
    let running = count_descendants_by(lead, |n| n.status == FleetStatus::Running);
    let ratio = (finished as f32) / (subs_total as f32);

    if waiting > 0 {
        (
            ratio,
            format!("{waiting} {}", tr(cx, "needs you", "待确认")).into(),
            p.amber_ink,
        )
    } else if failed > 0 {
        (
            ratio,
            format!("{failed} {}", tr(cx, "failed", "已失败")).into(),
            p.red_ink,
        )
    } else if running > 0 {
        (
            ratio,
            format!("{running}/{subs_total} {}", tr(cx, "running", "运行中")).into(),
            p.muted,
        )
    } else {
        (
            1.0,
            format!(
                "{} {subs_total} {}",
                tr(cx, "All", "全部"),
                tr(cx, "done", "已完成")
            )
            .into(),
            p.muted,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_sub_elbow(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    radius: f32,
    color: Hsla,
    stroke_w: f32,
    window: &mut Window,
) {
    let dy = y1 - y0;
    let dx = x1 - x0;
    let r = radius.min(dy.abs() * 0.5).min(dx.abs() * 0.5).max(0.0);
    let mut pb = PathBuilder::stroke(px(stroke_w));
    pb.move_to(point(px(x0), px(y0)));
    if r < 1.0 {
        pb.line_to(point(px(x0), px(y1)));
        pb.line_to(point(px(x1), px(y1)));
    } else {
        let sy = dy.signum();
        let sx = dx.signum();
        let b = (x0, y1 - sy * r);
        let c = (x0, y1);
        let d = (x0 + sx * r, y1);
        pb.line_to(point(px(b.0), px(b.1)));
        for k in 1..=6 {
            let u = k as f32 / 6.0;
            let inv = 1.0 - u;
            pb.line_to(point(
                px(inv * inv * b.0 + 2.0 * inv * u * c.0 + u * u * d.0),
                px(inv * inv * b.1 + 2.0 * inv * u * c.1 + u * u * d.1),
            ));
        }
        pb.line_to(point(px(x1), px(y1)));
    }
    if let Ok(path) = pb.build() {
        window.paint_path(path, color);
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

fn branch_cost(node: &FleetAgent) -> f64 {
    let own = if node.cost.is_finite() && node.cost >= 0.0 {
        node.cost
    } else {
        0.0
    };
    own + node.children.iter().map(branch_cost).sum::<f64>()
}

fn count_fleet_all(node: &FleetAgent) -> usize {
    1 + node.children.iter().map(count_fleet_all).sum::<usize>()
}

fn count_fleet_by(node: &FleetAgent, pred: impl Fn(&FleetAgent) -> bool + Copy) -> usize {
    usize::from(pred(node))
        + node
            .children
            .iter()
            .map(|c| count_fleet_by(c, pred))
            .sum::<usize>()
}

fn count_descendants_by(node: &FleetAgent, pred: impl Fn(&FleetAgent) -> bool + Copy) -> usize {
    node.children.iter().map(|c| count_fleet_by(c, pred)).sum()
}

fn find_fleet_mut<'a>(node: &'a mut FleetAgent, id: &SharedString) -> Option<&'a mut FleetAgent> {
    if &node.id == id {
        return Some(node);
    }
    for ch in &mut node.children {
        if let Some(found) = find_fleet_mut(ch, id) {
            return Some(found);
        }
    }
    None
}

fn fleet_avatar_seed<'a>(model: &'a str, name: &'a str, id: &'a str) -> &'static str {
    let m = model.to_ascii_lowercase();
    if m.contains("claude") || m.contains("opus") || m.contains("sonnet") || m.contains("haiku") {
        return "claude";
    }
    if m.contains("gpt") || m.contains("openai") || m.contains("codex") {
        return "gpt";
    }
    if m.contains("gemini") {
        return "gemini";
    }
    if m.contains("deepseek") {
        return "deepseek";
    }
    if m.contains("mistral") {
        return "mistral";
    }
    if m.contains("qwen") {
        return "qwen";
    }
    let combined = format!("{name} {id}").to_ascii_lowercase();
    if combined.contains("claude")
        || combined.contains("writer")
        || combined.contains("research")
        || combined.contains("billing")
        || combined.contains("notes")
        || combined.contains("change")
        || combined.contains("launch")
        || combined.contains("support")
    {
        "claude"
    } else if combined.contains("gemini")
        || combined.contains("qa")
        || combined.contains("triage")
        || combined.contains("router")
        || combined.contains("analyst")
        || combined.contains("price")
        || combined.contains("deck")
    {
        "gemini"
    } else if combined.contains("deepseek") {
        "deepseek"
    } else if combined.contains("mistral") {
        "mistral"
    } else if combined.contains("qwen") {
        "qwen"
    } else {
        "gpt"
    }
}

fn format_duration_secs(secs: u64) -> String {
    let mins = secs / 60;
    let rem = secs % 60;
    if mins >= 60 {
        let hrs = mins / 60;
        let m = mins % 60;
        format!("{hrs}h {m:02}m")
    } else {
        format!("{mins}m {rem:02}s")
    }
}

fn tr_fleet_name<'a>(id: &str, fallback: &'a str, cx: &App) -> &'a str {
    match id {
        "launch" => tr(cx, "Launch lead", "发布负责人"),
        "support" => tr(cx, "Support lead", "工单负责人"),
        "release" => tr(cx, "Release manager", "发版管理员"),
        "analyst" => tr(cx, "Market analyst", "市场分析师"),
        "l-research" => tr(cx, "Researcher", "竞品调研员"),
        "l-writer" => tr(cx, "Writer", "文案撰写员"),
        "l-coder" => tr(cx, "Coder", "前端编码员"),
        "l-qa" => tr(cx, "QA verifier", "视觉验收员"),
        "s-triage" => tr(cx, "Triage bot", "工单分拣器"),
        "s-billing" => tr(cx, "Billing responder", "账单回复员"),
        "s-repro" => tr(cx, "Bug reproducer", "故障复现员"),
        "s-router" => tr(cx, "Escalation router", "升级路由员"),
        "r-notes" => tr(cx, "Changelog writer", "变更日志撰写"),
        "r-ios" => tr(cx, "iOS build runner", "iOS 构建器"),
        "r-tf" => tr(cx, "TestFlight uploader", "TestFlight 上传"),
        "m-prices" => tr(cx, "Pricing tracker", "价格追踪员"),
        "m-changes" => tr(cx, "Changelog reader", "动态抓取员"),
        "m-deck" => tr(cx, "Deck synthesizer", "周报合成器"),
        _ => fallback,
    }
}

fn tr_fleet_task<'a>(en: &'a str, cx: &App) -> &'a str {
    match en {
        "Launch the new pricing page by Friday" => tr(
            cx,
            "Launch the new pricing page by Friday",
            "周五前上线新版定价页面",
        ),
        "Answer the 48 open tickets in the queue" => tr(
            cx,
            "Answer the 48 open tickets in the queue",
            "回复队列中 48 张待处理工单",
        ),
        "Ship version 2.4 to the App Store" => tr(
            cx,
            "Ship version 2.4 to the App Store",
            "将 2.4 版本提交至 App Store",
        ),
        "Write the weekly competitor report" => tr(
            cx,
            "Write the weekly competitor report",
            "撰写本周竞品动态分析报告",
        ),
        "Compare five competitors" => tr(cx, "Compare five competitors", "对比五家竞品定价方案"),
        "Write the plan copy" => tr(cx, "Write the plan copy", "撰写套餐卡片与主标题文案"),
        "Build billing toggle & tiers" => tr(
            cx,
            "Build billing toggle & tiers",
            "构建计费周期切换与档位表",
        ),
        "Verify mobile Safari & dark mode" => tr(
            cx,
            "Verify mobile Safari & dark mode",
            "验收移动端与深色模式表现",
        ),
        _ => en,
    }
}

fn default_fleet_agents() -> Vec<FleetAgent> {
    vec![
        FleetAgent::new(
            "launch",
            "Launch lead",
            "Launch the new pricing page by Friday",
            FleetStatus::Running,
        )
        .with_model("Opus 5.5")
        .with_tools(["Figma", "Slack", "GitHub"])
        .with_usage(0.42, 843)
        .with_children(vec![
            FleetAgent::new(
                "l-research",
                "Researcher",
                "Compare five competitors",
                FleetStatus::Done,
            )
            .with_usage(0.31, 420)
            .with_steps(vec![
                FleetStep::done("Scrape competitor tier tables", Some("Globe"), "2m 10s"),
                FleetStep::done("Normalize annual billing discounts", None, "1m 55s"),
                FleetStep::done("Publish matrix to shared doc", Some("Notion"), "2m 55s"),
            ]),
            FleetAgent::new(
                "l-writer",
                "Writer",
                "Write the plan copy",
                FleetStatus::Waiting,
            )
            .with_usage(0.42, 723)
            .with_ask(
                "Approve hero headline \"Simple pricing that scales with every team\" before staging publish?",
            )
            .with_steps(vec![
                FleetStep::done("Draft Pro & Enterprise card bullets", None, "3m 40s"),
                FleetStep::done("Check Kense tone & terminology guide", None, "1m 20s"),
                FleetStep::with_status("Await headline sign-off", FleetStatus::Waiting, None),
            ]),
            FleetAgent::new(
                "l-coder",
                "Coder",
                "Build billing toggle & tiers",
                FleetStatus::Running,
            )
            .with_usage(0.48, 680)
            .with_steps(vec![
                FleetStep::done("Scaffold responsive tier grid", Some("GitHub"), "4m 12s"),
                FleetStep::with_status("Wire live Stripe price IDs", FleetStatus::Running, Some("Stripe")),
                FleetStep::with_status("Run visual regression suite", FleetStatus::Queued, None),
            ])
            .with_children(vec![
                FleetAgent::new(
                    "l-qa",
                    "QA verifier",
                    "Verify mobile Safari & dark mode",
                    FleetStatus::Running,
                )
                .with_usage(0.12, 255)
                .with_steps(vec![
                    FleetStep::done("Desktop 1440px contrast audit", None, "1m 48s"),
                    FleetStep::with_status("iPhone 390px viewport sweep", FleetStatus::Running, None),
                ]),
            ]),
        ]),
        FleetAgent::new(
            "support",
            "Support lead",
            "Answer the 48 open tickets in the queue",
            FleetStatus::Running,
        )
        .with_model("Sonnet 5.5")
        .with_tools(["Slack", "Notion"])
        .with_usage(0.12, 1_563)
        .with_children(vec![
            FleetAgent::new(
                "s-triage",
                "Triage bot",
                "Cluster tickets by topic & SLA",
                FleetStatus::Done,
            )
            .with_usage(0.18, 310)
            .with_steps(vec![
                FleetStep::done("Classify 48 open threads", None, "2m 05s"),
                FleetStep::done("Tag enterprise priority accounts", None, "1m 05s"),
            ]),
            FleetAgent::new(
                "s-billing",
                "Billing responder",
                "Draft invoice & refund replies",
                FleetStatus::Running,
            )
            .with_usage(0.29, 640)
            .with_steps(vec![
                FleetStep::done("Verify Stripe invoice webhooks", Some("Stripe"), "3m 10s"),
                FleetStep::with_status("Drafting 14 customer replies", FleetStatus::Running, None),
            ]),
            FleetAgent::new(
                "s-repro",
                "Bug reproducer",
                "Reproduce SSO callback timeout",
                FleetStatus::Running,
            )
            .with_usage(0.24, 590)
            .with_steps(vec![
                FleetStep::with_status("Replay SAML assertion trace", FleetStatus::Running, None),
            ]),
            FleetAgent::new(
                "s-router",
                "Escalation router",
                "Notify on-call engineer for P1s",
                FleetStatus::Queued,
            )
            .with_usage(0.04, 0),
        ]),
        FleetAgent::new(
            "release",
            "Release manager",
            "Ship version 2.4 to the App Store",
            FleetStatus::Running,
        )
        .with_model("GPT-6 Sol")
        .with_tools(["GitHub", "Linear"])
        .with_usage(0.18, 2_283)
        .with_children(vec![
            FleetAgent::new(
                "r-notes",
                "Changelog writer",
                "Compile 34 merged PRs into notes",
                FleetStatus::Done,
            )
            .with_usage(0.14, 380)
            .with_steps(vec![FleetStep::done("Summarize customer-facing fixes", None, "3m 50s")]),
            FleetAgent::new(
                "r-ios",
                "iOS build runner",
                "Archive & sign release ipa",
                FleetStatus::Failed,
            )
            .with_usage(0.22, 810)
            .with_error("Provisioning profile expired for com.kense.app.release (exit code 65).")
            .with_steps(vec![
                FleetStep::done("Compile Release arm64 targets", None, "9m 40s"),
                FleetStep::with_status("CodeSign release archive", FleetStatus::Failed, None),
            ]),
            FleetAgent::new(
                "r-tf",
                "TestFlight uploader",
                "Upload signed build to App Store Connect",
                FleetStatus::Stopped,
            )
            .with_usage(0.05, 45),
        ]),
        FleetAgent::new(
            "analyst",
            "Market analyst",
            "Write the weekly competitor report",
            FleetStatus::Done,
        )
        .with_model("Gemini 3.8 Pro")
        .with_tools(["Google Drive", "Stripe"])
        .with_usage(0.15, 2_520)
        .with_children(vec![
            FleetAgent::new(
                "m-prices",
                "Pricing tracker",
                "Diff public pricing changes",
                FleetStatus::Done,
            )
            .with_usage(0.24, 780)
            .with_steps(vec![FleetStep::done("Compare 12 tracked SKUs", None, "6m 20s")]),
            FleetAgent::new(
                "m-changes",
                "Changelog reader",
                "Summarize shipped competitor features",
                FleetStatus::Done,
            )
            .with_usage(0.21, 840)
            .with_steps(vec![FleetStep::done("Extract 9 product launches", None, "7m 10s")]),
            FleetAgent::new(
                "m-deck",
                "Deck synthesizer",
                "Publish executive summary brief",
                FleetStatus::Done,
            )
            .with_usage(0.22, 900)
            .with_steps(vec![FleetStep::done("Share weekly digest to Slack", None, "4m 05s")]),
        ]),
    ]
}

#[cfg(test)]
mod tests {
    use gpui::AppContext;

    use super::{
        AgentFleet, FleetResolveAction, FleetStatus, branch_cost, count_fleet_all,
        default_fleet_agents,
    };
    use gpui::TestAppContext;

    #[test]
    fn default_fleet_totals_four_leads_fourteen_subs_and_exact_rollup_cost() {
        let agents = default_fleet_agents();
        assert_eq!(agents.len(), 4);
        let subs: usize = agents.iter().map(|a| count_fleet_all(a) - 1).sum();
        assert_eq!(subs, 14);
        assert!((branch_cost(&agents[0]) - 1.75).abs() < 1e-6);
        assert!((branch_cost(&agents[1]) - 0.87).abs() < 1e-6);
        assert!((branch_cost(&agents[2]) - 0.59).abs() < 1e-6);
        assert!((branch_cost(&agents[3]) - 0.82).abs() < 1e-6);
    }

    #[test]
    fn toggling_leads_and_resolving_sub_agents_update_state_cleanly() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let fleet = app.new(|cx| AgentFleet::new("fleet-test", cx));
        fleet.update(&mut app, |f, cx| {
            assert_eq!(f.open_id().map(|s| s.as_ref()), Some("launch"));
            assert!((f.total_fleet_cost() - 4.03).abs() < 0.02);

            // Folding open lead sets open_id to None
            f.toggle_lead("launch".into(), cx);
            assert!(f.open_id().is_none());

            // Opening release lead switches open_id
            f.toggle_lead("release".into(), cx);
            assert_eq!(f.open_id().map(|s| s.as_ref()), Some("release"));

            // Resolving waiting writer updates cost and status
            f.resolve_sub_agent("l-writer", FleetResolveAction::Approve, cx);
            let writer = &f.agents[0].children[1];
            assert_eq!(writer.status, FleetStatus::Done);
            assert!(writer.ask.is_none());
        });
    }
}
