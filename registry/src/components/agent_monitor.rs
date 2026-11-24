//! Supervisory card for a working agent with tool strip, approval gate, and fold bar.
//!
//! Tracks one autonomous agent's active step, live elapsed clock, shimmering/rolling USD spend,
//! proportional tool time strip, expandable finished step rows ([`pro_app_tile`] + [`pro_fold`]),
//! exact command approval gate (`Approve` / `Deny`), and a collapse button that folds the card into
//! a slim status bar displaying overlapping icons of every tool used so far.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_check_draw, pro_eid,
        pro_empty_state, pro_fold, pro_mark_selected, pro_pill_indicator, pro_popover_presence,
        pro_rolling_number, pro_selection_sensor, pro_shimmer_text, pro_skeleton_bar,
        pro_spring_bar, pro_spring_f32, pro_swap_label, pro_tactile, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        family::disabled_motion,
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use web_time::Instant;

const FOLD_IN_SVG: &str = r#"<path d="m5 3 3 3 3-3M5 13l3-3 3 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const FOLD_OUT_SVG: &str = r#"<path d="m5 6 3-3 3 3M5 10l3 3 3-3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const EXT_LOG_SVG: &str = r#"<path d="M5.5 10.5 10.75 5.25M6 5h4.75v4.75" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"></path>"#;

/// Events emitted by [`AgentMonitor`].
#[derive(Clone, Debug, PartialEq)]
pub enum AgentMonitorEvent {
    /// User approved the pending command gate (`Approve` or `Enter`).
    Approved(MonitorAsk),
    /// User denied the pending command gate (`Deny` or `Escape`).
    Denied(MonitorAsk),
    /// User paused the running agent.
    Paused,
    /// User resumed a paused agent.
    Resumed,
    /// User cancelled the current run.
    Cancelled,
    /// User retried a failed or cancelled run.
    Retried,
    /// User clicked `Log` to inspect the full trace externally.
    OpenLog,
    /// User folded (`true`) or unfolded (`false`) the supervisory card.
    FoldChanged(bool),
    /// User toggled details for a completed step id.
    StepInspected(Option<SharedString>),
}

/// High-level lifecycle state of the monitored agent run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MonitorState {
    /// Waiting to start.
    Idle,
    /// Actively executing a step.
    #[default]
    Running,
    /// Blocked on human confirmation (`Approve` / `Deny`) before running an external command.
    Waiting,
    /// Temporarily paused by the user.
    Paused,
    /// Completed all steps cleanly.
    Finished,
    /// Stopped due to an error.
    Failed,
    /// Cancelled by the user.
    Cancelled,
}

impl MonitorState {
    fn pill_label(self, cx: &App) -> &'static str {
        match self {
            Self::Idle => tr(cx, "Idle", "空闲"),
            Self::Running => tr(cx, "Running", "运行中"),
            Self::Waiting => tr(cx, "Needs you", "待审批"),
            Self::Paused => tr(cx, "Paused", "已暂停"),
            Self::Finished => tr(cx, "Finished", "已完成"),
            Self::Failed => tr(cx, "Failed", "失败"),
            Self::Cancelled => tr(cx, "Cancelled", "已取消"),
        }
    }

    fn pill_colors(self, p: &ProPalette) -> (Hsla, Hsla) {
        match self {
            Self::Running => (p.accent_soft, p.accent_info),
            Self::Waiting => (p.amber_soft, p.amber_ink),
            Self::Finished => (p.green_soft, p.green_ink),
            Self::Failed => (p.red_soft, p.red_ink),
            Self::Idle | Self::Paused | Self::Cancelled => (p.chip, p.muted),
        }
    }
}

/// Fallback plain step category when no third-party app logo is set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MonitorStepKind {
    Read,
    Search,
    Edit,
    #[default]
    Run,
    Think,
    Web,
}

impl MonitorStepKind {
    fn pro_icon(self) -> ProIcon {
        match self {
            Self::Read => ProIcon::PageDoc,
            Self::Search => ProIcon::Search,
            Self::Edit => ProIcon::Code,
            Self::Run => ProIcon::Terminal,
            Self::Think => ProIcon::Sparkles,
            Self::Web => ProIcon::Globe,
        }
    }
}

/// Execution state of an individual [`MonitorStep`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MonitorStepState {
    /// Scheduled to run after earlier steps complete.
    Queued,
    /// Currently executing in the live well.
    Running,
    /// Completed and recorded in the time strip and step rows.
    #[default]
    Done,
    /// Failed with an error.
    Failed,
}

/// Identity header descriptor for the supervised agent.
#[derive(Clone, Debug, PartialEq)]
pub struct MonitoredAgent {
    /// Model or agent name (`"Claude Opus 5.5"`).
    pub name: SharedString,
    /// Role and target subtitle (`"Release agent · web-app v3.2"`).
    pub role: Option<SharedString>,
    /// Optional Chinese role subtitle.
    pub role_zh: Option<SharedString>,
    /// Optional provider mark key (`"claude"`, `"openai"`, `"gemini"`).
    pub provider: Option<SharedString>,
}

impl MonitoredAgent {
    /// Create a [`MonitoredAgent`] with `name`.
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            role: None,
            role_zh: None,
            provider: None,
        }
    }

    /// Set the role subtitle and its localized Chinese counterpart.
    pub fn with_role(
        mut self,
        role: impl Into<SharedString>,
        role_zh: impl Into<SharedString>,
    ) -> Self {
        self.role = Some(role.into());
        self.role_zh = Some(role_zh.into());
        self
    }

    /// Set the provider mark key rendered via [`pro_app_tile`].
    pub fn with_provider(mut self, provider: impl Into<SharedString>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    fn display_role<'a>(&'a self, cx: &App) -> Option<&'a str> {
        match (&self.role, &self.role_zh) {
            (Some(en), Some(zh)) => Some(tr(cx, en.as_ref(), zh.as_ref())),
            (Some(en), None) => Some(en.as_ref()),
            _ => None,
        }
    }
}

/// One step in the supervised agent run.
#[derive(Clone, Debug, PartialEq)]
pub struct MonitorStep {
    /// Stable step identifier.
    pub id: SharedString,
    /// Concise action label (`"Collected 14 issues"`).
    pub label: SharedString,
    /// Optional Chinese action label.
    pub label_zh: Option<SharedString>,
    /// Optional app/tool logo key (`"linear"`, `"github"`, `"notion"`, `"slack"`).
    pub app: Option<SharedString>,
    /// Tool display name shown inside the step detail card.
    pub tool_name: Option<SharedString>,
    /// Fallback icon kind when `app` is `None`.
    pub kind: MonitorStepKind,
    /// Step execution state.
    pub state: MonitorStepState,
    /// Duration spent on this step in milliseconds (`1_800` -> `"1.8s"`).
    pub took_ms: u64,
    /// One-sentence explanation shown when the step row is expanded.
    pub detail: Option<SharedString>,
    /// Optional Chinese detail sentence.
    pub detail_zh: Option<SharedString>,
    /// Monospace input/output command or artifact line shown in the detail card.
    pub output: Option<SharedString>,
}

