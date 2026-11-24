//! Task delegation board with drag-and-drop hand-off, `"Busy until …"` timeline previews, and `Auto` routing.
//!
//! Lets people hand unassigned tasks to specialist AI agents by dragging cards onto them (or via
//! click / `1`–`4` shortcuts), previewing each agent's updated `"Busy until …"` time, timeline bar,
//! and incremental USD cost, or flip `"Auto"` ([`pro_switch`]) so unassigned work routes automatically
//! to whichever agent frees up earliest.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_check_draw, pro_eid,
        pro_empty_state, pro_mix_color, pro_popover_presence, pro_rolling_number, pro_skeleton_bar,
        pro_spring_f32, pro_swap_label, pro_switch, pro_tactile, tr,
    },
    foundation::Theme,
    primitives::{
        family::disabled_motion,
        pro_icons::{ProIcon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::f32::consts::TAU;
use web_time::Instant;

const SPARK_SVG: &str = r#"<path d="M7 1.2c.4 2.9 1.9 4.4 4.8 4.8-2.9.4-4.4 1.9-4.8 4.8C6.6 7.9 5.1 6.4 2.2 6 5.1 5.6 6.6 4.1 7 1.2Z" fill="currentColor"></path><circle cx="11.6" cy="11.4" r="1.1" fill="currentColor" opacity="0.55"></circle>"#;

const GRIP_DOTS_SVG: &str = r#"<circle cx="4.3" cy="3" r="0.95" fill="currentColor"></circle><circle cx="7.7" cy="3" r="0.95" fill="currentColor"></circle><circle cx="4.3" cy="6" r="0.95" fill="currentColor"></circle><circle cx="7.7" cy="6" r="0.95" fill="currentColor"></circle><circle cx="4.3" cy="9" r="0.95" fill="currentColor"></circle><circle cx="7.7" cy="9" r="0.95" fill="currentColor"></circle>"#;

const BASE_HOUR: u32 = 7;
const BASE_MIN: u32 = 57;

/// Events emitted by [`DelegateBoard`].
#[derive(Clone, Debug, PartialEq)]
pub enum DelegateBoardEvent {
    /// Task `task_id` was assigned to `agent_id` (`None` when returned to `Unassigned`).
    Assigned {
        task_id: SharedString,
        agent_id: Option<SharedString>,
    },
    /// User toggled between `Manual` and `Auto` delegation modes.
    ModeChanged(DelegateMode),
    /// A new unassigned task was added to the pile.
    TaskAdded(DelegateTask),
    /// Board state was reset to its initial snapshot.
    Reset,
}

/// Routing mode controlled by the header `Auto` switch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DelegateMode {
    /// Wait for manual drag, click, or keyboard assignment.
    #[default]
    Manual,
    /// Automatically route unassigned tasks one by one to whichever agent frees up earliest.
    Auto,
}

/// Specialist AI agent row on the right side of [`DelegateBoard`].
#[derive(Clone, Debug, PartialEq)]
pub struct DelegateAgent {
    /// Stable agent id referenced by [`DelegateTask::agent_id`].
    pub id: SharedString,
    /// Display name (`"Researcher"`, `"Coder"`, `"Ops"`).
    pub name: SharedString,
    /// Optional Chinese display name.
    pub name_zh: Option<SharedString>,
    /// Model subtitle (`"Opus 5.5"`, `"GPT-6 Sol"`).
    pub model: Option<SharedString>,
    /// Provider logo key (`"claude"`, `"openai"`, `"gemini"`).
    pub provider: Option<SharedString>,
    /// Remaining minutes on the currently running task (`None` uses the first assigned task's estimate).
    pub current_minutes: Option<u32>,
    /// Cost rate in USD per minute of work (`0.06` -> `"$0.06/min"`).
    pub rate_per_min: Option<f64>,
}

impl DelegateAgent {
    /// Create a [`DelegateAgent`] with `id` and `name`.
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            name_zh: None,
            model: None,
            provider: None,
            current_minutes: None,
            rate_per_min: None,
        }
    }

    /// Set localized Chinese name.
    pub fn with_name_zh(mut self, name_zh: impl Into<SharedString>) -> Self {
        self.name_zh = Some(name_zh.into());
        self
    }

    /// Set model name (`"GPT-6 Sol"`).
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set provider mark key (`"claude"`, `"openai"`, `"gemini"`).
    pub fn with_provider(mut self, provider: impl Into<SharedString>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    /// Override remaining minutes on the currently running task.
    pub fn with_current_minutes(mut self, minutes: u32) -> Self {
        self.current_minutes = Some(minutes);
        self
    }

    /// Set USD cost per minute (`0.08` -> `"$0.08/min"`).
    pub fn with_rate(mut self, rate_per_min: f64) -> Self {
        self.rate_per_min = Some(rate_per_min.max(0.0));
        self
    }

    fn display_name<'a>(&'a self, cx: &App) -> &'a str {
        match &self.name_zh {
            Some(zh) => tr(cx, self.name.as_ref(), zh.as_ref()),
            None => self.name.as_ref(),
        }
    }
}

/// One delegatable work item in [`DelegateBoard`].
#[derive(Clone, Debug, PartialEq)]
pub struct DelegateTask {
    /// Stable task identifier.
    pub id: SharedString,
    /// Task title (`"Fix flaky checkout test"`).
    pub title: SharedString,
    /// Optional Chinese task title.
    pub title_zh: Option<SharedString>,
    /// Estimated execution duration in minutes.
    pub estimate_minutes: u32,
    /// Tool logo keys (`"github"`, `"slack"`, `"notion"`, `"figma"`, `"stripe"`, `"hubspot"`).
    pub tools: Vec<SharedString>,
    /// Assigned agent id (`None` places the task in `Unassigned`).
    pub agent_id: Option<SharedString>,
}

impl DelegateTask {
    /// Create an unassigned [`DelegateTask`] with `id`, `title`, and `estimate_minutes`.
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        estimate_minutes: u32,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            title_zh: None,
            estimate_minutes: estimate_minutes.max(1),
            tools: Vec::new(),
            agent_id: None,
        }
    }

    /// Set localized Chinese task title.
    pub fn with_title_zh(mut self, title_zh: impl Into<SharedString>) -> Self {
        self.title_zh = Some(title_zh.into());
        self
    }

    /// Attach required tool/app keys (`"github"`, `"slack"`, `"stripe"`).
    pub fn with_tools(mut self, tools: Vec<SharedString>) -> Self {
        self.tools = tools;
        self
    }

    /// Pre-assign this task to `agent_id`.
    pub fn with_agent(mut self, agent_id: impl Into<SharedString>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    fn display_title<'a>(&'a self, cx: &App) -> &'a str {
        match &self.title_zh {
            Some(zh) => tr(cx, self.title.as_ref(), zh.as_ref()),
            None => self.title.as_ref(),
        }
    }
}

