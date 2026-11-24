//! Calm unified inbox of background agent tasks with rolling filter counts, unfolding step timelines, and FLIP group glides.
//!
//! Collects background agent runs across `All`, `Needs you`, `Running`, and `Done` filter tabs
//! (`pro_pill_indicator` + `pro_rolling_number`), supports switching between unified `List` and
//! 3-column `Board` layouts, ticks live elapsed clocks, unfolds recent steps and inline
//! `Approve` / `Reject` / `Retry` / `Open` actions via [`pro_fold`], and smoothly glides tasks
//! vertically or across lanes when their status moves them to a new group.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, ProSelectionSensor, pro_btn, pro_check_draw, pro_ease, pro_eid,
        pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected, pro_pill_indicator,
        pro_rolling_number, pro_selection_sensor, pro_skeleton_bar, pro_swap_label, pro_switch,
        pro_tactile, pro_toast_banner, tr,
    },
    primitives::{
        control::activate,
        family::{PhaseClock, Slide, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    rc::Rc,
};
use web_time::Instant;

const SPARK_SVG: &str = r#"<path d="M7 1.2c.4 2.9 1.9 4.4 4.8 4.8-2.9.4-4.4 1.9-4.8 4.8C6.6 7.9 5.1 6.4 2.2 6 5.1 5.6 6.6 4.1 7 1.2Z" fill="currentColor"></path><circle cx="11.6" cy="11.4" r="1.1" fill="currentColor" opacity="0.55"></circle>"#;
const BOARD_ICON_SVG: &str = r#"<rect x="1.8" y="2.2" width="3.2" height="9.6" rx="1" stroke="currentColor" stroke-width="1.25"></rect><rect x="8.9" y="2.2" width="3.2" height="6.4" rx="1" stroke="currentColor" stroke-width="1.25"></rect>"#;
const LIST_ICON_SVG: &str = r#"<path d="M2.2 3.6h9.6M2.2 7h9.6M2.2 10.4h9.6" stroke="currentColor" stroke-width="1.25" stroke-linecap="round"></path>"#;
const SPIN_RING_SVG: &str = r#"<circle cx="6" cy="6" r="4.5" stroke="currentColor" stroke-opacity="0.25" stroke-width="1.6"></circle><path d="M6 1.5A4.5 4.5 0 0 1 10.5 6" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"></path>"#;
const WRENCH_TINY_SVG: &str = r#"<path d="M7.6 1.6a2.6 2.6 0 0 0-2.4 3.5L1.7 8.6a1.1 1.1 0 0 0 1.6 1.6l3.5-3.5a2.6 2.6 0 0 0 3.5-2.4l-1.5 1.1-1.5-.4-.4-1.5 1.1-1.5c-.1 0-.3-.4-.4-.4Z" stroke="currentColor" stroke-width="1.15" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const SEARCH_TINY_SVG: &str = r#"<circle cx="5.2" cy="5.2" r="3.3" stroke="currentColor" stroke-width="1.15"></circle><path d="m7.7 7.7 2.6 2.6" stroke="currentColor" stroke-width="1.15" stroke-linecap="round"></path>"#;

/// Status filter tab in [`AgentInbox`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AgentInboxFilter {
    /// Show all groups (`Needs you`, `Running`, and `Done`).
    #[default]
    All,
    /// Show tasks waiting on user approval or attention (`Waiting` + `Failed`).
    NeedsYou,
    /// Show actively running background tasks.
    Running,
    /// Show completed tasks.
    Done,
}

impl AgentInboxFilter {
    const ALL: [Self; 4] = [Self::All, Self::NeedsYou, Self::Running, Self::Done];

    fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::NeedsYou => "needs",
            Self::Running => "running",
            Self::Done => "done",
        }
    }

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::All => tr(cx, "All", "全部"),
            Self::NeedsYou => tr(cx, "Needs you", "待处理"),
            Self::Running => tr(cx, "Running", "运行中"),
            Self::Done => tr(cx, "Done", "已完成"),
        }
    }
}

/// Layout mode of [`AgentInbox`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AgentInboxView {
    /// Calm vertical grouped list (`Needs you`, `Running`, `Done`).
    #[default]
    List,
    /// Three side-by-side kanban columns on wide containers (`>= 620px`).
    Board,
}

impl AgentInboxView {
    fn key(self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Board => "board",
        }
    }
}

/// Live execution state of an [`AgentTask`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AgentTaskStatus {
    /// Actively executing tools in the background.
    #[default]
    Running,
    /// Paused waiting for human approval (`Needs you` group).
    Waiting,
    /// Stopped due to an error (`Needs you` group, offers `Retry`).
    Failed,
    /// Finished cleanly (`Done` group).
    Done,
}

impl AgentTaskStatus {
    fn group_index(self) -> usize {
        match self {
            Self::Waiting | Self::Failed => 0, // Needs you
            Self::Running => 1,                // Running
            Self::Done => 2,                   // Done
        }
    }

    fn word(self, cx: &App) -> &'static str {
        match self {
            Self::Running => tr(cx, "Running", "运行中"),
            Self::Waiting => tr(cx, "Waiting", "待确认"),
            Self::Failed => tr(cx, "Failed", "失败"),
            Self::Done => tr(cx, "Done", "已完成"),
        }
    }
}

/// Single step entry inside an [`AgentTask`] unfolded history timeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentTaskStep {
    /// Concise step summary.
    pub label: SharedString,
    /// Optional relative timestamp (`"2m ago"`, `"now"`).
    pub time: Option<SharedString>,
}

impl AgentTaskStep {
    /// Construct a new [`AgentTaskStep`].
    pub fn new(label: impl Into<SharedString>, time: impl Into<SharedString>) -> Self {
        let t: SharedString = time.into();
        Self {
            label: label.into(),
            time: if t.is_empty() { None } else { Some(t) },
        }
    }
}

/// Background agent task entry in [`AgentInbox`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentTask {
    /// Unique task identifier.
    pub id: SharedString,
    /// Short task title.
    pub title: SharedString,
    /// One- or two-sentence summary of what the task is doing or waiting on.
    pub description: Option<SharedString>,
    /// Responsible model or agent label (`"Claude Opus 4.6"`).
    pub agent: SharedString,
    /// Current status (`Running`, `Waiting`, `Failed`, `Done`).
    pub status: AgentTaskStatus,
    /// Base elapsed duration in seconds (ticks live while `Running` or `Waiting`).
    pub elapsed_secs: u64,
    /// Optional count of sources/documents read.
    pub sources: Option<u32>,
    /// Optional count of tool calls invoked.
    pub tools: Option<u32>,
    /// Recent step log (oldest first) displayed when the row unfolds.
    pub steps: Vec<AgentTaskStep>,
}