impl MonitorStep {
    /// Create a [`MonitorStep`] with `id`, `label`, and `state`.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        state: MonitorStepState,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            label_zh: None,
            app: None,
            tool_name: None,
            kind: MonitorStepKind::Run,
            state,
            took_ms: 1_500,
            detail: None,
            detail_zh: None,
            output: None,
        }
    }

    /// Set localized Chinese label.
    pub fn with_label_zh(mut self, label_zh: impl Into<SharedString>) -> Self {
        self.label_zh = Some(label_zh.into());
        self
    }

    /// Attach a provider/app logo key (`"linear"`, `"github"`, `"notion"`, `"slack"`).
    pub fn with_app(
        mut self,
        app: impl Into<SharedString>,
        tool_name: impl Into<SharedString>,
    ) -> Self {
        self.app = Some(app.into());
        self.tool_name = Some(tool_name.into());
        self
    }

    /// Set fallback plain icon category.
    pub fn with_kind(mut self, kind: MonitorStepKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set step duration in milliseconds.
    pub fn with_took_ms(mut self, took_ms: u64) -> Self {
        self.took_ms = took_ms;
        self
    }

    /// Attach input/output summary details revealed when clicked.
    pub fn with_io_details(
        mut self,
        detail: impl Into<SharedString>,
        detail_zh: impl Into<SharedString>,
        output: impl Into<SharedString>,
    ) -> Self {
        self.detail = Some(detail.into());
        self.detail_zh = Some(detail_zh.into());
        self.output = Some(output.into());
        self
    }

    fn display_label<'a>(&'a self, cx: &App) -> &'a str {
        match &self.label_zh {
            Some(zh) => tr(cx, self.label.as_ref(), zh.as_ref()),
            None => self.label.as_ref(),
        }
    }

    fn display_detail<'a>(&'a self, cx: &App) -> Option<&'a str> {
        match (&self.detail, &self.detail_zh) {
            (Some(en), Some(zh)) => Some(tr(cx, en.as_ref(), zh.as_ref())),
            (Some(en), None) => Some(en.as_ref()),
            _ => None,
        }
    }
}

/// Approval prompt displayed when [`MonitorState::Waiting`] requires user confirmation.
#[derive(Clone, Debug, PartialEq)]
pub struct MonitorAsk {
    /// Short prompt heading (`"Ready to ship"`).
    pub title: SharedString,
    /// Optional Chinese prompt heading.
    pub title_zh: Option<SharedString>,
    /// Exact shell/API command awaiting approval.
    pub command: SharedString,
    /// Optional app logo key (`"github"`).
    pub app: Option<SharedString>,
}

impl MonitorAsk {
    /// Create a [`MonitorAsk`] with `title` and `command`.
    pub fn new(title: impl Into<SharedString>, command: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            title_zh: None,
            command: command.into(),
            app: None,
        }
    }

    /// Set localized Chinese heading.
    pub fn with_title_zh(mut self, title_zh: impl Into<SharedString>) -> Self {
        self.title_zh = Some(title_zh.into());
        self
    }

    /// Set the target tool logo key (`"github"`, `"stripe"`, etc.).
    pub fn with_app(mut self, app: impl Into<SharedString>) -> Self {
        self.app = Some(app.into());
        self
    }

    fn display_title<'a>(&'a self, cx: &App) -> &'a str {
        match &self.title_zh {
            Some(zh) => tr(cx, self.title.as_ref(), zh.as_ref()),
            None => self.title.as_ref(),
        }
    }
}

/// Final summary shown once [`MonitorState::Finished`] is reached.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MonitorOutcome {
    /// Lines added, if applicable.
    pub added: Option<u32>,
    /// Lines removed, if applicable.
    pub removed: Option<u32>,
    /// Files modified, if applicable.
    pub files: Option<u32>,
    /// Concise completion note (`"v3.2.0 is live · 14 changes"`).
    pub note: Option<SharedString>,
    /// Optional Chinese completion note.
    pub note_zh: Option<SharedString>,
}

impl MonitorOutcome {
    /// Create a [`MonitorOutcome`] with a completion note.
    pub fn new(note: impl Into<SharedString>, note_zh: impl Into<SharedString>) -> Self {
        Self {
            added: None,
            removed: None,
            files: None,
            note: Some(note.into()),
            note_zh: Some(note_zh.into()),
        }
    }

    /// Attach diff line and file counts.
    pub fn with_diff(mut self, added: u32, removed: u32, files: u32) -> Self {
        self.added = Some(added);
        self.removed = Some(removed);
        self.files = Some(files);
        self
    }

    fn display_note<'a>(&'a self, cx: &App) -> Option<&'a str> {
        match (&self.note, &self.note_zh) {
            (Some(en), Some(zh)) => Some(tr(cx, en.as_ref(), zh.as_ref())),
            (Some(en), None) => Some(en.as_ref()),
            _ => None,
        }
    }
}

/// Format milliseconds into compact step timing (`"1.8s"`, `"19s"`, `"1:04"`).
pub fn format_step_duration(ms: u64) -> String {
    if ms < 10_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else if ms < 60_000 {
        format!("{}s", ms / 1000)
    } else {
        let secs = ms / 1000;
        format!("{}:{:02}", secs / 60, secs % 60)
    }
}

/// Format elapsed seconds into a tabular `m:ss` live clock (`"0:08"`, `"1:24"`).
pub fn format_live_clock(secs: u64) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// Compact supervisory card for one working agent with tool time strip, approval gate, and foldable slim bar.
pub struct AgentMonitor {
    id: ElementId,
    state: MonitorState,
    agent: MonitoredAgent,
    steps: Vec<MonitorStep>,
    ask: Option<MonitorAsk>,
    outcome: Option<MonitorOutcome>,
    error_msg: Option<SharedString>,
    base_cost: f64,
    base_elapsed_secs: u64,
    folded: bool,
    log_open: bool,
    open_step_id: Option<SharedString>,
    hovered_step_id: Option<SharedString>,
    hovered_folded_tool: Option<usize>,
    loading: bool,
    empty: bool,
    started_at: Instant,
    paused_accum_secs: f32,
    paused_since: Option<Instant>,
}

impl EventEmitter<AgentMonitorEvent> for AgentMonitor {}