/// Proposed hand-off preview highlighting a task and target agent before assignment.
#[derive(Clone, Debug, PartialEq)]
pub struct DelegateSuggestion {
    pub task_id: SharedString,
    pub agent_id: SharedString,
}

/// Compute total busy minutes queued on `agent` across `tasks`.
pub fn agent_busy_minutes(agent: &DelegateAgent, tasks: &[DelegateTask]) -> u32 {
    let assigned: Vec<&DelegateTask> = tasks
        .iter()
        .filter(|t| t.agent_id.as_ref() == Some(&agent.id))
        .collect();
    if assigned.is_empty() {
        return agent.current_minutes.unwrap_or(0);
    }
    let first_mins = agent
        .current_minutes
        .unwrap_or(assigned[0].estimate_minutes);
    let rest_mins: u32 = assigned.iter().skip(1).map(|t| t.estimate_minutes).sum();
    first_mins + rest_mins
}

/// Return the id of whichever [`DelegateAgent`] frees up earliest (minimum [`agent_busy_minutes`]).
pub fn earliest_free_agent_id(
    agents: &[DelegateAgent],
    tasks: &[DelegateTask],
) -> Option<SharedString> {
    agents
        .iter()
        .min_by_key(|a| agent_busy_minutes(a, tasks))
        .map(|a| a.id.clone())
}

/// Format `base_hour:base_min` plus `added_minutes` into `HH:MM` 24-hour clock notation.
pub fn format_busy_clock(base_hour: u32, base_min: u32, added_minutes: u32) -> String {
    let total = base_hour * 60 + base_min + added_minutes;
    let hh = (total / 60) % 24;
    let mm = total % 60;
    format!("{hh:02}:{mm:02}")
}

#[derive(Clone)]
struct TaskDragPayload {
    board_id: ElementId,
    task_id: SharedString,
    title: SharedString,
    estimate_minutes: u32,
}

struct TaskDragGhost {
    title: SharedString,
    estimate_minutes: u32,
}

impl Render for TaskDragGhost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        div()
            .w(px(210.0))
            .px(px(10.0))
            .py(px(7.5))
            .rounded(px(8.0))
            .bg(p.card)
            .border_1()
            .border_color(p.accent_info)
            .shadow_lg()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.0))
            .child(
                div()
                    .truncate()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(self.title.clone()),
            )
            .child(
                div()
                    .flex_none()
                    .text_size(px(11.5))
                    .text_color(p.accent_info)
                    .child(format!("~{}m", self.estimate_minutes)),
            )
    }
}

/// Task delegation board with drag-and-drop hand-off, `"Busy until …"` previews, and `Auto` routing.
pub struct DelegateBoard {
    id: ElementId,
    title: SharedString,
    title_zh: Option<SharedString>,
    agents: Vec<DelegateAgent>,
    tasks: Vec<DelegateTask>,
    initial_tasks: Vec<DelegateTask>,
    mode: DelegateMode,
    suggestion: Option<DelegateSuggestion>,
    picked_task_id: Option<SharedString>,
    hovered_agent_idx: Option<usize>,
    drag_preview: Option<(SharedString, SharedString)>,
    hovered_overflow_agent: Option<SharedString>,
    last_auto_step: Instant,
    task_seq: usize,
    loading: bool,
    empty: bool,
    started_at: Instant,
}

impl EventEmitter<DelegateBoardEvent> for DelegateBoard {}