impl AgentTask {
    /// Construct a new [`AgentTask`].
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        agent: impl Into<SharedString>,
        status: AgentTaskStatus,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            agent: agent.into(),
            status,
            elapsed_secs: 0,
            sources: None,
            tools: None,
            steps: Vec::new(),
        }
    }

    /// Set the task description paragraph.
    pub fn with_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set base elapsed time in seconds.
    pub fn with_elapsed_secs(mut self, elapsed_secs: u64) -> Self {
        self.elapsed_secs = elapsed_secs;
        self
    }

    /// Set source and tool call counters shown in the card footer.
    pub fn with_counts(mut self, sources: Option<u32>, tools: Option<u32>) -> Self {
        self.sources = sources;
        self.tools = tools;
        self
    }

    /// Provide recent chronological steps shown when unfolded.
    pub fn with_steps(mut self, steps: Vec<AgentTaskStep>) -> Self {
        self.steps = steps;
        self
    }
}

/// Strongly typed events emitted by [`AgentInbox`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentInboxEvent {
    /// Emitted when switching the status filter tab (`All`, `NeedsYou`, `Running`, `Done`).
    FilterChanged(AgentInboxFilter),
    /// Emitted when switching between `List` and `Board` layouts.
    ViewChanged(AgentInboxView),
    /// Emitted when a task row unfolds or closes (`Some(id)` or `None`).
    ExpandedChanged(Option<SharedString>),
    /// Emitted when `Approve` is clicked on a waiting task.
    Approve(SharedString),
    /// Emitted when `Reject` is clicked on a waiting task.
    Reject(SharedString),
    /// Emitted when `Retry` is clicked on a failed task.
    Retry(SharedString),
    /// Emitted when `Open` is clicked on a task.
    Open(SharedString),
    /// Emitted when the optional header action button is pressed.
    HeaderAction,
}

impl EventEmitter<AgentInboxEvent> for AgentInbox {}

struct FlipCardState {
    group: usize,
    last_rel: Option<(f32, f32)>,
    from_delta: (f32, f32),
    pending_glide: Option<(f32, f32)>,
    clock: PhaseClock,
}

#[derive(IntoElement)]
struct FlipTaskWrap {
    id: ElementId,
    group: usize,
    origin_cell: Rc<Cell<(f32, f32)>>,
    child: AnyElement,
}

impl RenderOnce for FlipTaskWrap {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let init_grp = self.group;
        let st = w.use_keyed_state(pro_eid(&self.id, "flip"), cx, |_, _| FlipCardState {
            group: init_grp,
            last_rel: None,
            from_delta: (0.0, 0.0),
            pending_glide: None,
            clock: PhaseClock::default(),
        });

        let (dx, dy) = st.update(cx, |s, cx| {
            if s.group != self.group {
                s.group = self.group;
                s.pending_glide = s.last_rel;
            }
            let age = s.clock.time(0.32, w, cx);
            let t = pro_ease((age / 0.32).clamp(0.0, 1.0));
            if t >= 0.999 || disabled_motion(cx) {
                (0.0, 0.0)
            } else {
                (s.from_delta.0 * (1.0 - t), s.from_delta.1 * (1.0 - t))
            }
        });

        let st_pre = st.clone();
        let origin_cell = self.origin_cell;
        let sensor = canvas(
            move |bounds, window, app| {
                let (ox, oy) = origin_cell.get();
                let rx = f32::from(bounds.origin.x) - ox - dx;
                let ry = f32::from(bounds.origin.y) - oy - dy;
                st_pre.update(app, |s, cx| {
                    if let Some((old_x, old_y)) = s.pending_glide.take() {
                        let d_x = (old_x - rx).clamp(-420.0, 420.0);
                        let d_y = (old_y - ry).clamp(-420.0, 420.0);
                        if d_x.abs() > 1.5 || d_y.abs() > 1.5 {
                            s.from_delta = (d_x, d_y);
                            s.clock.restart(cx);
                            window.request_animation_frame();
                        }
                    }
                    s.last_rel = Some((rx, ry));
                });
            },
            |_, _, _, _| (),
        )
        .absolute()
        .inset_0();

        div()
            .relative()
            .left(px(dx))
            .top(px(dy))
            .w_full()
            .child(sensor)
            .child(self.child)
    }
}

pub(crate) fn format_elapsed_clock(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3_600 {
        format!("{}m {:02}s", secs / 60, secs % 60)
    } else {
        format!("{}h {:02}m", secs / 3_600, (secs % 3_600) / 60)
    }
}

pub(crate) fn count_inbox_tasks(tasks: &[AgentTask], filter: AgentInboxFilter) -> usize {
    tasks
        .iter()
        .filter(|t| match filter {
            AgentInboxFilter::All => true,
            AgentInboxFilter::NeedsYou => {
                matches!(t.status, AgentTaskStatus::Waiting | AgentTaskStatus::Failed)
            }
            AgentInboxFilter::Running => t.status == AgentTaskStatus::Running,
            AgentInboxFilter::Done => t.status == AgentTaskStatus::Done,
        })
        .count()
}

