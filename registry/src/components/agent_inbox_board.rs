//! Multi-column background agent task board with live status glyphs and step inspector.
//!
//! Organizes background agent jobs across `Needs you`, `Running`, and `Done`
//! columns (or grouped list rows) with agent filtering, inline task creation,
//! and a right-hand execution step inspector.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, ProSelectionSensor, pro_btn, pro_eid, pro_empty_state,
            pro_icon_btn, pro_mark_selected, pro_pill_indicator, pro_presence,
            pro_selection_sensor, pro_skeleton_bar, pro_view_transition, pro_view_transition_fill,
            tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, EditorShortcutEvent, InputMode},
        pro_icons::{AGENT_WAVE_MS, ProIcon, agent_pixel_cells, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const SVG_FUNNEL: &str = r#"<path d="M8.85746 12.5061C6.36901 10.6456 4.59564 8.59915 3.62734 7.44867C3.3276 7.09253 3.22938 6.8319 3.17033 6.3728C2.96811 4.8008 2.86701 4.0148 3.32795 3.5074C3.7889 3 4.60404 3 6.23433 3H17.7657C19.396 3 20.2111 3 20.672 3.5074C21.133 4.0148 21.0319 4.8008 20.8297 6.37281C20.7706 6.83191 20.6724 7.09254 20.3726 7.44867C19.403 8.60062 17.6261 10.6507 15.1326 12.5135C14.907 12.6821 14.7583 12.9567 14.7307 13.2614C14.4837 15.992 14.2559 17.4876 14.1141 18.2442C13.8853 19.4657 12.1532 20.2006 11.226 20.8563C10.6741 21.2466 10.0043 20.782 9.93278 20.1778C9.79643 19.0261 9.53961 16.6864 9.25927 13.2614C9.23409 12.9539 9.08486 12.6761 8.85746 12.5061Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_BOARD_COLS: &str = r#"<path d="M20.1088 3.89124C21.5 5.28249 21.5 7.52166 21.5 12C21.5 16.4783 21.5 18.7175 20.1088 20.1088C18.7175 21.5 16.4783 21.5 12 21.5C7.52166 21.5 5.28249 21.5 3.89124 20.1088C2.5 18.7175 2.5 16.4783 2.5 12C2.5 7.52166 2.5 5.28249 3.89124 3.89124C5.28249 2.5 7.52166 2.5 12 2.5C16.4783 2.5 18.7175 2.5 20.1088 3.89124Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M8.5 2.5V21.5" stroke="currentColor" stroke-width="1.9"></path><path d="M15.5 2.5V21.5" stroke="currentColor" stroke-width="1.9"></path>"#;

const SVG_WRENCH: &str = r#"<path d="M20.3584 13.3567C19.1689 14.546 16.9308 14.4998 13.4992 14.4998C11.2914 14.4998 9.50138 12.7071 9.50024 10.4993C9.50024 7.07001 9.454 4.83065 10.6435 3.64138C11.8329 2.45212 12.3583 2.50027 17.6274 2.50027C18.1366 2.49809 18.3929 3.11389 18.0329 3.47394L15.3199 6.18714C14.6313 6.87582 14.6294 7.99233 15.3181 8.68092C16.0068 9.36952 17.1234 9.36959 17.8122 8.68109L20.5259 5.96855C20.886 5.60859 21.5019 5.86483 21.4997 6.37395C21.4997 11.6422 21.5479 12.1675 20.3584 13.3567Z" stroke="currentColor" stroke-width="1.9"></path><path d="M13.5 14.5L7.32842 20.6716C6.22386 21.7761 4.433 21.7761 3.32843 20.6716C2.22386 19.567 2.22386 17.7761 3.32843 16.6716L9.5 10.5" stroke="currentColor" stroke-linecap="round" stroke-width="1.9"></path><path d="M5.50896 18.5H5.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_STATE_WAITING: &str = r#"<path d="M7 2.2 12.4 11.4H1.6Z" fill="currentColor" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"></path>"#;

const SVG_STATE_FAILED: &str = r#"<path fill-rule="evenodd" clip-rule="evenodd" d="M7 1.6a5.4 5.4 0 1 0 0 10.8A5.4 5.4 0 0 0 7 1.6ZM6.25 4.2a.75.75 0 0 1 1.5 0v3.1a.75.75 0 0 1-1.5 0V4.2Zm.75 6.15a.85.85 0 1 0 0-1.7.85.85 0 0 0 0 1.7Z" fill="currentColor"></path>"#;

const SVG_STATE_RUNNING_STATIC: &str = r#"<circle cx="7" cy="7" r="5" fill="none" stroke="currentColor" stroke-opacity="0.25" stroke-width="1.6"></circle><path d="M7 2a5 5 0 0 1 5 5" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"></path>"#;

const SVG_STATE_DONE: &str = r#"<circle cx="7" cy="7" r="5" fill="none" stroke="currentColor" stroke-width="1.6"></circle><circle cx="7" cy="7" r="2.6" fill="currentColor"></circle>"#;

const SVG_STATE_STOPPED: &str = r#"<circle cx="7" cy="7" r="5" fill="none" stroke="currentColor" stroke-width="1.6"></circle><rect x="5" y="5" width="4" height="4" rx="0.8" fill="currentColor"></rect>"#;

/// Active layout presentation of [`AgentInboxBoardScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoardViewMode {
    #[default]
    Board,
    List,
}

/// Top toolbar scope filter tab.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoardScope {
    #[default]
    All,
    NeedsYou,
    Running,
    Done,
}

/// Canonical three-column grouping on the board.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoardColumn {
    #[default]
    NeedsYou,
    Running,
    Done,
}

impl BoardColumn {
    const ALL: [Self; 3] = [Self::NeedsYou, Self::Running, Self::Done];

    fn matches_scope(self, scope: BoardScope) -> bool {
        match scope {
            BoardScope::All => true,
            BoardScope::NeedsYou => self == Self::NeedsYou,
            BoardScope::Running => self == Self::Running,
            BoardScope::Done => self == Self::Done,
        }
    }
}

/// Execution state of a [`BoardTask`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoardTaskStatus {
    #[default]
    Waiting,
    Failed,
    Running,
    Done,
    Stopped,
}

impl BoardTaskStatus {
    /// Map the task's execution state to its board column.
    pub fn column(self) -> BoardColumn {
        match self {
            Self::Waiting | Self::Failed => BoardColumn::NeedsYou,
            Self::Running => BoardColumn::Running,
            Self::Done | Self::Stopped => BoardColumn::Done,
        }
    }
}

/// Agent worker identity and 3x3 pixel mark color.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardAgent {
    pub id: SharedString,
    pub name: SharedString,
    pub color_rgb: u32,
}

impl BoardAgent {
    /// Construct an agent definition with `id`, display `name`, and hex `color_rgb`.
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>, color_rgb: u32) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            color_rgb,
        }
    }
}

/// Recorded step entry inside [`BoardTask`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardTaskStep {
    pub label: SharedString,
    pub time: SharedString,
}

impl BoardTaskStep {
    /// Create a timeline step with `label` and relative `time`.
    pub fn new(label: impl Into<SharedString>, time: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            time: time.into(),
        }
    }
}

/// Background agent task card displayed on [`AgentInboxBoardScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardTask {
    pub id: SharedString,
    pub title: SharedString,
    pub description: SharedString,
    pub agent: SharedString,
    pub status: BoardTaskStatus,
    pub elapsed: SharedString,
    pub sources: u32,
    pub tools: u32,
    pub steps: Vec<BoardTaskStep>,
}

/// Events emitted by [`AgentInboxBoardScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentInboxBoardEvent {
    Navigate(SharedString),
    Select(Option<SharedString>),
    Approve(SharedString),
    Reject(SharedString),
    Retry(SharedString),
    Open(SharedString),
    Create(BoardTask),
}

impl EventEmitter<AgentInboxBoardEvent> for AgentInboxBoardScreen {}

/// Filter tasks matching `scope`, optional `agent_filter`, and optional `query`.
fn filter_board_tasks<'a>(
    tasks: &'a [BoardTask],
    scope: BoardScope,
    agent_filter: Option<&str>,
    query: &str,
) -> Vec<&'a BoardTask> {
    let q = query.trim().to_ascii_lowercase();
    tasks
        .iter()
        .filter(|t| t.status.column().matches_scope(scope))
        .filter(|t| agent_filter.is_none_or(|af| t.agent.as_ref() == af))
        .filter(|t| {
            q.is_empty()
                || t.title.to_ascii_lowercase().contains(&q)
                || t.description.to_ascii_lowercase().contains(&q)
                || t.agent.to_ascii_lowercase().contains(&q)
        })
        .collect()
}

/// Return task ids in column-by-column visual navigation order (`NeedsYou`, `Running`, `Done`).
fn ordered_navigation_ids(
    tasks: &[BoardTask],
    scope: BoardScope,
    agent_filter: Option<&str>,
    query: &str,
) -> Vec<SharedString> {
    let filtered = filter_board_tasks(tasks, scope, agent_filter, query);
    let mut ids = Vec::with_capacity(filtered.len());
    for col in BoardColumn::ALL {
        if !col.matches_scope(scope) {
            continue;
        }
        for task in &filtered {
            if task.status.column() == col {
                ids.push(task.id.clone());
            }
        }
    }
    ids
}

/// Approve a waiting task, moving it to [`BoardTaskStatus::Running`].
fn apply_approve_task(tasks: &mut [BoardTask], task_id: &str) -> bool {
    let Some(task) = tasks
        .iter_mut()
        .find(|t| t.id.as_ref() == task_id && t.status == BoardTaskStatus::Waiting)
    else {
        return false;
    };
    task.status = BoardTaskStatus::Running;
    task.elapsed = "just now".into();
    task.steps.push(BoardTaskStep::new(
        "Approved – running on production",
        "now",
    ));
    true
}