impl DelegateBoard {
    /// Create a [`DelegateBoard`] preloaded with 4 unassigned tasks, 4 specialist agents, and a live preview on `Coder`.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let tasks = default_tasks();
        Self {
            id: id.into(),
            title: SharedString::from("Delegate"),
            title_zh: Some(SharedString::from("任务委派看板")),
            agents: default_agents(),
            initial_tasks: tasks.clone(),
            tasks,
            mode: DelegateMode::Manual,
            suggestion: Some(DelegateSuggestion {
                task_id: SharedString::from("t-flaky"),
                agent_id: SharedString::from("coder"),
            }),
            picked_task_id: Some(SharedString::from("t-flaky")),
            hovered_agent_idx: Some(2),
            drag_preview: None,
            hovered_overflow_agent: None,
            last_auto_step: Instant::now(),
            task_seq: 10,
            loading: false,
            empty: false,
            started_at: Instant::now(),
        }
    }

    /// Set header title (`"Delegate"` default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self.title_zh = None;
        self
    }

    /// Set localized Chinese header title.
    pub fn with_title_zh(mut self, title_zh: impl Into<SharedString>) -> Self {
        self.title_zh = Some(title_zh.into());
        self
    }

    /// Supply specialist [`DelegateAgent`] rows.
    pub fn with_agents(mut self, agents: Vec<DelegateAgent>) -> Self {
        self.agents = agents;
        self
    }

    /// Supply unassigned and assigned [`DelegateTask`] items.
    pub fn with_tasks(mut self, tasks: Vec<DelegateTask>) -> Self {
        self.initial_tasks = tasks.clone();
        self.tasks = tasks;
        self.suggestion = None;
        self.picked_task_id = None;
        self
    }

    /// Set initial [`DelegateMode`] (`Manual` or `Auto`).
    pub fn with_mode(mut self, mode: DelegateMode) -> Self {
        self.mode = mode;
        self.last_auto_step = Instant::now();
        self
    }

    /// Highlight a proposed task-to-agent routing suggestion.
    pub fn with_suggestion(mut self, suggestion: Option<DelegateSuggestion>) -> Self {
        self.suggestion = suggestion;
        self
    }

    /// Show skeleton placeholder state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Show empty state when no agents are configured.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return current [`DelegateMode`].
    pub fn mode(&self) -> DelegateMode {
        self.mode
    }

    /// Return all tasks on the board.
    pub fn tasks(&self) -> &[DelegateTask] {
        &self.tasks
    }

    /// Return count of unassigned tasks.
    pub fn unassigned_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.agent_id.is_none()).count()
    }

    /// Switch between `Manual` and `Auto` delegation modes.
    pub fn set_mode(&mut self, mode: DelegateMode, cx: &mut Context<Self>) {
        if self.mode != mode {
            self.mode = mode;
            self.last_auto_step = Instant::now();
            if mode == DelegateMode::Auto {
                self.picked_task_id = None;
                self.suggestion = None;
                // Under reduced_motion, route all unassigned tasks immediately without waiting on animation frames
                if disabled_motion(cx) || Theme::global(cx).reduced_motion {
                    while self.route_next_auto(cx) {}
                }
            }
            cx.emit(DelegateBoardEvent::ModeChanged(mode));
            cx.notify();
        }
    }

    /// Assign `task_id` to `agent_id` (`None` returns the task to `Unassigned`).
    pub fn assign_task(
        &mut self,
        task_id: &str,
        agent_id: Option<&str>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(pos) = self.tasks.iter().position(|t| t.id.as_ref() == task_id) else {
            return false;
        };
        let next_owner = agent_id.map(|s| SharedString::from(s.to_string()));
        if self.tasks[pos].agent_id == next_owner {
            return false;
        }
        // Move modified task to the end of its new lane so queue order stays chronological
        let mut task = self.tasks.remove(pos);
        task.agent_id = next_owner.clone();
        let tid = task.id.clone();
        self.tasks.push(task);

        if self.picked_task_id.as_deref() == Some(task_id) {
            self.picked_task_id = self
                .tasks
                .iter()
                .find(|t| t.agent_id.is_none())
                .map(|t| t.id.clone());
        }
        if self
            .suggestion
            .as_ref()
            .is_some_and(|s| s.task_id.as_ref() == task_id)
        {
            self.suggestion = None;
        }
        self.drag_preview = None;

        cx.emit(DelegateBoardEvent::Assigned {
            task_id: tid,
            agent_id: next_owner,
        });
        cx.notify();
        true
    }

    /// Route the next unassigned task to whichever agent frees up earliest.
    pub fn route_next_auto(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(next_task) = self.tasks.iter().find(|t| t.agent_id.is_none()).cloned() else {
            return false;
        };
        let Some(best_agent) = earliest_free_agent_id(&self.agents, &self.tasks) else {
            return false;
        };
        self.assign_task(next_task.id.as_ref(), Some(best_agent.as_ref()), cx)
    }

    /// Add a fresh unassigned task to the pile so users can continue delegating.
    pub fn add_sample_task(&mut self, cx: &mut Context<Self>) {
        self.task_seq += 1;
        let n = self.task_seq;
        let task = match n % 3 {
            0 => DelegateTask::new(format!("t-auto-{n}"), "Audit webhook retry backoff", 14)
                .with_title_zh("审查 Webhook 退避重试逻辑")
                .with_tools(vec![
                    SharedString::from("stripe"),
                    SharedString::from("github"),
                ]),
            1 => DelegateTask::new(format!("t-auto-{n}"), "Draft Q4 customer digest", 12)
                .with_title_zh("撰写 Q4 客户进展双周报")
                .with_tools(vec![SharedString::from("notion")]),
            _ => DelegateTask::new(format!("t-auto-{n}"), "Verify staging CSP headers", 9)
                .with_title_zh("核验预发环境 CSP 安全响应头")
                .with_tools(vec![SharedString::from("github")]),
        };
        self.tasks.push(task.clone());
        if self.mode == DelegateMode::Auto
            && (disabled_motion(cx) || Theme::global(cx).reduced_motion)
        {
            self.route_next_auto(cx);
        }
        cx.emit(DelegateBoardEvent::TaskAdded(task));
        cx.notify();
    }

    /// Reset all tasks back to the initial unassigned/queued layout.
    pub fn reset_board(&mut self, cx: &mut Context<Self>) {
        self.tasks = self.initial_tasks.clone();
        self.mode = DelegateMode::Manual;
        self.picked_task_id = Some(SharedString::from("t-flaky"));
        self.hovered_agent_idx = Some(2);
        self.suggestion = Some(DelegateSuggestion {
            task_id: SharedString::from("t-flaky"),
            agent_id: SharedString::from("coder"),
        });
        self.drag_preview = None;
        cx.emit(DelegateBoardEvent::Reset);
        cx.notify();
    }

    fn total_queued_spend(&self) -> f64 {
        let mut sum = 0.0;
        for ag in &self.agents {
            let mins = agent_busy_minutes(ag, &self.tasks) as f64;
            let rate = ag.rate_per_min.unwrap_or(0.0);
            sum += mins * rate;
        }
        sum
    }

    fn active_preview_pair(&self) -> Option<(DelegateTask, SharedString)> {
        if let Some((tid, aid)) = &self.drag_preview
            && let Some(t) = self.tasks.iter().find(|t| &t.id == tid).cloned()
            && t.agent_id.as_ref() != Some(aid)
        {
            return Some((t, aid.clone()));
        }
        if self.mode == DelegateMode::Auto {
            let next_t = self.tasks.iter().find(|t| t.agent_id.is_none()).cloned()?;
            let best_a = earliest_free_agent_id(&self.agents, &self.tasks)?;
            return Some((next_t, best_a));
        }
        if let Some(picked_id) = &self.picked_task_id
            && let Some(t) = self.tasks.iter().find(|t| &t.id == picked_id).cloned()
            && let Some(idx) = self.hovered_agent_idx
            && let Some(ag) = self.agents.get(idx)
            && t.agent_id.as_ref() != Some(&ag.id)
        {
            return Some((t, ag.id.clone()));
        }
        if let Some(sug) = &self.suggestion
            && let Some(t) = self.tasks.iter().find(|t| t.id == sug.task_id).cloned()
            && t.agent_id.is_none()
        {
            return Some((t, sug.agent_id.clone()));
        }
        None
    }
}