fn default_inbox_tasks(cx: &App) -> Vec<AgentTask> {
    vec![
        AgentTask::new(
            "migrate",
            tr(cx, "Migrate users table", "迁移用户核心表"),
            "Claude Opus 4.6",
            AgentTaskStatus::Waiting,
        )
        .with_description(tr(
            cx,
            "Split users into accounts and profiles on Postgres 17. The dry run on staging passed.",
            "在 Postgres 17 上将 users 拆分为账户与档案表，预发环境演练已零错通过。",
        ))
        .with_elapsed_secs(359)
        .with_counts(Some(9), Some(14))
        .with_steps(vec![
            AgentTaskStep::new(
                tr(
                    cx,
                    "Backed up schema & foreign-key constraints",
                    "备份表结构与外键约束",
                ),
                tr(cx, "4m ago", "4 分钟前"),
            ),
            AgentTaskStep::new(
                tr(
                    cx,
                    "Dry run on staging – no errors across 48k rows",
                    "预发环境演练完成（4.8 万行零报错）",
                ),
                tr(cx, "2m ago", "2 分钟前"),
            ),
            AgentTaskStep::new(
                tr(
                    cx,
                    "Wants to run migration on production cluster",
                    "请求在生产集群执行正式迁移",
                ),
                tr(cx, "1m ago", "1 分钟前"),
            ),
        ]),
        AgentTask::new(
            "crm",
            tr(cx, "Clean up CRM contacts", "清理 CRM 重复联系人"),
            "Claude Sonnet 4.6",
            AgentTaskStatus::Failed,
        )
        .with_description(tr(
            cx,
            "Matched 1,312 likely duplicates, then the merge hit the API rate limit.",
            "已匹配 1,312 组疑似重复联系人，批量合并时触发 API 速率限制。",
        ))
        .with_elapsed_secs(194)
        .with_counts(Some(2), Some(6))
        .with_steps(vec![
            AgentTaskStep::new(
                tr(
                    cx,
                    "Indexed 14,200 HubSpot contact records",
                    "扫描 14,200 条 HubSpot 联系人档案",
                ),
                tr(cx, "3m ago", "3 分钟前"),
            ),
            AgentTaskStep::new(
                tr(
                    cx,
                    "429 Too Many Requests on merge batch #4",
                    "第 4 批合并请求返回 HTTP 429 限流",
                ),
                tr(cx, "just now", "刚刚"),
            ),
        ]),
        AgentTask::new(
            "webhooks",
            tr(cx, "Review billing webhooks", "审查账单 Webhook"),
            "Claude Sonnet 4.6",
            AgentTaskStatus::Running,
        )
        .with_description(tr(
            cx,
            "Billing webhooks: reading the diff and the test run, writing inline PR comments.",
            "正在阅读计费回调变更集与测试日志，并撰写行内审查建议。",
        ))
        .with_elapsed_secs(142)
        .with_counts(Some(5), Some(8))
        .with_steps(vec![
            AgentTaskStep::new(
                tr(
                    cx,
                    "Checked out PR #489 diff (9 files)",
                    "拉取 PR #489 变更（9 个文件）",
                ),
                tr(cx, "2m ago", "2 分钟前"),
            ),
            AgentTaskStep::new(
                tr(
                    cx,
                    "Writing idempotency comment on stripe_webhook.rs…",
                    "正在为 stripe_webhook.rs 补充幂等校验批注…",
                ),
                tr(cx, "now", "当前"),
            ),
        ]),
        AgentTask::new(
            "notes",
            tr(cx, "Release notes v2.4", "整理 v2.4 发布说明"),
            "GPT-5",
            AgentTaskStatus::Running,
        )
        .with_description(tr(
            cx,
            "Grouping 23 merged pull requests into features and fixes.",
            "正在将 23 个已合并拉取请求归类为新特性与问题修复。",
        ))
        .with_elapsed_secs(84)
        .with_counts(Some(23), Some(4))
        .with_steps(vec![
            AgentTaskStep::new(
                tr(
                    cx,
                    "Fetched 23 merged PRs since v2.3.0",
                    "汇总自 v2.3.0 以来的 23 个 PR",
                ),
                tr(cx, "1m ago", "1 分钟前"),
            ),
            AgentTaskStep::new(
                tr(
                    cx,
                    "Drafting customer-facing highlights…",
                    "正在生成面向用户的更新摘要…",
                ),
                tr(cx, "now", "当前"),
            ),
        ]),
        AgentTask::new(
            "digest",
            tr(cx, "Weekly metrics digest", "生成周度核心指标简报"),
            "Claude Haiku 4.5",
            AgentTaskStatus::Done,
        )
        .with_description(tr(
            cx,
            "Pulled numbers from three dashboards and posted the summary to #metrics.",
            "已汇总三张业务看板指标并将周报发布至 #metrics 频道。",
        ))
        .with_elapsed_secs(252)
        .with_counts(Some(3), Some(7))
        .with_steps(vec![
            AgentTaskStep::new(
                tr(
                    cx,
                    "Queried revenue, activation, and retention",
                    "拉取营收、激活与留存指标",
                ),
                tr(cx, "18m ago", "18 分钟前"),
            ),
            AgentTaskStep::new(
                tr(
                    cx,
                    "Posted formatted summary to #metrics",
                    "已推送排版摘要至 #metrics",
                ),
                tr(cx, "14m ago", "14 分钟前"),
            ),
        ]),
    ]
}

/// Calm unified list and kanban board for background agent tasks.
pub struct AgentInbox {
    id: ElementId,
    title: Option<SharedString>,
    header_action_label: Option<SharedString>,
    tasks: Vec<AgentTask>,
    task_reset_at: HashMap<SharedString, Instant>,
    filter: AgentInboxFilter,
    view: AgentInboxView,
    open_id: Option<SharedString>,
    show_done: bool,
    archived_ids: HashSet<SharedString>,
    known_ids: HashSet<SharedString>,
    entering_clocks: HashMap<SharedString, (Slide, PhaseClock)>,
    done_clocks: HashMap<SharedString, PhaseClock>,
    created_seq: usize,
    filter_sensor: ProSelectionSensor,
    view_sensor: ProSelectionSensor,
    loading: bool,
    empty: bool,
    started_at: Instant,
    measured_w: Rc<Cell<f32>>,
    board_origin: Rc<Cell<(f32, f32)>>,
    toast: Option<(SharedString, Instant)>,
    focus: FocusHandle,
}