impl AgentMonitor {
    /// Create an [`AgentMonitor`] supervisory card preloaded with the `Release agent · web-app v3.2` run awaiting ship approval.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            state: MonitorState::Waiting,
            agent: MonitoredAgent::new("Claude Opus 5.5")
                .with_role("Release agent · web-app v3.2", "发布智能体 · web-app v3.2")
                .with_provider("claude"),
            steps: default_steps(),
            ask: Some(
                MonitorAsk::new(
                    "Ready to ship release",
                    "gh release create v3.2.0 --latest --notes-file RELEASE.md",
                )
                .with_title_zh("准备发布正式版本")
                .with_app("github"),
            ),
            outcome: Some(
                MonitorOutcome::new(
                    "v3.2.0 is live · 14 changes shipped",
                    "v3.2.0 已上线 · 共交付 14 项变更",
                )
                .with_diff(142, 28, 6),
            ),
            error_msg: None,
            base_cost: 0.042,
            base_elapsed_secs: 8,
            folded: false,
            log_open: false,
            open_step_id: None,
            hovered_step_id: None,
            hovered_folded_tool: None,
            loading: false,
            empty: false,
            started_at: Instant::now(),
            paused_accum_secs: 0.0,
            paused_since: None,
        }
    }

    /// Set the initial [`MonitorState`].
    pub fn with_state(mut self, state: MonitorState) -> Self {
        self.state = state;
        self
    }

    /// Configure the supervised [`MonitoredAgent`].
    pub fn with_agent(mut self, agent: MonitoredAgent) -> Self {
        self.agent = agent;
        self
    }

    /// Supply ordered [`MonitorStep`] entries (oldest first).
    pub fn with_steps(mut self, steps: Vec<MonitorStep>) -> Self {
        self.steps = steps;
        self
    }

    /// Configure the command approval request shown while [`MonitorState::Waiting`].
    pub fn with_ask(mut self, ask: Option<MonitorAsk>) -> Self {
        self.ask = ask;
        self
    }

    /// Configure the completion summary shown when [`MonitorState::Finished`].
    pub fn with_outcome(mut self, outcome: Option<MonitorOutcome>) -> Self {
        self.outcome = outcome;
        self
    }

    /// Set error summary text shown when [`MonitorState::Failed`].
    pub fn with_error(mut self, error: impl Into<SharedString>) -> Self {
        self.error_msg = Some(error.into());
        self
    }

    /// Set starting USD spend (`$0.042` default).
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.base_cost = cost.max(0.0);
        self
    }

    /// Start collapsed as the slim supervisory status bar (`false` by default).
    pub fn with_default_folded(mut self, folded: bool) -> Self {
        self.folded = folded;
        self
    }

    /// Show skeleton placeholder state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Show empty state when no run is attached.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return current [`MonitorState`].
    pub fn state(&self) -> MonitorState {
        self.state
    }

    /// Return whether the card is currently folded into the slim status bar.
    pub fn is_folded(&self) -> bool {
        self.folded
    }

    /// Return the steps slice.
    pub fn steps(&self) -> &[MonitorStep] {
        &self.steps
    }

    /// Fold or unfold the supervisory card.
    pub fn set_folded(&mut self, folded: bool, cx: &mut Context<Self>) {
        if self.folded != folded {
            self.folded = folded;
            cx.emit(AgentMonitorEvent::FoldChanged(folded));
            cx.notify();
        }
    }

    /// Approve the pending [`MonitorAsk`], executing the gated step and advancing the run.
    pub fn approve(&mut self, cx: &mut Context<Self>) -> bool {
        if self.state != MonitorState::Waiting {
            return false;
        }
        let ask = self
            .ask
            .clone()
            .unwrap_or_else(|| MonitorAsk::new("Approved action", "gh release create v3.2.0"));
        for st in &mut self.steps {
            if st.state == MonitorStepState::Running {
                st.state = MonitorStepState::Done;
            }
        }
        if let Some(next_q) = self
            .steps
            .iter_mut()
            .find(|s| s.state == MonitorStepState::Queued)
        {
            next_q.state = MonitorStepState::Running;
            self.state = MonitorState::Running;
        } else {
            self.state = MonitorState::Finished;
        }
        self.base_cost += 0.014;
        cx.emit(AgentMonitorEvent::Approved(ask));
        cx.notify();
        true
    }

    /// Deny the pending [`MonitorAsk`], halting the gated command safely.
    pub fn deny(&mut self, cx: &mut Context<Self>) -> bool {
        if self.state != MonitorState::Waiting {
            return false;
        }
        let ask = self
            .ask
            .clone()
            .unwrap_or_else(|| MonitorAsk::new("Declined action", "gh release create v3.2.0"));
        self.state = MonitorState::Cancelled;
        cx.emit(AgentMonitorEvent::Denied(ask));
        cx.notify();
        true
    }

    /// Pause a running agent.
    pub fn pause(&mut self, cx: &mut Context<Self>) {
        if self.state == MonitorState::Running {
            self.state = MonitorState::Paused;
            self.paused_since = Some(Instant::now());
            cx.emit(AgentMonitorEvent::Paused);
            cx.notify();
        }
    }

    /// Resume a paused agent.
    pub fn resume(&mut self, cx: &mut Context<Self>) {
        if self.state == MonitorState::Paused {
            if let Some(ps) = self.paused_since.take() {
                self.paused_accum_secs += ps.elapsed().as_secs_f32();
            }
            self.state = MonitorState::Running;
            cx.emit(AgentMonitorEvent::Resumed);
            cx.notify();
        }
    }

    /// Cancel an active or paused run.
    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        if matches!(
            self.state,
            MonitorState::Running | MonitorState::Paused | MonitorState::Waiting
        ) {
            self.state = MonitorState::Cancelled;
            cx.emit(AgentMonitorEvent::Cancelled);
            cx.notify();
        }
    }

    /// Complete all remaining steps (or reset & retry after cancel/failure).
    pub fn retry_or_replay(&mut self, cx: &mut Context<Self>) {
        self.steps = default_steps();
        self.state = MonitorState::Waiting;
        self.base_cost = 0.042;
        self.open_step_id = None;
        self.started_at = Instant::now();
        self.paused_accum_secs = 0.0;
        self.paused_since = None;
        cx.emit(AgentMonitorEvent::Retried);
        cx.notify();
    }

    fn render_step_mark(
        &self,
        step: &MonitorStep,
        side: f32,
        p: &ProPalette,
        cx: &App,
    ) -> AnyElement {
        if let Some(app) = &step.app {
            return pro_app_tile(app.as_ref(), side, cx);
        }
        div()
            .size(px(side))
            .flex_none()
            .rounded(px((side * 0.28).clamp(4.0, 6.0)))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .justify_center()
            .child(pro_icon(
                step.kind.pro_icon(),
                (side * 0.62).max(10.0),
                p.muted,
            ))
            .into_any_element()
    }

    fn active_elapsed_secs(&self, reduced: bool) -> f32 {
        if reduced {
            return self.base_elapsed_secs as f32;
        }
        let raw = self.started_at.elapsed().as_secs_f32();
        let cur_paused = self.paused_since.map_or(0.0, |t| t.elapsed().as_secs_f32());
        let net = (raw - self.paused_accum_secs - cur_paused).max(0.0);
        match self.state {
            MonitorState::Running | MonitorState::Waiting => self.base_elapsed_secs as f32 + net,
            MonitorState::Paused => self.base_elapsed_secs as f32 + net,
            MonitorState::Finished => 19.0,
            MonitorState::Failed | MonitorState::Cancelled | MonitorState::Idle => {
                self.base_elapsed_secs as f32
            }
        }
    }
}