/// Reject/stop a waiting task, moving it to [`BoardTaskStatus::Stopped`].
fn apply_reject_task(tasks: &mut [BoardTask], task_id: &str) -> bool {
    let Some(task) = tasks
        .iter_mut()
        .find(|t| t.id.as_ref() == task_id && t.status == BoardTaskStatus::Waiting)
    else {
        return false;
    };
    task.status = BoardTaskStatus::Stopped;
    task.steps
        .push(BoardTaskStep::new("Rejected and stopped", "now"));
    true
}

/// Retry a failed or stopped task, moving it to [`BoardTaskStatus::Running`].
fn apply_retry_task(tasks: &mut [BoardTask], task_id: &str) -> bool {
    let Some(task) = tasks.iter_mut().find(|t| {
        t.id.as_ref() == task_id
            && matches!(t.status, BoardTaskStatus::Failed | BoardTaskStatus::Stopped)
    }) else {
        return false;
    };
    task.status = BoardTaskStatus::Running;
    task.elapsed = "1s".into();
    task.steps.push(BoardTaskStep::new(
        "Retrying failed step with backoff",
        "now",
    ));
    true
}

/// Multi-column background agent task board hosted in [`AppShell`].
pub struct AgentInboxBoardScreen {
    id: ElementId,
    standalone: bool,
    workspace: SharedString,
    user_name: SharedString,
    user_email: SharedString,
    tasks: Vec<BoardTask>,
    agents: Vec<BoardAgent>,
    view_mode: BoardViewMode,
    scope: BoardScope,
    agent_filter: Option<SharedString>,
    agent_menu_open: bool,
    search_open: bool,
    open_task_id: Option<SharedString>,
    modal_open: bool,
    modal_agent_id: SharedString,
    next_created_seq: u32,
    loading: bool,
    empty: bool,
    new_task_editor: Option<Entity<Editor>>,
    search_editor: Option<Entity<Editor>>,
    pending_clear_new_task: bool,
    pending_focus_new_task: bool,
    pending_focus_search: bool,
    pending_clear_search: bool,
    focus: FocusHandle,
    shell: Entity<AppShell>,
    measured_width: Rc<Cell<f32>>,
    started_at: Instant,
}

impl AgentInboxBoardScreen {
    /// Construct an [`AgentInboxBoardScreen`] with the default `Fieldwork` background tasks.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let tasks = default_board_tasks();
        let agents = default_board_agents();
        let needs_you_count = tasks
            .iter()
            .filter(|t| t.status.column() == BoardColumn::NeedsYou)
            .count() as u32;

        let workspace: SharedString = "Fieldwork".into();
        let user_name: SharedString = "Alex Morgan".into();
        let user_email: SharedString = "alex@fieldwork.co".into();

        let shell = Self::build_shell(
            &id_str,
            workspace.clone(),
            user_name.clone(),
            user_email.clone(),
            Some(needs_you_count),
            true,
            cx,
        );