impl AgentInbox {
    /// Create an [`AgentInbox`] preloaded with 5 background tasks across `Needs you`, `Running`, and `Done`.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let initial_tasks = default_inbox_tasks(cx);
        let known_ids = initial_tasks.iter().map(|t| t.id.clone()).collect();
        Self {
            id: id.into(),
            title: None,
            header_action_label: None,
            tasks: initial_tasks,
            task_reset_at: HashMap::new(),
            filter: AgentInboxFilter::All,
            view: AgentInboxView::List,
            open_id: Some("migrate".into()),
            show_done: true,
            archived_ids: HashSet::new(),
            known_ids,
            entering_clocks: HashMap::new(),
            done_clocks: HashMap::new(),
            created_seq: 0,
            filter_sensor: pro_selection_sensor(),
            view_sensor: pro_selection_sensor(),
            loading: false,
            empty: false,
            started_at: Instant::now(),
            measured_w: Rc::new(Cell::new(760.0)),
            board_origin: Rc::new(Cell::new((0.0, 0.0))),
            toast: None,
            focus: cx.focus_handle(),
        }
    }

    /// Override the heading after `"Agents /"` (`"Agent inbox"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Supply custom background [`AgentTask`] items.
    pub fn with_tasks(mut self, tasks: Vec<AgentTask>) -> Self {
        self.known_ids = tasks.iter().map(|t| t.id.clone()).collect();
        self.tasks = tasks;
        self
    }

    /// Set the initial status filter tab (`All`, `NeedsYou`, `Running`, `Done`).
    pub fn with_default_filter(mut self, filter: AgentInboxFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Set the initial layout mode (`List` or `Board`).
    pub fn with_default_view(mut self, view: AgentInboxView) -> Self {
        self.view = view;
        self
    }

    /// Pre-open a specific task's step history (`None` starts all cards folded).
    pub fn with_open_id(mut self, open_id: Option<SharedString>) -> Self {
        self.open_id = open_id;
        self
    }

    /// Add an optional CTA button in the top bar (such as `"New task"`).
    pub fn with_header_action(mut self, label: impl Into<SharedString>) -> Self {
        self.header_action_label = Some(label.into());
        self
    }

    /// Render skeleton cards while background tasks load.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Render the empty inbox state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    fn set_filter(&mut self, filter: AgentInboxFilter, cx: &mut Context<Self>) {
        if self.filter != filter {
            self.filter = filter;
            cx.emit(AgentInboxEvent::FilterChanged(filter));
            cx.notify();
        }
    }

    fn set_view(&mut self, view: AgentInboxView, cx: &mut Context<Self>) {
        if self.view != view {
            self.view = view;
            cx.emit(AgentInboxEvent::ViewChanged(view));
            cx.notify();
        }
    }

    fn toggle_card(&mut self, id: SharedString, cx: &mut Context<Self>) {
        if self.open_id.as_ref() == Some(&id) {
            self.open_id = None;
        } else {
            self.open_id = Some(id);
        }
        cx.emit(AgentInboxEvent::ExpandedChanged(self.open_id.clone()));
        cx.notify();
    }

    fn approve_task(&mut self, id: SharedString, cx: &mut Context<Self>) {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
            t.status = AgentTaskStatus::Running;
            t.steps.push(AgentTaskStep::new(
                tr(
                    cx,
                    "Approved — running on production",
                    "已批准 — 正在生产环境执行",
                ),
                tr(cx, "just now", "刚刚"),
            ));
        }
        let msg: SharedString = tr(cx, "Approved — moved to Running", "已批准并移入运行中").into();
        self.toast = Some((msg, Instant::now()));
        cx.emit(AgentInboxEvent::Approve(id));
        cx.notify();
    }

    fn reject_task(&mut self, id: SharedString, cx: &mut Context<Self>) {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
            t.status = AgentTaskStatus::Failed;
            t.steps.push(AgentTaskStep::new(
                tr(cx, "Rejected by operator", "已被操作员拒绝"),
                tr(cx, "just now", "刚刚"),
            ));
        }
        let msg: SharedString = tr(cx, "Rejected task", "已拒绝该任务").into();
        self.toast = Some((msg, Instant::now()));
        cx.emit(AgentInboxEvent::Reject(id));
        cx.notify();
    }

    fn retry_task(&mut self, id: SharedString, cx: &mut Context<Self>) {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
            t.status = AgentTaskStatus::Running;
            t.elapsed_secs = 0;
            t.steps.push(AgentTaskStep::new(
                tr(
                    cx,
                    "Retrying with fresh token & backoff…",
                    "已刷新令牌并重新启动任务…",
                ),
                tr(cx, "now", "当前"),
            ));
        }
        self.task_reset_at.insert(id.clone(), Instant::now());
        let msg: SharedString = tr(cx, "Retrying — moved to Running", "已重试并移入运行中").into();
        self.toast = Some((msg, Instant::now()));
        cx.emit(AgentInboxEvent::Retry(id));
        cx.notify();
    }

    fn complete_task(&mut self, id: SharedString, cx: &mut Context<Self>) {
        if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
            t.status = AgentTaskStatus::Done;
            t.steps.push(AgentTaskStep::new(
                tr(
                    cx,
                    "Completed cleanly — checks passed",
                    "执行完成 — 全部检查已通过",
                ),
                tr(cx, "just now", "刚刚"),
            ));
        }
        let mut clock = PhaseClock::default();
        clock.restart(cx);
        self.done_clocks.insert(id, clock);
        let msg: SharedString =
            tr(cx, "Completed — moved to Done", "已完成并归入已完成分组").into();
        self.toast = Some((msg, Instant::now()));
        cx.notify();
    }

    fn archive_task(&mut self, id: SharedString, cx: &mut Context<Self>) {
        if self.open_id.as_ref() == Some(&id) {
            self.open_id = None;
            cx.emit(AgentInboxEvent::ExpandedChanged(None));
        }
        self.archived_ids.insert(id);
        let msg: SharedString = tr(cx, "Archived completed task", "已归档完成任务").into();
        self.toast = Some((msg, Instant::now()));
        cx.notify();
    }

    fn restore_archived(&mut self, cx: &mut Context<Self>) {
        for id in self.archived_ids.drain() {
            let mut slide = Slide::new(0.0);
            slide.duration(1.0, 0.26, cx);
            let mut clock = PhaseClock::default();
            clock.restart(cx);
            self.entering_clocks.insert(id, (slide, clock));
        }
        cx.notify();
    }

    fn spawn_queued_task(&mut self, cx: &mut Context<Self>) {
        self.created_seq += 1;
        let seq = self.created_seq;
        let tid: SharedString = format!("queued-{seq}").into();
        let presets: [(&str, &str, &str, &str, &str); 3] = [
            (
                "Audit dependency advisories",
                "审计依赖安全通告",
                "Scanning Cargo.lock against RustSec advisory database and preparing patch diff.",
                "正在对照 RustSec 通告库扫描 Cargo.lock 并生成最小升级补丁。",
                "Claude Sonnet 4.6",
            ),
            (
                "Backfill search embeddings",
                "回填知识库向量索引",
                "Batching 2,400 updated docs through embedding worker with rate-limit guard.",
                "正在按限流配额分批为 2,400 篇更新文档生成向量索引。",
                "GPT-5",
            ),
            (
                "Verify staging smoke suite",
                "验证预发冒烟测试集",
                "Running headless WASM & native parity checks before tagging release.",
                "正在执行预发环境双端一致性回归检查。",
                "Claude Opus 4.6",
            ),
        ];
        let (en_t, zh_t, en_d, zh_d, agent) = presets[(seq - 1) % presets.len()];
        let task = AgentTask::new(
            tid.clone(),
            tr(cx, en_t, zh_t),
            agent,
            AgentTaskStatus::Running,
        )
        .with_description(tr(cx, en_d, zh_d))
        .with_elapsed_secs(1)
        .with_counts(Some(3), Some(2))
        .with_steps(vec![AgentTaskStep::new(
            tr(
                cx,
                "Initialized sandbox & fetched context",
                "已初始化沙箱并拉取上下文",
            ),
            tr(cx, "now", "当前"),
        )]);
        self.known_ids.insert(tid.clone());
        self.task_reset_at.insert(tid.clone(), Instant::now());
        let mut slide = Slide::new(0.0);
        slide.duration(1.0, 0.26, cx);
        let mut clock = PhaseClock::default();
        clock.restart(cx);
        self.entering_clocks.insert(tid, (slide, clock));
        self.tasks.insert(0, task);
        let msg: SharedString = tr(cx, "Queued background run", "已加入后台运行队列").into();
        self.toast = Some((msg, Instant::now()));
        cx.notify();
    }

    fn open_task(&mut self, id: SharedString, cx: &mut Context<Self>) {
        if let Some(t) = self.tasks.iter().find(|t| t.id == id) {
            let msg: SharedString =
                format!("{}: {}", tr(cx, "Opened run", "已打开运行详情"), t.title).into();
            self.toast = Some((msg, Instant::now()));
        }
        cx.emit(AgentInboxEvent::Open(id));
        cx.notify();
    }

    fn live_elapsed(&self, task: &AgentTask, reduced: bool) -> u64 {
        if !matches!(
            task.status,
            AgentTaskStatus::Running | AgentTaskStatus::Waiting
        ) || reduced
        {
            return task.elapsed_secs;
        }
        let since = self
            .task_reset_at
            .get(&task.id)
            .copied()
            .unwrap_or(self.started_at);
        task.elapsed_secs + since.elapsed().as_secs()
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if ev.keystroke.key == "escape" && self.open_id.take().is_some() {
            cx.emit(AgentInboxEvent::ExpandedChanged(None));
            cx.notify();
        }
    }

    fn render_task_card(
        &mut self,
        task: &AgentTask,
        is_archived: bool,
        hide_done_card: bool,
        reduced: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let card_bg: Hsla = if p.dark {
            rgb(0x1e1e21).into()
        } else {
            rgb(0xffffff).into()
        };
        let row_hover_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.035)
        } else {
            hsla(0., 0., 0., 0.025)
        };
        let is_open = !is_archived && !hide_done_card && self.open_id.as_ref() == Some(&task.id);
        let tid = task.id.clone();

        if !self.known_ids.contains(&tid) {
            self.known_ids.insert(tid.clone());
            let mut slide = Slide::new(0.0);
            slide.duration(1.0, 0.26, cx);
            let mut clock = PhaseClock::default();
            clock.restart(cx);
            self.entering_clocks.insert(tid.clone(), (slide, clock));
        }

        let (enter_t, enter_open) = if let Some((slide, clock)) = self.entering_clocks.get_mut(&tid)
        {
            let s = slide.get(window, cx).clamp(0.0, 1.0);
            let age = clock.time(0.30, window, cx);
            (s, reduced || age > 0.012)
        } else {
            (1.0, true)
        };
        let life_open = !is_archived && !hide_done_card && enter_open;

        let done_checked = if task.status == AgentTaskStatus::Done {
            if let Some(clock) = self.done_clocks.get_mut(&tid) {
                reduced || clock.time(0.28, window, cx) > 0.012
            } else {
                true
            }
        } else {
            false
        };

        let tone_c = match task.status {
            AgentTaskStatus::Running => p.blue,
            AgentTaskStatus::Waiting => p.amber,
            AgentTaskStatus::Failed => p.red,
            AgentTaskStatus::Done => p.green,
        };

        let state_glyph: AnyElement = match task.status {
            AgentTaskStatus::Running => pro_svg("0 0 12 12", SPIN_RING_SVG, 11.5, p.blue),
            AgentTaskStatus::Waiting => div()
                .size(px(7.5))
                .rounded_full()
                .bg(p.amber)
                .into_any_element(),
            AgentTaskStatus::Failed => pro_icon(ProIcon::AlertTriangle, 12.0, p.red),
            AgentTaskStatus::Done => pro_check_draw(
                pro_eid(&self.id, format!("idone-chk-{tid}")),
                done_checked,
                13.5,
                false,
                p.green,
            ),
        };

        let live_secs = self.live_elapsed(task, reduced);
        let state_str = format!(
            "{} · {}",
            task.status.word(cx),
            format_elapsed_clock(live_secs)
        );
        let status_fg = if task.status == AgentTaskStatus::Failed {
            p.red_ink
        } else {
            p.text
        };
        let time_el: AnyElement = if live_secs < 60 {
            pro_rolling_number(
                pro_eid(&self.id, format!("isec-{tid}")),
                live_secs as f64,
                0,
                "",
                "s",
                12.5,
                FontWeight::NORMAL,
                p.muted,
            )
            .into_any_element()
        } else {
            div()
                .text_size(px(12.5))
                .text_color(p.muted)
                .child(format_elapsed_clock(live_secs))
                .into_any_element()
        };

        let mut meta_counters = div()
            .flex_none()
            .flex()
            .items_center()
            .text_size(px(12.0))
            .text_color(p.muted);
        if let Some(src) = task.sources {
            meta_counters = meta_counters.child(
                div()
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(4.5))
                    .child(pro_svg("0 0 12 12", SEARCH_TINY_SVG, 11.5, p.muted))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, format!("isrc-{tid}")),
                        src as f64,
                        0,
                        "",
                        "",
                        12.0,
                        FontWeight::NORMAL,
                        p.muted,
                    )),
            );
        }
        if let Some(tls) = task.tools {
            meta_counters = meta_counters.child(
                div()
                    .pl(px(8.0))
                    .when(task.sources.is_some(), |d| {
                        d.border_l_1().border_color(p.line)
                    })
                    .flex()
                    .items_center()
                    .gap(px(4.5))
                    .child(pro_svg("0 0 12 12", WRENCH_TINY_SVG, 11.5, p.muted))
                    .child(pro_rolling_number(
                        pro_eid(&self.id, format!("itls-{tid}")),
                        tls as f64,
                        0,
                        "",
                        "",
                        12.0,
                        FontWeight::NORMAL,
                        p.muted,
                    )),
            );
        }

        let click_id = tid.clone();
        let card_face = div()
            .w_full()
            .p(px(12.0))
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .min_w_0()
                    .child(
                        div()
                            .w(px(16.0))
                            .h(px(20.0))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_svg("0 0 14 14", SPARK_SVG, 14.0, tone_c)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(14.0))
                            .line_height(px(20.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .when(!is_open, |d| d.truncate())
                            .child(task.title.clone()),
                    )
                    .child(pro_icon(
                        if is_open {
                            ProIcon::ChevronUp
                        } else {
                            ProIcon::ChevronDown
                        },
                        13.0,
                        p.muted,
                    )),
            )
            .when_some(task.description.clone(), |d, desc| {
                d.child(
                    div()
                        .pl(px(26.0))
                        .text_size(px(13.0))
                        .line_height(px(19.0))
                        .text_color(p.text)
                        .when(!is_open, |t| t.line_clamp(2))
                        .child(desc),
                )
            })
            .child(
                div()
                    .w_full()
                    .mt(px(2.0))
                    .pl(px(26.0))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .text_size(px(12.5))
                            .text_color(status_fg)
                            .child(state_glyph)
                            .child(pro_swap_label(
                                pro_eid(&self.id, format!("istatus-{tid}")),
                                task.status.word(cx),
                                12.5,
                                FontWeight::MEDIUM,
                                status_fg,
                            ))
                            .child(div().text_color(p.faint).child("·"))
                            .child(time_el),
                    )
                    .child(meta_counters),
            );

        let card_main = pro_tactile(
            pro_eid(&self.id, format!("icard-btn-{tid}")),
            format!("{}, {}", task.title, state_str),
            10.0,
            transparent_black(),
            row_hover_bg,
            transparent_black(),
            transparent_black(),
            false,
            card_face,
            cx,
            move |this, _, cx| this.toggle_card(click_id.clone(), cx),
        )
        .w_full();

        // Unfolding step timeline + inline Approve / Reject / Retry / Complete / Archive / Open buttons (`pro_fold`).
        let agent_str = task.agent.clone();
        let steps_list = task.steps.clone();
        let status = task.status;
        let act_tid = tid.clone();
        let base_id = self.id.clone();
        let entity_weak = cx.entity().downgrade();
        let line_c = p.line;
        let muted_c = p.muted;
        let text_c = p.text;
        let ink_c = p.ink;
        let green_c = p.green;

        let actions_row: AnyElement = match status {
            AgentTaskStatus::Waiting => {
                let rej_id = act_tid.clone();
                let app_id = act_tid.clone();
                div()
                    .mt(px(10.0))
                    .flex()
                    .justify_end()
                    .gap(px(6.0))
                    .child(pro_btn(
                        pro_eid(&base_id, format!("irej-{rej_id}")),
                        tr(cx, "Reject", "拒绝"),
                        None,
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        move |this, _, cx| this.reject_task(rej_id.clone(), cx),
                    ))
                    .child(pro_btn(
                        pro_eid(&base_id, format!("iapp-{app_id}")),
                        tr(cx, "Approve", "批准"),
                        Some(ProIcon::Check),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        move |this, _, cx| this.approve_task(app_id.clone(), cx),
                    ))
                    .into_any_element()
            }
            AgentTaskStatus::Failed => {
                let opn_id = act_tid.clone();
                let ret_id = act_tid.clone();
                div()
                    .mt(px(10.0))
                    .flex()
                    .justify_end()
                    .gap(px(6.0))
                    .child(pro_btn(
                        pro_eid(&base_id, format!("iopn-{opn_id}")),
                        tr(cx, "Open", "查看日志"),
                        Some(ProIcon::ExternalArrow),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        move |this, _, cx| this.open_task(opn_id.clone(), cx),
                    ))
                    .child(pro_btn(
                        pro_eid(&base_id, format!("iret-{ret_id}")),
                        tr(cx, "Retry", "重试"),
                        Some(ProIcon::Refresh),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        move |this, _, cx| this.retry_task(ret_id.clone(), cx),
                    ))
                    .into_any_element()
            }
            AgentTaskStatus::Running => {
                let opn_id = act_tid.clone();
                let dn_id = act_tid.clone();
                div()
                    .mt(px(10.0))
                    .flex()
                    .justify_end()
                    .gap(px(6.0))
                    .child(pro_btn(
                        pro_eid(&base_id, format!("iopn-{opn_id}")),
                        tr(cx, "Open", "打开详情"),
                        Some(ProIcon::ExternalArrow),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        move |this, _, cx| this.open_task(opn_id.clone(), cx),
                    ))
                    .child(pro_btn(
                        pro_eid(&base_id, format!("idone-{dn_id}")),
                        tr(cx, "Mark done", "标记完成"),
                        Some(ProIcon::Check),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        move |this, _, cx| this.complete_task(dn_id.clone(), cx),
                    ))
                    .into_any_element()
            }
            AgentTaskStatus::Done => {
                let arch_id = act_tid.clone();
                let opn_id = act_tid.clone();
                div()
                    .mt(px(10.0))
                    .flex()
                    .justify_end()
                    .gap(px(6.0))
                    .child(pro_btn(
                        pro_eid(&base_id, format!("iarch-{arch_id}")),
                        tr(cx, "Archive", "归档"),
                        None,
                        ProBtnStyle::Ghost,
                        true,
                        cx,
                        move |this, _, cx| this.archive_task(arch_id.clone(), cx),
                    ))
                    .child(pro_btn(
                        pro_eid(&base_id, format!("iopn-{opn_id}")),
                        tr(cx, "Open", "打开详情"),
                        Some(ProIcon::ExternalArrow),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        move |this, _, cx| this.open_task(opn_id.clone(), cx),
                    ))
                    .into_any_element()
            }
        };

        let fold_tid = tid.clone();
        let fold_base_id = self.id.clone();
        let fold_section = pro_fold(
            pro_eid(&self.id, format!("ifold-{tid}")),
            is_open,
            136.0,
            0.0,
            move || {
                let _ = &entity_weak;
                let step_len = steps_list.len();
                let mut timeline = div().mt(px(6.0)).flex().flex_col().gap(px(6.0));
                for (i, st) in steps_list.into_iter().enumerate() {
                    let is_last = i + 1 == step_len;
                    let step_done = status == AgentTaskStatus::Done || !is_last;
                    let marker: AnyElement = if step_done {
                        div()
                            .absolute()
                            .left(px(-2.0))
                            .top(px(3.0))
                            .child(pro_check_draw(
                                pro_eid(&fold_base_id, format!("istp-chk-{fold_tid}-{i}")),
                                true,
                                12.0,
                                false,
                                green_c,
                            ))
                            .into_any_element()
                    } else {
                        div()
                            .absolute()
                            .left(px(1.5))
                            .top(px(6.5))
                            .size(px(5.0))
                            .rounded_full()
                            .bg(ink_c)
                            .into_any_element()
                    };
                    timeline = timeline.child(
                        div()
                            .relative()
                            .pl(px(16.0))
                            .flex()
                            .items_start()
                            .justify_between()
                            .gap(px(10.0))
                            .text_size(px(12.5))
                            .line_height(px(18.0))
                            .child(marker)
                            .when(!is_last, |d| {
                                d.child(
                                    div()
                                        .absolute()
                                        .left(px(3.5))
                                        .top(px(15.0))
                                        .bottom(px(-5.0))
                                        .w(px(1.0))
                                        .bg(line_c),
                                )
                            })
                            .child(div().flex_1().min_w_0().text_color(text_c).child(st.label))
                            .when_some(st.time, |d, tm| {
                                d.child(div().flex_none().text_color(muted_c).child(tm))
                            }),
                    );
                }

                div()
                    .mx(px(12.0))
                    .pt(px(10.0))
                    .pb(px(12.0))
                    .border_t_1()
                    .border_color(line_c)
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(muted_c)
                            .child(agent_str),
                    )
                    .when(step_len > 0, |d| d.child(timeline))
                    .child(actions_row)
                    .into_any_element()
            },
        );

        let border_c = if is_open { p.line_strong } else { p.line };
        let card_el = div()
            .w_full()
            .rounded(px(10.0))
            .bg(card_bg)
            .border_1()
            .border_color(border_c)
            .when(!p.dark, |d| d.shadow_xs())
            .flex()
            .flex_col()
            .child(card_main)
            .child(fold_section)
            .into_any_element();

        let flipped = FlipTaskWrap {
            id: pro_eid(&self.id, format!("flip-card-{tid}")),
            group: task.status.group_index(),
            origin_cell: self.board_origin.clone(),
            child: card_el,
        }
        .into_any_element();

        pro_fold(
            pro_eid(&self.id, format!("icard-life-{tid}")),
            life_open,
            108.0,
            6.0,
            move || {
                div()
                    .relative()
                    .top(px(-5.0 * (1.0 - enter_t)))
                    .w_full()
                    .child(flipped)
                    .into_any_element()
            },
        )
    }
}