impl Render for AgentMonitor {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx) || Theme::global(cx).reduced_motion;
        let live_ticking = matches!(self.state, MonitorState::Running | MonitorState::Waiting);

        if !reduced && (self.loading || live_ticking) {
            w.request_animation_frame();
        }

        let elapsed_s = self.active_elapsed_secs(reduced);
        // Auto-complete a running release step after 5s of post-approval execution so users see Finished too!
        if !reduced
            && self.state == MonitorState::Running
            && elapsed_s > (self.base_elapsed_secs as f32 + 6.5)
        {
            for st in &mut self.steps {
                if st.state == MonitorStepState::Running || st.state == MonitorStepState::Queued {
                    st.state = MonitorStepState::Done;
                }
            }
            self.state = MonitorState::Finished;
        }

        let live_cost = if self.state == MonitorState::Running {
            self.base_cost + ((elapsed_s - self.base_elapsed_secs as f32).max(0.0) as f64) * 0.002
        } else {
            self.base_cost
        };

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1f).into()
        } else {
            rgb(0xffffff).into()
        };
        let lane_bg: Hsla = if p.dark {
            rgb(0x161618).into()
        } else {
            rgb(0xf4f4f5).into()
        };

        if self.loading {
            let phase = (self.started_at.elapsed().as_secs_f32() * 0.9).fract();
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(384.0))
                        .p(px(14.0))
                        .rounded(px(14.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(10.0))
                        .child(pro_skeleton_bar(220.0, 22.0, 6.0, phase, cx))
                        .child(pro_skeleton_bar(340.0, 36.0, 8.0, phase + 0.15, cx))
                        .child(pro_skeleton_bar(340.0, 6.0, 3.0, phase + 0.30, cx))
                        .child(pro_skeleton_bar(300.0, 24.0, 6.0, phase + 0.45, cx)),
                )
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(384.0))
                        .rounded(px(14.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .child(pro_empty_state(
                            ProIcon::Activity,
                            tr(cx, "No active agent run", "暂无运行中的智能体"),
                            tr(
                                cx,
                                "Attach an agent run to supervise its tools, cost, and command approvals.",
                                "绑定智能体任务即可监督工具耗时、实时账单与敏感命令审批。",
                            ),
                            Some((
                                pro_eid(&self.id, "start-monitor"),
                                SharedString::from(tr(cx, "Load release run", "加载发布任务")),
                                Box::new(|this: &mut Self, _, cx| {
                                    this.empty = false;
                                    cx.notify();
                                }),
                            )),
                            cx,
                        )),
                )
                .into_any_element();
        }

        let agent_mark = match &self.agent.provider {
            Some(prov) => pro_app_tile(prov.as_ref(), 26.0, cx),
            None => pro_agent_avatar(
                self.agent.name.as_ref(),
                p.accent_info,
                26.0,
                live_ticking,
                elapsed_s * 1000.0,
                cx,
            ),
        };

        // Mode switcher pills above the card so users can also preview Needs you / Running / Done / Folded effortlessly
        let quick_bar = self.render_scenario_bar(&p, w, cx);

        if self.folded {
            let folded_el = self.render_folded_bar(elapsed_s, live_cost, card_bg, w, &p, cx);
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(10.0))
                .child(quick_bar)
                .child(pro_view_transition(
                    pro_eid(&self.id, "fold-shell-tx"),
                    "folded",
                    div().w_full().flex().justify_center().child(folded_el),
                ))
                .into_any_element();
        }

        // Top supervisory header row
        let (pill_bg, pill_fg) = self.state.pill_colors(&p);
        let pill_lbl = self.state.pill_label(cx);
        let breathe_alpha = if !reduced && live_ticking {
            0.55 + 0.45 * (elapsed_s * 3.8).sin().abs()
        } else {
            1.0
        };

        let fold_away_btn = pro_tactile(
            pro_eid(&self.id, "fold-away-btn"),
            tr(cx, "Fold away", "折叠为精简状态栏"),
            6.0,
            transparent_black(),
            p.hover,
            transparent_black(),
            transparent_black(),
            false,
            div()
                .size(px(24.0))
                .flex()
                .items_center()
                .justify_center()
                .child(pro_svg("0 0 16 16", FOLD_IN_SVG, 12.0, p.muted)),
            cx,
            |this, _, cx| {
                this.set_folded(true, cx);
            },
        )
        .size(px(24.0))
        .flex_none();

        let top_row = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(agent_mark)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .truncate()
                            .text_size(px(13.0))
                            .line_height(px(16.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(self.agent.name.clone()),
                    )
                    .when_some(self.agent.display_role(cx), |col, role_str| {
                        col.child(
                            div()
                                .truncate()
                                .text_size(px(11.0))
                                .line_height(px(14.0))
                                .text_color(p.muted)
                                .child(role_str.to_string()),
                        )
                    }),
            )
            .child(pro_rolling_number(
                pro_eid(&self.id, "run-cost"),
                live_cost,
                3,
                "$",
                "",
                11.5,
                FontWeight::MEDIUM,
                p.ink,
            ))
            .child(
                div()
                    .h(px(20.0))
                    .px(px(6.5))
                    .rounded(px(5.0))
                    .bg(pill_bg)
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .child(if self.state == MonitorState::Finished {
                        pro_check_draw(
                            pro_eid(&self.id, "hdr-done-chk"),
                            true,
                            12.0,
                            false,
                            p.green,
                        )
                    } else {
                        div()
                            .size(px(6.0))
                            .rounded(px(2.0))
                            .bg(pill_fg.opacity(breathe_alpha))
                            .into_any_element()
                    })
                    .child(pro_swap_label(
                        pro_eid(&self.id, "hdr-state-lbl"),
                        pill_lbl,
                        11.0,
                        FontWeight::MEDIUM,
                        pill_fg,
                    )),
            )
            .child(fold_away_btn);

        // Current Step Well + Optional Exact Approval Command Box
        let clock_str = format_live_clock(elapsed_s as u64);
        let well_raw = self.render_well_and_approval(&clock_str, lane_bg, &p, cx);
        let well_section = pro_view_transition(
            pro_eid(&self.id, "well-tx"),
            format!("{:?}", self.state),
            well_raw,
        );

        // Segmented Time Strip (`height: 6px`) + Overall Progress Spring Bar
        let strip_section = self.render_time_strip(&p, cx);
        let done_ratio = (self
            .steps
            .iter()
            .filter(|s| s.state == MonitorStepState::Done)
            .count() as f32)
            / (self.steps.len().max(1) as f32);
        let prog_bar = pro_spring_bar(
            pro_eid(&self.id, "overall-prog"),
            done_ratio,
            3.0,
            1.5,
            p.chip,
            if self.state == MonitorState::Finished {
                p.green
            } else if self.state == MonitorState::Waiting {
                p.amber
            } else {
                p.blue
            },
        );

        // Completed Step Rows + Expandable Input/Output Details (`pro_fold`)
        let steps_section = self.render_completed_steps(lane_bg, &p, cx);

        // Footer Controls + Expandable Trace Log (`pro_fold`)
        let footer_section = self.render_footer(lane_bg, &p, cx);
        let log_drawer = self.render_log_drawer(lane_bg, &p);

        let expanded_body = pro_fold(
            pro_eid(&self.id, "card-body-fold"),
            !self.folded,
            260.0,
            0.0,
            move || {
                div()
                    .w_full()
                    .pt(px(10.0))
                    .flex()
                    .flex_col()
                    .gap(px(10.0))
                    .child(well_section)
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(5.0))
                            .child(strip_section)
                            .child(prog_bar),
                    )
                    .child(steps_section)
                    .child(footer_section)
                    .child(log_drawer)
                    .into_any_element()
            },
        );

        div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(10.0))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                match ev.keystroke.key.as_str() {
                    "enter" if this.state == MonitorState::Waiting => {
                        this.approve(cx);
                        cx.stop_propagation();
                    }
                    "escape" if this.open_step_id.is_some() => {
                        this.open_step_id = None;
                        cx.emit(AgentMonitorEvent::StepInspected(None));
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "escape" if this.state == MonitorState::Waiting => {
                        this.deny(cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(quick_bar)
            .child(pro_view_transition(
                pro_eid(&self.id, "fold-shell-tx"),
                "open",
                div().w_full().flex().justify_center().child(
                    div()
                        .w_full()
                        .max_w(px(388.0))
                        .p(px(14.0))
                        .rounded(px(14.0))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .when(!p.dark, |d| d.shadow_md())
                        .flex()
                        .flex_col()
                        .child(top_row)
                        .child(expanded_body),
                ),
            ))
            .into_any_element()
    }
}

impl AgentMonitor {
    fn render_scenario_bar(
        &self,
        p: &ProPalette,
        _w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let items: [(Option<MonitorState>, &str, &str, &str); 4] = [
            (
                Some(MonitorState::Waiting),
                "waiting",
                "Needs you",
                "待审批",
            ),
            (Some(MonitorState::Running), "running", "Running", "运行中"),
            (Some(MonitorState::Finished), "done", "Done", "已完成"),
            (None, "folded", "Folded", "精简栏"),
        ];
        let active_key: SharedString = if self.folded {
            "folded".into()
        } else {
            match self.state {
                MonitorState::Running | MonitorState::Paused | MonitorState::Idle => {
                    "running".into()
                }
                MonitorState::Finished => "done".into(),
                _ => "waiting".into(),
            }
        };
        let sensor = pro_selection_sensor();
        let mut bar = div()
            .id(pro_eid(&self.id, "scenario-tabs"))
            .role(Role::TabList)
            .aria_label(tr(cx, "Monitor state", "运行状态"))
            .h(px(30.0))
            .p(px(2.0))
            .rounded(px(8.0))
            .bg(p.chip)
            .border_1()
            .border_color(p.line_soft)
            .relative()
            .flex()
            .items_center()
            .gap(px(2.0))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "scenario-pill"),
                active_key.clone(),
                sensor.clone(),
                6.0,
                p.card,
                Some(p.line_strong),
                !p.dark,
            ));

        for (st_opt, key, en, zh) in items {
            let active = active_key.as_ref() == key;
            bar = bar.child(
                div()
                    .id(pro_eid(&self.id, format!("mode-{key}")))
                    .role(Role::Tab)
                    .aria_label(tr(cx, en, zh))
                    .aria_selected(active)
                    .tab_index(if active { 0 } else { -1 })
                    .relative()
                    .h(px(26.0))
                    .px(px(10.0))
                    .rounded(px(6.0))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .text_size(px(12.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if active { p.ink } else { p.muted })
                    .hover(move |s| s.text_color(p.ink))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .on_click(cx.listener(move |this, _, _, cx| match st_opt {
                        None => {
                            this.set_folded(true, cx);
                        }
                        Some(st) => {
                            this.folded = false;
                            this.open_step_id = None;
                            this.started_at = Instant::now();
                            match st {
                                MonitorState::Waiting => {
                                    this.steps = default_steps();
                                    this.state = MonitorState::Waiting;
                                }
                                MonitorState::Running => {
                                    let mut s = default_steps();
                                    if let Some(q) =
                                        s.iter_mut().find(|x| x.state == MonitorStepState::Queued)
                                    {
                                        q.state = MonitorStepState::Running;
                                    }
                                    this.steps = s;
                                    this.state = MonitorState::Running;
                                }
                                MonitorState::Finished => {
                                    let mut s = default_steps();
                                    for x in &mut s {
                                        x.state = MonitorStepState::Done;
                                    }
                                    this.steps = s;
                                    this.state = MonitorState::Finished;
                                }
                                _ => {}
                            }
                            cx.notify();
                        }
                    }))
                    .when(active, |d| d.child(pro_mark_selected(&sensor)))
                    .child(div().relative().child(tr(cx, en, zh))),
            );
        }
        bar
    }

    fn render_folded_bar(
        &self,
        elapsed_s: f32,
        live_cost: f64,
        card_bg: Hsla,
        w: &mut Window,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let lead_mark = match &self.agent.provider {
            Some(prov) => pro_app_tile(prov.as_ref(), 24.0, cx),
            None => pro_agent_avatar(
                self.agent.name.as_ref(),
                p.accent_info,
                24.0,
                true,
                elapsed_s * 1000.0,
                cx,
            ),
        };

        // Deduplicated list of tools used across done/running steps
        let mut used_steps: Vec<MonitorStep> = Vec::new();
        for st in &self.steps {
            if matches!(st.state, MonitorStepState::Done | MonitorStepState::Running) {
                let key = st.app.as_deref().unwrap_or(st.id.as_ref());
                if let Some(pos) = used_steps
                    .iter()
                    .position(|u| u.app.as_deref().unwrap_or(u.id.as_ref()) == key)
                {
                    used_steps[pos] = st.clone();
                } else {
                    used_steps.push(st.clone());
                }
            }
        }

        let mut tools_stack = div().flex().items_center();
        let total_tools = used_steps.len();
        for (idx, st) in used_steps.into_iter().enumerate() {
            let is_last = idx + 1 == total_tools;
            let hovered = self.hovered_folded_tool == Some(idx);
            let lift = pro_spring_f32(
                pro_eid(&self.id, format!("ftool-lift-{idx}")),
                if hovered { -3.0 } else { 0.0 },
                0.18,
                w,
                cx,
            );
            let step_lbl = st.display_label(cx).to_string();
            let took_str = format_step_duration(st.took_ms);
            let mark = self.render_step_mark(&st, 22.0, p, cx);
            let p_tip = *p;
            let tip_pop =
                div()
                    .absolute()
                    .bottom(px(30.0))
                    .left(px(-40.0))
                    .child(pro_popover_presence(
                        pro_eid(&self.id, format!("ftool-tip-{idx}")),
                        hovered,
                        move || {
                            div()
                                .px(px(8.5))
                                .py(px(5.0))
                                .rounded(px(7.0))
                                .bg(p_tip.card)
                                .border_1()
                                .border_color(p_tip.line_strong)
                                .shadow_md()
                                .whitespace_nowrap()
                                .flex()
                                .flex_col()
                                .gap(px(1.0))
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p_tip.strong)
                                        .child(step_lbl),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.5))
                                        .text_color(p_tip.muted)
                                        .child(took_str),
                                )
                                .into_any_element()
                        },
                    ));

            tools_stack = tools_stack.child(
                div()
                    .id(pro_eid(&self.id, format!("ftool-{idx}")))
                    .relative()
                    .when(idx > 0, |d| d.ml(px(-5.0)))
                    .top(px(lift))
                    .opacity(
                        if is_last || hovered || self.state == MonitorState::Finished {
                            1.0
                        } else {
                            0.68
                        },
                    )
                    .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                        if *inside {
                            this.hovered_folded_tool = Some(idx);
                        } else if this.hovered_folded_tool == Some(idx) {
                            this.hovered_folded_tool = None;
                        }
                        cx.notify();
                    }))
                    .child(mark)
                    .child(tip_pop),
            );
        }

        let (pill_bg, pill_fg) = self.state.pill_colors(p);
        let pill_lbl = self.state.pill_label(cx);
        let clock_str = format_live_clock(elapsed_s as u64);
        let aria_fold = format!(
            "{} – {} ({})",
            self.agent.name,
            pill_lbl,
            tr(cx, "Click to expand", "点击展开监督卡片")
        );

        let bar_body = div()
            .h(px(44.0))
            .pl(px(10.0))
            .pr(px(12.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(lead_mark)
            .child(
                div()
                    .max_w(px(140.0))
                    .truncate()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(self.agent.name.clone()),
            )
            .child(tools_stack)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .text_size(px(12.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(format!("{clock_str} ·"))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "fold-cost"),
                        live_cost,
                        2,
                        "$",
                        "",
                        12.0,
                        FontWeight::MEDIUM,
                        p.ink,
                    )),
            )
            .child(
                div()
                    .h(px(22.0))
                    .px(px(7.0))
                    .rounded(px(6.0))
                    .bg(pill_bg)
                    .flex()
                    .items_center()
                    .child(pro_swap_label(
                        pro_eid(&self.id, "fold-pill-lbl"),
                        pill_lbl,
                        11.5,
                        FontWeight::MEDIUM,
                        pill_fg,
                    )),
            )
            .child(pro_svg("0 0 16 16", FOLD_OUT_SVG, 12.0, p.muted));

        pro_tactile(
            pro_eid(&self.id, "folded-bar"),
            aria_fold,
            13.0,
            card_bg,
            p.subtle,
            p.line,
            p.line_strong,
            false,
            bar_body,
            cx,
            |this, _, cx| {
                this.set_folded(false, cx);
            },
        )
        .w_auto()
        .when(!p.dark, |d| d.shadow_md())
        .into_any_element()
    }

    fn render_well_and_approval(
        &self,
        clock_str: &str,
        lane_bg: Hsla,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match self.state {
            MonitorState::Waiting => {
                let ask = self.ask.clone().unwrap_or_else(|| {
                    MonitorAsk::new("Ready to ship", "gh release create v3.2.0 --latest")
                        .with_app("github")
                });
                let ask_mark = match &ask.app {
                    Some(app) => pro_app_tile(app.as_ref(), 22.0, cx),
                    None => div()
                        .size(px(22.0))
                        .rounded(px(6.0))
                        .bg(p.card)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_icon(ProIcon::Terminal, 13.0, p.amber_ink))
                        .into_any_element(),
                };
                div()
                    .w_full()
                    .p(px(9.0))
                    .rounded(px(9.0))
                    .bg(p.amber_soft)
                    .border_1()
                    .border_color(p.amber.opacity(0.28))
                    .flex()
                    .flex_col()
                    .gap(px(7.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(7.0))
                            .child(ask_mark)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(p.ink)
                                    .child(ask.display_title(cx).to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.amber_ink)
                                    .child(clock_str.to_string()),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .px(px(8.5))
                            .py(px(6.0))
                            .rounded(px(6.0))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .truncate()
                            .text_size(px(11.5))
                            .text_color(p.ink)
                            .child(ask.command.clone()),
                    )
                    .into_any_element()
            }
            MonitorState::Finished => {
                let outcome = self.outcome.as_ref();
                let note = outcome
                    .and_then(|o| o.display_note(cx))
                    .unwrap_or_else(|| tr(cx, "Run finished cleanly", "智能体任务已顺利完成"))
                    .to_string();
                let diff_tuple = outcome.and_then(|o| Some((o.added?, o.removed?)));

                div()
                    .h(px(36.0))
                    .px(px(10.0))
                    .rounded(px(8.0))
                    .bg(p.green_soft)
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .child(pro_check_draw(
                        pro_eid(&self.id, "well-done-chk"),
                        true,
                        15.0,
                        false,
                        p.green,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(note),
                    )
                    .when_some(diff_tuple, |row, (add, rem)| {
                        row.child(
                            div()
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .child(pro_rolling_number(
                                    pro_eid(&self.id, "well-add"),
                                    add as f64,
                                    0,
                                    "+",
                                    "",
                                    11.5,
                                    FontWeight::MEDIUM,
                                    p.green_ink,
                                ))
                                .child(pro_rolling_number(
                                    pro_eid(&self.id, "well-rem"),
                                    rem as f64,
                                    0,
                                    "-",
                                    "",
                                    11.5,
                                    FontWeight::MEDIUM,
                                    p.red_ink,
                                )),
                        )
                    })
                    .child(div().text_size(px(11.5)).text_color(p.muted).child("19s"))
                    .into_any_element()
            }
            MonitorState::Failed | MonitorState::Cancelled => {
                let is_fail = self.state == MonitorState::Failed;
                let msg = if is_fail {
                    self.error_msg
                        .as_deref()
                        .unwrap_or_else(|| {
                            tr(cx, "Command exited with status 1", "命令执行异常退出")
                        })
                        .to_string()
                } else {
                    tr(cx, "Run stopped before shipping", "任务已在发布前终止").to_string()
                };
                div()
                    .h(px(36.0))
                    .px(px(10.0))
                    .rounded(px(8.0))
                    .bg(if is_fail { p.red_soft } else { lane_bg })
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .child(div().size(px(6.0)).rounded(px(2.0)).bg(if is_fail {
                        p.red
                    } else {
                        p.muted
                    }))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.0))
                            .text_color(if is_fail { p.red_ink } else { p.muted })
                            .child(msg),
                    )
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(p.muted)
                            .child(clock_str.to_string()),
                    )
                    .into_any_element()
            }
            MonitorState::Running | MonitorState::Paused | MonitorState::Idle => {
                let active_step = self
                    .steps
                    .iter()
                    .find(|s| s.state == MonitorStepState::Running)
                    .or_else(|| self.steps.last());
                let tile = active_step.map(|s| self.render_step_mark(s, 22.0, p, cx));
                let task_lbl = active_step
                    .map(|s| s.display_label(cx).to_string())
                    .unwrap_or_else(|| tr(cx, "Working…", "正在执行…").to_string());
                let shimmer_on = self.state == MonitorState::Running;

                div()
                    .h(px(36.0))
                    .pl(px(7.0))
                    .pr(px(10.0))
                    .rounded(px(8.0))
                    .bg(lane_bg)
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .when_some(tile, |d, t| d.child(t))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .child(pro_shimmer_text(
                                pro_eid(&self.id, "well-shimmer"),
                                task_lbl,
                                p.ink,
                                p.accent_info,
                                12.0,
                                FontWeight::MEDIUM,
                                shimmer_on,
                            )),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(12.0))
                            .text_color(p.muted)
                            .child(clock_str.to_string()),
                    )
                    .into_any_element()
            }
        }
    }

    fn render_time_strip(&self, p: &ProPalette, cx: &mut Context<Self>) -> Stateful<Div> {
        let visible_segs: Vec<MonitorStep> = self
            .steps
            .iter()
            .filter(|s| s.state != MonitorStepState::Queued)
            .cloned()
            .collect();

        let any_hover = self.hovered_step_id.is_some() || self.open_step_id.is_some();
        let mut strip = div()
            .id(pro_eid(&self.id, "time-strip"))
            .role(Role::Image)
            .aria_label(tr(cx, "Step duration breakdown", "各步骤耗时占比条"))
            .w_full()
            .h(px(6.0))
            .flex()
            .items_center()
            .gap(px(2.5));

        let total_ms: f32 = visible_segs
            .iter()
            .map(|s| s.took_ms.max(400) as f32)
            .sum::<f32>()
            .max(1.0);

        for seg in visible_segs {
            let sid = seg.id.clone();
            let hot = self.hovered_step_id.as_ref() == Some(&sid)
                || self.open_step_id.as_ref() == Some(&sid);
            let frac = (seg.took_ms.max(400) as f32 / total_ms).clamp(0.08, 1.0);
            let base_col =
                if self.state == MonitorState::Finished && seg.state != MonitorStepState::Failed {
                    p.green.opacity(if hot { 0.72 } else { 0.36 })
                } else {
                    match seg.state {
                        MonitorStepState::Running => p.blue.opacity(0.45),
                        MonitorStepState::Failed => p.red.opacity(0.55),
                        MonitorStepState::Done | MonitorStepState::Queued => {
                            if hot {
                                p.ink.opacity(0.42)
                            } else {
                                p.ink.opacity(0.14)
                            }
                        }
                    }
                };

            strip = strip.child(
                div()
                    .id(pro_eid(&self.id, format!("seg-{sid}")))
                    .h_full()
                    .w(relative(frac))
                    .min_w(px(8.0))
                    .rounded(px(3.0))
                    .bg(base_col)
                    .opacity(if any_hover && !hot { 0.38 } else { 1.0 })
                    .cursor_pointer()
                    .on_hover(cx.listener({
                        let sid = sid.clone();
                        move |this, inside: &bool, _, cx| {
                            if *inside {
                                this.hovered_step_id = Some(sid.clone());
                            } else if this.hovered_step_id.as_ref() == Some(&sid) {
                                this.hovered_step_id = None;
                            }
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.open_step_id.as_ref() == Some(&sid) {
                            this.open_step_id = None;
                        } else {
                            this.open_step_id = Some(sid.clone());
                        }
                        cx.emit(AgentMonitorEvent::StepInspected(this.open_step_id.clone()));
                        cx.notify();
                    })),
            );
        }

        if self.state == MonitorState::Waiting {
            strip = strip.child(
                div()
                    .h_full()
                    .w(px(14.0))
                    .flex_none()
                    .rounded(px(3.0))
                    .bg(p.amber.opacity(0.75)),
            );
        }

        strip
    }

    fn render_completed_steps(&self, lane_bg: Hsla, p: &ProPalette, cx: &mut Context<Self>) -> Div {
        let finished: Vec<MonitorStep> = self
            .steps
            .iter()
            .filter(|s| matches!(s.state, MonitorStepState::Done | MonitorStepState::Failed))
            .cloned()
            .collect();

        let any_hover = self.hovered_step_id.is_some() || self.open_step_id.is_some();
        let mut list = div().mx(px(-4.0)).flex().flex_col().gap(px(2.0));

        for (ord, st) in finished.into_iter().enumerate() {
            let sid = st.id.clone();
            let is_open = self.open_step_id.as_ref() == Some(&sid);
            let is_hot = is_open || self.hovered_step_id.as_ref() == Some(&sid);
            let icon = self.render_step_mark(&st, 18.0, p, cx);
            let lbl = st.display_label(cx).to_string();
            let dur = format_step_duration(st.took_ms);
            let failed = st.state == MonitorStepState::Failed;

            let sid_click = sid.clone();
            let sid_hover = sid.clone();
            let row = pro_tactile(
                pro_eid(&self.id, format!("step-row-{sid}")),
                format!("{lbl} – {dur}"),
                7.0,
                if is_open {
                    lane_bg
                } else {
                    transparent_black()
                },
                lane_bg,
                transparent_black(),
                if is_open { p.line } else { transparent_black() },
                is_open,
                div()
                    .h(px(28.0))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(icon)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.0))
                            .text_color(p.text)
                            .child(lbl.clone()),
                    )
                    .when(!failed, |d| {
                        d.child(pro_check_draw(
                            pro_eid(&self.id, format!("st-chk-{sid}")),
                            true,
                            12.0,
                            false,
                            p.green,
                        ))
                    })
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(12.0))
                            .text_color(if failed { p.red_ink } else { p.faint })
                            .child(dur.clone()),
                    ),
                cx,
                move |this, _, cx| {
                    if this.open_step_id.as_ref() == Some(&sid_click) {
                        this.open_step_id = None;
                    } else {
                        this.open_step_id = Some(sid_click.clone());
                    }
                    cx.emit(AgentMonitorEvent::StepInspected(this.open_step_id.clone()));
                    cx.notify();
                },
            )
            .opacity(if any_hover && !is_hot { 0.44 } else { 1.0 })
            .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                if *inside {
                    this.hovered_step_id = Some(sid_hover.clone());
                } else if this.hovered_step_id.as_ref() == Some(&sid_hover) {
                    this.hovered_step_id = None;
                }
                cx.notify();
            }));

            let tool_str = st
                .tool_name
                .clone()
                .unwrap_or_else(|| SharedString::from(tr(cx, "Agent tool", "内置工具")));
            let status_str = if failed {
                tr(cx, "Failed", "失败").to_string()
            } else {
                tr(cx, "Done", "完成").to_string()
            };
            let step_num = format!("#{}", ord + 1);
            let detail_txt = st
                .display_detail(cx)
                .unwrap_or_else(|| tr(cx, "Completed cleanly.", "步骤已执行完毕。"))
                .to_string();
            let output_txt = st.output.clone();
            let lbl_tool = tr(cx, "Tool", "工具").to_string();
            let lbl_state = tr(cx, "State", "状态").to_string();
            let lbl_took = tr(cx, "Took", "耗时").to_string();
            let lbl_step = tr(cx, "Step", "序号").to_string();
            let card_c = p.card;
            let line_c = p.line;
            let muted_c = p.muted;
            let ink_c = p.ink;
            let text_c = p.text;
            let red_c = p.red_ink;

            let detail_drawer = pro_fold(
                pro_eid(&self.id, format!("step-fold-{sid}")),
                is_open,
                98.0,
                0.0,
                move || {
                    div()
                        .mx(px(4.0))
                        .my(px(4.0))
                        .p(px(10.0))
                        .rounded(px(9.0))
                        .bg(card_c)
                        .border_1()
                        .border_color(line_c)
                        .flex()
                        .flex_col()
                        .gap(px(7.0))
                        .child(
                            div()
                                .rounded(px(6.0))
                                .border_1()
                                .border_color(line_c)
                                .overflow_hidden()
                                .flex()
                                .child(fact_cell(&lbl_tool, tool_str.as_ref(), ink_c, muted_c))
                                .child(fact_cell(
                                    &lbl_state,
                                    &status_str,
                                    if failed { red_c } else { ink_c },
                                    muted_c,
                                ))
                                .child(fact_cell(&lbl_took, &dur, ink_c, muted_c))
                                .child(fact_cell(&lbl_step, &step_num, ink_c, muted_c)),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .line_height(px(17.5))
                                .text_color(text_c)
                                .child(detail_txt),
                        )
                        .when_some(output_txt, |d, out_line| {
                            d.child(
                                div()
                                    .w_full()
                                    .px(px(8.0))
                                    .py(px(5.0))
                                    .rounded(px(6.0))
                                    .bg(lane_bg)
                                    .truncate()
                                    .text_size(px(11.0))
                                    .text_color(ink_c)
                                    .child(out_line),
                            )
                        })
                        .into_any_element()
                },
            );

            list = list.child(div().flex().flex_col().child(row).child(detail_drawer));
        }

        list
    }

    fn render_footer(&self, _lane_bg: Hsla, p: &ProPalette, cx: &mut Context<Self>) -> Div {
        let done_n = self
            .steps
            .iter()
            .filter(|s| s.state == MonitorStepState::Done)
            .count();
        let total_n = self.steps.len();

        let controls: AnyElement = match self.state {
            MonitorState::Waiting => div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(pro_btn(
                    pro_eid(&self.id, "approve-btn"),
                    tr(cx, "Approve", "批准执行"),
                    Some(ProIcon::Check),
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    |this, _, cx| {
                        this.approve(cx);
                    },
                ))
                .child(pro_btn(
                    pro_eid(&self.id, "deny-btn"),
                    tr(cx, "Deny", "拒绝"),
                    Some(ProIcon::Close),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    |this, _, cx| {
                        this.deny(cx);
                    },
                ))
                .into_any_element(),
            MonitorState::Running => div()
                .flex()
                .items_center()
                .gap(px(5.0))
                .child(pro_btn(
                    pro_eid(&self.id, "pause-btn"),
                    tr(cx, "Pause", "暂停"),
                    Some(ProIcon::Pause),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    |this, _, cx| {
                        this.pause(cx);
                    },
                ))
                .child(pro_btn(
                    pro_eid(&self.id, "cancel-btn"),
                    tr(cx, "Cancel", "取消"),
                    Some(ProIcon::Close),
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    |this, _, cx| {
                        this.cancel(cx);
                    },
                ))
                .into_any_element(),
            MonitorState::Paused => div()
                .flex()
                .items_center()
                .gap(px(5.0))
                .child(pro_btn(
                    pro_eid(&self.id, "resume-btn"),
                    tr(cx, "Resume", "继续"),
                    Some(ProIcon::Play),
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    |this, _, cx| {
                        this.resume(cx);
                    },
                ))
                .child(pro_btn(
                    pro_eid(&self.id, "cancel-paused-btn"),
                    tr(cx, "Cancel", "取消"),
                    None,
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    |this, _, cx| {
                        this.cancel(cx);
                    },
                ))
                .into_any_element(),
            MonitorState::Finished
            | MonitorState::Failed
            | MonitorState::Cancelled
            | MonitorState::Idle => pro_btn(
                pro_eid(&self.id, "replay-run-btn"),
                tr(cx, "Replay run", "重新运行"),
                Some(ProIcon::Refresh),
                ProBtnStyle::Secondary,
                true,
                cx,
                |this, _, cx| {
                    this.retry_or_replay(cx);
                },
            )
            .into_any_element(),
        };

        let log_active = self.log_open;
        let log_btn = pro_tactile(
            pro_eid(&self.id, "open-log-btn"),
            tr(cx, "Open full trace log", "打开完整追踪日志"),
            7.0,
            if log_active {
                p.chip
            } else {
                transparent_black()
            },
            p.hover,
            transparent_black(),
            if log_active {
                p.line
            } else {
                transparent_black()
            },
            log_active,
            div()
                .h(px(26.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .gap(px(3.5))
                .text_size(px(12.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if log_active { p.ink } else { p.muted })
                .child(tr(cx, "Log", "日志"))
                .child(pro_svg("0 0 16 16", EXT_LOG_SVG, 11.0, p.muted)),
            cx,
            |this, _, cx| {
                this.log_open = !this.log_open;
                cx.emit(AgentMonitorEvent::OpenLog);
                cx.notify();
            },
        )
        .w_auto()
        .flex_none();

        div()
            .pt(px(2.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .child(controls)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .justify_end()
                    .child(pro_rolling_number(
                        pro_eid(&self.id, "done-step-cnt"),
                        done_n as f64,
                        0,
                        "",
                        format!("/{total_n}"),
                        11.0,
                        FontWeight::NORMAL,
                        p.faint,
                    )),
            )
            .child(log_btn)
    }

    fn render_log_drawer(&self, lane_bg: Hsla, p: &ProPalette) -> AnyElement {
        let lines: Vec<(String, String, bool)> = self
            .steps
            .iter()
            .filter(|s| s.state != MonitorStepState::Queued)
            .map(|s| {
                let cmd = s
                    .output
                    .as_ref()
                    .map(|o| o.to_string())
                    .unwrap_or_else(|| s.label.to_string());
                (
                    format_step_duration(s.took_ms),
                    cmd,
                    s.state == MonitorStepState::Failed,
                )
            })
            .collect();
        let p_copy = *p;
        pro_fold(
            pro_eid(&self.id, "trace-log-fold"),
            self.log_open,
            104.0,
            4.0,
            move || {
                let mut col = div()
                    .w_full()
                    .p(px(9.0))
                    .rounded(px(8.0))
                    .bg(lane_bg)
                    .border_1()
                    .border_color(p_copy.line)
                    .flex()
                    .flex_col()
                    .gap(px(4.5));
                for (dur, line, failed) in lines {
                    col = col.child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .gap(px(7.0))
                            .text_size(px(11.0))
                            .child(
                                div()
                                    .w(px(34.0))
                                    .flex_none()
                                    .text_color(if failed { p_copy.red_ink } else { p_copy.muted })
                                    .child(dur),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_color(p_copy.ink)
                                    .child(line),
                            ),
                    );
                }
                col.into_any_element()
            },
        )
    }
}

fn fact_cell(label: &str, value: &str, val_color: Hsla, lbl_color: Hsla) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .px(px(7.5))
        .py(px(5.0))
        .flex()
        .flex_col()
        .gap(px(1.0))
        .child(
            div()
                .truncate()
                .text_size(px(10.0))
                .text_color(lbl_color)
                .child(label.to_string()),
        )
        .child(
            div()
                .truncate()
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(val_color)
                .child(value.to_string()),
        )
}

fn default_steps() -> Vec<MonitorStep> {
    vec![
        MonitorStep::new("issues", "Collected 14 issues", MonitorStepState::Done)
            .with_label_zh("收集 14 个里程碑问题")
            .with_app("linear", "Linear")
            .with_took_ms(1_800)
            .with_io_details(
                "Queried closed milestone issues tagged for web-app v3.2.",
                "查询 web-app v3.2 里程碑下已关闭的全部问题单。",
                "linear issues list --milestone v3.2 -> 14 closed",
            ),
        MonitorStep::new("diff", "Compared v3.1…main", MonitorStepState::Done)
            .with_label_zh("比对 v3.1…main 提交差异")
            .with_app("github", "GitHub")
            .with_took_ms(2_400)
            .with_io_details(
                "Inspected 28 merged commits and verified CI checks on main.",
                "检查 28 次合入提交并核验主干 CI 状态。",
                "gh api repos/kense/web-app/compare/v3.1.0...main -> +142 -28",
            ),
        MonitorStep::new("notes", "Drafted release notes", MonitorStepState::Done)
            .with_label_zh("撰写版本发布说明")
            .with_app("notion", "Notion")
            .with_took_ms(3_100)
            .with_io_details(
                "Grouped highlights, breaking changes, and contributor credits into RELEASE.md.",
                "将核心特性、变更项与贡献者整理生成 RELEASE.md。",
                "notion page export release-v3-2 -> RELEASE.md (64 lines)",
            ),
        MonitorStep::new(
            "ship",
            "Publishing release v3.2.0",
            MonitorStepState::Queued,
        )
        .with_label_zh("发布正式版 v3.2.0")
        .with_app("github", "GitHub")
        .with_took_ms(2_100)
        .with_io_details(
            "Created annotated tag v3.2.0 and uploaded signed release artifacts.",
            "创建签名标签 v3.2.0 并上传构建产物。",
            "gh release create v3.2.0 --latest -> https://github.com/kense/web-app/releases/v3.2.0",
        ),
        MonitorStep::new(
            "announce",
            "Posted ship note to #releases",
            MonitorStepState::Queued,
        )
        .with_label_zh("向 #releases 频道同步上线通知")
        .with_app("slack", "Slack")
        .with_took_ms(1_200)
        .with_io_details(
            "Notified engineering and support channels with the v3.2 changelog link.",
            "向研发与支持频道发送 v3.2 更新日志摘要。",
            "slack chat.postMessage #releases -> ts 1732409812.0042",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        AgentMonitor, MonitorState, MonitorStepState, format_live_clock, format_step_duration,
    };
    use gpui::{AppContext as _, TestAppContext};

    #[test]
    fn duration_and_clock_formatters_match_compact_metrics() {
        assert_eq!(format_step_duration(1_800), "1.8s");
        assert_eq!(format_step_duration(19_000), "19s");
        assert_eq!(format_step_duration(64_000), "1:04");
        assert_eq!(format_live_clock(8), "0:08");
        assert_eq!(format_live_clock(84), "1:24");
    }

    #[gpui::test]
    fn approve_deny_pause_and_fold_transitions_work(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let view = cx.new(|cx| AgentMonitor::new("test-monitor", cx));

        view.update(cx, |v, cx| {
            assert_eq!(v.state(), MonitorState::Waiting);
            assert!(!v.is_folded());

            v.set_folded(true, cx);
            assert!(v.is_folded());
            v.set_folded(false, cx);
            assert!(!v.is_folded());

            assert!(v.approve(cx));
            assert_eq!(v.state(), MonitorState::Running);
            assert!(
                v.steps()
                    .iter()
                    .any(|s| s.state == MonitorStepState::Running)
            );

            v.pause(cx);
            assert_eq!(v.state(), MonitorState::Paused);
            v.resume(cx);
            assert_eq!(v.state(), MonitorState::Running);

            v.retry_or_replay(cx);
            assert_eq!(v.state(), MonitorState::Waiting);
            assert!(v.deny(cx));
            assert_eq!(v.state(), MonitorState::Cancelled);
        });
    }
}