        Self {
            id: ElementId::Name(id_str),
            standalone: false,
            workspace,
            user_name,
            user_email,
            tasks,
            agents,
            view_mode: BoardViewMode::Board,
            scope: BoardScope::All,
            agent_filter: None,
            agent_menu_open: false,
            search_open: false,
            open_task_id: Some("migrate".into()),
            modal_open: false,
            modal_agent_id: "builder".into(),
            next_created_seq: 1,
            loading: false,
            empty: false,
            new_task_editor: None,
            search_editor: None,
            pending_clear_new_task: false,
            pending_focus_new_task: false,
            pending_focus_search: false,
            pending_clear_search: false,
            focus: cx.focus_handle().tab_stop(true),
            shell,
            measured_width: Rc::new(Cell::new(920.)),
            started_at: Instant::now(),
        }
    }

    fn build_shell(
        id_str: &str,
        workspace: SharedString,
        user_name: SharedString,
        user_email: SharedString,
        inbox_count: Option<u32>,
        panel_open: bool,
        cx: &mut Context<Self>,
    ) -> Entity<AppShell> {
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let mut inbox_nav =
                ShellNavItem::new("inbox", tr(shell_cx, "Inbox", "收件箱"), ProIcon::Inbox);
            if let Some(cnt) = inbox_count.filter(|&c| c > 0) {
                inbox_nav = inbox_nav.with_count(cnt);
            }
            let groups = vec![
                ShellNavGroup::primary(vec![
                    ShellNavItem::new("thread", tr(shell_cx, "Thread", "会话"), ProIcon::Thread),
                    inbox_nav,
                    ShellNavItem::new("tasks", tr(shell_cx, "Tasks", "任务"), ProIcon::Tasks),
                    ShellNavItem::new("memory", tr(shell_cx, "Memory", "记忆"), ProIcon::Memory),
                    ShellNavItem::new("skills", tr(shell_cx, "Skills", "技能"), ProIcon::Sparkles),
                ]),
                ShellNavGroup::named(
                    tr(shell_cx, "Workspace", "工作区"),
                    vec![
                        ShellNavItem::new(
                            "schedules",
                            tr(shell_cx, "Schedules", "定时计划"),
                            ProIcon::Calendar,
                        ),
                        ShellNavItem::new(
                            "activity",
                            tr(shell_cx, "Activity", "活动记录"),
                            ProIcon::Activity,
                        ),
                        ShellNavItem::new(
                            "usage",
                            tr(shell_cx, "Usage", "用量统计"),
                            ProIcon::UsageChart,
                        ),
                        ShellNavItem::new(
                            "permissions",
                            tr(shell_cx, "Permissions", "权限策略"),
                            ProIcon::Shield,
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
                .with_preset(AppShellPreset::Agent, shell_cx)
                .with_workspace(workspace)
                .with_user(user_name, user_email)
                .with_groups(groups)
                .with_active("inbox")
                .with_side_cards(false)
                .with_work_area(true, panel_open)
                .with_panel_width(348.);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_inspector(w, cx));
            s
        });
        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                cx.emit(AgentInboxBoardEvent::Navigate(label.clone()));
            }
            AppShellEvent::PanelOpen(_) => cx.notify(),
            _ => {}
        })
        .detach();
        shell
    }

    fn sync_shell_meta(&mut self, cx: &mut Context<Self>) {
        let panel_open = self.shell.read(cx).is_panel_open();
        let needs_you = if self.empty {
            None
        } else {
            Some(
                self.tasks
                    .iter()
                    .filter(|t| t.status.column() == BoardColumn::NeedsYou)
                    .count() as u32,
            )
        };
        let id_str = self.id.to_string();
        self.shell = Self::build_shell(
            &id_str,
            self.workspace.clone(),
            self.user_name.clone(),
            self.user_email.clone(),
            needs_you,
            panel_open,
            cx,
        );
    }

    /// Render without the outer [`AppShell`] frame when embedded directly.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Override the workspace title in the sidebar.
    pub fn with_workspace(
        mut self,
        workspace: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        self.workspace = workspace.into();
        self.sync_shell_meta(cx);
        self
    }

    /// Override the sidebar user account display name and email.
    pub fn with_user(
        mut self,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        self.user_name = name.into();
        self.user_email = email.into();
        self.sync_shell_meta(cx);
        self
    }

    /// Replace the initial task cards on the board.
    pub fn with_tasks(mut self, tasks: Vec<BoardTask>, cx: &mut Context<Self>) -> Self {
        self.open_task_id = tasks.first().map(|t| t.id.clone());
        self.tasks = tasks;
        self.sync_shell_meta(cx);
        self
    }

    /// Replace the agent roster used for pixel marks and filtering.
    pub fn with_agents(mut self, agents: Vec<BoardAgent>) -> Self {
        if let Some(first) = agents.first() {
            self.modal_agent_id = first.id.clone();
        }
        self.agents = agents;
        self
    }

    /// Set the initial view mode (`Board` or `List`).
    pub fn with_view_mode(mut self, mode: BoardViewMode) -> Self {
        self.view_mode = mode;
        self
    }

    /// Set which task id opens initially in the right-hand inspector (`None` folds it).
    pub fn with_open_task(mut self, task_id: Option<SharedString>, cx: &mut Context<Self>) -> Self {
        let has_open = task_id.is_some();
        self.open_task_id = task_id;
        self.shell
            .update(cx, |s, s_cx| s.set_panel_open(has_open, s_cx));
        self
    }

    /// Enable or disable the skeleton loading placeholder state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Enable or disable the empty board state.
    pub fn with_empty(mut self, empty: bool, cx: &mut Context<Self>) -> Self {
        self.empty = empty;
        self.sync_shell_meta(cx);
        self
    }

    /// Mutably toggle the skeleton loading state.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Mutably toggle the empty board state.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        self.sync_shell_meta(cx);
        cx.notify();
    }

    /// Return all tasks on the board.
    pub fn tasks(&self) -> &[BoardTask] {
        &self.tasks
    }

    /// Return the currently selected task id shown in the inspector, if any.
    pub fn open_task_id(&self) -> Option<&SharedString> {
        self.open_task_id.as_ref()
    }

    /// Access the underlying [`AppShell`] entity.
    pub fn shell(&self) -> &Entity<AppShell> {
        &self.shell
    }

    fn agent_info(&self, agent_id: &str, p: &ProPalette) -> (SharedString, Hsla) {
        if let Some(a) = self.agents.iter().find(|ag| ag.id.as_ref() == agent_id) {
            (a.name.clone(), rgb(a.color_rgb).into())
        } else {
            (SharedString::from(agent_id.to_owned()), p.blue)
        }
    }

    fn current_search_query(&self, cx: &App) -> String {
        if !self.search_open {
            return String::new();
        }
        self.search_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_owned())
            .unwrap_or_default()
    }

    fn select_task(&mut self, id: SharedString, open_panel: bool, cx: &mut Context<Self>) {
        self.open_task_id = Some(id.clone());
        if open_panel {
            self.shell
                .update(cx, |s, s_cx| s.set_panel_open(true, s_cx));
        }
        cx.emit(AgentInboxBoardEvent::Select(Some(id)));
        cx.notify();
    }

    fn approve_task(&mut self, id: SharedString, cx: &mut Context<Self>) {
        if apply_approve_task(&mut self.tasks, id.as_ref()) {
            cx.emit(AgentInboxBoardEvent::Approve(id));
            self.sync_shell_meta(cx);
            cx.notify();
        }
    }

    fn reject_task(&mut self, id: SharedString, cx: &mut Context<Self>) {
        if apply_reject_task(&mut self.tasks, id.as_ref()) {
            cx.emit(AgentInboxBoardEvent::Reject(id));
            self.sync_shell_meta(cx);
            cx.notify();
        }
    }

    fn retry_task(&mut self, id: SharedString, cx: &mut Context<Self>) {
        if apply_retry_task(&mut self.tasks, id.as_ref()) {
            cx.emit(AgentInboxBoardEvent::Retry(id));
            self.sync_shell_meta(cx);
            cx.notify();
        }
    }

    fn create_task_from_modal(&mut self, cx: &mut Context<Self>) {
        let prompt = self
            .new_task_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_owned())
            .unwrap_or_default();
        if prompt.is_empty() {
            return;
        }
        let first_line = prompt.lines().next().unwrap_or(&prompt).trim();
        let title_str: String = if first_line.chars().count() > 42 {
            let mut s: String = first_line.chars().take(41).collect();
            s.push('…');
            s
        } else {
            first_line.to_owned()
        };
        let new_id: SharedString = format!("task-new-{}", self.next_created_seq).into();
        self.next_created_seq += 1;

        let new_task = BoardTask {
            id: new_id.clone(),
            title: title_str.into(),
            description: prompt.into(),
            agent: self.modal_agent_id.clone(),
            status: BoardTaskStatus::Running,
            elapsed: "1s".into(),
            sources: 1,
            tools: 1,
            steps: vec![BoardTaskStep::new(
                "Queued task and reading workspace",
                "now",
            )],
        };

        self.empty = false;
        self.modal_open = false;
        self.pending_clear_new_task = true;
        self.tasks.insert(0, new_task.clone());
        self.open_task_id = Some(new_id.clone());
        self.shell
            .update(cx, |s, s_cx| s.set_panel_open(true, s_cx));
        cx.emit(AgentInboxBoardEvent::Create(new_task));
        cx.emit(AgentInboxBoardEvent::Select(Some(new_id)));
        self.sync_shell_meta(cx);
        cx.notify();
    }

    fn is_typing(&self, w: &Window, cx: &App) -> bool {
        self.new_task_editor
            .as_ref()
            .is_some_and(|e| e.read(cx).focus.is_focused(w))
            || self
                .search_editor
                .as_ref()
                .is_some_and(|e| e.read(cx).focus.is_focused(w))
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.new_task_editor.is_none() {
            let ph: SharedString = tr(
                cx,
                "Describe what the agent should do…",
                "描述希望智能体执行的后台任务…",
            )
            .into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Multi, w, cx);
                e.set_placeholder(ph);
                e.set_command_submit(true);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |_, _, ev: &EditEvent, cx| {
                if matches!(ev, EditEvent::Changed) {
                    cx.notify();
                }
            })
            .detach();
            cx.subscribe(&ed, |this, _, ev: &EditorShortcutEvent, cx| {
                if matches!(ev, EditorShortcutEvent::CommandSubmit) {
                    this.create_task_from_modal(cx);
                }
            })
            .detach();
            self.new_task_editor = Some(ed);
        }

        if self.search_editor.is_none() {
            let ph: SharedString = tr(cx, "Filter tasks…", "筛选任务…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 12.5;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
                EditEvent::Changed | EditEvent::Submitted => cx.notify(),
                EditEvent::Escape => {
                    this.search_open = false;
                    this.pending_clear_search = true;
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.search_editor = Some(ed);
        }

        if self.pending_clear_new_task {
            self.pending_clear_new_task = false;
            if let Some(ed) = &self.new_task_editor {
                ed.update(cx, |e, cx| e.set_value("", w, cx));
            }
        }
        if self.pending_focus_new_task {
            self.pending_focus_new_task = false;
            if let Some(ed) = &self.new_task_editor {
                let fh = ed.read(cx).focus.clone();
                fh.focus(w, cx);
            }
        }
        if self.pending_clear_search {
            self.pending_clear_search = false;
            if let Some(ed) = &self.search_editor {
                ed.update(cx, |e, cx| e.set_value("", w, cx));
            }
        }
        if self.pending_focus_search {
            self.pending_focus_search = false;
            if let Some(ed) = &self.search_editor {
                let fh = ed.read(cx).focus.clone();
                fh.focus(w, cx);
            }
        }
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let m = &ev.keystroke.modifiers;
        let key = ev.keystroke.key.as_str();

        if self.modal_open {
            if key == "escape" {
                self.modal_open = false;
                cx.notify();
            } else if m.platform && key == "enter" {
                self.create_task_from_modal(cx);
            }
            return;
        }

        if self.is_typing(w, cx) {
            return;
        }
        if m.platform || m.control || m.alt {
            return;
        }

        match key {
            "j" | "J" | "k" | "K" => {
                let query = self.current_search_query(cx);
                let nav_ids = ordered_navigation_ids(
                    &self.tasks,
                    self.scope,
                    self.agent_filter.as_deref(),
                    &query,
                );
                if nav_ids.is_empty() {
                    return;
                }
                let delta: isize = if matches!(key, "j" | "J") { 1 } else { -1 };
                let cur = self
                    .open_task_id
                    .as_ref()
                    .and_then(|oid| nav_ids.iter().position(|id| id == oid))
                    .unwrap_or(0);
                let next = (cur as isize + delta).clamp(0, nav_ids.len() as isize - 1) as usize;
                self.select_task(nav_ids[next].clone(), false, cx);
            }
            "enter" => {
                if let Some(id) = self.open_task_id.clone() {
                    self.select_task(id, true, cx);
                }
            }
            "left" => {
                if self.view_mode != BoardViewMode::Board {
                    self.view_mode = BoardViewMode::Board;
                    cx.notify();
                }
            }
            "right" => {
                if self.view_mode != BoardViewMode::List {
                    self.view_mode = BoardViewMode::List;
                    cx.notify();
                }
            }
            "a" | "A" => {
                if let Some(id) = self.open_task_id.clone() {
                    self.approve_task(id, cx);
                }
            }
            "x" | "X" => {
                if let Some(id) = self.open_task_id.clone() {
                    self.reject_task(id, cx);
                }
            }
            "r" | "R" => {
                if let Some(id) = self.open_task_id.clone() {
                    self.retry_task(id, cx);
                }
            }
            "n" | "N" => {
                self.modal_open = true;
                self.pending_focus_new_task = true;
                cx.notify();
            }
            "/" => {
                self.search_open = true;
                self.pending_focus_search = true;
                cx.notify();
            }
            "escape" => {
                if self.agent_menu_open {
                    self.agent_menu_open = false;
                    cx.notify();
                } else if self.search_open {
                    self.search_open = false;
                    self.pending_clear_search = true;
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    /// Bare 16x16 3x3 mirrored agent pixel mark matching `.ui-aib-agent`.
    fn render_bare_agent_mark(
        seed: &str,
        color: Hsla,
        busy: bool,
        elapsed_ms: f32,
        cx: &App,
    ) -> Div {
        let reduced = Theme::global(cx).reduced_motion;
        let cells = agent_pixel_cells(seed);
        let mut grid = div()
            .size(px(16.))
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(1.5));

        for r in 0..3usize {
            let mut row_el = div().flex().items_center().gap(px(1.5));
            for c in 0..3usize {
                let i = r * 3 + c;
                let on = cells[i];
                let alpha = if busy && !reduced {
                    let phase = ((elapsed_ms + AGENT_WAVE_MS[i]) / 1200.) * std::f32::consts::TAU;
                    let wave = 0.60 + 0.40 * phase.cos();
                    if on { wave } else { 0.16 * wave }
                } else if on {
                    1.0
                } else {
                    0.0
                };
                row_el = row_el.child(
                    div()
                        .size(px(3.))
                        .rounded(px(0.75))
                        .bg(color.opacity(alpha)),
                );
            }
            grid = grid.child(row_el);
        }
        grid
    }

    /// Render the 13x13 status glyph (spinning smoothly when `spinning && status == Running`).
    fn render_state_glyph(
        status: BoardTaskStatus,
        spinning: bool,
        elapsed_ms: f32,
        p: &ProPalette,
        cx: &App,
    ) -> AnyElement {
        let reduced = Theme::global(cx).reduced_motion;
        match status {
            BoardTaskStatus::Waiting => pro_svg("0 0 14 14", SVG_STATE_WAITING, 13., p.amber),
            BoardTaskStatus::Failed => pro_svg("0 0 14 14", SVG_STATE_FAILED, 13., p.red),
            BoardTaskStatus::Done => pro_svg("0 0 14 14", SVG_STATE_DONE, 13., p.green),
            BoardTaskStatus::Stopped => pro_svg("0 0 14 14", SVG_STATE_STOPPED, 13., p.faint),
            BoardTaskStatus::Running => {
                if !spinning || reduced {
                    return pro_svg("0 0 14 14", SVG_STATE_RUNNING_STATIC, 13., p.blue);
                }
                let color = p.blue;
                let rot = ((elapsed_ms / 900.) % 1.0) * std::f32::consts::TAU;
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let center = bounds.center();
                        let r = 4.8;
                        let steps = 24usize;
                        let mut track = PathBuilder::stroke(px(1.5));
                        for i in 0..=steps {
                            let a = (i as f32 / steps as f32) * std::f32::consts::TAU;
                            let pt = center + point(px(r * a.cos()), px(r * a.sin()));
                            if i == 0 {
                                track.move_to(pt);
                            } else {
                                track.line_to(pt);
                            }
                        }
                        if let Ok(path) = track.build() {
                            window.paint_path(path, color.opacity(0.25));
                        }
                        let mut arc = PathBuilder::stroke(px(1.6));
                        for i in 0..=8usize {
                            let a = rot - std::f32::consts::FRAC_PI_2
                                + (i as f32 / 8.) * std::f32::consts::FRAC_PI_2;
                            let pt = center + point(px(r * a.cos()), px(r * a.sin()));
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
                .size(px(13.))
                .flex_none()
                .into_any_element()
            }
        }
    }

    fn card_state_text(task: &BoardTask, cx: &App) -> SharedString {
        match task.status {
            BoardTaskStatus::Waiting | BoardTaskStatus::Running | BoardTaskStatus::Done => {
                task.elapsed.clone()
            }
            BoardTaskStatus::Failed => tr(cx, "Failed", "失败").into(),
            BoardTaskStatus::Stopped => tr(cx, "Stopped", "已停止").into(),
        }
    }

    fn inspector_status_label(status: BoardTaskStatus, cx: &App) -> &'static str {
        match status {
            BoardTaskStatus::Waiting => tr(cx, "Waiting for you", "等待你确认"),
            BoardTaskStatus::Failed => tr(cx, "Failed", "执行失败"),
            BoardTaskStatus::Running => tr(cx, "Running", "运行中"),
            BoardTaskStatus::Done => tr(cx, "Done", "已完成"),
            BoardTaskStatus::Stopped => tr(cx, "Stopped", "已停止"),
        }
    }

    fn render_toolbar(&self, compact_bar: bool, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let query = self.current_search_query(cx);
        let af = self.agent_filter.as_deref();

        let all_n = filter_board_tasks(&self.tasks, BoardScope::All, af, &query).len();
        let needs_n = filter_board_tasks(&self.tasks, BoardScope::NeedsYou, af, &query).len();
        let run_n = filter_board_tasks(&self.tasks, BoardScope::Running, af, &query).len();
        let done_n = filter_board_tasks(&self.tasks, BoardScope::Done, af, &query).len();

        let scope_sensor = pro_selection_sensor();
        let selected_scope_key = match self.scope {
            BoardScope::All => "all",
            BoardScope::NeedsYou => "needs",
            BoardScope::Running => "run",
            BoardScope::Done => "done",
        };
        let scope_btn = |scope: BoardScope,
                         label: SharedString,
                         count: usize,
                         suffix: &'static str,
                         sensor: ProSelectionSensor,
                         cx: &mut Context<Self>| {
            let selected = self.scope == scope;
            activate(
                div()
                    .id(pro_eid(&self.id, suffix))
                    .role(Role::Tab)
                    .tab_index(0)
                    .relative()
                    .h(px(28.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .when(!selected, |d| {
                        d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                    })
                    .when(selected, |d| d.child(pro_mark_selected(&sensor)))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .font_weight(if selected {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if selected { p.ink } else { p.muted })
                    .whitespace_nowrap()
                    .cursor_pointer()
                    .child(label)
                    .when(!self.empty && !self.loading, |d| {
                        d.child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::NORMAL)
                                .text_color(p.faint)
                                .child(count.to_string()),
                        )
                    }),
                cx,
                move |this, _, cx| {
                    this.scope = scope;
                    cx.notify();
                },
            )
        };

        let lbl_all: SharedString = tr(cx, "All tasks", "全部任务").into();
        let lbl_needs: SharedString = tr(cx, "Needs you", "待你处理").into();
        let lbl_run: SharedString = tr(cx, "Running", "运行中").into();
        let lbl_done: SharedString = tr(cx, "Done", "已完成").into();
        let sb_all = scope_btn(
            BoardScope::All,
            lbl_all,
            all_n,
            "sc-all",
            scope_sensor.clone(),
            cx,
        );
        let sb_needs = scope_btn(
            BoardScope::NeedsYou,
            lbl_needs,
            needs_n,
            "sc-needs",
            scope_sensor.clone(),
            cx,
        );
        let sb_run = scope_btn(
            BoardScope::Running,
            lbl_run,
            run_n,
            "sc-run",
            scope_sensor.clone(),
            cx,
        );
        let sb_done = scope_btn(
            BoardScope::Done,
            lbl_done,
            done_n,
            "sc-done",
            scope_sensor.clone(),
            cx,
        );

        let scopes = div()
            .relative()
            .flex()
            .items_center()
            .gap(px(4.))
            .min_w_0()
            .overflow_hidden()
            .child(pro_pill_indicator(
                pro_eid(&self.id, "board-scopes"),
                selected_scope_key,
                scope_sensor,
                8.,
                p.chip,
                None,
                false,
            ))
            .child(sb_all)
            .child(sb_needs)
            .child(sb_run)
            .child(sb_done);

        let agent_active = self.agent_filter.is_some();
        let agent_btn = activate(
            div()
                .id(pro_eid(&self.id, "tool-agents"))
                .role(Role::Button)
                .aria_label(tr(cx, "Filter by agent", "按智能体筛选"))
                .tab_index(0)
                .h(px(28.))
                .px(px(if compact_bar { 7. } else { 10. }))
                .rounded(px(8.))
                .bg(if self.agent_menu_open {
                    p.hover
                } else {
                    transparent_black()
                })
                .hover(move |s| s.bg(p.hover))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(13.))
                .text_color(if agent_active || self.agent_menu_open {
                    p.ink
                } else {
                    p.muted
                })
                .cursor_pointer()
                .child(pro_svg(
                    "0 0 24 24",
                    SVG_FUNNEL,
                    15.,
                    if agent_active { p.ink } else { p.muted },
                ))
                .when(!compact_bar, |d| d.child(tr(cx, "Agents", "智能体")))
                .when(agent_active, |d| {
                    d.child(
                        div()
                            .min_w(px(16.))
                            .h(px(16.))
                            .px(px(4.))
                            .rounded_full()
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(11.))
                            .text_color(p.ink)
                            .child("1"),
                    )
                }),
            cx,
            |this, _, cx| {
                this.agent_menu_open = !this.agent_menu_open;
                cx.notify();
            },
        );

        let board_active = self.view_mode == BoardViewMode::Board;
        let list_active = self.view_mode == BoardViewMode::List;
        let view_sensor = pro_selection_sensor();
        let selected_view_key = if board_active { "board" } else { "list" };

        let views = div()
            .relative()
            .mr(px(4.))
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "board-views"),
                selected_view_key,
                view_sensor.clone(),
                7.,
                p.chip,
                None,
                false,
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "view-board"))
                    .role(Role::RadioButton)
                    .aria_label(tr(cx, "Board view", "看板视图"))
                    .tab_index(0)
                    .relative()
                    .size(px(28.))
                    .rounded(px(7.))
                    .when(!board_active, |d| d.hover(move |s| s.bg(p.hover)))
                    .when(board_active, |d| d.child(pro_mark_selected(&view_sensor)))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(pro_svg(
                        "0 0 24 24",
                        SVG_BOARD_COLS,
                        15.,
                        if board_active { p.ink } else { p.muted },
                    )),
                cx,
                |this, _, cx| {
                    this.view_mode = BoardViewMode::Board;
                    cx.notify();
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "view-list"))
                    .role(Role::RadioButton)
                    .aria_label(tr(cx, "List view", "列表视图"))
                    .tab_index(0)
                    .relative()
                    .size(px(28.))
                    .rounded(px(7.))
                    .when(!list_active, |d| d.hover(move |s| s.bg(p.hover)))
                    .when(list_active, |d| d.child(pro_mark_selected(&view_sensor)))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(pro_icon(
                        ProIcon::List,
                        15.,
                        if list_active { p.ink } else { p.muted },
                    )),
                cx,
                |this, _, cx| {
                    this.view_mode = BoardViewMode::List;
                    cx.notify();
                },
            ));

        let new_task_btn = activate(
            div()
                .id(pro_eid(&self.id, "btn-new-task"))
                .role(Role::Button)
                .aria_label(tr(cx, "New task", "新建任务"))
                .tab_index(0)
                .h(px(28.))
                .when(compact_bar, |d| d.w(px(28.)).justify_center())
                .when(!compact_bar, |d| d.pl(px(10.)).pr(px(12.)))
                .rounded(px(8.))
                .bg(p.primary)
                .hover(move |s| s.bg(p.primary.opacity(0.88)))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.on_primary)
                .whitespace_nowrap()
                .cursor_pointer()
                .child(pro_icon(ProIcon::Plus, 14., p.on_primary))
                .when(!compact_bar, |d| d.child(tr(cx, "New task", "新建任务"))),
            cx,
            |this, _, cx| {
                this.modal_open = true;
                this.agent_menu_open = false;
                this.pending_focus_new_task = true;
                cx.notify();
            },
        );

        let search_ed = self.search_editor.clone();
        let search_box = self.search_open.then(|| {
            div()
                .w(px(150.))
                .h(px(28.))
                .px(px(8.))
                .rounded(px(7.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .items_center()
                .gap(px(5.))
                .child(pro_icon(ProIcon::Search, 12.5, p.faint))
                .when_some(search_ed, |d, ed| {
                    d.child(div().flex_1().min_w_0().child(ed))
                })
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "clear-search"))
                        .role(Role::Button)
                        .tab_index(0)
                        .size(px(16.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Close, 10., p.faint)),
                    cx,
                    |this, _, cx| {
                        this.search_open = false;
                        this.pending_clear_search = true;
                        cx.notify();
                    },
                ))
        });

        let panel_closed = !self.standalone && !self.shell.read(cx).is_panel_open();

        div()
            .min_h(px(48.))
            .pl(px(14.))
            .pr(px(if panel_closed { 48. } else { 12. }))
            .py(px(10.))
            .flex_none()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(10.))
            .child(scopes)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .when_some(search_box, |d, sb| d.child(sb))
                    .child(agent_btn)
                    .child(views)
                    .child(new_task_btn),
            )
    }

    fn render_card_counts(sources: u32, tools: u32, p: &ProPalette) -> Div {
        div()
            .flex_none()
            .flex()
            .items_center()
            .text_size(px(12.))
            .text_color(p.faint)
            .child(
                div()
                    .pr(px(7.))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(pro_icon(ProIcon::Search, 12., p.faint))
                    .child(sources.to_string()),
            )
            .child(div().w(px(1.)).h(px(11.)).bg(p.line_strong))
            .child(
                div()
                    .pl(px(7.))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(pro_svg("0 0 24 24", SVG_WRENCH, 12., p.faint))
                    .child(tools.to_string()),
            )
    }

    fn render_board_view(
        &self,
        stack_cols: bool,
        compact_bar: bool,
        elapsed_ms: f32,
        phase: f32,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let frame_bg = if p.dark {
            hsla(0., 0., 1., 0.03)
        } else {
            hsla(0., 0., 0., 0.022)
        };
        let query = self.current_search_query(cx);
        let filtered = filter_board_tasks(
            &self.tasks,
            self.scope,
            self.agent_filter.as_deref(),
            &query,
        );

        let mut board = div()
            .id(pro_eid(&self.id, "board-scroll"))
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .px(px(12.))
            .pt(px(2.))
            .pb(px(16.))
            .flex()
            .when(stack_cols, |d| d.flex_col().items_stretch())
            .when(!stack_cols, |d| d.items_start())
            .gap(px(10.));

        for col in BoardColumn::ALL {
            if !col.matches_scope(self.scope) {
                continue;
            }
            let (col_label, head_status) = match col {
                BoardColumn::NeedsYou => {
                    (tr(cx, "NEEDS YOU", "待你处理"), BoardTaskStatus::Waiting)
                }
                BoardColumn::Running => (tr(cx, "RUNNING", "运行中"), BoardTaskStatus::Running),
                BoardColumn::Done => (tr(cx, "DONE", "已完成"), BoardTaskStatus::Done),
            };
            let col_tasks: Vec<&BoardTask> = filtered
                .iter()
                .copied()
                .filter(|t| t.status.column() == col)
                .collect();

            let col_head = div()
                .h(px(36.))
                .px(px(6.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(6.))
                .child(
                    div()
                        .w(px(20.))
                        .flex()
                        .items_center()
                        .child(Self::render_state_glyph(
                            head_status,
                            false,
                            elapsed_ms,
                            &p,
                            cx,
                        )),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .when(!stack_cols, |d| d.text_center())
                        .truncate()
                        .child(col_label),
                )
                .child(
                    div()
                        .min_w(px(20.))
                        .h(px(18.))
                        .px(px(5.))
                        .rounded(px(6.))
                        .bg(p.chip)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(11.5))
                        .text_color(p.muted)
                        .child(if self.loading {
                            "–".to_owned()
                        } else {
                            col_tasks.len().to_string()
                        }),
                );

            let mut cards_wrap = div().flex().flex_col().gap(px(6.));

            if self.loading {
                for si in 0..2usize {
                    cards_wrap = cards_wrap.child(
                        div()
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .pt(px(12.))
                            .pb(px(10.))
                            .px(px(14.))
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(pro_skeleton_bar(14., 14., 4., phase, cx))
                                    .child(pro_skeleton_bar(
                                        if si == 0 { 132. } else { 112. },
                                        12.,
                                        4.,
                                        phase,
                                        cx,
                                    )),
                            )
                            .child(
                                div()
                                    .ml(px(22.))
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(pro_skeleton_bar(176., 10., 4., phase, cx))
                                    .child(pro_skeleton_bar(142., 10., 4., phase, cx)),
                            )
                            .child(
                                div()
                                    .ml(px(22.))
                                    .pt(px(2.))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(pro_skeleton_bar(54., 11., 4., phase, cx))
                                    .child(pro_skeleton_bar(62., 11., 4., phase, cx)),
                            ),
                    );
                }
            } else if col_tasks.is_empty() {
                cards_wrap = cards_wrap.child(
                    div()
                        .px(px(8.))
                        .py(px(6.))
                        .text_size(px(12.5))
                        .text_color(p.faint)
                        .child(tr(cx, "No tasks", "暂无任务")),
                );
            } else {
                for task in col_tasks {
                    let is_open = self.open_task_id.as_ref() == Some(&task.id);
                    let tid = task.id.clone();
                    let (agent_name, agent_color) = self.agent_info(&task.agent, &p);
                    let is_running = task.status == BoardTaskStatus::Running;
                    let state_txt = Self::card_state_text(task, cx);
                    let aria = format!("{} – {}", task.title, agent_name);

                    cards_wrap = cards_wrap.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("card-{tid}")))
                            .role(Role::Button)
                            .aria_label(aria)
                            .tab_index(0)
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(if is_open { p.line_strong } else { p.line })
                            .hover(move |s| s.border_color(p.line_strong))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .pt(px(12.))
                            .pb(px(10.))
                            .px(px(14.))
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .cursor_pointer()
                            .child(
                                div()
                                    .min_h(px(20.))
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .min_w_0()
                                    .child(Self::render_bare_agent_mark(
                                        &task.agent,
                                        agent_color,
                                        is_running,
                                        elapsed_ms,
                                        cx,
                                    ))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .text_size(px(13.))
                                            .line_height(px(20.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .truncate()
                                            .child(task.title.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .when(!compact_bar, |d| d.ml(px(24.)))
                                    .h(px(38.))
                                    .overflow_hidden()
                                    .text_size(px(12.5))
                                    .line_height(px(19.))
                                    .text_color(p.muted)
                                    .child(task.description.clone()),
                            )
                            .child(
                                div()
                                    .when(!compact_bar, |d| d.ml(px(24.)))
                                    .mt(px(2.))
                                    .min_h(px(22.))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .min_w_0()
                                            .text_size(px(12.5))
                                            .text_color(p.text)
                                            .child(Self::render_state_glyph(
                                                task.status,
                                                true,
                                                elapsed_ms,
                                                &p,
                                                cx,
                                            ))
                                            .child(div().truncate().child(state_txt)),
                                    )
                                    .child(Self::render_card_counts(task.sources, task.tools, &p)),
                            ),
                        cx,
                        move |this, _, cx| {
                            this.select_task(tid.clone(), true, cx);
                        },
                    ));
                }
            }

            let single_scope = self.scope != BoardScope::All;
            board = board.child(
                div()
                    .when(!stack_cols, |d| {
                        d.flex_1()
                            .min_w_0()
                            .max_w(px(if single_scope { 460. } else { 320. }))
                    })
                    .when(stack_cols, |d| d.w_full())
                    .p(px(4.))
                    .pb(px(6.))
                    .rounded(px(14.))
                    .bg(frame_bg)
                    .flex()
                    .flex_col()
                    .child(col_head)
                    .child(cards_wrap),
            );
        }

        board
    }

    fn render_list_view(
        &self,
        stack_cols: bool,
        elapsed_ms: f32,
        phase: f32,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let frame_bg = if p.dark {
            hsla(0., 0., 1., 0.03)
        } else {
            hsla(0., 0., 0., 0.022)
        };
        let query = self.current_search_query(cx);
        let filtered = filter_board_tasks(
            &self.tasks,
            self.scope,
            self.agent_filter.as_deref(),
            &query,
        );

        let mut list = div()
            .id(pro_eid(&self.id, "list-scroll"))
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .px(px(12.))
            .pb(px(16.))
            .flex()
            .flex_col()
            .gap(px(14.));

        for col in BoardColumn::ALL {
            if !col.matches_scope(self.scope) {
                continue;
            }
            let (grp_label, head_status) = match col {
                BoardColumn::NeedsYou => {
                    (tr(cx, "Needs you", "待你处理"), BoardTaskStatus::Waiting)
                }
                BoardColumn::Running => (tr(cx, "Running", "运行中"), BoardTaskStatus::Running),
                BoardColumn::Done => (tr(cx, "Done", "已完成"), BoardTaskStatus::Done),
            };
            let grp_tasks: Vec<&BoardTask> = filtered
                .iter()
                .copied()
                .filter(|t| t.status.column() == col)
                .collect();

            let head = div()
                .h(px(34.))
                .px(px(12.))
                .rounded(px(10.))
                .bg(frame_bg)
                .flex()
                .items_center()
                .gap(px(8.))
                .child(Self::render_state_glyph(
                    head_status,
                    false,
                    elapsed_ms,
                    &p,
                    cx,
                ))
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(grp_label),
                )
                .child(
                    div()
                        .min_w(px(20.))
                        .h(px(18.))
                        .px(px(5.))
                        .rounded(px(6.))
                        .bg(p.chip)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(11.5))
                        .text_color(p.muted)
                        .child(grp_tasks.len().to_string()),
                );

            let mut rows = div().flex().flex_col().gap(px(2.));
            if self.loading {
                for _ in 0..2 {
                    rows = rows.child(
                        div()
                            .h(px(42.))
                            .px(px(12.))
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(pro_skeleton_bar(14., 14., 4., phase, cx))
                            .child(pro_skeleton_bar(150., 12., 4., phase, cx))
                            .child(
                                div()
                                    .flex_1()
                                    .child(pro_skeleton_bar(180., 11., 4., phase, cx)),
                            )
                            .child(pro_skeleton_bar(80., 11., 4., phase, cx)),
                    );
                }
            } else if grp_tasks.is_empty() {
                rows = rows.child(
                    div()
                        .px(px(12.))
                        .py(px(6.))
                        .text_size(px(12.5))
                        .text_color(p.faint)
                        .child(tr(cx, "No tasks", "暂无任务")),
                );
            } else {
                for task in grp_tasks {
                    let is_open = self.open_task_id.as_ref() == Some(&task.id);
                    let tid = task.id.clone();
                    let (_, agent_color) = self.agent_info(&task.agent, &p);
                    let is_running = task.status == BoardTaskStatus::Running;
                    let state_txt = Self::card_state_text(task, cx);

                    rows = rows.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("lrow-{tid}")))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(42.))
                            .px(px(12.))
                            .rounded(px(10.))
                            .bg(if is_open { p.chip } else { transparent_black() })
                            .hover(move |s| s.bg(if is_open { p.chip } else { p.hover }))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .cursor_pointer()
                            .child(Self::render_bare_agent_mark(
                                &task.agent,
                                agent_color,
                                is_running,
                                elapsed_ms,
                                cx,
                            ))
                            .child(
                                div()
                                    .when(stack_cols, |d| d.flex_1())
                                    .when(!stack_cols, |d| d.max_w(px(210.)))
                                    .min_w_0()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .truncate()
                                    .child(task.title.clone()),
                            )
                            .when(!stack_cols, |d| {
                                d.child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_size(px(12.5))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(task.description.clone()),
                                )
                            })
                            .child(
                                div()
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .gap(px(12.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .text_size(px(12.5))
                                            .text_color(p.text)
                                            .child(Self::render_state_glyph(
                                                task.status,
                                                true,
                                                elapsed_ms,
                                                &p,
                                                cx,
                                            ))
                                            .child(state_txt),
                                    )
                                    .child(Self::render_card_counts(task.sources, task.tools, &p)),
                            ),
                        cx,
                        move |this, _, cx| {
                            this.select_task(tid.clone(), true, cx);
                        },
                    ));
                }
            }

            list = list.child(div().flex().flex_col().gap(px(4.)).child(head).child(rows));
        }

        list
    }

    fn render_agent_popover(&self, elapsed_ms: f32, cx: &mut Context<Self>) -> AnyElement {
        let p = ProPalette::get(cx);
        let all_checked = self.agent_filter.is_none();

        let pop_card = div()
            .id(pro_eid(&self.id, "agent-menu"))
            .w(px(208.))
            .p(px(4.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_col()
            .gap(px(1.))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .px(px(10.))
                    .pt(px(5.))
                    .pb(px(3.))
                    .text_size(px(11.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.faint)
                    .child(tr(cx, "Filter by agent", "按智能体筛选")),
            )
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "ag-opt-all"))
                    .role(Role::MenuItem)
                    .tab_index(0)
                    .h(px(32.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(13.))
                    .font_weight(if all_checked {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if all_checked { p.ink } else { p.text })
                    .cursor_pointer()
                    .child(tr(cx, "All agents", "全部智能体"))
                    .when(all_checked, |d| {
                        d.child(pro_icon(ProIcon::Check, 13., p.ink))
                    }),
                cx,
                |this, _, cx| {
                    this.agent_filter = None;
                    this.agent_menu_open = false;
                    cx.notify();
                },
            ))
            .child(div().my(px(3.)).mx(px(6.)).h(px(1.)).bg(p.line))
            .children(self.agents.iter().map(|ag| {
                let checked = self.agent_filter.as_ref() == Some(&ag.id);
                let aid = ag.id.clone();
                let acolor: Hsla = rgb(ag.color_rgb).into();
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("ag-opt-{aid}")))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(32.))
                        .px(px(10.))
                        .rounded(px(8.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .text_size(px(13.))
                        .font_weight(if checked {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(if checked { p.ink } else { p.text })
                        .cursor_pointer()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(Self::render_bare_agent_mark(
                                    &ag.id, acolor, false, elapsed_ms, cx,
                                ))
                                .child(ag.name.clone()),
                        )
                        .when(checked, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink))),
                    cx,
                    move |this, _, cx| {
                        this.agent_filter = Some(aid.clone());
                        this.agent_menu_open = false;
                        cx.notify();
                    },
                )
            }));

        pro_presence(
            pro_eid(&self.id, "agent-menu-pres"),
            self.agent_menu_open,
            0.18,
            0.14,
            move |amount| {
                div()
                    .absolute()
                    .top(px(44. - 6. * (1. - amount)))
                    .right(px(86.))
                    .opacity(amount)
                    .child(pop_card)
                    .into_any_element()
            },
        )
    }

    fn render_new_task_modal(&self, elapsed_ms: f32, cx: &mut Context<Self>) -> AnyElement {
        let p = ProPalette::get(cx);
        let new_ed = self.new_task_editor.clone();
        let has_prompt = self
            .new_task_editor
            .as_ref()
            .is_some_and(|e| !e.read(cx).value().trim().is_empty());

        let modal_ag_sensor = pro_selection_sensor();
        let modal_ag_pills = div()
            .relative()
            .flex()
            .flex_wrap()
            .gap(px(6.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "modal-ag-pills"),
                self.modal_agent_id.clone(),
                modal_ag_sensor.clone(),
                8.,
                p.chip,
                Some(p.line_strong),
                false,
            ))
            .children(self.agents.iter().map(|ag| {
                let picked = self.modal_agent_id == ag.id;
                let aid = ag.id.clone();
                let acolor: Hsla = rgb(ag.color_rgb).into();
                let sensor_ref = modal_ag_sensor.clone();
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("modal-ag-{aid}")))
                        .role(Role::RadioButton)
                        .tab_index(0)
                        .relative()
                        .h(px(30.))
                        .pl(px(8.))
                        .pr(px(10.))
                        .rounded(px(8.))
                        .when(picked, |d| {
                            d.font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(pro_mark_selected(&sensor_ref))
                        })
                        .when(!picked, |d| {
                            d.border_1()
                                .border_color(p.line)
                                .text_color(p.muted)
                                .hover(move |s| s.bg(p.hover).text_color(p.ink))
                        })
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(13.))
                        .cursor_pointer()
                        .child(Self::render_bare_agent_mark(
                            &ag.id, acolor, false, elapsed_ms, cx,
                        ))
                        .child(ag.name.clone()),
                    cx,
                    move |this, _, cx| {
                        this.modal_agent_id = aid.clone();
                        cx.notify();
                    },
                )
            }));

        let scrim_id = pro_eid(&self.id, "modal-scrim");
        let scrim_alpha = if p.dark { 0.42 } else { 0.16 };
        let on_dismiss = cx.listener(|this, _, _, cx| {
            this.modal_open = false;
            cx.notify();
        });

        let dialog_card = div()
            .w_full()
            .max_w(px(468.))
            .rounded(px(16.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .overflow_hidden()
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .pt(px(16.))
                    .pl(px(20.))
                    .pr(px(12.))
                    .pb(px(4.))
                    .flex()
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
                                    .text_size(px(15.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(tr(cx, "New task", "新建后台任务")),
                            )
                            .child(div().text_size(px(12.5)).text_color(p.muted).child(tr(
                                cx,
                                "Hand a background job to an agent – press ⌘Enter to start",
                                "将后台任务交给智能体执行——按 ⌘Enter 立即启动",
                            ))),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "modal-close"),
                        "Close",
                        ProIcon::Close,
                        28.,
                        false,
                        cx,
                        |this, _, cx| {
                            this.modal_open = false;
                            cx.notify();
                        },
                    )),
            )
            .child(
                div()
                    .px(px(20.))
                    .py(px(16.))
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "What should it do?", "任务说明")),
                            )
                            .child(
                                div()
                                    .h(px(84.))
                                    .p(px(10.))
                                    .rounded(px(8.))
                                    .bg(p.subtle)
                                    .border_1()
                                    .border_color(p.line)
                                    .when_some(new_ed, |d, ed| d.child(ed)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "Agent", "执行智能体")),
                            )
                            .child(modal_ag_pills),
                    ),
            )
            .child(
                div()
                    .px(px(16.))
                    .py(px(12.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(8.))
                    .child(pro_btn(
                        pro_eid(&self.id, "modal-cancel"),
                        tr(cx, "Cancel", "取消"),
                        None,
                        ProBtnStyle::Ghost,
                        false,
                        cx,
                        |this, _, cx| {
                            this.modal_open = false;
                            cx.notify();
                        },
                    ))
                    .child(div().when(!has_prompt, |d| d.opacity(0.4)).child(pro_btn(
                        pro_eid(&self.id, "modal-start"),
                        tr(cx, "Start task", "启动任务"),
                        None,
                        ProBtnStyle::Primary,
                        false,
                        cx,
                        |this, _, cx| this.create_task_from_modal(cx),
                    ))),
            );

        pro_presence(
            pro_eid(&self.id, "new-task-pres"),
            self.modal_open,
            0.22,
            0.16,
            move |amount| {
                div()
                    .id(scrim_id)
                    .role(Role::Dialog)
                    .absolute()
                    .inset_0()
                    .p(px(20.))
                    .bg(hsla(0., 0., 0., scrim_alpha * amount))
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_mouse_down(MouseButton::Left, on_dismiss)
                    .child(
                        div()
                            .w_full()
                            .max_w(px(468.))
                            .relative()
                            .top(px(8. * (1. - amount)))
                            .opacity(amount)
                            .child(dialog_card),
                    )
                    .into_any_element()
            },
        )
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let any_running = self
            .tasks
            .iter()
            .any(|t| t.status == BoardTaskStatus::Running);
        let elapsed_ms = self.started_at.elapsed().as_secs_f32() * 1000.;
        if !reduced && (self.loading || (any_running && !self.empty)) {
            w.request_animation_frame();
        }
        let phase = if self.loading && !reduced {
            (elapsed_ms / 1100.) % 1.0
        } else {
            0.35
        };

        let width = self.measured_width.get();
        let stack_cols = width < 760.;
        let compact_bar = width < 560.;
        let measure = self.measured_width.clone();

        let main_body: AnyElement = if self.empty && !self.loading {
            div()
                .flex_1()
                .px(px(12.))
                .pb(px(12.))
                .flex()
                .items_center()
                .justify_center()
                .child(pro_empty_state(
                    ProIcon::Inbox,
                    tr(cx, "No background tasks yet", "暂无后台任务"),
                    tr(
                        cx,
                        "Hand work to an agent – running tasks, approvals and finished runs will organize themselves here.",
                        "将工作交给智能体执行——运行中、待审批与已完成的任务会自动归类在看板上。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-new-task"),
                        tr(cx, "New task", "新建任务").into(),
                        Box::new(|this: &mut Self, _, cx| {
                            this.modal_open = true;
                            this.pending_focus_new_task = true;
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element()
        } else if self.view_mode == BoardViewMode::Board {
            self.render_board_view(stack_cols, compact_bar, elapsed_ms, phase, cx)
                .into_any_element()
        } else {
            self.render_list_view(stack_cols, elapsed_ms, phase, cx)
                .into_any_element()
        };

        let wrapped_main: AnyElement = if self.loading {
            main_body
        } else {
            let scope_key = format!(
                "{:?}-{:?}-{}-{}",
                self.scope,
                self.view_mode,
                self.agent_filter.as_deref().unwrap_or("all"),
                self.empty
            );
            div()
                .flex_1()
                .min_h_0()
                .w_full()
                .flex()
                .flex_col()
                .child(pro_view_transition_fill(
                    pro_eid(&self.id, "board-vtx"),
                    scope_key,
                    main_body,
                ))
                .into_any_element()
        };

        let agent_pop = self.render_agent_popover(elapsed_ms, cx);
        let modal_el = self.render_new_task_modal(elapsed_ms, cx);

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .size_full()
            .min_h_0()
            .bg(p.panel)
            .relative()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.agent_menu_open {
                        this.agent_menu_open = false;
                        cx.notify();
                    }
                }),
            )
            .child(
                canvas(
                    move |b, window, _| {
                        let nw = f32::from(b.size.width);
                        if nw > 10. && (measure.get() - nw).abs() > 1.0 {
                            measure.set(nw);
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .size_0(),
            )
            .child(self.render_toolbar(compact_bar, cx))
            .child(wrapped_main)
            .child(agent_pop)
            .child(modal_el)
    }

    fn render_inspector(
        &mut self,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SharedString, ProIcon, AnyElement)> {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let elapsed_ms = self.started_at.elapsed().as_secs_f32() * 1000.;
        let phase = if self.loading && !Theme::global(cx).reduced_motion {
            (elapsed_ms / 1100.) % 1.0
        } else {
            0.35
        };

        if self.loading {
            let skel = div()
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(22.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(168., 15., 4., phase, cx))
                        .child(pro_skeleton_bar(220., 12., 4., phase, cx))
                        .child(pro_skeleton_bar(186., 12., 4., phase, cx))
                        .child(
                            div()
                                .pt(px(4.))
                                .flex()
                                .gap(px(8.))
                                .child(pro_skeleton_bar(72., 28., 7., phase, cx))
                                .child(pro_skeleton_bar(62., 28., 7., phase, cx)),
                        ),
                )
                .child(div().flex().flex_col().gap(px(10.)).children(
                    [120., 84., 72., 88.].into_iter().map(|bar_w| {
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(pro_skeleton_bar(56., 12., 4., phase, cx))
                            .child(pro_skeleton_bar(bar_w, 12., 4., phase, cx))
                    }),
                ))
                .child(
                    div()
                        .p(px(14.))
                        .rounded(px(10.))
                        .bg(p.chip)
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(188., 12., 4., phase, cx))
                        .child(pro_skeleton_bar(168., 12., 4., phase, cx))
                        .child(pro_skeleton_bar(154., 12., 4., phase, cx)),
                )
                .into_any_element();
            return Some((tr(cx, "Task", "任务").into(), ProIcon::Inbox, skel));
        }

        if self.empty {
            return None;
        }

        let Some(ref open_id) = self.open_task_id else {
            let none_el = div()
                .min_h(px(240.))
                .p(px(24.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .text_color(p.muted)
                .child(tr(
                    cx,
                    "Select a task card to inspect its steps",
                    "选择任意任务卡片以查看执行详情",
                ))
                .into_any_element();
            return Some((tr(cx, "Task", "任务").into(), ProIcon::Inbox, none_el));
        };

        let task = self.tasks.iter().find(|t| &t.id == open_id).cloned()?;

        let (agent_name, agent_color) = self.agent_info(&task.agent, &p);
        let is_running = task.status == BoardTaskStatus::Running;
        let tid_approve = task.id.clone();
        let tid_reject = task.id.clone();
        let tid_retry = task.id.clone();
        let tid_open = task.id.clone();

        let actions = match task.status {
            BoardTaskStatus::Waiting => div()
                .mt(px(6.))
                .flex()
                .flex_wrap()
                .gap(px(8.))
                .child(pro_btn(
                    pro_eid(&self.id, "act-approve"),
                    tr(cx, "Approve", "批准"),
                    None,
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    move |this, _, cx| this.approve_task(tid_approve.clone(), cx),
                ))
                .child(pro_btn(
                    pro_eid(&self.id, "act-reject"),
                    tr(cx, "Reject", "拒绝"),
                    None,
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    move |this, _, cx| this.reject_task(tid_reject.clone(), cx),
                )),
            BoardTaskStatus::Failed => div()
                .mt(px(6.))
                .flex()
                .flex_wrap()
                .gap(px(8.))
                .child(pro_btn(
                    pro_eid(&self.id, "act-retry"),
                    tr(cx, "Retry", "重试"),
                    Some(ProIcon::Refresh),
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    move |this, _, cx| this.retry_task(tid_retry.clone(), cx),
                ))
                .child(pro_btn(
                    pro_eid(&self.id, "act-open"),
                    tr(cx, "Open", "打开"),
                    Some(ProIcon::ExternalArrow),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    move |_, _, cx| cx.emit(AgentInboxBoardEvent::Open(tid_open.clone())),
                )),
            BoardTaskStatus::Running | BoardTaskStatus::Done => div()
                .mt(px(6.))
                .flex()
                .flex_wrap()
                .gap(px(8.))
                .child(pro_btn(
                    pro_eid(&self.id, "act-open"),
                    tr(cx, "Open", "打开"),
                    Some(ProIcon::ExternalArrow),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    move |_, _, cx| cx.emit(AgentInboxBoardEvent::Open(tid_open.clone())),
                )),
            BoardTaskStatus::Stopped => div()
                .mt(px(6.))
                .flex()
                .flex_wrap()
                .gap(px(8.))
                .child(pro_btn(
                    pro_eid(&self.id, "act-retry"),
                    tr(cx, "Retry", "重新运行"),
                    Some(ProIcon::Refresh),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    move |this, _, cx| this.retry_task(tid_retry.clone(), cx),
                ))
                .child(pro_btn(
                    pro_eid(&self.id, "act-open"),
                    tr(cx, "Open", "打开"),
                    None,
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    move |_, _, cx| cx.emit(AgentInboxBoardEvent::Open(tid_open.clone())),
                )),
        };

        let status_word = Self::inspector_status_label(task.status, cx);
        let step_len = task.steps.len();
        let tray_bg = if p.dark {
            hsla(0., 0., 1., 0.04)
        } else {
            hsla(0., 0., 0., 0.03)
        };

        let prop_row = |label: &'static str, value_el: Div| {
            div()
                .min_h(px(22.))
                .flex()
                .items_center()
                .child(
                    div()
                        .w(px(76.))
                        .flex_none()
                        .text_size(px(13.))
                        .text_color(p.muted)
                        .child(label),
                )
                .child(value_el.flex_1().min_w_0())
        };

        let body = div()
            .p(px(16.))
            .pb(px(24.))
            .flex()
            .flex_col()
            .gap(px(22.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(15.))
                            .line_height(px(22.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(task.title.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(p.text)
                            .child(task.description.clone()),
                    )
                    .child(actions),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(9.))
                    .child(prop_row(
                        tr(cx, "Status", "状态"),
                        div()
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .text_size(px(13.))
                            .text_color(p.ink)
                            .child(Self::render_state_glyph(
                                task.status,
                                true,
                                elapsed_ms,
                                &p,
                                cx,
                            ))
                            .child(div().truncate().child(status_word))
                            .when(
                                !matches!(
                                    task.status,
                                    BoardTaskStatus::Failed | BoardTaskStatus::Stopped
                                ),
                                |d| {
                                    d.child(
                                        div()
                                            .flex_none()
                                            .text_color(p.faint)
                                            .child(task.elapsed.clone()),
                                    )
                                },
                            ),
                    ))
                    .child(prop_row(
                        tr(cx, "Agent", "智能体"),
                        div()
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .text_size(px(13.))
                            .text_color(p.ink)
                            .child(Self::render_bare_agent_mark(
                                &task.agent,
                                agent_color,
                                is_running,
                                elapsed_ms,
                                cx,
                            ))
                            .child(div().truncate().child(agent_name)),
                    ))
                    .child(prop_row(
                        tr(cx, "Sources", "读取来源"),
                        div()
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .text_size(px(13.))
                            .text_color(p.ink)
                            .child(pro_icon(ProIcon::Search, 13., p.muted))
                            .child(format!("{} {}", task.sources, tr(cx, "read", "条"))),
                    ))
                    .child(prop_row(
                        tr(cx, "Tools", "工具调用"),
                        div()
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .text_size(px(13.))
                            .text_color(p.ink)
                            .child(pro_svg("0 0 24 24", SVG_WRENCH, 13., p.muted))
                            .child(format!("{} {}", task.tools, tr(cx, "calls", "次"))),
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "What it did", "执行轨迹"))
                            .child(
                                div()
                                    .font_weight(FontWeight::NORMAL)
                                    .text_color(p.faint)
                                    .child(step_len.to_string()),
                            ),
                    )
                    .child(
                        div()
                            .p(px(12.))
                            .px(px(14.))
                            .rounded(px(10.))
                            .bg(tray_bg)
                            .flex()
                            .flex_col()
                            .gap(px(10.))
                            .children(task.steps.into_iter().enumerate().map(|(si, step)| {
                                let is_last = si + 1 == step_len;
                                div()
                                    .relative()
                                    .flex()
                                    .items_start()
                                    .justify_between()
                                    .gap(px(10.))
                                    .when(!is_last, |d| {
                                        d.child(
                                            div()
                                                .absolute()
                                                .left(px(2.))
                                                .top(px(14.))
                                                .bottom(px(-12.))
                                                .w(px(1.))
                                                .bg(p.line_strong),
                                        )
                                    })
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .flex()
                                            .items_start()
                                            .gap(px(10.))
                                            .child(
                                                div()
                                                    .mt(px(7.))
                                                    .size(px(5.))
                                                    .flex_none()
                                                    .rounded_full()
                                                    .bg(if is_last {
                                                        p.ink
                                                    } else {
                                                        p.line_strong
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .text_size(px(13.))
                                                    .line_height(px(19.))
                                                    .text_color(p.text)
                                                    .child(step.label),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex_none()
                                            .text_size(px(12.5))
                                            .line_height(px(19.))
                                            .text_color(p.faint)
                                            .child(step.time),
                                    )
                            })),
                    ),
            )
            .into_any_element();

        let body_el = pro_view_transition(pro_eid(&self.id, "task-insp-vtx"), task.id, body);

        Some((task.title, ProIcon::Inbox, body_el))
    }
}

impl Render for AgentInboxBoardScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

fn default_board_agents() -> Vec<BoardAgent> {
    vec![
        BoardAgent::new("builder", "Builder", 0x0190fe),
        BoardAgent::new("reviewer", "Reviewer", 0xa651f0),
        BoardAgent::new("writer", "Writer", 0xf26b5b),
        BoardAgent::new("ops", "Ops", 0x13b3a0),
    ]
}

fn default_board_tasks() -> Vec<BoardTask> {
    vec![
        BoardTask {
            id: "migrate".into(),
            title: "Migrate users table".into(),
            description: "Split users into accounts and profiles on Postgres 17. The dry run on staging passed.".into(),
            agent: "builder".into(),
            status: BoardTaskStatus::Waiting,
            elapsed: "6m 13s".into(),
            sources: 9,
            tools: 14,
            steps: vec![
                BoardTaskStep::new("Read the schema and 14 migrations", "6m ago"),
                BoardTaskStep::new("Wrote migration 0042_split_users", "4m ago"),
                BoardTaskStep::new("Dry run on staging – no errors", "2m ago"),
                BoardTaskStep::new("Wants to run it on production", "1m ago"),
            ],
        },
        BoardTask {
            id: "crm-cleanup".into(),
            title: "Clean up CRM contacts".into(),
            description: "Matched 1,312 likely duplicates, then the merge hit the API rate limit.".into(),
            agent: "ops".into(),
            status: BoardTaskStatus::Failed,
            elapsed: "8m 04s".into(),
            sources: 2,
            tools: 6,
            steps: vec![
                BoardTaskStep::new("Fetched 4,890 contact records", "12m ago"),
                BoardTaskStep::new("Matched 1,312 likely duplicates", "9m ago"),
                BoardTaskStep::new("Hit 429 rate limit on batch merge", "8m ago"),
            ],
        },
        BoardTask {
            id: "review-pr".into(),
            title: "Review PR #482".into(),
            description: "Billing webhooks: reading the diff and the test run, writing comments.".into(),
            agent: "reviewer".into(),
            status: BoardTaskStatus::Running,
            elapsed: "3m 41s".into(),
            sources: 12,
            tools: 21,
            steps: vec![
                BoardTaskStep::new("Read 18 changed files in PR #482", "3m ago"),
                BoardTaskStep::new("Verified webhook signature replay check", "2m ago"),
                BoardTaskStep::new("Writing inline review comments", "1m ago"),
            ],
        },
        BoardTask {
            id: "release-notes".into(),
            title: "Release notes v2.4".into(),
            description: "Grouping 23 merged pull requests into features and fixes.".into(),
            agent: "writer".into(),
            status: BoardTaskStatus::Running,
            elapsed: "1m 19s".into(),
            sources: 23,
            tools: 5,
            steps: vec![
                BoardTaskStep::new("Collected 23 merged pull requests", "1m ago"),
                BoardTaskStep::new("Grouped breaking changes and fixes", "40s ago"),
                BoardTaskStep::new("Drafting customer highlights", "12s ago"),
            ],
        },
        BoardTask {
            id: "dep-audit".into(),
            title: "Dependency audit".into(),
            description: "Checking 214 packages for known vulnerabilities and outdated majors.".into(),
            agent: "ops".into(),
            status: BoardTaskStatus::Running,
            elapsed: "43s".into(),
            sources: 3,
            tools: 4,
            steps: vec![
                BoardTaskStep::new("Parsed lockfiles across 4 crates", "40s ago"),
                BoardTaskStep::new("Cross-checking security advisories", "15s ago"),
            ],
        },
        BoardTask {
            id: "metrics-digest".into(),
            title: "Weekly metrics digest".into(),
            description: "Pulled numbers from three dashboards and posted the summary to #metrics.".into(),
            agent: "writer".into(),
            status: BoardTaskStatus::Done,
            elapsed: "4m 12s".into(),
            sources: 3,
            tools: 8,
            steps: vec![
                BoardTaskStep::new("Queried activation, retention and MRR", "2h ago"),
                BoardTaskStep::new("Computed week-over-week deltas", "2h ago"),
                BoardTaskStep::new("Posted digest to #metrics", "2h ago"),
            ],
        },
        BoardTask {
            id: "onboarding-emails".into(),
            title: "Onboarding emails".into(),
            description: "Rewrote the three welcome emails in the brand voice and scheduled them.".into(),
            agent: "writer".into(),
            status: BoardTaskStatus::Done,
            elapsed: "7m 03s".into(),
            sources: 5,
            tools: 11,
            steps: vec![
                BoardTaskStep::new("Audited existing welcome drip", "4h ago"),
                BoardTaskStep::new("Rewrote Day 1, Day 3 and Day 7 emails", "4h ago"),
                BoardTaskStep::new("Scheduled send windows", "4h ago"),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        BoardColumn, BoardScope, BoardTaskStatus, apply_approve_task, apply_reject_task,
        apply_retry_task, default_board_tasks, filter_board_tasks, ordered_navigation_ids,
    };

    #[test]
    fn default_tasks_partition_into_needs_you_running_and_done() {
        let tasks = default_board_tasks();
        assert_eq!(tasks.len(), 7);
        assert_eq!(
            filter_board_tasks(&tasks, BoardScope::NeedsYou, None, "").len(),
            2
        );
        assert_eq!(
            filter_board_tasks(&tasks, BoardScope::Running, None, "").len(),
            3
        );
        assert_eq!(
            filter_board_tasks(&tasks, BoardScope::Done, None, "").len(),
            2
        );
        assert_eq!(
            filter_board_tasks(&tasks, BoardScope::All, Some("writer"), "").len(),
            3
        );

        let nav = ordered_navigation_ids(&tasks, BoardScope::All, None, "");
        assert_eq!(nav.first().map(|s| s.as_ref()), Some("migrate"));
        assert_eq!(nav.last().map(|s| s.as_ref()), Some("onboarding-emails"));
    }

    #[test]
    fn approve_reject_and_retry_move_tasks_across_columns() {
        let mut tasks = default_board_tasks();
        assert!(apply_approve_task(&mut tasks, "migrate"));
        let migrated = tasks
            .iter()
            .find(|t| t.id.as_ref() == "migrate")
            .expect("migrate exists");
        assert_eq!(migrated.status, BoardTaskStatus::Running);
        assert_eq!(migrated.status.column(), BoardColumn::Running);

        assert!(apply_retry_task(&mut tasks, "crm-cleanup"));
        let retried = tasks
            .iter()
            .find(|t| t.id.as_ref() == "crm-cleanup")
            .expect("crm-cleanup exists");
        assert_eq!(retried.status, BoardTaskStatus::Running);

        tasks[0].status = BoardTaskStatus::Waiting;
        assert!(apply_reject_task(&mut tasks, "migrate"));
        assert_eq!(tasks[0].status, BoardTaskStatus::Stopped);
        assert_eq!(tasks[0].status.column(), BoardColumn::Done);
    }
}