impl Render for DelegateBoard {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx) || Theme::global(cx).reduced_motion;
        let elapsed_secs = if reduced {
            0.0
        } else {
            self.started_at.elapsed().as_secs_f32()
        };

        let has_unassigned = self.tasks.iter().any(|t| t.agent_id.is_none());
        let any_busy_agent = self
            .agents
            .iter()
            .any(|a| agent_busy_minutes(a, &self.tasks) > 0);

        if self.mode == DelegateMode::Auto && has_unassigned {
            if reduced {
                while self.route_next_auto(cx) {}
            } else {
                if self.last_auto_step.elapsed().as_secs_f32() >= 0.92 {
                    self.last_auto_step = Instant::now();
                    self.route_next_auto(cx);
                }
                w.request_animation_frame();
            }
        } else if !reduced && (self.loading || any_busy_agent) {
            w.request_animation_frame();
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let lane_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.035)
        } else {
            hsla(0., 0., 0., 0.035)
        };

        let title_str = match &self.title_zh {
            Some(zh) => tr(cx, self.title.as_ref(), zh.as_ref()).to_string(),
            None => self.title.to_string(),
        };

        let hint_str = if self.mode == DelegateMode::Auto {
            if has_unassigned {
                tr(
                    cx,
                    "Auto-routing tasks to whichever agent frees up earliest…",
                    "正自动将待办任务分发给最早空闲的智能体…",
                )
            } else {
                tr(
                    cx,
                    "All tasks delegated — add a task to watch Auto route it",
                    "全部任务已分派 —— 点击“+ 任务”可观察自动路由",
                )
            }
        } else if self.picked_task_id.is_some() {
            tr(
                cx,
                "Drag onto an agent, click an agent card, or press 1–4",
                "拖拽至智能体卡片，或直接点击智能体 / 按数字键 1–4 指派",
            )
        } else {
            tr(
                cx,
                "Drag a task onto an agent",
                "将任务拖放到右侧智能体卡片上",
            )
        };

        let is_auto = self.mode == DelegateMode::Auto;
        let header = div()
            .px(px(16.0))
            .py(px(12.0))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(pro_svg("0 0 14 14", SPARK_SVG, 15.0, p.accent_info))
            .child(
                div()
                    .text_size(px(13.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(title_str),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(12.0))
                    .text_color(p.muted)
                    .child(hint_str),
            )
            .when(!has_unassigned, |bar| {
                bar.child(pro_btn(
                    pro_eid(&self.id, "reset-board-btn"),
                    tr(cx, "Reset", "重置"),
                    Some(ProIcon::Undo),
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    |this, _, cx| {
                        this.reset_board(cx);
                    },
                ))
            })
            .child(pro_btn(
                pro_eid(&self.id, "add-task-btn"),
                tr(cx, "+ Task", "+ 任务"),
                None,
                ProBtnStyle::Secondary,
                true,
                cx,
                |this, _, cx| {
                    this.add_sample_task(cx);
                },
            ))
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if is_auto { p.strong } else { p.muted })
                            .child(tr(cx, "Auto", "自动指派")),
                    )
                    .child(pro_switch(
                        pro_eid(&self.id, "auto-assign-switch"),
                        tr(cx, "Auto-assign tasks", "自动将任务分发给最早空闲的智能体"),
                        is_auto,
                        cx,
                        |this, _, cx| {
                            let next = if this.mode == DelegateMode::Auto {
                                DelegateMode::Manual
                            } else {
                                DelegateMode::Auto
                            };
                            this.set_mode(next, cx);
                        },
                    )),
            );

        if self.loading {
            let phase = (elapsed_secs * 0.9).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(740.0))
                        .rounded(px(12.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(
                            div()
                                .p(px(16.0))
                                .flex()
                                .gap(px(14.0))
                                .child(pro_skeleton_bar(228.0, 280.0, 10.0, phase, cx))
                                .child(pro_skeleton_bar(460.0, 280.0, 10.0, phase + 0.2, cx)),
                        ),
                )
                .into_any_element();
        }

        if self.empty || self.agents.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(740.0))
                        .rounded(px(12.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(header)
                        .child(pro_empty_state(
                            ProIcon::Users,
                            tr(cx, "No agents available", "暂无可指派智能体"),
                            tr(
                                cx,
                                "Configure specialist agents to dispatch tasks and preview completion times.",
                                "配置专职智能体即可拖拽分发任务并预览预计繁忙截止时间。",
                            ),
                            Some((
                                pro_eid(&self.id, "seed-delegate"),
                                SharedString::from(tr(cx, "Load specialist team", "加载智能体团队")),
                                Box::new(|this: &mut Self, _, cx| {
                                    this.empty = false;
                                    this.agents = default_agents();
                                    this.tasks = default_tasks();
                                    cx.notify();
                                }),
                            )),
                            cx,
                        )),
                )
                .into_any_element();
        }

        let preview_pair = self.active_preview_pair();
        let left_pile =
            self.render_unassigned_pile(preview_pair.as_ref(), card_bg, lane_bg, &p, cx);
        let right_roster = self.render_agents_roster(
            preview_pair.as_ref(),
            elapsed_secs,
            reduced,
            card_bg,
            w,
            &p,
            cx,
        );

        let board_id = self.id.clone();
        div()
            .id(board_id)
            .w_full()
            .flex()
            .justify_center()
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                let key = ev.keystroke.key.as_str();
                if let Some(digit) = match key {
                    "1" => Some(0usize),
                    "2" => Some(1usize),
                    "3" => Some(2usize),
                    "4" => Some(3usize),
                    _ => None,
                } {
                    if let Some(tid) = this.picked_task_id.clone()
                        && let Some(ag) = this.agents.get(digit)
                    {
                        let aid = ag.id.clone();
                        this.assign_task(tid.as_ref(), Some(aid.as_ref()), cx);
                        cx.stop_propagation();
                    }
                    return;
                }
                match key {
                    "down" | "arrowdown"
                        if this.picked_task_id.is_some() && !this.agents.is_empty() =>
                    {
                        let n = this.agents.len();
                        let cur = this.hovered_agent_idx.unwrap_or(0);
                        this.hovered_agent_idx = Some((cur + 1) % n);
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "up" | "arrowup"
                        if this.picked_task_id.is_some() && !this.agents.is_empty() =>
                    {
                        let n = this.agents.len();
                        let cur = this.hovered_agent_idx.unwrap_or(0);
                        this.hovered_agent_idx = Some(if cur == 0 { n - 1 } else { cur - 1 });
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "enter"
                        if this.picked_task_id.is_some()
                            && this
                                .hovered_agent_idx
                                .is_some_and(|idx| idx < this.agents.len()) =>
                    {
                        let tid = this.picked_task_id.clone().unwrap_or_default();
                        let idx = this.hovered_agent_idx.unwrap_or(0);
                        let aid = this.agents[idx].id.clone();
                        this.assign_task(tid.as_ref(), Some(aid.as_ref()), cx);
                        cx.stop_propagation();
                    }
                    "escape" => {
                        this.picked_task_id = None;
                        this.suggestion = None;
                        this.drag_preview = None;
                        cx.stop_propagation();
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            .child(
                div()
                    .w_full()
                    .max_w(px(740.0))
                    .rounded(px(12.0))
                    .bg(card_bg)
                    .border_1()
                    .border_color(p.line)
                    .when(!p.dark, |d| d.shadow_sm())
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_stretch()
                            .child(left_pile)
                            .child(right_roster),
                    ),
            )
            .into_any_element()
    }
}

impl DelegateBoard {
    fn render_unassigned_pile(
        &self,
        preview: Option<&(DelegateTask, SharedString)>,
        card_bg: Hsla,
        lane_bg: Hsla,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let unassigned: Vec<DelegateTask> = self
            .tasks
            .iter()
            .filter(|t| t.agent_id.is_none())
            .cloned()
            .collect();

        let owner_board = self.id.clone();
        let line_strong = p.line_strong;

        let mut stack = div().flex().flex_col().gap(px(5.0));
        if unassigned.is_empty() {
            stack = stack.child(
                div()
                    .min_h(px(72.0))
                    .px(px(10.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(6.0))
                    .text_size(px(12.0))
                    .text_color(p.muted)
                    .text_center()
                    .child(pro_check_draw(
                        pro_eid(&self.id, "all-delegated-chk"),
                        true,
                        18.0,
                        true,
                        p.green_ink,
                    ))
                    .child(tr(cx, "All tasks handed off", "待办池已全部指派")),
            );
        } else {
            for task in &unassigned {
                let tid = task.id.clone();
                let is_picked = self.picked_task_id.as_ref() == Some(&tid)
                    || preview.is_some_and(|(pt, _)| pt.id == tid);
                let title_str = task.display_title(cx).to_string();
                let payload = TaskDragPayload {
                    board_id: self.id.clone(),
                    task_id: tid.clone(),
                    title: SharedString::from(title_str.clone()),
                    estimate_minutes: task.estimate_minutes,
                };

                let mut tools_row = div().flex().items_center().gap(px(4.0));
                for app_key in &task.tools {
                    tools_row = tools_row.child(pro_app_tile(app_key.as_ref(), 15.0, cx));
                }

                let target_bg = p.accent_soft;
                let target_edge = p.blue.opacity(0.38);
                let hover_bg = p.hover;

                stack = stack.child(
                    pro_tactile(
                        pro_eid(&self.id, format!("utask-{tid}")),
                        format!("{title_str}, ~{} min", task.estimate_minutes),
                        8.0,
                        if is_picked { target_bg } else { card_bg },
                        if is_picked { target_bg } else { hover_bg },
                        if is_picked { target_edge } else { p.line },
                        if is_picked {
                            target_edge
                        } else {
                            p.line_strong
                        },
                        is_picked,
                        div()
                            .w_full()
                            .px(px(8.0))
                            .py(px(8.5))
                            .flex()
                            .items_start()
                            .gap(px(7.0))
                            .child(div().pt(px(3.0)).child(pro_svg(
                                "0 0 12 12",
                                GRIP_DOTS_SVG,
                                12.5,
                                p.faint,
                            )))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .truncate()
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(title_str),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(5.5))
                                            .when(!task.tools.is_empty(), |d| d.child(tools_row))
                                            .child(pro_rolling_number(
                                                pro_eid(&self.id, format!("utask-est-{}", task.id)),
                                                task.estimate_minutes as f64,
                                                0,
                                                "~",
                                                " min",
                                                11.5,
                                                FontWeight::NORMAL,
                                                p.muted,
                                            )),
                                    ),
                            ),
                        cx,
                        move |this, _, cx| {
                            if this.picked_task_id.as_ref() == Some(&tid) {
                                this.picked_task_id = None;
                                this.suggestion = None;
                            } else {
                                this.picked_task_id = Some(tid.clone());
                                if this.hovered_agent_idx.is_none() {
                                    this.hovered_agent_idx = Some(0);
                                }
                            }
                            cx.notify();
                        },
                    )
                    .aria_toggled(is_picked.into())
                    .on_drag(payload, |d, _, _, cx| {
                        cx.new(|_| TaskDragGhost {
                            title: d.title.clone(),
                            estimate_minutes: d.estimate_minutes,
                        })
                    }),
                );
            }
        }

        div()
            .id(pro_eid(&self.id, "unassigned-pile"))
            .role(Role::Group)
            .aria_label(tr(cx, "Unassigned tasks", "未分配任务列表"))
            .w(px(236.0))
            .flex_none()
            .m(px(12.0))
            .mr(px(0.0))
            .p(px(8.0))
            .rounded(px(10.0))
            .bg(lane_bg)
            .drag_over::<TaskDragPayload>(move |s, _, _, _| s.border_1().border_color(line_strong))
            .on_drop(cx.listener(move |this, drag: &TaskDragPayload, _, cx| {
                if drag.board_id == owner_board {
                    this.assign_task(drag.task_id.as_ref(), None, cx);
                }
            }))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .px(px(4.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(div().text_size(px(12.0)).text_color(p.muted).child(tr(
                        cx,
                        "Unassigned",
                        "待分配",
                    )))
                    .child(
                        div()
                            .min_w(px(17.0))
                            .h(px(17.0))
                            .px(px(4.5))
                            .rounded(px(4.0))
                            .bg(card_bg)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "unassigned-cnt"),
                                unassigned.len() as f64,
                                0,
                                "",
                                "",
                                10.5,
                                FontWeight::MEDIUM,
                                p.text,
                            )),
                    ),
            )
            .child(stack)
    }

    #[allow(clippy::too_many_arguments)]
    fn render_agents_roster(
        &self,
        preview: Option<&(DelegateTask, SharedString)>,
        elapsed_secs: f32,
        reduced: bool,
        card_bg: Hsla,
        w: &mut Window,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let total_spend = self.total_queued_spend();
        let spin_angle = (elapsed_secs / 0.90).fract() * TAU;
        let show_shortcuts = self.picked_task_id.is_some();

        let mut roster = div().flex().flex_col().gap(px(5.0));

        for (idx, ag) in self.agents.iter().enumerate() {
            let aid = ag.id.clone();
            let busy_mins = agent_busy_minutes(ag, &self.tasks);
            let agent_tasks: Vec<DelegateTask> = self
                .tasks
                .iter()
                .filter(|t| t.agent_id.as_ref() == Some(&aid))
                .cloned()
                .collect();

            let preview_task = preview
                .filter(|(_, target_aid)| target_aid == &aid)
                .map(|(t, _)| t);
            let is_targeted = preview_task.is_some();

            let avatar = match &ag.provider {
                Some(prov) => pro_app_tile(prov.as_ref(), 28.0, cx),
                None => pro_agent_avatar(aid.as_ref(), p.accent_info, 28.0, busy_mins > 0, 0.0, cx),
            };
            let ag_name = ag.display_name(cx).to_string();

            // Load / Busy-until badge + optional `+$1.20` preview
            let load_badge = if let Some(pt) = preview_task {
                let extra_m = pt.estimate_minutes;
                let new_until = format_busy_clock(BASE_HOUR, BASE_MIN, busy_mins + extra_m);
                let delta_cost = ag.rate_per_min.map(|r| r * extra_m as f64);

                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .when_some(delta_cost, |row, dc| {
                        row.child(pro_rolling_number(
                            pro_eid(&self.id, format!("delta-cost-{aid}")),
                            dc,
                            2,
                            "+$",
                            "",
                            11.5,
                            FontWeight::MEDIUM,
                            p.accent_info,
                        ))
                    })
                    .child(
                        div()
                            .h(px(21.0))
                            .px(px(7.5))
                            .rounded(px(5.5))
                            .bg(card_bg)
                            .border_1()
                            .border_color(p.blue.opacity(0.35))
                            .flex()
                            .items_center()
                            .child(pro_swap_label(
                                pro_eid(&self.id, format!("load-st-{aid}")),
                                format!("+{extra_m}m {} {new_until}", tr(cx, "until", "至")),
                                11.0,
                                FontWeight::MEDIUM,
                                p.accent_info,
                            )),
                    )
            } else if busy_mins == 0 {
                div().child(
                    div()
                        .h(px(21.0))
                        .px(px(7.5))
                        .rounded(px(5.5))
                        .bg(p.green_soft)
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(pro_check_draw(
                            pro_eid(&self.id, format!("free-chk-{aid}")),
                            true,
                            12.0,
                            false,
                            p.green_ink,
                        ))
                        .child(pro_swap_label(
                            pro_eid(&self.id, format!("load-st-{aid}")),
                            tr(cx, "Free", "空闲"),
                            11.0,
                            FontWeight::MEDIUM,
                            p.green_ink,
                        )),
                )
            } else {
                let until_str = format_busy_clock(BASE_HOUR, BASE_MIN, busy_mins);
                div().child(
                    div()
                        .h(px(21.0))
                        .px(px(7.5))
                        .rounded(px(5.5))
                        .bg(p.chip)
                        .flex()
                        .items_center()
                        .child(pro_swap_label(
                            pro_eid(&self.id, format!("load-st-{aid}")),
                            format!("{} {until_str}", tr(cx, "Busy until", "忙至")),
                            11.0,
                            FontWeight::MEDIUM,
                            p.text,
                        )),
                )
            };

            // Proportional "Busy until …" timeline bar (`0..60m` scale)
            let cur_ratio = (busy_mins as f32 / 60.0).clamp(0.0, 1.0);
            let anim_cur = pro_spring_f32(
                pro_eid(&self.id, format!("load-cur-{aid}")),
                cur_ratio,
                0.34,
                w,
                cx,
            )
            .clamp(0.0, 1.0);
            let add_mins = preview_task.map_or(0, |t| t.estimate_minutes);
            let add_ratio = (add_mins as f32 / 60.0).clamp(0.0, (1.0 - anim_cur).max(0.0));
            let anim_add = pro_spring_f32(
                pro_eid(&self.id, format!("load-add-{aid}")),
                add_ratio,
                0.28,
                w,
                cx,
            )
            .clamp(0.0, (1.0 - anim_cur).max(0.0));

            let timeline_bar = div()
                .w_full()
                .h(px(4.0))
                .rounded(px(2.0))
                .bg(p.chip)
                .overflow_hidden()
                .flex()
                .items_center()
                .when(anim_cur > 0.002, |d| {
                    d.child(div().h_full().w(relative(anim_cur)).bg(p.ink.opacity(0.24)))
                })
                .when(anim_add > 0.002, |d| {
                    d.child(div().h_full().w(relative(anim_add)).bg(p.accent_info))
                });

            // Queued task chips row (`max 3` visible + `+N` overflow chip with hover tooltip)
            let mut queue_chips = div().flex().flex_wrap().items_center().gap(px(5.0));

            for (q_idx, qt) in agent_tasks.iter().take(3).enumerate() {
                let is_running = q_idx == 0;
                let qtid = qt.id.clone();
                let qtitle = qt.display_title(cx).to_string();

                if is_running {
                    let accent_c = p.accent_info;
                    let spinner = canvas(
                        |_, _, _| (),
                        move |bounds, _, window, _| {
                            let c = bounds.center();
                            let r = 4.6f32;
                            // Faint full circle
                            let mut ring = PathBuilder::stroke(px(1.5));
                            for i in 0..=16 {
                                let a = (i as f32 / 16.0) * TAU;
                                let pt = c + point(px(r * a.cos()), px(r * a.sin()));
                                if i == 0 {
                                    ring.move_to(pt);
                                } else {
                                    ring.line_to(pt);
                                }
                            }
                            if let Ok(path) = ring.build() {
                                window.paint_path(path, accent_c.opacity(0.25));
                            }
                            // Active quarter arc
                            let mut arc = PathBuilder::stroke(px(1.6));
                            let a0 = if reduced { -1.2 } else { spin_angle };
                            for i in 0..=8 {
                                let a = a0 + (i as f32 / 8.0) * (TAU * 0.28);
                                let pt = c + point(px(r * a.cos()), px(r * a.sin()));
                                if i == 0 {
                                    arc.move_to(pt);
                                } else {
                                    arc.line_to(pt);
                                }
                            }
                            if let Ok(path) = arc.build() {
                                window.paint_path(path, accent_c);
                            }
                        },
                    )
                    .size(px(13.0))
                    .flex_none();

                    queue_chips = queue_chips.child(
                        div()
                            .h(px(24.0))
                            .pl(px(6.5))
                            .pr(px(8.5))
                            .rounded(px(6.0))
                            .bg(p.accent_soft)
                            .flex()
                            .items_center()
                            .gap(px(5.5))
                            .text_size(px(12.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.accent_info)
                            .child(spinner)
                            .child(div().max_w(px(160.0)).truncate().child(qtitle)),
                    );
                } else {
                    let drag_payload = TaskDragPayload {
                        board_id: self.id.clone(),
                        task_id: qtid.clone(),
                        title: SharedString::from(qtitle.clone()),
                        estimate_minutes: qt.estimate_minutes,
                    };
                    let mut mini_tools = div().flex().items_center().gap(px(3.0));
                    for app_key in qt.tools.iter().take(2) {
                        mini_tools = mini_tools.child(pro_app_tile(app_key.as_ref(), 14.0, cx));
                    }

                    queue_chips = queue_chips.child(
                        pro_tactile(
                            pro_eid(&self.id, format!("qchip-{qtid}")),
                            format!(
                                "{qtitle} ({})",
                                tr(cx, "click or drag to unassign", "点击退回待分配")
                            ),
                            6.0,
                            card_bg,
                            p.hover,
                            p.line_strong,
                            p.line_strong,
                            false,
                            div()
                                .h(px(24.0))
                                .pl(px(6.0))
                                .pr(px(8.0))
                                .flex()
                                .items_center()
                                .gap(px(5.0))
                                .text_size(px(12.0))
                                .text_color(p.ink)
                                .child(pro_check_draw(
                                    pro_eid(&self.id, format!("qchip-chk-{qtid}")),
                                    true,
                                    12.0,
                                    false,
                                    p.green_ink,
                                ))
                                .when(!qt.tools.is_empty(), |d| d.child(mini_tools))
                                .child(div().max_w(px(150.0)).truncate().child(qtitle)),
                            cx,
                            move |this, _, cx| {
                                cx.stop_propagation();
                                this.assign_task(qtid.as_ref(), None, cx);
                            },
                        )
                        .w_auto()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_drag(drag_payload, |d, _, _, cx| {
                            cx.new(|_| TaskDragGhost {
                                title: d.title.clone(),
                                estimate_minutes: d.estimate_minutes,
                            })
                        }),
                    );
                }
            }

            if agent_tasks.len() > 3 {
                let extra_tasks: Vec<DelegateTask> = agent_tasks.iter().skip(3).cloned().collect();
                let extra_n = extra_tasks.len();
                let show_tip = self.hovered_overflow_agent.as_ref() == Some(&aid);
                let aid_tip = aid.clone();
                let mut tip_rows: Vec<(String, String)> = Vec::new();
                for et in &extra_tasks {
                    tip_rows.push((
                        et.display_title(cx).to_string(),
                        format!("~{}m", et.estimate_minutes),
                    ));
                }
                let tip_bg = p.primary;
                let tip_fg = p.on_primary;

                queue_chips = queue_chips.child(
                    div()
                        .id(pro_eid(&self.id, format!("more-{aid}")))
                        .relative()
                        .h(px(24.0))
                        .px(px(7.5))
                        .rounded(px(6.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.text)
                        .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                            if *inside {
                                this.hovered_overflow_agent = Some(aid_tip.clone());
                            } else if this.hovered_overflow_agent.as_ref() == Some(&aid_tip) {
                                this.hovered_overflow_agent = None;
                            }
                            cx.notify();
                        }))
                        .child(format!("+{extra_n}"))
                        .child(div().absolute().bottom(px(30.0)).right_0().child(
                            pro_popover_presence(
                                pro_eid(&self.id, format!("more-pop-{aid}")),
                                show_tip,
                                move || {
                                    let mut list = div()
                                        .px(px(10.0))
                                        .py(px(7.5))
                                        .rounded(px(8.0))
                                        .bg(tip_bg)
                                        .text_color(tip_fg)
                                        .shadow_lg()
                                        .whitespace_nowrap()
                                        .flex()
                                        .flex_col()
                                        .gap(px(3.0));
                                    for (ttl, mins_s) in tip_rows {
                                        list = list.child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .gap(px(10.0))
                                                .text_size(px(11.5))
                                                .child(ttl)
                                                .child(
                                                    div()
                                                        .text_color(tip_fg.opacity(0.65))
                                                        .child(mins_s),
                                                ),
                                        );
                                    }
                                    list.into_any_element()
                                },
                            ),
                        )),
                );
            }

            let target_bg = p.accent_soft;
            let target_edge = p.blue.opacity(0.35);
            let spot = pro_spring_f32(
                pro_eid(&self.id, format!("agent-spot-{aid}")),
                if is_targeted { 1.0 } else { 0.0 },
                0.22,
                w,
                cx,
            );
            let rest_row_bg = pro_mix_color(transparent_black(), target_bg, spot);
            let rest_row_edge = pro_mix_color(transparent_black(), target_edge, spot);
            let owner_board = self.id.clone();
            let aid_drop = aid.clone();
            let aid_click = aid.clone();
            let aid_drag = aid.clone();

            roster = roster.child(
                pro_tactile(
                    pro_eid(&self.id, format!("agent-row-{aid}")),
                    format!("{ag_name} queue"),
                    9.0,
                    rest_row_bg,
                    if is_targeted { target_bg } else { p.hover },
                    rest_row_edge,
                    if is_targeted { target_edge } else { p.line },
                    is_targeted,
                    div()
                        .w_full()
                        .p(px(9.0))
                        .flex()
                        .items_start()
                        .gap(px(10.0))
                        .child(avatar)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(6.0))
                                .child(
                                    div()
                                        .flex()
                                        .items_start()
                                        .justify_between()
                                        .gap(px(8.0))
                                        .child(
                                            div()
                                                .min_w_0()
                                                .flex()
                                                .flex_col()
                                                .gap(px(1.5))
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap(px(6.0))
                                                        .child(
                                                            div()
                                                                .truncate()
                                                                .text_size(px(13.0))
                                                                .font_weight(FontWeight::MEDIUM)
                                                                .text_color(p.strong)
                                                                .child(ag_name),
                                                        )
                                                        .when(show_shortcuts && idx < 9, |row| {
                                                            row.child(
                                                                div()
                                                                    .size(px(16.0))
                                                                    .rounded(px(4.0))
                                                                    .bg(card_bg)
                                                                    .border_1()
                                                                    .border_color(p.line)
                                                                    .flex()
                                                                    .items_center()
                                                                    .justify_center()
                                                                    .text_size(px(10.0))
                                                                    .font_weight(FontWeight::MEDIUM)
                                                                    .text_color(p.muted)
                                                                    .child((idx + 1).to_string()),
                                                            )
                                                        }),
                                                )
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap(px(6.0))
                                                        .text_size(px(12.0))
                                                        .text_color(p.muted)
                                                        .when_some(ag.model.clone(), |r, m| {
                                                            r.child(m)
                                                        })
                                                        .when_some(ag.rate_per_min, |r, rate| {
                                                            r.child(
                                                                div().text_color(p.faint).child(
                                                                    format!("${rate:.2}/min"),
                                                                ),
                                                            )
                                                        }),
                                                ),
                                        )
                                        .child(load_badge),
                                )
                                .child(timeline_bar)
                                .when(!agent_tasks.is_empty(), |col| col.child(queue_chips)),
                        ),
                    cx,
                    move |this, _, cx| {
                        if let Some(tid) = this.picked_task_id.clone() {
                            this.assign_task(tid.as_ref(), Some(aid_click.as_ref()), cx);
                        }
                    },
                )
                .role(Role::Group)
                .drag_over::<TaskDragPayload>(move |s, _, _, _| {
                    s.bg(target_bg).border_1().border_color(target_edge)
                })
                .on_drag_move::<TaskDragPayload>(cx.listener(
                    move |this, ev: &DragMoveEvent<TaskDragPayload>, _, cx| {
                        let drag = ev.drag(cx);
                        if drag.board_id == owner_board {
                            let inside = ev.bounds.contains(&ev.event.position);
                            let cur_match = this
                                .drag_preview
                                .as_ref()
                                .is_some_and(|(_, a)| a == &aid_drag);
                            if inside && !cur_match {
                                this.drag_preview = Some((drag.task_id.clone(), aid_drag.clone()));
                                cx.notify();
                            } else if !inside && cur_match {
                                this.drag_preview = None;
                                cx.notify();
                            }
                        }
                    },
                ))
                .on_drop(cx.listener({
                    let owner_board = self.id.clone();
                    move |this, drag: &TaskDragPayload, _, cx| {
                        if drag.board_id == owner_board {
                            this.assign_task(drag.task_id.as_ref(), Some(aid_drop.as_ref()), cx);
                        }
                    }
                }))
                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                    if *inside {
                        this.hovered_agent_idx = Some(idx);
                        cx.notify();
                    }
                })),
            );
        }

        div()
            .flex_1()
            .min_w_0()
            .p(px(14.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .px(px(4.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_size(px(12.0)).text_color(p.muted).child(tr(
                        cx,
                        "Agents",
                        "智能体队列",
                    )))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .text_size(px(12.0))
                            .text_color(p.muted)
                            .child(pro_rolling_number(
                                pro_eid(&self.id, "queue-spend"),
                                total_spend,
                                2,
                                "$",
                                "",
                                12.0,
                                FontWeight::MEDIUM,
                                p.muted,
                            ))
                            .child(tr(cx, "in queues", "队列预计费用")),
                    ),
            )
            .child(roster)
    }
}