impl Render for AgentInbox {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let elapsed_secs = self.started_at.elapsed().as_secs_f32();
        let lane_bg: Hsla = if p.dark {
            rgb(0x161618).into()
        } else {
            rgb(0xf4f4f5).into()
        };
        let card_bg: Hsla = if p.dark {
            rgb(0x1e1e21).into()
        } else {
            rgb(0xffffff).into()
        };
        let title_str: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Agent inbox", "智能体收件箱").into());

        if self.loading {
            if !reduced {
                window.request_animation_frame();
            }
            let phase = (elapsed_secs / 1.4).fract();
            return div()
                .id(self.id.clone())
                .role(Role::Group)
                .aria_label(title_str)
                .w_full()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(pro_skeleton_bar(160.0, 18.0, 5.0, phase, cx))
                        .child(pro_skeleton_bar(240.0, 28.0, 8.0, phase + 0.15, cx)),
                )
                .child(
                    div()
                        .p(px(6.0))
                        .rounded(px(14.0))
                        .bg(lane_bg)
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .children((0..3usize).map(|i| {
                            div()
                                .p(px(12.0))
                                .rounded(px(10.0))
                                .bg(card_bg)
                                .border_1()
                                .border_color(p.line)
                                .flex()
                                .flex_col()
                                .gap(px(8.0))
                                .child(pro_skeleton_bar(
                                    200.0,
                                    15.0,
                                    4.0,
                                    phase + i as f32 * 0.08,
                                    cx,
                                ))
                                .child(pro_skeleton_bar(
                                    320.0,
                                    13.0,
                                    4.0,
                                    phase + i as f32 * 0.12,
                                    cx,
                                ))
                        })),
                );
        }

        let has_live = self.tasks.iter().any(|t| {
            matches!(
                t.status,
                AgentTaskStatus::Running | AgentTaskStatus::Waiting
            )
        }) || self.toast.is_some();
        if has_live && !reduced {
            window.request_animation_frame();
        }

        let width_px = self.measured_w.get().max(280.0);
        let can_board = width_px >= 620.0;
        let effective_view = if can_board {
            self.view
        } else {
            AgentInboxView::List
        };

        // Top bar: Breadcrumb + Filter Tabs (`pro_pill_indicator` + `pro_rolling_number`) + `pro_switch` + View Switcher.
        let filter_sensor = self.filter_sensor.clone();
        let cur_filter = self.filter;
        let show_done_now = self.show_done;
        let live_tasks: Vec<AgentTask> = self
            .tasks
            .iter()
            .filter(|t| !self.archived_ids.contains(&t.id))
            .cloned()
            .collect();
        let tab_hover_bg = p.hover;
        let filter_strip = div()
            .id(pro_eid(&self.id, "filter-tabs"))
            .role(Role::TabList)
            .aria_label(tr(cx, "Filter tasks", "按状态筛选"))
            .relative()
            .p(px(3.0))
            .rounded(px(9.0))
            .bg(lane_bg)
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(2.0))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "filter-pill"),
                cur_filter.key(),
                filter_sensor.clone(),
                6.5,
                card_bg,
                Some(p.line),
                true,
            ))
            .children(AgentInboxFilter::ALL.into_iter().map(|flt| {
                let active = cur_filter == flt;
                let cnt = count_inbox_tasks(&live_tasks, flt) as f64;
                let focus_c = p.focus;
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("ftab-{}", flt.key())))
                        .role(Role::Tab)
                        .aria_selected(active)
                        .tab_index(0)
                        .relative()
                        .h(px(26.0))
                        .px(px(9.0))
                        .rounded(px(6.5))
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if active { p.ink } else { p.muted })
                        .when(!active, |d| d.hover(move |s| s.bg(tab_hover_bg)))
                        .cursor_pointer()
                        .focus_visible(move |s| s.border_1().border_color(focus_c))
                        .when(active, |d| d.child(pro_mark_selected(&filter_sensor)))
                        .child(flt.label(cx))
                        .child(
                            div()
                                .h(px(17.0))
                                .min_w(px(18.0))
                                .px(px(4.5))
                                .rounded(px(5.0))
                                .bg(p.chip)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(pro_rolling_number(
                                    pro_eid(&self.id, format!("fcnt-{}", flt.key())),
                                    cnt,
                                    0,
                                    "",
                                    "",
                                    11.0,
                                    FontWeight::MEDIUM,
                                    if active { p.ink } else { p.muted },
                                )),
                        ),
                    cx,
                    move |this, _, cx| this.set_filter(flt, cx),
                )
            }));

        let view_sensor = self.view_sensor.clone();
        let cur_view = effective_view;
        let view_switch = can_board.then(|| {
            div()
                .id(pro_eid(&self.id, "view-switch"))
                .role(Role::Group)
                .aria_label(tr(cx, "Layout", "视图布局"))
                .relative()
                .p(px(3.0))
                .rounded(px(9.0))
                .bg(lane_bg)
                .flex()
                .items_center()
                .gap(px(2.0))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "view-pill"),
                    cur_view.key(),
                    view_sensor.clone(),
                    6.5,
                    card_bg,
                    Some(p.line),
                    true,
                ))
                .children(
                    [
                        (AgentInboxView::List, LIST_ICON_SVG, tr(cx, "List", "列表")),
                        (
                            AgentInboxView::Board,
                            BOARD_ICON_SVG,
                            tr(cx, "Board", "看板"),
                        ),
                    ]
                    .into_iter()
                    .map(|(vw, svg_ic, lbl)| {
                        let active = cur_view == vw;
                        let focus_c = p.focus;
                        let fg = if active { p.ink } else { p.muted };
                        activate(
                            div()
                                .id(pro_eid(&self.id, format!("vw-{}", vw.key())))
                                .role(Role::RadioButton)
                                .aria_selected(active)
                                .tab_index(0)
                                .relative()
                                .h(px(26.0))
                                .px(px(9.0))
                                .rounded(px(6.5))
                                .flex()
                                .items_center()
                                .gap(px(5.0))
                                .text_size(px(12.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(fg)
                                .when(!active, |d| d.hover(move |s| s.bg(tab_hover_bg)))
                                .cursor_pointer()
                                .focus_visible(move |s| s.border_1().border_color(focus_c))
                                .when(active, |d| d.child(pro_mark_selected(&view_sensor)))
                                .child(pro_svg("0 0 14 14", svg_ic, 12.5, fg))
                                .child(lbl),
                            cx,
                            move |this, _, cx| this.set_view(vw, cx),
                        )
                    }),
                )
        });

        let show_done_toggle = (cur_filter == AgentInboxFilter::All).then(|| {
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .when(width_px >= 500.0, |d| {
                    d.child(div().text_size(px(12.0)).text_color(p.muted).child(tr(
                        cx,
                        "Show done",
                        "显示已完成",
                    )))
                })
                .child(pro_switch(
                    pro_eid(&self.id, "show-done-sw"),
                    tr(cx, "Show completed tasks", "显示已完成任务"),
                    show_done_now,
                    cx,
                    move |this, _, cx| {
                        this.show_done = !show_done_now;
                        cx.notify();
                    },
                ))
        });

        let top_bar = div()
            .w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(10.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .text_size(px(14.0))
                    .child(div().text_color(p.muted).child(tr(cx, "Agents", "智能体")))
                    .child(div().text_color(p.faint).child("/"))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .truncate()
                            .child(title_str.clone()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.0))
                    .child(filter_strip)
                    .when_some(show_done_toggle, |d, sw| d.child(sw))
                    .when_some(view_switch, |d, vs| d.child(vs))
                    .when(self.header_action_label.is_none(), |d| {
                        d.child(pro_icon_btn(
                            pro_eid(&self.id, "quick-new-task"),
                            tr(cx, "Queue background task", "加入后台任务"),
                            ProIcon::Plus,
                            28.0,
                            false,
                            cx,
                            |this, _, cx| this.spawn_queued_task(cx),
                        ))
                    })
                    .when_some(self.header_action_label.clone(), |d, lbl| {
                        d.child(pro_btn(
                            pro_eid(&self.id, "hdr-act"),
                            lbl,
                            Some(ProIcon::Plus),
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            |_, _, cx| cx.emit(AgentInboxEvent::HeaderAction),
                        ))
                    }),
            );

        if self.empty || self.tasks.is_empty() {
            return div()
                .id(self.id.clone())
                .role(Role::Group)
                .aria_label(title_str)
                .w_full()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(top_bar)
                .child(div().rounded(px(14.0)).bg(lane_bg).child(pro_empty_state(
                    ProIcon::Inbox,
                    tr(cx, "All clear in Agent inbox", "智能体收件箱已清空"),
                    tr(
                        cx,
                        "Background migrations, reviews, and digests appear here when agents run.",
                        "后台智能体启动数据迁移、代码审查或简报汇总后将在此列出。",
                    ),
                    None,
                    cx,
                )));
        }

        // Group tasks into the 3 canonical lanes (`Needs you`, `Running`, `Done`).
        let lane_defs: [(usize, AgentInboxFilter, Hsla, SharedString); 3] = [
            (
                0,
                AgentInboxFilter::NeedsYou,
                p.amber,
                tr(cx, "NEEDS YOU", "待处理").into(),
            ),
            (
                1,
                AgentInboxFilter::Running,
                p.blue,
                tr(cx, "RUNNING", "运行中").into(),
            ),
            (
                2,
                AgentInboxFilter::Done,
                p.green,
                tr(cx, "DONE", "已完成").into(),
            ),
        ];

        let is_board_cols =
            effective_view == AgentInboxView::Board && cur_filter == AgentInboxFilter::All;

        let origin_cell = self.board_origin.clone();
        let mut lanes_container = div()
            .relative()
            .w_full()
            .flex()
            .when(!is_board_cols, |d| d.flex_col())
            .when(is_board_cols, |d| d.items_start())
            .gap(px(8.0))
            .child(
                canvas(
                    move |bounds, _, _| {
                        origin_cell.set((f32::from(bounds.origin.x), f32::from(bounds.origin.y)));
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            );

        for (g_idx, flt_match, spark_c, group_title) in lane_defs {
            if cur_filter != AgentInboxFilter::All && cur_filter != flt_match {
                continue;
            }
            let group_tasks: Vec<AgentTask> = self
                .tasks
                .iter()
                .filter(|t| t.status.group_index() == g_idx)
                .cloned()
                .collect();
            let live_count = group_tasks
                .iter()
                .filter(|t| !self.archived_ids.contains(&t.id))
                .count();
            let archived_in_lane = group_tasks.len().saturating_sub(live_count);
            let hide_done_cards_in_board = g_idx == 2
                && is_board_cols
                && !show_done_now
                && cur_filter == AgentInboxFilter::All;

            let cards_el = if group_tasks.is_empty() {
                div()
                    .w_full()
                    .py(px(18.0))
                    .text_size(px(12.5))
                    .text_color(p.faint)
                    .text_center()
                    .child(tr(cx, "No tasks in this group", "该分组暂无任务"))
                    .into_any_element()
            } else {
                let mut rendered: Vec<AnyElement> = group_tasks
                    .iter()
                    .map(|t| {
                        let is_arch = self.archived_ids.contains(&t.id);
                        self.render_task_card(
                            t,
                            is_arch,
                            hide_done_cards_in_board,
                            reduced,
                            window,
                            cx,
                        )
                    })
                    .collect();
                if live_count == 0 || hide_done_cards_in_board {
                    rendered.push(
                        div()
                            .w_full()
                            .py(px(14.0))
                            .text_size(px(12.5))
                            .text_color(p.faint)
                            .text_center()
                            .child(if hide_done_cards_in_board {
                                tr(cx, "Completed tasks hidden", "已隐藏完成任务")
                            } else {
                                tr(cx, "All completed tasks archived", "已完成任务已全部归档")
                            })
                            .into_any_element(),
                    );
                }
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .children(rendered)
                    .into_any_element()
            };

            let restore_btn = (g_idx == 2 && archived_in_lane > 0).then(|| {
                pro_btn(
                    pro_eid(&self.id, "restore-archived-btn"),
                    format!("{} ({archived_in_lane})", tr(cx, "Restore", "恢复归档")),
                    Some(ProIcon::Undo),
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    |this, _, cx| this.restore_archived(cx),
                )
            });

            let lane_header = div()
                .w_full()
                .h(px(34.0))
                .px(px(10.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(pro_svg("0 0 14 14", SPARK_SVG, 13.5, spark_c.opacity(0.8)))
                .child(
                    div()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(group_title.clone()),
                )
                .child(
                    div()
                        .h(px(18.0))
                        .min_w(px(20.0))
                        .px(px(5.0))
                        .rounded(px(6.0))
                        .bg(p.chip)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_rolling_number(
                            pro_eid(&self.id, format!("lcnt-{g_idx}")),
                            live_count as f64,
                            0,
                            "",
                            "",
                            11.0,
                            FontWeight::MEDIUM,
                            p.text,
                        )),
                )
                .when_some(restore_btn, |d, btn| d.child(div().ml_auto().child(btn)));

            let lane_box = div()
                .id(pro_eid(&self.id, format!("lane-sec-{g_idx}")))
                .role(Role::Group)
                .aria_label(group_title)
                .when(is_board_cols, |d| d.flex_1().min_w_0())
                .when(!is_board_cols, |d| d.w_full())
                .p(px(4.0))
                .rounded(px(14.0))
                .bg(lane_bg)
                .flex()
                .flex_col()
                .child(lane_header)
                .child(cards_el);

            if g_idx == 2 && !is_board_cols && cur_filter == AgentInboxFilter::All {
                lanes_container = lanes_container.child(pro_fold(
                    pro_eid(&self.id, "done-lane-fold"),
                    show_done_now,
                    152.0,
                    8.0,
                    move || lane_box.into_any_element(),
                ));
            } else {
                lanes_container = lanes_container.child(lane_box);
            }
        }

        let measure_cell = self.measured_w.clone();
        div()
            .id(self.id.clone())
            .role(Role::Group)
            .aria_label(title_str)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .relative()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(12.0))
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
            .child(top_bar)
            .child(lanes_container)
            .child(pro_toast_banner(self.toast.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgentInboxFilter, AgentTask, AgentTaskStatus, count_inbox_tasks, format_elapsed_clock,
    };

    #[test]
    fn inbox_filter_counts_group_waiting_and_failed_under_needs_you() {
        let tasks = vec![
            AgentTask::new("a", "Migrate", "Opus", AgentTaskStatus::Waiting),
            AgentTask::new("b", "CRM", "Sonnet", AgentTaskStatus::Failed),
            AgentTask::new("c", "Review", "Sonnet", AgentTaskStatus::Running),
            AgentTask::new("d", "Digest", "Haiku", AgentTaskStatus::Done),
        ];

        assert_eq!(count_inbox_tasks(&tasks, AgentInboxFilter::All), 4);
        assert_eq!(count_inbox_tasks(&tasks, AgentInboxFilter::NeedsYou), 2);
        assert_eq!(count_inbox_tasks(&tasks, AgentInboxFilter::Running), 1);
        assert_eq!(count_inbox_tasks(&tasks, AgentInboxFilter::Done), 1);
        assert_eq!(format_elapsed_clock(359), "5m 59s");
    }
}