fn default_agents() -> Vec<DelegateAgent> {
    vec![
        DelegateAgent::new("researcher", "Researcher")
            .with_name_zh("研究智能体")
            .with_model("Opus 5.5")
            .with_provider("claude")
            .with_current_minutes(6)
            .with_rate(0.08),
        DelegateAgent::new("writer", "Writer")
            .with_name_zh("撰稿智能体")
            .with_model("Sonnet 5.5")
            .with_provider("claude")
            .with_current_minutes(12)
            .with_rate(0.03),
        DelegateAgent::new("coder", "Coder")
            .with_name_zh("编码智能体")
            .with_model("GPT-6 Sol")
            .with_provider("openai")
            .with_current_minutes(8)
            .with_rate(0.06),
        DelegateAgent::new("ops", "Ops")
            .with_name_zh("运维智能体")
            .with_model("Gemini 3.8 Flash")
            .with_provider("gemini")
            .with_rate(0.01),
    ]
}

fn default_tasks() -> Vec<DelegateTask> {
    vec![
        // Unassigned pile (4 tasks)
        DelegateTask::new("t-flaky", "Fix flaky checkout test", 20)
            .with_title_zh("修复偶发失败的结账单测")
            .with_tools(vec![SharedString::from("github")]),
        DelegateTask::new("t-churn", "Summarize churn calls", 15)
            .with_title_zh("汇总流失客户访谈纪要")
            .with_tools(vec![
                SharedString::from("slack"),
                SharedString::from("notion"),
            ]),
        DelegateTask::new("t-pricing", "Update pricing page", 10)
            .with_title_zh("更新官网定价页视觉")
            .with_tools(vec![SharedString::from("figma")]),
        DelegateTask::new("t-crm", "Clean up CRM duplicates", 18)
            .with_title_zh("清理 CRM 重复客户档案")
            .with_tools(vec![SharedString::from("hubspot")]),
        // Active & queued agent tasks matching reference ($2.16 total)
        DelegateTask::new("r-scan", "Competitor scan", 6)
            .with_title_zh("竞品动态扫描")
            .with_agent("researcher"),
        DelegateTask::new("w-notes", "Write release notes", 12)
            .with_title_zh("撰写版本发布说明")
            .with_agent("writer"),
        DelegateTask::new("c-auth", "Refactor auth", 8)
            .with_title_zh("重构鉴权模块")
            .with_agent("coder"),
        DelegateTask::new("c-stripe", "Upgrade Stripe SDK", 14)
            .with_title_zh("升级 Stripe SDK")
            .with_tools(vec![
                SharedString::from("stripe"),
                SharedString::from("github"),
            ])
            .with_agent("coder"),
    ]
}

#[cfg(test)]
mod tests {
    use super::{DelegateBoard, DelegateMode, earliest_free_agent_id, format_busy_clock};
    use gpui::{AppContext as _, TestAppContext};

    #[test]
    fn busy_clock_and_earliest_free_agent_routing_are_accurate() {
        assert_eq!(format_busy_clock(7, 57, 6), "08:03");
        assert_eq!(format_busy_clock(7, 57, 12), "08:09");
        assert_eq!(format_busy_clock(7, 57, 42), "08:39");

        let agents = super::default_agents();
        let tasks = super::default_tasks();
        // Ops has 0 busy minutes so it frees up earliest
        assert_eq!(
            earliest_free_agent_id(&agents, &tasks).as_deref(),
            Some("ops")
        );
    }

    #[gpui::test]
    fn manual_assign_unassign_and_auto_routing_work(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx.new(|cx| DelegateBoard::new("test-delegate-board", cx));

        view.update(cx, |v, cx| {
            assert_eq!(v.unassigned_count(), 4);
            assert_eq!(v.mode(), DelegateMode::Manual);

            assert!(v.assign_task("t-flaky", Some("coder"), cx));
            assert_eq!(v.unassigned_count(), 3);

            // Unassigning returns task back to Unassigned pile
            assert!(v.assign_task("t-flaky", None, cx));
            assert_eq!(v.unassigned_count(), 4);

            // Routing next auto picks `ops` (0m busy) first, then `researcher` (6m busy)
            assert!(v.route_next_auto(cx));
            let routed = v
                .tasks()
                .iter()
                .find(|t| t.id.as_ref() == "t-churn")
                .expect("first unassigned task");
            assert_eq!(routed.agent_id.as_deref(), Some("ops"));
        });
    }
}
