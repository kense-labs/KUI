//! Project-grouped agent task list with progress rings, decision badges, and live inspector.
//!
//! Renders [`AgentTasksScreen`] inside [`AppShell`] (`AppShellPreset::Agent`, active `"Tasks"`),
//! grouping active and completed tasks by project or status, surfacing pending Inbox asks,
//! and mounting a multi-tab task and artifact inspector in the right work area.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_ease, pro_eid,
            pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected, pro_pill_indicator,
            pro_presence, pro_progress_ring, pro_selection_sensor, pro_skeleton_bar,
            pro_spring_bar, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const FILE_DOC_SVG: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><path d="M4 6.6h4M4 8.6h2.6" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"></path>"#;
const FILE_SHEET_SVG: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><rect x="3.4" y="5.6" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="6.3" y="5.6" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="3.4" y="7.8" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="6.3" y="7.8" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect>"#;
const FILE_CODE_SVG: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><path d="M4 6.6h4M4 8.6h2.5" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"></path>"#;
const FILE_FOLDER_SVG: &str = r#"<path d="M1.4 3.1c0-.72.58-1.3 1.3-1.3h1.72c.38 0 .73.19.94.5l.6.9h3.34c.72 0 1.3.58 1.3 1.3V6H1.4Z" fill="currentColor"></path><path d="M1 6.95C.86 6.3 1.36 5.7 2.02 5.7h7.96c.66 0 1.16.6 1.02 1.25l-.5 2.4a1.3 1.3 0 0 1-1.27 1.05H2.77A1.3 1.3 0 0 1 1.5 9.35Z" fill="currentColor" opacity="0.45"></path>"#;
const FILE_IMAGE_SVG: &str = r#"<rect x="1.5" y="2" width="9" height="8" rx="1.3" fill="currentColor" opacity="0.45"></rect><circle cx="4.3" cy="4.7" r="1" fill="currentColor"></circle><path d="M2.2 9.2l2.6-2.7 1.9 1.8 1.4-1.3 1.7 2.2Z" fill="currentColor"></path>"#;

/// Execution lifecycle state of an [`AgentTask`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TaskStatus {
    Planning,
    #[default]
    Running,
    Paused,
    Done,
    Cancelled,
}

/// Top-level segmented filter tab on [`AgentTasksScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TaskTab {
    #[default]
    Active,
    Done,
}

/// Grouping dimension for the task tree on [`AgentTasksScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TaskGroupBy {
    #[default]
    Project,
    Status,
}

/// Kind of user input awaited by a [`TaskAsk`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TaskAskKind {
    #[default]
    Decision,
    Pick,
    Approval,
    Question,
}

/// Semantic accent color for a task's progress ring.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TaskRingTone {
    Amber,
    #[default]
    Teal,
    Orange,
    Green,
    Blue,
    Muted,
}

impl TaskRingTone {
    fn resolve(self, p: &ProPalette) -> Hsla {
        match self {
            Self::Amber => p.amber,
            Self::Teal => p.teal,
            Self::Orange => p.orange,
            Self::Green => p.green,
            Self::Blue => p.blue,
            Self::Muted => p.muted,
        }
    }
}

/// One checklist milestone inside an [`AgentTask`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskStep {
    pub label: SharedString,
    pub done: bool,
}

impl TaskStep {
    /// Construct a task step with `label` and completion flag `done`.
    pub fn new(label: impl Into<SharedString>, done: bool) -> Self {
        Self {
            label: label.into(),
            done,
        }
    }
}

/// Pending decision, pick, or approval waiting on the user in Inbox.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskAsk {
    pub id: SharedString,
    pub question: SharedString,
    pub kind: TaskAskKind,
}

impl TaskAsk {
    /// Construct a pending task ask linked to Inbox item `id`.
    pub fn new(
        id: impl Into<SharedString>,
        question: impl Into<SharedString>,
        kind: TaskAskKind,
    ) -> Self {
        Self {
            id: id.into(),
            question: question.into(),
            kind,
        }
    }
}

/// Previously resolved user decision recorded on an [`AgentTask`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskDecided {
    pub question: SharedString,
    pub answer: SharedString,
    pub at: SharedString,
}

impl TaskDecided {
    /// Construct a historical decision record.
    pub fn new(
        question: impl Into<SharedString>,
        answer: impl Into<SharedString>,
        at: impl Into<SharedString>,
    ) -> Self {
        Self {
            question: question.into(),
            answer: answer.into(),
            at: at.into(),
        }
    }
}

/// File viewer category for task artifacts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TaskArtifactKind {
    #[default]
    Doc,
    Sheet,
    Code,
    Folder,
    Image,
}

impl TaskArtifactKind {
    /// Infer the artifact glyph kind from a file or directory name.
    pub fn from_filename(name: &str) -> Self {
        let lower = name.trim().to_ascii_lowercase();
        if lower.ends_with(".csv") || lower.ends_with(".xlsx") || lower.ends_with(".tsv") {
            Self::Sheet
        } else if lower.ends_with(".json")
            || lower.ends_with(".ts")
            || lower.ends_with(".rs")
            || lower.ends_with(".sql")
            || lower.ends_with(".py")
        {
            Self::Code
        } else if lower.ends_with(".png")
            || lower.ends_with(".jpg")
            || lower.ends_with(".svg")
            || lower.ends_with(".webp")
        {
            Self::Image
        } else if !lower.contains('.') {
            Self::Folder
        } else {
            Self::Doc
        }
    }

    fn color(self, p: &ProPalette) -> Hsla {
        match self {
            Self::Doc => p.blue,
            Self::Sheet => p.green,
            Self::Code => p.orange,
            Self::Folder => p.amber,
            Self::Image => p.pink,
        }
    }

    fn svg(self) -> &'static str {
        match self {
            Self::Doc => FILE_DOC_SVG,
            Self::Sheet => FILE_SHEET_SVG,
            Self::Code => FILE_CODE_SVG,
            Self::Folder => FILE_FOLDER_SVG,
            Self::Image => FILE_IMAGE_SVG,
        }
    }
}

/// Previewable workspace artifact generated by an [`AgentTask`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskArtifact {
    pub name: SharedString,
    pub kind: TaskArtifactKind,
    pub updated: SharedString,
    pub heading: SharedString,
    pub body: SharedString,
    pub table: Vec<Vec<SharedString>>,
}

impl TaskArtifact {
    /// Construct a document or code artifact preview.
    pub fn doc(
        name: impl Into<SharedString>,
        updated: impl Into<SharedString>,
        heading: impl Into<SharedString>,
        body: impl Into<SharedString>,
    ) -> Self {
        let name_s: SharedString = name.into();
        let kind = TaskArtifactKind::from_filename(&name_s);
        Self {
            name: name_s,
            kind,
            updated: updated.into(),
            heading: heading.into(),
            body: body.into(),
            table: Vec::new(),
        }
    }

    /// Construct a tabular sheet artifact preview.
    pub fn sheet(
        name: impl Into<SharedString>,
        updated: impl Into<SharedString>,
        rows: Vec<Vec<SharedString>>,
    ) -> Self {
        let name_s: SharedString = name.into();
        Self {
            heading: name_s.clone(),
            name: name_s,
            kind: TaskArtifactKind::Sheet,
            updated: updated.into(),
            body: SharedString::default(),
            table: rows,
        }
    }
}

/// One orchestrator task row and its inspector details.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentTask {
    pub id: SharedString,
    pub title: SharedString,
    pub project: SharedString,
    pub status: TaskStatus,
    pub status_text: SharedString,
    pub ring_tone: TaskRingTone,
    pub steps: Vec<TaskStep>,
    pub artifacts: Vec<SharedString>,
    pub asks: Vec<TaskAsk>,
    pub updated_label: SharedString,
    pub done_label: Option<SharedString>,
    pub goal: SharedString,
    pub started: SharedString,
    pub history: Vec<SharedString>,
    pub decided: Vec<TaskDecided>,
    pub agent: Option<SharedString>,
    pub apps: Vec<SharedString>,
}

impl AgentTask {
    /// Return whether this task is completed or cancelled and belongs under `Done`.
    pub fn is_done(&self) -> bool {
        matches!(self.status, TaskStatus::Done | TaskStatus::Cancelled) || self.done_label.is_some()
    }

    /// Return the count of open questions/decisions waiting on the user.
    pub fn needs_count(&self) -> usize {
        if self.is_done() { 0 } else { self.asks.len() }
    }

    /// Return `(completed_steps, total_steps)`.
    pub fn step_counts(&self) -> (usize, usize) {
        let done = self.steps.iter().filter(|s| s.done).count();
        (done, self.steps.len())
    }

    /// Return normalized step completion in `0.0..=1.0`.
    pub fn progress(&self) -> f32 {
        if self.is_done() {
            return 1.0;
        }
        let (done, total) = self.step_counts();
        if total == 0 {
            0.0
        } else {
            (done as f32 / total as f32).clamp(0.0, 1.0)
        }
    }
}

/// Events emitted by [`AgentTasksScreen`].
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
pub enum AgentTasksEvent {
    Navigate(SharedString),
    SelectTask(SharedString),
    CreateTask(AgentTask),
    OpenInbox {
        task_id: SharedString,
        item_id: Option<SharedString>,
    },
    OpenArtifact {
        task_id: SharedString,
        name: SharedString,
    },
    StepToggled {
        task_id: SharedString,
        step_index: usize,
        done: bool,
    },
    StatusChanged {
        task_id: SharedString,
        status: TaskStatus,
    },
    Approved {
        task_id: SharedString,
        ask_id: SharedString,
    },
}

impl EventEmitter<AgentTasksEvent> for AgentTasksScreen {}

#[derive(Clone, Debug, PartialEq, Eq)]
enum InspectorPane {
    Task,
    Artifact(SharedString),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TreeFocusTarget {
    Group(SharedString),
    Task(SharedString),
}

/// Interactive Agent / Tasks workspace screen (`agent-tasks`).
pub struct AgentTasksScreen {
    id: ElementId,
    id_name: SharedString,
    shell: Entity<AppShell>,
    standalone: bool,
    tasks: Vec<AgentTask>,
    projects: Vec<SharedString>,
    artifacts: Vec<TaskArtifact>,
    tab: TaskTab,
    group_by: TaskGroupBy,
    selected_id: Option<SharedString>,
    folded_groups: Vec<SharedString>,
    inspector_tabs: Vec<InspectorPane>,
    active_inspector_tab: usize,
    show_agents_drawer: bool,
    new_open: bool,
    new_project: SharedString,
    project_menu_open: bool,
    new_editor: Option<Entity<Editor>>,
    editor_focused: bool,
    loading: bool,
    empty: bool,
    focus: FocusHandle,
    tree_cursor: Option<TreeFocusTarget>,
    kb_nav: bool,
    fresh_task: Option<(SharedString, Instant)>,
    boot_at: Instant,
    measured_width: Rc<Cell<f32>>,
    mobile_sheet_open: bool,
    announcement: SharedString,
}

impl Focusable for AgentTasksScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl AgentTasksScreen {
    fn build_shell(
        id_str: &str,
        workspace: Option<SharedString>,
        cx: &mut Context<Self>,
    ) -> Entity<AppShell> {
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Agent, shell_cx)
                .with_active("Tasks")
                .with_work_area(true, true)
                .with_panel_width(340.);
            if let Some(ws) = workspace {
                s = s.with_workspace(ws);
            }
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_inspector(w, cx));
            s
        });
        cx.subscribe(&shell, |this, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => cx.emit(AgentTasksEvent::Navigate(label.clone())),
            AppShellEvent::PanelOpen(false) => {
                this.mobile_sheet_open = false;
                cx.notify();
            }
            _ => {}
        })
        .detach();
        shell
    }

    /// Construct an [`AgentTasksScreen`] populated with the canonical sample tasks and artifacts.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let shell = Self::build_shell(&id_str, None, cx);

        let projects = vec!["Northwind".into(), "Platform".into(), "Evalkit".into()];
        let tasks = sample_tasks();
        let artifacts = sample_artifacts();

        Self {
            id: ElementId::Name(id_str.clone()),
            id_name: id_str,
            shell,
            standalone: false,
            tasks,
            projects,
            artifacts,
            tab: TaskTab::Active,
            group_by: TaskGroupBy::Project,
            selected_id: Some("ingest".into()),
            folded_groups: Vec::new(),
            inspector_tabs: vec![InspectorPane::Task],
            active_inspector_tab: 0,
            show_agents_drawer: false,
            new_open: false,
            new_project: "Northwind".into(),
            project_menu_open: false,
            new_editor: None,
            editor_focused: false,
            loading: false,
            empty: false,
            focus: cx.focus_handle(),
            tree_cursor: Some(TreeFocusTarget::Task("ingest".into())),
            kb_nav: false,
            fresh_task: None,
            boot_at: Instant::now(),
            measured_width: Rc::new(Cell::new(920.)),
            mobile_sheet_open: false,
            announcement: SharedString::default(),
        }
    }

    /// Render only the inner Tasks page without the surrounding [`AppShell`].
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Override the workspace name displayed in the [`AppShell`] sidebar.
    pub fn with_workspace(
        mut self,
        workspace: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let ws: SharedString = workspace.into();
        self.shell = Self::build_shell(&self.id_name, Some(ws), cx);
        self
    }

    /// Replace the initial task list.
    pub fn with_tasks(mut self, tasks: Vec<AgentTask>) -> Self {
        self.tasks = tasks;
        if let Some(sel) = &self.selected_id
            && !self.tasks.iter().any(|t| &t.id == sel)
        {
            self.selected_id = self.tasks.first().map(|t| t.id.clone());
        }
        self
    }

    /// Override the ordered project names used for grouping and task creation.
    pub fn with_projects(mut self, projects: Vec<SharedString>) -> Self {
        if let Some(first) = projects.first() {
            self.new_project = first.clone();
        }
        self.projects = projects;
        self
    }

    /// Choose the initial `Active` or `Done` tab.
    pub fn with_tab(mut self, tab: TaskTab) -> Self {
        self.tab = tab;
        self
    }

    /// Group tasks by `Project` (default) or `Status`.
    pub fn with_group_by(mut self, group_by: TaskGroupBy) -> Self {
        self.group_by = group_by;
        self
    }

    /// Set the initially selected task id (`None` leaves no task selected).
    pub fn with_selected(mut self, selected: Option<SharedString>) -> Self {
        self.selected_id = selected;
        self
    }

    /// Configure the skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        if empty {
            self.selected_id = None;
        }
        self
    }

    /// Toggle skeleton loading mode at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Toggle empty mode at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        if empty {
            self.selected_id = None;
        } else if self.selected_id.is_none() {
            self.selected_id = self.tasks.first().map(|t| t.id.clone());
        }
        cx.notify();
    }

    /// Switch between `Active` and `Done` tabs.
    pub fn set_tab(&mut self, tab: TaskTab, cx: &mut Context<Self>) {
        if self.tab != tab {
            self.tab = tab;
            let groups = self.visible_groups();
            let first_visible = groups
                .iter()
                .flat_map(|(_, idxs)| idxs.iter())
                .next()
                .and_then(|&idx| self.tasks.get(idx))
                .map(|t| t.id.clone());
            if let Some(sel) = &self.selected_id {
                let still_visible = self
                    .tasks
                    .iter()
                    .any(|t| &t.id == sel && t.is_done() == (tab == TaskTab::Done));
                if !still_visible {
                    self.selected_id = first_visible;
                    self.inspector_tabs = vec![InspectorPane::Task];
                    self.active_inspector_tab = 0;
                }
            } else {
                self.selected_id = first_visible;
            }
            cx.notify();
        }
    }

    /// Switch between `Project` and `Status` grouping.
    pub fn set_group_by(&mut self, group_by: TaskGroupBy, cx: &mut Context<Self>) {
        if self.group_by != group_by {
            self.group_by = group_by;
            self.folded_groups.clear();
            cx.notify();
        }
    }

    /// Select a task by id and ensure the right inspector is open.
    pub fn select_task(&mut self, id: impl Into<SharedString>, cx: &mut Context<Self>) {
        let id: SharedString = id.into();
        self.selected_id = Some(id.clone());
        self.tree_cursor = Some(TreeFocusTarget::Task(id.clone()));
        self.active_inspector_tab = 0;
        self.show_agents_drawer = false;
        self.mobile_sheet_open = true;
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        cx.emit(AgentTasksEvent::SelectTask(id));
        cx.notify();
    }

    /// Create a new planning task at the top of `project` and return it.
    pub fn create_task(
        &mut self,
        title: impl Into<SharedString>,
        project: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Option<AgentTask> {
        let trimmed = title.into().trim().to_string();
        if trimmed.is_empty() {
            return None;
        }
        let proj: SharedString = project.into();
        if !self.projects.iter().any(|p| p == &proj) {
            self.projects.push(proj.clone());
        }
        let new_id: SharedString = format!("task-{}", self.tasks.len() + 1).into();
        let task = AgentTask {
            id: new_id.clone(),
            title: trimmed.clone().into(),
            project: proj.clone(),
            status: TaskStatus::Planning,
            status_text: "Planning the steps".into(),
            ring_tone: TaskRingTone::Muted,
            steps: vec![
                TaskStep::new("Scope project context and inputs", false),
                TaskStep::new("Draft execution plan", false),
                TaskStep::new("Deliver verified output", false),
            ],
            artifacts: Vec::new(),
            asks: Vec::new(),
            updated_label: "now".into(),
            done_label: None,
            goal: trimmed.into(),
            started: "Just now".into(),
            history: vec!["Created task and queued orchestrator planning".into()],
            decided: Vec::new(),
            agent: Some("Orchestrator".into()),
            apps: Vec::new(),
        };
        self.empty = false;
        self.tab = TaskTab::Active;
        self.folded_groups.retain(|g| g != &proj);
        self.tasks.insert(0, task.clone());
        self.selected_id = Some(new_id.clone());
        self.tree_cursor = Some(TreeFocusTarget::Task(new_id.clone()));
        self.active_inspector_tab = 0;
        self.fresh_task = Some((new_id, Instant::now()));
        self.announcement = format!("Task created in {proj}").into();
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        cx.emit(AgentTasksEvent::CreateTask(task.clone()));
        cx.notify();
        Some(task)
    }

    /// Toggle completion of `step_index` on `task_id`.
    pub fn toggle_step(&mut self, task_id: &str, step_index: usize, cx: &mut Context<Self>) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id.as_ref() == task_id)
            && let Some(step) = task.steps.get_mut(step_index)
        {
            step.done = !step.done;
            let done_now = step.done;
            cx.emit(AgentTasksEvent::StepToggled {
                task_id: task.id.clone(),
                step_index,
                done: done_now,
            });
            cx.notify();
        }
    }

    /// Pause a running task or resume a paused task.
    pub fn pause_or_resume_task(&mut self, task_id: &str, cx: &mut Context<Self>) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id.as_ref() == task_id)
            && !task.is_done()
        {
            task.status = match task.status {
                TaskStatus::Paused => TaskStatus::Running,
                _ => TaskStatus::Paused,
            };
            let next_status = task.status;
            if next_status == TaskStatus::Paused {
                task.history.insert(0, "Paused by you".into());
            } else {
                task.history.insert(0, "Resumed execution".into());
            }
            cx.emit(AgentTasksEvent::StatusChanged {
                task_id: task.id.clone(),
                status: next_status,
            });
            cx.notify();
        }
    }

    /// Cancel an active task and move it out of the active queue.
    pub fn cancel_task(&mut self, task_id: &str, cx: &mut Context<Self>) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id.as_ref() == task_id)
            && !task.is_done()
        {
            task.status = TaskStatus::Cancelled;
            task.status_text = "Cancelled".into();
            task.done_label = Some("Cancelled".into());
            task.asks.clear();
            task.history.insert(0, "Cancelled by you".into());
            let tid = task.id.clone();
            cx.emit(AgentTasksEvent::StatusChanged {
                task_id: tid,
                status: TaskStatus::Cancelled,
            });
            cx.notify();
        }
    }

    /// Approve the pending approval ask on `task_id` and advance the task.
    pub fn approve_task(&mut self, task_id: &str, cx: &mut Context<Self>) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id.as_ref() == task_id)
            && !task.asks.is_empty()
        {
            let ask = task.asks.remove(0);
            task.decided
                .insert(0, TaskDecided::new(ask.question, "Approved", "Just now"));
            if let Some(step) = task.steps.iter_mut().find(|s| !s.done) {
                step.done = true;
            }
            if task.steps.iter().all(|s| s.done) && !task.steps.is_empty() {
                task.status = TaskStatus::Done;
                task.done_label = Some("Now".into());
                task.ring_tone = TaskRingTone::Green;
            }
            task.history.insert(0, "Approved by you".into());
            let tid = task.id.clone();
            cx.emit(AgentTasksEvent::Approved {
                task_id: tid,
                ask_id: ask.id,
            });
            cx.notify();
        }
    }

    /// Compute the visible `(group_heading, task_indices)` sections for the active tab and grouping mode.
    pub fn visible_groups(&self) -> Vec<(SharedString, Vec<usize>)> {
        if self.empty {
            return Vec::new();
        }
        let want_done = self.tab == TaskTab::Done;
        match self.group_by {
            TaskGroupBy::Project => {
                let mut ordered_projects: Vec<SharedString> = self.projects.clone();
                for t in &self.tasks {
                    if t.is_done() == want_done && !ordered_projects.contains(&t.project) {
                        ordered_projects.push(t.project.clone());
                    }
                }
                let mut out = Vec::new();
                for proj in ordered_projects {
                    let idxs: Vec<usize> = self
                        .tasks
                        .iter()
                        .enumerate()
                        .filter(|(_, t)| t.is_done() == want_done && t.project == proj)
                        .map(|(i, _)| i)
                        .collect();
                    if !idxs.is_empty() {
                        out.push((proj, idxs));
                    }
                }
                out
            }
            TaskGroupBy::Status => {
                if want_done {
                    let idxs: Vec<usize> = self
                        .tasks
                        .iter()
                        .enumerate()
                        .filter(|(_, t)| t.is_done())
                        .map(|(i, _)| i)
                        .collect();
                    if idxs.is_empty() {
                        Vec::new()
                    } else {
                        vec![("Done".into(), idxs)]
                    }
                } else {
                    let mut waiting = Vec::new();
                    let mut running = Vec::new();
                    let mut queued = Vec::new();
                    for (i, t) in self.tasks.iter().enumerate() {
                        if t.is_done() {
                            continue;
                        }
                        if t.needs_count() > 0 {
                            waiting.push(i);
                        } else if matches!(t.status, TaskStatus::Planning | TaskStatus::Paused) {
                            queued.push(i);
                        } else {
                            running.push(i);
                        }
                    }
                    let mut out = Vec::new();
                    if !running.is_empty() {
                        out.push(("Running".into(), running));
                    }
                    if !waiting.is_empty() {
                        out.push(("Waiting on you".into(), waiting));
                    }
                    if !queued.is_empty() {
                        out.push(("Queued".into(), queued));
                    }
                    out
                }
            }
        }
    }

    /// Count active (non-completed) tasks.
    pub fn active_task_count(&self) -> usize {
        if self.empty {
            0
        } else {
            self.tasks.iter().filter(|t| !t.is_done()).count()
        }
    }

    /// Count open decisions/questions across all active tasks.
    pub fn total_needs_count(&self) -> usize {
        if self.empty {
            0
        } else {
            self.tasks.iter().map(AgentTask::needs_count).sum()
        }
    }

    fn open_artifact_tab(
        &mut self,
        task_id: SharedString,
        name: SharedString,
        cx: &mut Context<Self>,
    ) {
        if let Some(pos) = self
            .inspector_tabs
            .iter()
            .position(|p| matches!(p, InspectorPane::Artifact(n) if n == &name))
        {
            self.active_inspector_tab = pos;
        } else {
            self.inspector_tabs
                .push(InspectorPane::Artifact(name.clone()));
            self.active_inspector_tab = self.inspector_tabs.len() - 1;
        }
        self.mobile_sheet_open = true;
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        cx.emit(AgentTasksEvent::OpenArtifact { task_id, name });
        cx.notify();
    }

    fn close_inspector_tab(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx == 0 && self.inspector_tabs.len() == 1 {
            self.mobile_sheet_open = false;
            self.shell.update(cx, |s, cx| s.set_panel_open(false, cx));
            cx.notify();
            return;
        }
        if idx < self.inspector_tabs.len() && idx > 0 {
            self.inspector_tabs.remove(idx);
            if self.active_inspector_tab >= self.inspector_tabs.len() {
                self.active_inspector_tab = self.inspector_tabs.len().saturating_sub(1);
            }
            cx.notify();
        }
    }

    fn toggle_group(&mut self, name: &SharedString, cx: &mut Context<Self>) {
        if let Some(pos) = self.folded_groups.iter().position(|g| g == name) {
            self.folded_groups.remove(pos);
        } else {
            self.folded_groups.push(name.clone());
        }
        cx.notify();
    }

    fn flat_tree_items(&self) -> Vec<TreeFocusTarget> {
        let mut items = Vec::new();
        for (grp, idxs) in self.visible_groups() {
            let folded = self.folded_groups.contains(&grp);
            items.push(TreeFocusTarget::Group(grp));
            if !folded {
                for idx in idxs {
                    if let Some(t) = self.tasks.get(idx) {
                        items.push(TreeFocusTarget::Task(t.id.clone()));
                    }
                }
            }
        }
        items
    }

    fn move_row_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let task_ids: Vec<SharedString> = self
            .visible_groups()
            .into_iter()
            .filter(|(g, _)| !self.folded_groups.contains(g))
            .flat_map(|(_, idxs)| idxs)
            .filter_map(|i| self.tasks.get(i).map(|t| t.id.clone()))
            .collect();
        if task_ids.is_empty() {
            return;
        }
        let cur = self
            .selected_id
            .as_ref()
            .and_then(|s| task_ids.iter().position(|id| id == s))
            .unwrap_or(0);
        let next = (cur as isize + delta).clamp(0, task_ids.len() as isize - 1) as usize;
        if let Some(target) = task_ids.get(next).cloned() {
            self.kb_nav = true;
            self.select_task(target, cx);
        }
    }

    fn move_tree_cursor(&mut self, delta: isize, to_edge: Option<bool>, cx: &mut Context<Self>) {
        let items = self.flat_tree_items();
        if items.is_empty() {
            return;
        }
        let next_idx = if let Some(end) = to_edge {
            if end { items.len() - 1 } else { 0 }
        } else {
            let cur = self
                .tree_cursor
                .as_ref()
                .and_then(|c| items.iter().position(|it| it == c))
                .unwrap_or(0);
            (cur as isize + delta).clamp(0, items.len() as isize - 1) as usize
        };
        if let Some(target) = items.get(next_idx).cloned() {
            self.kb_nav = true;
            if let TreeFocusTarget::Task(ref tid) = target {
                self.selected_id = Some(tid.clone());
                cx.emit(AgentTasksEvent::SelectTask(tid.clone()));
            }
            self.tree_cursor = Some(target);
            cx.notify();
        }
    }

    fn handle_key(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if self.editor_focused || ev.keystroke.modifiers.platform || ev.keystroke.modifiers.control
        {
            return;
        }
        let k = ev.keystroke.key.as_str();
        match k {
            "j" => self.move_row_selection(1, cx),
            "k" => self.move_row_selection(-1, cx),
            "down" => self.move_tree_cursor(1, None, cx),
            "up" => self.move_tree_cursor(-1, None, cx),
            "home" => self.move_tree_cursor(0, Some(false), cx),
            "end" => self.move_tree_cursor(0, Some(true), cx),
            "c" | "n" => {
                self.new_open = true;
                self.project_menu_open = false;
                if let Some(ed) = self.new_editor.clone() {
                    ed.update(cx, |e, cx| e.focus(w, cx));
                }
                cx.notify();
            }
            "escape" => {
                if self.project_menu_open {
                    self.project_menu_open = false;
                    cx.notify();
                } else if self.new_open {
                    self.new_open = false;
                    cx.notify();
                } else if self.mobile_sheet_open {
                    self.mobile_sheet_open = false;
                    cx.notify();
                }
            }
            "enter" | "space" => {
                if let Some(cursor) = self.tree_cursor.clone() {
                    match cursor {
                        TreeFocusTarget::Group(g) => self.toggle_group(&g, cx),
                        TreeFocusTarget::Task(tid) => self.select_task(tid, cx),
                    }
                }
            }
            "left" => {
                if let Some(cursor) = self.tree_cursor.clone() {
                    match cursor {
                        TreeFocusTarget::Group(g) => {
                            if !self.folded_groups.contains(&g) {
                                self.folded_groups.push(g);
                                cx.notify();
                            }
                        }
                        TreeFocusTarget::Task(tid) => {
                            if let Some(task) = self.tasks.iter().find(|t| t.id == tid) {
                                let grp = match self.group_by {
                                    TaskGroupBy::Project => task.project.clone(),
                                    TaskGroupBy::Status => {
                                        if task.is_done() {
                                            "Done".into()
                                        } else if task.needs_count() > 0 {
                                            "Waiting on you".into()
                                        } else if matches!(
                                            task.status,
                                            TaskStatus::Planning | TaskStatus::Paused
                                        ) {
                                            "Queued".into()
                                        } else {
                                            "Running".into()
                                        }
                                    }
                                };
                                self.kb_nav = true;
                                self.tree_cursor = Some(TreeFocusTarget::Group(grp));
                                cx.notify();
                            }
                        }
                    }
                }
            }
            "right" => {
                if let Some(TreeFocusTarget::Group(g)) = self.tree_cursor.clone() {
                    if self.folded_groups.contains(&g) {
                        self.folded_groups.retain(|x| x != &g);
                        cx.notify();
                    } else {
                        self.move_tree_cursor(1, None, cx);
                    }
                }
            }
            "delete" | "backspace" if self.active_inspector_tab > 0 => {
                self.close_inspector_tab(self.active_inspector_tab, cx);
            }
            _ => {}
        }
    }

    fn ensure_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.new_editor.is_none() {
            let ph: SharedString = tr(
                cx,
                "Describe a task for the orchestrator…",
                "描述交由协调器执行的任务…",
            )
            .into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::InlineMulti, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_entity, ev: &EditEvent, w, cx| match ev {
                EditEvent::Focused => {
                    this.editor_focused = true;
                    cx.notify();
                }
                EditEvent::Blurred => {
                    this.editor_focused = false;
                    cx.notify();
                }
                EditEvent::Changed => {
                    cx.notify();
                }
                EditEvent::Submitted => {
                    let val = ed_entity.read(cx).value().to_string();
                    let proj = this.new_project.clone();
                    if this.create_task(val, proj, cx).is_some() {
                        ed_entity.update(cx, |e, cx| e.set_value("", w, cx));
                    }
                }
                EditEvent::Escape => {
                    this.new_open = false;
                    this.project_menu_open = false;
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.new_editor = Some(ed);
        }
    }

    fn render_new_composer(&self, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let draft = self
            .new_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_string())
            .unwrap_or_default();
        let can_create = !draft.is_empty();
        let proj_label = self.new_project.clone();
        let ed_clone = self.new_editor.clone();

        let mut bar = div()
            .relative()
            .mx(px(8.))
            .mt(px(8.))
            .pl(px(12.))
            .pr(px(6.))
            .py(px(8.))
            .min_h(px(48.))
            .rounded(px(8.))
            .bg(p.card)
            .border_1()
            .border_color(if self.editor_focused {
                p.line_strong
            } else {
                p.line
            })
            .flex()
            .items_center()
            .gap(px(8.))
            .child(task_pie_ring(0., 14., p.muted, TaskStatus::Planning))
            .when_some(ed_clone, |d, ed| {
                d.child(div().flex_1().min_w_0().h(px(30.)).child(ed))
            })
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "new-project-btn"))
                    .role(Role::Button)
                    .aria_label(proj_label.clone())
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .hover(move |s| s.bg(p.hover))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .cursor_pointer()
                    .child(pro_icon(ProIcon::Folder, 13., p.muted))
                    .child(proj_label)
                    .child(pro_icon(ProIcon::ChevronDown, 12., p.faint)),
                cx,
                |this, _, cx| {
                    this.project_menu_open = !this.project_menu_open;
                    cx.notify();
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "new-create-btn"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Create", "创建"))
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(10.))
                    .rounded(px(6.))
                    .bg(p.primary.opacity(if can_create { 1.0 } else { 0.40 }))
                    .text_color(p.on_primary)
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(can_create, |d| d.cursor_pointer())
                    .child(tr(cx, "Create", "创建")),
                cx,
                |this, w, cx| {
                    if let Some(ed) = this.new_editor.clone() {
                        let val = ed.read(cx).value().to_string();
                        let proj = this.new_project.clone();
                        if this.create_task(val, proj, cx).is_some() {
                            ed.update(cx, |e, cx| e.set_value("", w, cx));
                        }
                    }
                },
            ));

        let menu_items: Vec<Stateful<Div>> = self
            .projects
            .iter()
            .enumerate()
            .map(|(pi, proj)| {
                let chosen = proj == &self.new_project;
                let p_clone = proj.clone();
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("proj-opt-{pi}")))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(30.))
                        .px(px(9.))
                        .rounded(px(7.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(13.))
                        .font_weight(if chosen {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(if chosen { p.ink } else { p.text })
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Folder, 14., p.muted))
                        .child(div().flex_1().truncate().child(proj.clone()))
                        .when(chosen, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink))),
                    cx,
                    move |this, _, cx| {
                        this.new_project = p_clone.clone();
                        this.project_menu_open = false;
                        cx.notify();
                    },
                )
            })
            .collect();
        let menu_eid = pro_eid(&self.id, "project-menu");
        bar = bar.child(
            div()
                .absolute()
                .top(px(44.))
                .right(px(68.))
                .child(pro_presence(
                    pro_eid(&self.id, "proj-menu-pres"),
                    self.project_menu_open,
                    0.16,
                    0.12,
                    move |amount| {
                        div()
                            .id(menu_eid)
                            .role(Role::Menu)
                            .mt(px(-5. * (1. - amount)))
                            .opacity(amount)
                            .w(px(176.))
                            .p(px(4.))
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line_strong)
                            .shadow_md()
                            .flex()
                            .flex_col()
                            .gap(px(1.))
                            .children(menu_items)
                            .into_any_element()
                    },
                )),
        );
        bar
    }

    fn render_loading_list(phase: f32, cx: &App) -> Div {
        let mut wrap = div().p(px(8.)).pb(px(24.)).flex().flex_col().gap(px(12.));
        for (gi, row_widths) in [
            vec![(118., 104.), (132., 148.), (106., 92.)],
            vec![(124., 130.), (98., 142.)],
        ]
        .into_iter()
        .enumerate()
        {
            let mut grp = div().flex().flex_col().gap(px(2.)).child(
                div()
                    .h(px(32.))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(pro_skeleton_bar(14., 14., 4., phase, cx))
                    .child(pro_skeleton_bar(
                        if gi == 0 { 76. } else { 64. },
                        12.,
                        4.,
                        phase,
                        cx,
                    )),
            );
            for (tw, sw) in row_widths {
                grp = grp.child(
                    div()
                        .h(px(40.))
                        .px(px(12.))
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(14., 14., 7., phase, cx))
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .child(pro_skeleton_bar(tw, 12., 4., phase, cx))
                                .child(pro_skeleton_bar(sw, 11., 4., phase + 0.15, cx)),
                        )
                        .child(pro_skeleton_bar(24., 11., 4., phase + 0.25, cx))
                        .child(pro_skeleton_bar(24., 11., 4., phase + 0.3, cx)),
                );
            }
            wrap = wrap.child(grp);
        }
        wrap
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_editor(w, cx);
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let elapsed_s = self.boot_at.elapsed().as_secs_f32();
        let skel_phase = if reduced { 0. } else { elapsed_s * 0.75 };

        // Animate fresh row entrance or skeleton pulse when active
        let fresh_alpha = if let Some((_, created_at)) = &self.fresh_task {
            let ms = created_at.elapsed().as_secs_f32() * 1000.;
            if !reduced && ms < 420. {
                w.request_animation_frame();
                pro_ease(ms / 420.)
            } else {
                self.fresh_task = None;
                1.0
            }
        } else {
            1.0
        };
        if self.loading && !reduced {
            w.request_animation_frame();
        }

        let measure_cell = self.measured_width.clone();
        let page_w = measure_cell.get();
        let narrow = page_w < 560.;
        let tiny = page_w < 400.;
        let panel_open = !self.standalone && self.shell.read(cx).is_panel_open();

        let active_n = self.active_task_count();
        let tab_sensor = pro_selection_sensor();
        let tab_key: SharedString = match self.tab {
            TaskTab::Active => "active".into(),
            TaskTab::Done => "done".into(),
        };
        let grp_sensor = pro_selection_sensor();
        let grp_key: SharedString = match self.group_by {
            TaskGroupBy::Project => "project".into(),
            TaskGroupBy::Status => "status".into(),
        };
        let header = div()
            .h(px(48.))
            .pl(px(20.))
            .pr(px(if panel_open || self.standalone {
                12.
            } else {
                48.
            }))
            .border_b_1()
            .border_color(p.line)
            .flex_none()
            .flex()
            .items_center()
            .gap(px(if narrow { 8. } else { 12. }))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(px(7.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tr(cx, "Tasks", "任务")),
                    )
                    .when(!self.loading, |d| {
                        d.child(
                            div()
                                .text_size(px(13.))
                                .text_color(p.faint)
                                .child(active_n.to_string()),
                        )
                    }),
            )
            .child(
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .child(pro_pill_indicator(
                        pro_eid(&self.id, "task-tab-ind"),
                        tab_key,
                        tab_sensor.clone(),
                        6.,
                        p.chip,
                        None,
                        false,
                    ))
                    .children(
                        [
                            (TaskTab::Active, tr(cx, "Active", "进行中")),
                            (TaskTab::Done, tr(cx, "Done", "已完成")),
                        ]
                        .into_iter()
                        .map(|(t_val, lbl)| {
                            let selected = self.tab == t_val;
                            let suffix = if t_val == TaskTab::Active {
                                "tab-active"
                            } else {
                                "tab-done"
                            };
                            let tab_sensor_item = tab_sensor.clone();
                            activate(
                                div()
                                    .id(pro_eid(&self.id, suffix))
                                    .role(Role::Tab)
                                    .aria_label(lbl)
                                    .tab_index(0)
                                    .relative()
                                    .h(px(28.))
                                    .px(px(8.))
                                    .rounded(px(6.))
                                    .hover(move |s| {
                                        if selected {
                                            s.text_color(p.ink)
                                        } else {
                                            s.text_color(p.ink).bg(p.hover)
                                        }
                                    })
                                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                                    .flex()
                                    .items_center()
                                    .text_size(px(13.))
                                    .text_color(if selected { p.ink } else { p.muted })
                                    .cursor_pointer()
                                    .when(selected, |d| {
                                        d.child(pro_mark_selected(&tab_sensor_item))
                                    })
                                    .child(lbl),
                                cx,
                                move |this, _, cx| this.set_tab(t_val, cx),
                            )
                        }),
                    ),
            )
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .when(!tiny, |d| {
                        d.child(
                            div()
                                .relative()
                                .h(px(28.))
                                .p(px(2.))
                                .rounded(px(7.))
                                .bg(p.chip)
                                .flex()
                                .items_center()
                                .gap(px(2.))
                                .child(pro_pill_indicator(
                                    pro_eid(&self.id, "grp-by-ind"),
                                    grp_key,
                                    grp_sensor.clone(),
                                    5.,
                                    p.card,
                                    Some(p.line),
                                    true,
                                ))
                                .children(
                                    [
                                        (
                                            TaskGroupBy::Project,
                                            "grp-by-project",
                                            tr(cx, "Project", "按项目"),
                                        ),
                                        (
                                            TaskGroupBy::Status,
                                            "grp-by-status",
                                            tr(cx, "Status", "按状态"),
                                        ),
                                    ]
                                    .into_iter()
                                    .map(
                                        |(g_val, g_id, g_lbl)| {
                                            let selected = self.group_by == g_val;
                                            let grp_sensor_item = grp_sensor.clone();
                                            activate(
                                                div()
                                                    .id(pro_eid(&self.id, g_id))
                                                    .role(Role::Tab)
                                                    .aria_label(g_lbl)
                                                    .tab_index(0)
                                                    .relative()
                                                    .h(px(24.))
                                                    .px(px(8.))
                                                    .rounded(px(5.))
                                                    .hover(move |s| {
                                                        if selected {
                                                            s.text_color(p.ink)
                                                        } else {
                                                            s.text_color(p.ink).bg(p.hover)
                                                        }
                                                    })
                                                    .focus_visible(move |s| {
                                                        s.border_1().border_color(p.focus)
                                                    })
                                                    .flex()
                                                    .items_center()
                                                    .text_size(px(12.))
                                                    .font_weight(if selected {
                                                        FontWeight::MEDIUM
                                                    } else {
                                                        FontWeight::NORMAL
                                                    })
                                                    .text_color(if selected {
                                                        p.ink
                                                    } else {
                                                        p.muted
                                                    })
                                                    .cursor_pointer()
                                                    .when(selected, |d| {
                                                        d.child(pro_mark_selected(&grp_sensor_item))
                                                    })
                                                    .child(g_lbl),
                                                cx,
                                                move |this, _, cx| this.set_group_by(g_val, cx),
                                            )
                                        },
                                    ),
                                ),
                        )
                    })
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "head-new-task"))
                            .role(Role::Button)
                            .aria_label(tr(cx, "New task", "新建任务"))
                            .tab_index(0)
                            .h(px(28.))
                            .px(px(if tiny { 7. } else { 10. }))
                            .pl(px(if tiny { 7. } else { 8. }))
                            .rounded(px(6.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .hover(move |s| s.border_color(p.line_strong).bg(p.hover))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .cursor_pointer()
                            .child(pro_icon(ProIcon::Plus, 14., p.muted))
                            .when(!tiny, |d| d.child(tr(cx, "New task", "新建任务"))),
                        cx,
                        |this, w, cx| {
                            this.new_open = !this.new_open;
                            this.project_menu_open = false;
                            if this.new_open
                                && let Some(ed) = this.new_editor.clone()
                            {
                                ed.update(cx, |e, cx| e.focus(w, cx));
                            }
                            cx.notify();
                        },
                    )),
            );

        let body_scroll = if self.loading {
            div()
                .id(pro_eid(&self.id, "scroll"))
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(Self::render_loading_list(skel_phase, cx))
        } else {
            let groups = self.visible_groups();
            let list_vtx_key: SharedString =
                format!("{:?}-{:?}-{}", self.tab, self.group_by, groups.is_empty()).into();
            if groups.is_empty() {
                let empty_view = if self.tab == TaskTab::Active {
                    pro_empty_state(
                        ProIcon::Tasks,
                        tr(cx, "No active tasks", "暂无进行中的任务"),
                        tr(
                            cx,
                            "Describe what you want built or checked and the orchestrator will break it into steps.",
                            "描述你希望构建或核查的目标，协调器会自动拆分为可执行步骤。",
                        ),
                        Some((
                            pro_eid(&self.id, "empty-new-cta"),
                            tr(cx, "New task", "新建任务").into(),
                            Box::new(|this: &mut Self, w, cx| {
                                this.new_open = true;
                                if let Some(ed) = this.new_editor.clone() {
                                    ed.update(cx, |e, cx| e.focus(w, cx));
                                }
                                cx.notify();
                            }),
                        )),
                        cx,
                    )
                } else {
                    pro_empty_state(
                        ProIcon::Check,
                        tr(cx, "Nothing done yet", "尚无已完成任务"),
                        tr(
                            cx,
                            "Completed tasks and their artifacts will appear here once finished.",
                            "已完成的任务及其产物会在完成后归档于此。",
                        ),
                        None,
                        cx,
                    )
                };
                div()
                    .id(pro_eid(&self.id, "scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_view_transition(
                        pro_eid(&self.id, "tasks-view-vtx"),
                        list_vtx_key,
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(empty_view),
                    ))
            } else {
                let mut tree = div().p(px(8.)).pb(px(24.)).flex().flex_col().gap(px(12.));
                let fresh_id = self.fresh_task.as_ref().map(|(id, _)| id.clone());

                for (gi, (grp_name, idxs)) in groups.into_iter().enumerate() {
                    let folded = self.folded_groups.contains(&grp_name);
                    let count = idxs.len();
                    let grp_focused = self.kb_nav
                        && self.tree_cursor.as_ref()
                            == Some(&TreeFocusTarget::Group(grp_name.clone()));
                    let grp_toggle_name = grp_name.clone();
                    let group_icon = match self.group_by {
                        TaskGroupBy::Project => ProIcon::Folder,
                        TaskGroupBy::Status => ProIcon::Tasks,
                    };

                    let mut grp_el = div().flex().flex_col().child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("ghead-{gi}")))
                            .role(Role::Button)
                            .aria_label(format!("{grp_name}, {count} tasks"))
                            .tab_index(0)
                            .h(px(32.))
                            .pl(px(8.))
                            .pr(px(12.))
                            .rounded(px(8.))
                            .when(grp_focused, |d| d.border_1().border_color(p.focus))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .cursor_pointer()
                            .child(
                                div()
                                    .w(px(16.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(pro_icon(
                                        if folded {
                                            ProIcon::ChevronRight
                                        } else {
                                            ProIcon::ChevronDown
                                        },
                                        14.,
                                        p.faint,
                                    )),
                            )
                            .child(pro_icon(group_icon, 14., p.text))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .truncate()
                                    .child(grp_name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.faint)
                                    .child(count.to_string()),
                            ),
                        cx,
                        move |this, _, cx| {
                            this.kb_nav = false;
                            this.toggle_group(&grp_toggle_name, cx);
                        },
                    ));

                    let mut rows_vec: Vec<Stateful<Div>> = Vec::with_capacity(count);
                    for idx in idxs {
                        if let Some(task) = self.tasks.get(idx) {
                            rows_vec.push(self.render_task_row(
                                task,
                                narrow,
                                fresh_id.as_ref() == Some(&task.id),
                                fresh_alpha,
                                cx,
                            ));
                        }
                    }
                    let est_h = (rows_vec.len() as f32) * if narrow { 52. } else { 40. };
                    grp_el = grp_el.child(pro_fold(
                        pro_eid(&self.id, format!("gfold-{grp_name}")),
                        !folded,
                        est_h,
                        0.,
                        move || {
                            div()
                                .flex()
                                .flex_col()
                                .children(rows_vec)
                                .into_any_element()
                        },
                    ));
                    tree = tree.child(grp_el);
                }

                div()
                    .id(pro_eid(&self.id, "scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(pro_view_transition(
                        pro_eid(&self.id, "tasks-view-vtx"),
                        list_vtx_key,
                        tree,
                    ))
            }
        };

        // Responsive inspector drawer when standalone or viewport is narrow
        let inline_sheet = (self.standalone || narrow).then(|| {
            let insp_data = self.render_inspector(w, cx);
            let sheet_open = self.mobile_sheet_open && insp_data.is_some();
            let scrim_eid = pro_eid(&self.id, "mobile-task-scrim");
            let scroll_eid = pro_eid(&self.id, "mobile-sheet-scroll");
            let close_btn = pro_icon_btn(
                pro_eid(&self.id, "close-mobile-sheet"),
                tr(cx, "Close task", "关闭任务面板"),
                ProIcon::Close,
                28.,
                false,
                cx,
                |this, _, cx| {
                    this.mobile_sheet_open = false;
                    cx.notify();
                },
            );
            let dismiss = cx.listener(|this, _, _, cx| {
                this.mobile_sheet_open = false;
                cx.notify();
            });
            pro_presence(
                pro_eid(&self.id, "mobile-sheet-pres"),
                sheet_open,
                0.22,
                0.16,
                move |amount| {
                    let Some((title, ic, body)) = insp_data else {
                        return div().into_any_element();
                    };
                    let base_alpha = if p.dark { 0.50 } else { 0.22 };
                    div()
                        .id(scrim_eid)
                        .absolute()
                        .inset_0()
                        .bg(hsla(0., 0., 0., base_alpha * amount))
                        .flex()
                        .justify_end()
                        .on_mouse_down(MouseButton::Left, dismiss)
                        .child(
                            div()
                                .mr(px(-28. * (1. - amount)))
                                .opacity(amount)
                                .w(px(330.))
                                .max_w(relative(0.92))
                                .h_full()
                                .bg(p.panel)
                                .border_l_1()
                                .border_color(p.line_strong)
                                .flex()
                                .flex_col()
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .child(
                                    div()
                                        .h(px(44.))
                                        .px(px(12.))
                                        .border_b_1()
                                        .border_color(p.line)
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(6.))
                                                .px(px(8.))
                                                .h(px(28.))
                                                .rounded(px(7.))
                                                .bg(p.chip)
                                                .text_size(px(12.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .child(pro_icon(ic, 13., p.muted))
                                                .child(title),
                                        )
                                        .child(close_btn),
                                )
                                .child(
                                    div()
                                        .id(scroll_eid)
                                        .flex_1()
                                        .min_h_0()
                                        .overflow_y_scroll()
                                        .child(body),
                                ),
                        )
                        .into_any_element()
                },
            )
        });

        let composer_el = self.render_new_composer(cx);
        let new_bar = pro_fold(
            pro_eid(&self.id, "new-task-fold"),
            self.new_open,
            58.,
            0.,
            move || div().pb(px(2.)).child(composer_el).into_any_element(),
        );

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key))
            .relative()
            .size_full()
            .bg(p.panel)
            .flex()
            .flex_col()
            .overflow_hidden()
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
                .size_0(),
            )
            .child(header)
            .child(new_bar)
            .child(body_scroll)
            .when_some(inline_sheet, |d, sheet| d.child(sheet))
    }

    fn render_task_row(
        &self,
        task: &AgentTask,
        narrow: bool,
        is_fresh: bool,
        fresh_alpha: f32,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let selected = self.selected_id.as_ref() == Some(&task.id);
        let row_focused = self.kb_nav
            && self.tree_cursor.as_ref() == Some(&TreeFocusTarget::Task(task.id.clone()));
        let is_done = task.is_done();
        let (done_n, total_n) = task.step_counts();
        let art_n = task.artifacts.len();
        let needs_n = task.needs_count();
        let first_ask_id = task.asks.first().map(|a| a.id.clone());
        let tid = task.id.clone();
        let tid_needs = task.id.clone();

        let status_words: SharedString = if let Some(ref d) = task.done_label {
            format!("{} · {d}", tr(cx, "Done", "已完成")).into()
        } else if task.status == TaskStatus::Paused {
            format!("{} · {}", tr(cx, "Paused", "已暂停"), task.status_text).into()
        } else {
            task.status_text.clone()
        };

        let ring_color = if is_done {
            p.green
        } else {
            task.ring_tone.resolve(&p)
        };

        let wait_bg = if p.dark {
            hsla(25. / 360., 1.0, 0.49, 0.145)
        } else {
            hsla(37. / 360., 1.0, 0.50, 0.16)
        };

        let row_h = if narrow { 52. } else { 40. };
        let aria = format!(
            "{}, {}, {}/{} steps, {} artifacts, {}",
            task.title, status_words, done_n, total_n, art_n, task.updated_label
        );

        let main_col = if narrow {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(if is_done { p.text } else { p.ink })
                        .truncate()
                        .child(task.title.clone()),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .truncate()
                        .child(status_words),
                )
        } else {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_baseline()
                .gap(px(10.))
                .child(
                    div()
                        .flex_none()
                        .max_w(px(210.))
                        .text_size(px(13.))
                        .text_color(if is_done { p.text } else { p.ink })
                        .truncate()
                        .child(task.title.clone()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(13.))
                        .text_color(p.muted)
                        .truncate()
                        .child(status_words),
                )
        };

        activate(
            div()
                .id(pro_eid(&self.id, format!("row-{}", task.id)))
                .role(Role::ListBoxOption)
                .aria_label(aria)
                .tab_index(0)
                .h(px(
                    row_h * if is_fresh { fresh_alpha.max(0.25) } else { 1.0 }
                ))
                .px(px(if narrow { 10. } else { 12. }))
                .rounded(px(8.))
                .when(selected, |d| d.bg(p.chip))
                .when(row_focused, |d| d.border_1().border_color(p.focus))
                .when(is_fresh, |d| d.opacity(fresh_alpha))
                .hover(move |s| s.bg(if selected { p.chip } else { p.hover }))
                .flex()
                .items_center()
                .gap(px(if narrow { 10. } else { 12. }))
                .cursor_pointer()
                .child(task_pie_ring(
                    task.progress(),
                    14.,
                    if task.status == TaskStatus::Cancelled {
                        p.muted
                    } else {
                        ring_color
                    },
                    if task.status == TaskStatus::Cancelled {
                        TaskStatus::Cancelled
                    } else if is_done {
                        TaskStatus::Done
                    } else {
                        task.status
                    },
                ))
                .child(main_col)
                .when(needs_n > 0, |d| {
                    let badge_text: SharedString = if needs_n > 1 {
                        format!("{} · {needs_n}", tr(cx, "Needs you", "待你确认")).into()
                    } else {
                        tr(cx, "Needs you", "待你确认").into()
                    };
                    d.child(
                        div()
                            .id(pro_eid(&self.id, format!("needs-{}", task.id)))
                            .role(Role::Button)
                            .px(px(6.))
                            .h(px(18.))
                            .rounded(px(4.))
                            .bg(wait_bg)
                            .text_color(p.amber_ink)
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .whitespace_nowrap()
                            .flex_none()
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                cx.emit(AgentTasksEvent::OpenInbox {
                                    task_id: tid_needs.clone(),
                                    item_id: first_ask_id.clone(),
                                });
                                cx.emit(AgentTasksEvent::Navigate("Inbox".into()));
                                let _ = &this.announcement;
                            }))
                            .child(badge_text),
                    )
                })
                .when(!narrow && total_n > 0, |d| {
                    let bar_fill = if task.status == TaskStatus::Cancelled {
                        p.muted
                    } else {
                        ring_color
                    };
                    d.child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(div().w(px(40.)).child(pro_spring_bar(
                                pro_eid(&self.id, format!("row-prog-{}", task.id)),
                                task.progress(),
                                4.,
                                2.,
                                p.line_soft,
                                bar_fill,
                            )))
                            .child(
                                div()
                                    .w(px(28.))
                                    .text_size(px(12.))
                                    .text_color(p.faint)
                                    .text_right()
                                    .child(format!("{done_n}/{total_n}")),
                            ),
                    )
                })
                .when(!narrow && art_n > 0, |d| {
                    d.child(
                        div()
                            .w(px(34.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(4.))
                            .text_size(px(12.))
                            .text_color(p.faint)
                            .child(pro_icon(ProIcon::PageDoc, 13., p.faint))
                            .child(art_n.to_string()),
                    )
                })
                .child(
                    div()
                        .w(px(28.))
                        .flex_none()
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .text_right()
                        .child(task.updated_label.clone()),
                ),
            cx,
            move |this, _, cx| {
                this.kb_nav = false;
                this.select_task(tid.clone(), cx);
            },
        )
    }

    fn render_inspector(
        &mut self,
        _w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SharedString, ProIcon, AnyElement)> {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let elapsed_ms = self.boot_at.elapsed().as_secs_f32() * 1000.;

        if self.loading {
            let phase = if reduced {
                0.
            } else {
                (elapsed_ms / 1000.) * 0.75
            };
            let skel = div()
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(7.))
                        .child(pro_skeleton_bar(240., 12., 4., phase, cx))
                        .child(pro_skeleton_bar(185., 12., 4., phase + 0.1, cx)),
                )
                .child(div().mt(px(4.)).flex().flex_col().gap(px(8.)).children(
                    [110., 92., 104.].into_iter().map(|w_val| {
                        div()
                            .h(px(26.))
                            .flex()
                            .items_center()
                            .gap(px(16.))
                            .child(pro_skeleton_bar(52., 11., 4., phase, cx))
                            .child(pro_skeleton_bar(w_val, 12., 4., phase + 0.15, cx))
                    }),
                ))
                .child(pro_skeleton_bar(260., 92., 10., phase + 0.2, cx))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(pro_skeleton_bar(210., 11., 4., phase + 0.25, cx))
                        .child(pro_skeleton_bar(190., 11., 4., phase + 0.3, cx)),
                )
                .into_any_element();
            return Some((tr(cx, "Loading…", "加载中…").into(), ProIcon::Tasks, skel));
        }

        let Some(sel_id) = self.selected_id.clone() else {
            let empty_panel = div()
                .min_h(px(240.))
                .p(px(24.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .text_color(p.faint)
                .child(tr(cx, "Nothing open", "未打开任何任务"))
                .into_any_element();
            return Some((tr(cx, "Task", "任务").into(), ProIcon::Tasks, empty_panel));
        };

        let task = self.tasks.iter().find(|t| t.id == sel_id).cloned()?;

        let active_pane = self
            .inspector_tabs
            .get(self.active_inspector_tab)
            .cloned()
            .unwrap_or(InspectorPane::Task);

        // Optional sub-tab switcher when artifacts are opened alongside the Task tab
        let tab_bar =
            (self.inspector_tabs.len() > 1).then(|| {
                let itab_sensor = pro_selection_sensor();
                let itab_key: SharedString = format!("itab-{}", self.active_inspector_tab).into();
                div()
                    .relative()
                    .px(px(12.))
                    .py(px(6.))
                    .border_b_1()
                    .border_color(p.line_soft)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(4.))
                    .child(pro_pill_indicator(
                        pro_eid(&self.id, "itab-pill-ind"),
                        itab_key,
                        itab_sensor.clone(),
                        6.,
                        p.chip,
                        None,
                        false,
                    ))
                    .children(self.inspector_tabs.clone().into_iter().enumerate().map(
                        |(ti, pane)| {
                            let is_cur = ti == self.active_inspector_tab;
                            let (lbl, ic): (SharedString, ProIcon) = match &pane {
                                InspectorPane::Task => (task.title.clone(), ProIcon::Tasks),
                                InspectorPane::Artifact(n) => (n.clone(), ProIcon::PageDoc),
                            };
                            let itab_sensor_item = itab_sensor.clone();
                            div()
                                .relative()
                                .h(px(26.))
                                .pl(px(7.))
                                .pr(px(5.))
                                .rounded(px(6.))
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .when(is_cur, |d| d.child(pro_mark_selected(&itab_sensor_item)))
                                .child(activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("itab-{ti}")))
                                        .role(Role::Tab)
                                        .tab_index(0)
                                        .flex()
                                        .items_center()
                                        .gap(px(5.))
                                        .text_size(px(12.))
                                        .font_weight(if is_cur {
                                            FontWeight::MEDIUM
                                        } else {
                                            FontWeight::NORMAL
                                        })
                                        .text_color(if is_cur { p.ink } else { p.muted })
                                        .cursor_pointer()
                                        .child(pro_icon(ic, 12., p.muted))
                                        .child(div().max_w(px(110.)).truncate().child(lbl)),
                                    cx,
                                    move |this, _, cx| {
                                        this.active_inspector_tab = ti;
                                        cx.notify();
                                    },
                                ))
                                .when(ti > 0, |d| {
                                    d.child(activate(
                                        div()
                                            .id(pro_eid(&self.id, format!("itab-x-{ti}")))
                                            .role(Role::Button)
                                            .tab_index(0)
                                            .size(px(16.))
                                            .rounded(px(4.))
                                            .hover(move |s| s.bg(p.hover))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .child(pro_icon(ProIcon::Close, 10., p.faint)),
                                        cx,
                                        move |this, _, cx| this.close_inspector_tab(ti, cx),
                                    ))
                                })
                        },
                    ))
            });

        match active_pane {
            InspectorPane::Artifact(art_name) => {
                let insp_key: SharedString = format!("art:{sel_id}:{art_name}").into();
                let art_body = self.render_artifact_pane(&art_name, cx);
                let full = div()
                    .flex()
                    .flex_col()
                    .when_some(tab_bar, |d, tb| d.child(tb))
                    .child(pro_view_transition(
                        pro_eid(&self.id, "task-insp-vtx"),
                        insp_key,
                        art_body,
                    ))
                    .into_any_element();
                Some((art_name, ProIcon::PageDoc, full))
            }
            InspectorPane::Task => {
                let insp_key: SharedString = format!("task:{sel_id}").into();
                let task_body = self.render_task_detail(&task, elapsed_ms, cx);
                let full = div()
                    .flex()
                    .flex_col()
                    .when_some(tab_bar, |d, tb| d.child(tb))
                    .child(pro_view_transition(
                        pro_eid(&self.id, "task-insp-vtx"),
                        insp_key,
                        task_body,
                    ))
                    .into_any_element();
                Some((task.title, ProIcon::Tasks, full))
            }
        }
    }

    fn render_artifact_pane(&self, name: &str, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let kind = TaskArtifactKind::from_filename(name);
        let found = self.artifacts.iter().find(|a| a.name.as_ref() == name);
        let updated = found
            .map(|a| a.updated.clone())
            .unwrap_or_else(|| "Updated 4m ago".into());

        let mut pane = div().p(px(14.)).flex().flex_col().gap(px(14.)).child(
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(
                    div()
                        .size(px(28.))
                        .rounded(px(7.))
                        .bg(p.chip)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_svg("0 0 12 12", kind.svg(), 14., kind.color(&p))),
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
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .truncate()
                                .child(name.to_owned()),
                        )
                        .child(div().text_size(px(12.)).text_color(p.muted).child(updated)),
                )
                .child(pro_btn(
                    pro_eid(&self.id, "art-back-task"),
                    tr(cx, "Task", "返回任务"),
                    Some(ProIcon::ChevronLeft),
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    |this, _, cx| {
                        this.active_inspector_tab = 0;
                        cx.notify();
                    },
                )),
        );

        if let Some(art) = found {
            if !art.table.is_empty() {
                let mut tbl = div()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(p.line)
                    .overflow_hidden()
                    .flex()
                    .flex_col();
                for (ri, row) in art.table.iter().enumerate() {
                    tbl = tbl.child(
                        div()
                            .h(px(28.))
                            .px(px(8.))
                            .when(ri == 0, |d| d.bg(p.chip))
                            .when(ri > 0, |d| d.border_t_1().border_color(p.line_soft))
                            .flex()
                            .items_center()
                            .children(row.iter().enumerate().map(|(ci, cell)| {
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(12.))
                                    .font_weight(if ri == 0 {
                                        FontWeight::MEDIUM
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .text_color(if ri == 0 {
                                        p.muted
                                    } else if ci == 0 {
                                        p.ink
                                    } else {
                                        p.text
                                    })
                                    .child(cell.clone())
                            })),
                    );
                }
                pane = pane.child(tbl);
            } else if kind == TaskArtifactKind::Code {
                pane = pane.child(
                    div()
                        .p(px(12.))
                        .rounded(px(10.))
                        .bg(p.chip)
                        .font_family("Menlo")
                        .text_size(px(12.))
                        .line_height(px(19.))
                        .text_color(p.ink)
                        .child(art.body.clone()),
                );
            } else {
                pane = pane
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(art.heading.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(p.text)
                            .child(art.body.clone()),
                    );
            }
        } else {
            pane = pane.child(
                div()
                    .p(px(12.))
                    .rounded(px(10.))
                    .bg(p.chip)
                    .text_size(px(12.5))
                    .line_height(px(19.))
                    .text_color(p.text)
                    .child(format!(
                        "Generated workspace artifact `{name}` ready for review."
                    )),
            );
        }
        pane
    }

    fn render_task_detail(&self, task: &AgentTask, elapsed_ms: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let is_done = task.is_done();
        let (done_n, total_n) = task.step_counts();
        let ring_color = if is_done {
            p.green
        } else {
            task.ring_tone.resolve(&p)
        };
        let wait_bg = if p.dark {
            hsla(25. / 360., 1.0, 0.49, 0.145)
        } else {
            hsla(37. / 360., 1.0, 0.50, 0.16)
        };

        let mut col = div()
            .p(px(14.))
            .pb(px(18.))
            .flex()
            .flex_col()
            .gap(px(20.))
            .child(
                div()
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.text)
                    .child(task.goal.clone()),
            );

        // Needs you decision card
        if !is_done && let Some(first_ask) = task.asks.first() {
            let ask_count = task.asks.len();
            let waiting_lbl: SharedString = if ask_count == 1 {
                tr(cx, "1 decision waiting", "1 项待决定").into()
            } else {
                format!("{ask_count} {}", tr(cx, "decisions waiting", "项待决定")).into()
            };
            let decide_tid = task.id.clone();
            let decide_aid = Some(first_ask.id.clone());
            let approve_tid = task.id.clone();
            let is_approval_ask = first_ask.kind == TaskAskKind::Approval;
            col = col.child(
                div()
                    .p(px(10.))
                    .pr(px(8.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(3.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .px(px(6.))
                                            .h(px(18.))
                                            .rounded(px(4.))
                                            .bg(wait_bg)
                                            .text_color(p.amber_ink)
                                            .text_size(px(11.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .flex()
                                            .items_center()
                                            .child(tr(cx, "Needs you", "待你确认")),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.muted)
                                            .child(waiting_lbl),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(19.))
                                    .text_color(p.ink)
                                    .child(first_ask.question.clone()),
                            ),
                    )
                    .child(if is_approval_ask {
                        pro_btn(
                            pro_eid(&self.id, "panel-approve-btn"),
                            tr(cx, "Approve", "批准"),
                            Some(ProIcon::Check),
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.approve_task(&approve_tid, cx);
                            },
                        )
                    } else {
                        pro_btn(
                            pro_eid(&self.id, "panel-decide-btn"),
                            tr(cx, "Decide", "去决定"),
                            None,
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            move |_, _, cx| {
                                cx.emit(AgentTasksEvent::OpenInbox {
                                    task_id: decide_tid.clone(),
                                    item_id: decide_aid.clone(),
                                });
                                cx.emit(AgentTasksEvent::Navigate("Inbox".into()));
                            },
                        )
                    }),
            );
        }

        // Properties list (Status, Project, Started + optional Agent / Apps)
        let status_label: SharedString = if task.status == TaskStatus::Cancelled {
            tr(cx, "Cancelled", "已取消").into()
        } else if let Some(ref d) = task.done_label {
            format!("{} · {d}", tr(cx, "Done", "已完成")).into()
        } else if task.status == TaskStatus::Paused {
            tr(cx, "Paused", "已暂停").into()
        } else {
            task.status_text.clone()
        };

        let mut props = div()
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .min_h(px(28.))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w(px(76.))
                            .flex_none()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(tr(cx, "Status", "状态")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(task_pie_ring(
                                task.progress(),
                                14.,
                                if task.status == TaskStatus::Cancelled {
                                    p.muted
                                } else {
                                    ring_color
                                },
                                if task.status == TaskStatus::Cancelled {
                                    TaskStatus::Cancelled
                                } else if is_done {
                                    TaskStatus::Done
                                } else {
                                    task.status
                                },
                            ))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(p.ink)
                                    .truncate()
                                    .child(status_label),
                            )
                            .when(total_n > 0, |d| {
                                d.child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(p.faint)
                                        .child(format!("{done_n} / {total_n}")),
                                )
                            }),
                    ),
            )
            .child(
                div()
                    .min_h(px(28.))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w(px(76.))
                            .flex_none()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(tr(cx, "Project", "项目")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(13.))
                            .text_color(p.ink)
                            .truncate()
                            .child(task.project.clone()),
                    ),
            )
            .child(
                div()
                    .min_h(px(28.))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w(px(76.))
                            .flex_none()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(tr(cx, "Started", "开始于")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(13.))
                            .text_color(p.ink)
                            .truncate()
                            .child(task.started.clone()),
                    ),
            );

        if let Some(ref agent_name) = task.agent {
            props = props.child(
                div()
                    .min_h(px(28.))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w(px(76.))
                            .flex_none()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(tr(cx, "Agent", "代理")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(pro_agent_avatar(
                                agent_name,
                                p.purple,
                                18.,
                                !is_done && task.status == TaskStatus::Running,
                                elapsed_ms,
                                cx,
                            ))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(p.ink)
                                    .truncate()
                                    .child(agent_name.clone()),
                            ),
                    ),
            );
        }

        if !task.apps.is_empty() {
            props = props.child(
                div()
                    .min_h(px(28.))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w(px(76.))
                            .flex_none()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(tr(cx, "Apps", "连接应用")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .children(task.apps.iter().map(|app| pro_app_tile(app, 18., cx))),
                    ),
            );
        }

        col = col.child(props);

        // Steps section
        if !task.steps.is_empty() {
            let first_undone = task.steps.iter().position(|s| !s.done);
            let has_ask = task.needs_count() > 0;
            let step_count = task.steps.len();
            let tray_bg = if p.dark {
                hsla(0., 0., 1., 0.025)
            } else {
                hsla(0., 0., 0., 0.022)
            };

            let mut steps_box = div()
                .px(px(10.))
                .py(px(6.))
                .rounded(px(10.))
                .bg(tray_bg)
                .flex()
                .flex_col();

            for (si, step) in task.steps.iter().enumerate() {
                let is_active_step = Some(si) == first_undone && !is_done;
                let is_you = is_active_step && has_ask;
                let is_now = is_active_step && !has_ask;
                let is_last = si + 1 == step_count;
                let step_tid = task.id.clone();

                let mark = if step.done {
                    div()
                        .size(px(15.))
                        .rounded_full()
                        .bg(p.blue)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_icon(ProIcon::Check, 10., rgb(0xffffff).into()))
                } else if is_you {
                    div()
                        .size(px(15.))
                        .rounded_full()
                        .bg(p.orange.opacity(0.18))
                        .border_1()
                        .border_color(p.orange)
                } else if is_now {
                    div()
                        .size(px(15.))
                        .rounded_full()
                        .bg(p.blue.opacity(0.18))
                        .border_1()
                        .border_color(p.blue)
                } else {
                    div()
                        .size(px(15.))
                        .rounded_full()
                        .border_1()
                        .border_color(p.line_strong)
                };

                steps_box = steps_box.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("step-{}-{si}", task.id)))
                        .role(Role::CheckBox)
                        .tab_index(0)
                        .relative()
                        .min_h(px(28.))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .cursor_pointer()
                        .when(!is_last, |d| {
                            d.child(
                                div()
                                    .absolute()
                                    .left(px(7.))
                                    .top(px(21.))
                                    .bottom(px(-7.))
                                    .w(px(1.))
                                    .bg(p.line),
                            )
                        })
                        .child(mark)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(13.))
                                .text_color(if is_active_step { p.ink } else { p.muted })
                                .child(step.label.clone()),
                        )
                        .when(is_you, |d| {
                            d.child(
                                div()
                                    .size(px(18.))
                                    .rounded(px(4.))
                                    .bg(wait_bg)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(pro_icon(ProIcon::AlertTriangle, 12., p.amber_ink)),
                            )
                        }),
                    cx,
                    move |this, _, cx| this.toggle_step(&step_tid, si, cx),
                ));
            }

            let prog_fill = if task.status == TaskStatus::Cancelled {
                p.muted
            } else {
                ring_color
            };
            let pct_num = (task.progress() * 100.).round() as u32;
            col = col.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "Steps", "步骤")),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.faint)
                                    .child(format!("{done_n}/{total_n} · {pct_num}%")),
                            ),
                    )
                    .child(pro_spring_bar(
                        pro_eid(&self.id, "insp-task-prog"),
                        task.progress(),
                        5.,
                        2.5,
                        p.line_soft,
                        prog_fill,
                    ))
                    .child(steps_box),
            );
        }

        // What happened so far
        if !task.history.is_empty() {
            col = col.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "What happened so far", "进展记录")),
                    )
                    .child(div().flex().flex_col().gap(px(4.)).children(
                        task.history.iter().enumerate().map(|(hi, line)| {
                            div()
                                .flex()
                                .items_start()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .mt(px(8.))
                                        .ml(px(4.))
                                        .size(px(4.))
                                        .flex_none()
                                        .rounded_full()
                                        .bg(p.faint),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_size(px(13.))
                                        .line_height(px(20.))
                                        .text_color(if hi == 0 { p.ink } else { p.text })
                                        .child(line.clone()),
                                )
                        }),
                    )),
            );
        }

        // Artifacts section
        if !task.artifacts.is_empty() {
            let task_id_for_art = task.id.clone();
            col = col.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Artifacts", "产物"))
                            .child(
                                div()
                                    .text_color(p.faint)
                                    .child(task.artifacts.len().to_string()),
                            ),
                    )
                    .child(div().flex().flex_wrap().gap(px(6.)).children(
                        task.artifacts.iter().enumerate().map(|(ai, art_name)| {
                            let kind = TaskArtifactKind::from_filename(art_name);
                            let tid_c = task_id_for_art.clone();
                            let aname_c = art_name.clone();
                            activate(
                                div()
                                    .id(pro_eid(&self.id, format!("art-{}-{ai}", task.id)))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .h(px(26.))
                                    .pl(px(7.))
                                    .pr(px(9.))
                                    .rounded(px(6.))
                                    .bg(p.chip)
                                    .hover(move |s| s.bg(p.chip_hover))
                                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(12.))
                                    .text_color(p.ink)
                                    .cursor_pointer()
                                    .child(pro_svg("0 0 12 12", kind.svg(), 12., kind.color(&p)))
                                    .child(art_name.clone()),
                                cx,
                                move |this, _, cx| {
                                    this.open_artifact_tab(tid_c.clone(), aname_c.clone(), cx);
                                },
                            )
                        }),
                    )),
            );
        }

        // Decided section
        if !task.decided.is_empty() {
            col = col.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Decided", "已确认决策")),
                    )
                    .child(
                        div()
                            .px(px(10.))
                            .py(px(2.))
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .children(task.decided.iter().enumerate().map(|(di, dec)| {
                                div()
                                    .py(px(8.))
                                    .when(di > 0, |d| d.border_t_1().border_color(p.line_soft))
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .child(pro_icon(ProIcon::Check, 14., p.green))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .text_size(px(13.))
                                                    .line_height(px(19.))
                                                    .text_color(p.ink)
                                                    .truncate()
                                                    .child(dec.question.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .line_height(px(16.))
                                                    .text_color(p.muted)
                                                    .truncate()
                                                    .child(format!("{} · {}", dec.answer, dec.at)),
                                            ),
                                    )
                            })),
                    ),
            );
        }

        // Expanded background agents & apps drawer + bottom "Show agents" toggle
        let pause_tid = task.id.clone();
        let cancel_tid = task.id.clone();
        let pause_lbl = if task.status == TaskStatus::Paused {
            tr(cx, "Resume", "继续")
        } else {
            tr(cx, "Pause", "暂停")
        };
        let pause_ic = if task.status == TaskStatus::Paused {
            ProIcon::Play
        } else {
            ProIcon::Pause
        };
        let drawer_card = div()
            .p(px(10.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .child(pro_agent_avatar(
                                "Orchestrator",
                                p.purple,
                                20.,
                                !is_done && task.status == TaskStatus::Running,
                                elapsed_ms,
                                cx,
                            ))
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child("Orchestrator + 2 workers"),
                            ),
                    )
                    .when(!is_done, |d| {
                        d.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .child(pro_btn(
                                    pro_eid(&self.id, "task-pause-btn"),
                                    pause_lbl,
                                    Some(pause_ic),
                                    ProBtnStyle::Ghost,
                                    true,
                                    cx,
                                    move |this, _, cx| {
                                        this.pause_or_resume_task(&pause_tid, cx);
                                    },
                                ))
                                .child(pro_btn(
                                    pro_eid(&self.id, "task-cancel-btn"),
                                    tr(cx, "Cancel", "取消任务"),
                                    Some(ProIcon::Close),
                                    ProBtnStyle::Danger,
                                    true,
                                    cx,
                                    move |this, _, cx| {
                                        this.cancel_task(&cancel_tid, cx);
                                    },
                                )),
                        )
                    }),
            )
            .child(
                div().flex().items_center().gap(px(6.)).children(
                    ["GitHub", "Linear", "Slack"]
                        .into_iter()
                        .map(|app| pro_app_tile(app, 20., cx)),
                ),
            );
        let agents_open = self.show_agents_drawer;
        col = col.child(
            div()
                .flex()
                .flex_col()
                .child(pro_fold(
                    pro_eid(&self.id, "agents-drawer-fold"),
                    agents_open,
                    82.,
                    0.,
                    move || div().pb(px(12.)).child(drawer_card).into_any_element(),
                ))
                .child(
                    div().flex().items_center().justify_end().child(activate(
                        div()
                            .id(pro_eid(&self.id, "show-agents-link"))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(28.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .hover(move |s| s.bg(p.chip).text_color(p.text))
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .text_size(px(12.))
                            .text_color(p.faint)
                            .cursor_pointer()
                            .child(tr(cx, "Show agents", "查看代理"))
                            .child(pro_icon(
                                if agents_open {
                                    ProIcon::ChevronDown
                                } else {
                                    ProIcon::ChevronRight
                                },
                                12.,
                                p.faint,
                            )),
                        cx,
                        |this, _, cx| {
                            this.show_agents_drawer = !this.show_agents_drawer;
                            cx.emit(AgentTasksEvent::Navigate("Agent Orchestration".into()));
                            cx.notify();
                        },
                    )),
                ),
        );

        col
    }
}

impl Render for AgentTasksScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

/// Draw the 14x14 task status ring with outer circle and inner progress pie wedge.
fn task_pie_ring(progress: f32, side: f32, color: Hsla, status: TaskStatus) -> AnyElement {
    let pct = progress.clamp(0., 1.);
    if status == TaskStatus::Cancelled {
        return div()
            .relative()
            .size(px(side))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(pro_progress_ring(
                1.0,
                side,
                1.5,
                color,
                color.opacity(0.25),
            ))
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_icon(ProIcon::Minus, 9., color)),
            )
            .into_any_element();
    }
    if status == TaskStatus::Done {
        return div()
            .relative()
            .size(px(side))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(pro_progress_ring(
                1.0,
                side,
                1.5,
                color,
                color.opacity(0.25),
            ))
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_icon(ProIcon::Check, 9., color)),
            )
            .into_any_element();
    }
    if status == TaskStatus::Planning && pct <= 0.005 {
        return canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let center = bounds.center();
                let r = (f32::from(bounds.size.width) - 1.5) * 0.5;
                if r <= 1. {
                    return;
                }
                for seg in 0..8usize {
                    let a0 = (seg as f32 / 8.) * std::f32::consts::TAU;
                    let a1 = a0 + 0.45;
                    let mut dash = PathBuilder::stroke(px(1.4));
                    dash.move_to(center + point(px(r * a0.cos()), px(r * a0.sin())));
                    let mid = (a0 + a1) * 0.5;
                    dash.line_to(center + point(px(r * mid.cos()), px(r * mid.sin())));
                    dash.line_to(center + point(px(r * a1.cos()), px(r * a1.sin())));
                    if let Ok(path) = dash.build() {
                        window.paint_path(path, color);
                    }
                }
            },
        )
        .size(px(side))
        .flex_none()
        .into_any_element();
    }

    div()
        .relative()
        .size(px(side))
        .flex_none()
        .child(pro_progress_ring(1.0, side, 1.45, color, color))
        .child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    if pct <= 0.005 {
                        return;
                    }
                    let center = bounds.center();
                    let inner_r = (f32::from(bounds.size.width) * 0.5 - 3.0).max(1.5);
                    let steps = ((24. * pct).ceil() as usize).max(3);
                    let mut wedge = PathBuilder::fill();
                    wedge.move_to(center);
                    for i in 0..=steps {
                        let frac = (i as f32 / steps as f32) * pct;
                        let a = frac * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                        wedge.line_to(center + point(px(inner_r * a.cos()), px(inner_r * a.sin())));
                    }
                    wedge.close();
                    if let Ok(path) = wedge.build() {
                        window.paint_path(path, color);
                    }
                },
            )
            .absolute()
            .inset_0(),
        )
        .into_any_element()
}

fn sample_tasks() -> Vec<AgentTask> {
    vec![
        AgentTask {
            id: "ingest".into(),
            title: "Ingest pipeline".into(),
            project: "Northwind".into(),
            status: TaskStatus::Running,
            status_text: "Picking a queue".into(),
            ring_tone: TaskRingTone::Amber,
            steps: vec![
                TaskStep::new("Draft the event schema", true),
                TaskStep::new("Load test at 20k a minute", true),
                TaskStep::new("Pick a queue", false),
                TaskStep::new("Deploy to staging", false),
            ],
            artifacts: vec![
                "ingest-design.md".into(),
                "load-test-results.csv".into(),
                "event-schema.json".into(),
                "staging".into(),
            ],
            asks: vec![TaskAsk::new(
                "ingest-queue",
                "Which queue should the ingest service use?",
                TaskAskKind::Decision,
            )],
            updated_label: "4m".into(),
            done_label: None,
            goal: "Take in Northwind's order events and keep a day of them to replay after a bad deploy.".into(),
            started: "Mon, Nov 3".into(),
            history: vec![
                "Asked you which queue to use".into(),
                "Load test passed at 20k events a minute".into(),
                "Wrote the design doc and the event schema".into(),
            ],
            decided: vec![TaskDecided::new(
                "Which format for the events?",
                "JSON with a version field",
                "3 days ago",
            )],
            agent: None,
            apps: Vec::new(),
        },
        AgentTask {
            id: "dashboard".into(),
            title: "Client dashboard".into(),
            project: "Northwind".into(),
            status: TaskStatus::Running,
            status_text: "Choosing an empty state".into(),
            ring_tone: TaskRingTone::Amber,
            steps: vec![
                TaskStep::new("Map metrics and query latency", true),
                TaskStep::new("Choose the first-run empty state", false),
                TaskStep::new("Wire live customer filters", false),
            ],
            artifacts: vec![
                "dashboard-spec.md".into(),
                "empty-states.png".into(),
                "metrics-query.sql".into(),
            ],
            asks: vec![TaskAsk::new(
                "dash-empty",
                "Which empty state layout fits first-time workspaces?",
                TaskAskKind::Pick,
            )],
            updated_label: "18m".into(),
            done_label: None,
            goal: "Give Northwind account leads a single live dashboard for throughput, errors, and SLA drift.".into(),
            started: "Mon, Nov 3".into(),
            history: vec![
                "Prepared three wireframe candidates for the empty state".into(),
                "Verified query p95 under 45 ms on staging replica".into(),
            ],
            decided: Vec::new(),
            agent: None,
            apps: Vec::new(),
        },
        AgentTask {
            id: "weekly".into(),
            title: "Weekly update".into(),
            project: "Northwind".into(),
            status: TaskStatus::Running,
            status_text: "Ready to send".into(),
            ring_tone: TaskRingTone::Teal,
            steps: vec![
                TaskStep::new("Collect shipped PRs and incident notes", true),
                TaskStep::new("Draft executive summary", true),
                TaskStep::new("Send to Northwind stakeholders", false),
            ],
            artifacts: vec!["weekly-update-nov.md".into()],
            asks: vec![TaskAsk::new(
                "weekly-send",
                "Send the Northwind weekly update to 14 stakeholders?",
                TaskAskKind::Approval,
            )],
            updated_label: "42m".into(),
            done_label: None,
            goal: "Compile Friday's Northwind delivery recap from merged changes and support metrics.".into(),
            started: "Tue, Nov 4".into(),
            history: vec![
                "Drafted weekly-update-nov.md and staged recipient list".into(),
                "Pulled 11 merged PRs and 0 Sev-1 incidents".into(),
            ],
            decided: Vec::new(),
            agent: None,
            apps: Vec::new(),
        },
        AgentTask {
            id: "onboarding".into(),
            title: "Onboarding emails".into(),
            project: "Northwind".into(),
            status: TaskStatus::Running,
            status_text: "Writing the draft".into(),
            ring_tone: TaskRingTone::Teal,
            steps: vec![
                TaskStep::new("Audit activation drop-off funnel", true),
                TaskStep::new("Write 3-part onboarding sequence", false),
                TaskStep::new("Configure transactional triggers", false),
            ],
            artifacts: vec!["onboarding-sequence.md".into()],
            asks: Vec::new(),
            updated_label: "12m".into(),
            done_label: None,
            goal: "Shorten time-to-first-pipeline for new Northwind workspaces with targeted walkthrough emails.".into(),
            started: "Tue, Nov 4".into(),
            history: vec![
                "Writing email 2 of 3 around webhook verification".into(),
                "Identified API key setup as primary day-1 stall point".into(),
            ],
            decided: Vec::new(),
            agent: None,
            apps: Vec::new(),
        },
        AgentTask {
            id: "webhooks".into(),
            title: "Billing webhooks".into(),
            project: "Platform".into(),
            status: TaskStatus::Running,
            status_text: "Deploying to staging".into(),
            ring_tone: TaskRingTone::Teal,
            steps: vec![
                TaskStep::new("Add idempotency key store", true),
                TaskStep::new("Replay Stripe invoice fixtures", true),
                TaskStep::new("Verify signature rotation", true),
                TaskStep::new("Deploy to staging", false),
            ],
            artifacts: vec!["webhook-retry.ts".into(), "stripe-events.json".into()],
            asks: Vec::new(),
            updated_label: "6m".into(),
            done_label: None,
            goal: "Guarantee exactly-once invoice webhook handling across retries and secret rotations.".into(),
            started: "Mon, Nov 3".into(),
            history: vec![
                "Rolling out billing-webhooks build #418 to staging".into(),
                "Passed 240 duplicate-delivery replay checks".into(),
            ],
            decided: Vec::new(),
            agent: None,
            apps: Vec::new(),
        },
        AgentTask {
            id: "migration".into(),
            title: "API migration".into(),
            project: "Platform".into(),
            status: TaskStatus::Running,
            status_text: "Deciding the v1 end date".into(),
            ring_tone: TaskRingTone::Amber,
            steps: vec![
                TaskStep::new("Inventory remaining v1 callers", true),
                TaskStep::new("Move internal services to v2", true),
                TaskStep::new("Pick the v1 sunset date", false),
                TaskStep::new("Send deprecation notice", false),
            ],
            artifacts: vec!["v1-caller-audit.csv".into(), "deprecation-notice.md".into()],
            asks: vec![TaskAsk::new(
                "v1-sunset",
                "When should we sunset the legacy v1 endpoints?",
                TaskAskKind::Decision,
            )],
            updated_label: "3h".into(),
            done_label: None,
            goal: "Migrate internal and partner traffic off v1 REST routes onto typed v2 endpoints.".into(),
            started: "Fri, Oct 31".into(),
            history: vec![
                "Asked you to choose between 30-day and 60-day sunset windows".into(),
                "Moved all 6 internal workers onto v2".into(),
            ],
            decided: Vec::new(),
            agent: None,
            apps: Vec::new(),
        },
        AgentTask {
            id: "triage".into(),
            title: "Support triage".into(),
            project: "Platform".into(),
            status: TaskStatus::Running,
            status_text: "Rolling out to all queues".into(),
            ring_tone: TaskRingTone::Teal,
            steps: vec![
                TaskStep::new("Backtest classifier on 500 tickets", true),
                TaskStep::new("Enable shadow tagging in Slack", true),
                TaskStep::new("Roll out live routing to all queues", false),
            ],
            artifacts: vec!["triage-rules.json".into(), "queue-routing.md".into()],
            asks: Vec::new(),
            updated_label: "25m".into(),
            done_label: None,
            goal: "Auto-classify incoming billing, auth, and SDK tickets with 96%+ precision.".into(),
            started: "Mon, Nov 3".into(),
            history: vec![
                "Enabled live routing on enterprise and billing queues".into(),
                "Shadow run hit 97.4% agreement with human leads".into(),
            ],
            decided: Vec::new(),
            agent: None,
            apps: Vec::new(),
        },
        AgentTask {
            id: "launch".into(),
            title: "Launch post".into(),
            project: "Evalkit".into(),
            status: TaskStatus::Running,
            status_text: "Picking the tone".into(),
            ring_tone: TaskRingTone::Orange,
            steps: vec![
                TaskStep::new("Gather benchmark charts and snippets", true),
                TaskStep::new("Pick the opening angle", false),
                TaskStep::new("Write full announcement draft", false),
                TaskStep::new("Prepare changelog and social copy", false),
            ],
            artifacts: vec!["launch-draft.md".into(), "eval-chart.png".into()],
            asks: vec![TaskAsk::new(
                "launch-tone",
                "Should the launch post lead with benchmark numbers or developer workflow?",
                TaskAskKind::Decision,
            )],
            updated_label: "1h".into(),
            done_label: None,
            goal: "Write the Evalkit 2.0 release post with reproducible latency and accuracy numbers.".into(),
            started: "Tue, Nov 4".into(),
            history: vec![
                "Asked you which opening angle to lead with".into(),
                "Exported eval-chart.png from latest suite run".into(),
            ],
            decided: Vec::new(),
            agent: None,
            apps: Vec::new(),
        },
        AgentTask {
            id: "benchmarks".into(),
            title: "Benchmark suite".into(),
            project: "Evalkit".into(),
            status: TaskStatus::Running,
            status_text: "Running the evals".into(),
            ring_tone: TaskRingTone::Teal,
            steps: vec![
                TaskStep::new("Freeze 1,200-prompt golden dataset", true),
                TaskStep::new("Provision isolated runner pool", true),
                TaskStep::new("Score tool-calling accuracy", true),
                TaskStep::new("Run multi-turn latency pass", false),
                TaskStep::new("Publish comparison report", false),
            ],
            artifacts: vec!["eval-results.csv".into()],
            asks: Vec::new(),
            updated_label: "2m".into(),
            done_label: None,
            goal: "Run deterministic accuracy and latency evaluations across three candidate models.".into(),
            started: "Tue, Nov 4".into(),
            history: vec![
                "Running multi-turn latency pass (780 / 1,200 prompts)".into(),
                "Tool-calling accuracy finished at 94.8%".into(),
            ],
            decided: Vec::new(),
            agent: None,
            apps: Vec::new(),
        },
        // Completed tasks visible under the "Done" tab
        AgentTask {
            id: "sso-audit".into(),
            title: "SAML session audit".into(),
            project: "Platform".into(),
            status: TaskStatus::Done,
            status_text: "Verified 0 token leaks".into(),
            ring_tone: TaskRingTone::Green,
            steps: vec![
                TaskStep::new("Trace refresh rotation paths", true),
                TaskStep::new("Add revocation integration tests", true),
                TaskStep::new("Ship audit summary", true),
            ],
            artifacts: vec!["saml-audit.md".into()],
            asks: Vec::new(),
            updated_label: "1d".into(),
            done_label: Some("Tue".into()),
            goal: "Verify SAML assertion expiry and refresh-token revocation across multi-workspace accounts.".into(),
            started: "Mon, Nov 3".into(),
            history: vec![
                "Completed audit and merged 14 revocation tests".into(),
            ],
            decided: vec![TaskDecided::new(
                "Enforce strict 8h max session TTL?",
                "Yes, for enterprise workspaces",
                "2 days ago",
            )],
            agent: None,
            apps: Vec::new(),
        },
        AgentTask {
            id: "schema-v2".into(),
            title: "Order schema v2".into(),
            project: "Northwind".into(),
            status: TaskStatus::Done,
            status_text: "Merged and tagged".into(),
            ring_tone: TaskRingTone::Green,
            steps: vec![
                TaskStep::new("Draft backward-compatible fields", true),
                TaskStep::new("Validate replay fixtures", true),
            ],
            artifacts: vec!["event-schema.json".into()],
            asks: Vec::new(),
            updated_label: "2d".into(),
            done_label: Some("Mon".into()),
            goal: "Define versioned order event envelopes before starting the ingest pipeline.".into(),
            started: "Fri, Oct 31".into(),
            history: vec!["Published event-schema.json v2.0".into()],
            decided: Vec::new(),
            agent: None,
            apps: Vec::new(),
        },
    ]
}

fn sample_artifacts() -> Vec<TaskArtifact> {
    vec![
        TaskArtifact::doc(
            "ingest-design.md",
            "Updated 4m ago · Northwind",
            "Northwind Order Ingest — Architecture",
            "Incoming order webhooks land on stateless edge receivers, validate HMAC signatures, and append versioned envelopes into a 24-hour replayable log.\n\n• Sustained target: 20,000 events/min with p95 < 100 ms\n• Dead-letter isolation after 5 exponential retries\n• Zero-downtime consumer replay by timestamp cursor",
        ),
        TaskArtifact::sheet(
            "load-test-results.csv",
            "Updated 14m ago · 4 runs",
            vec![
                vec![
                    "Rate / min".into(),
                    "p50".into(),
                    "p95".into(),
                    "Errors".into(),
                ],
                vec![
                    "10,000".into(),
                    "18 ms".into(),
                    "42 ms".into(),
                    "0.00%".into(),
                ],
                vec![
                    "20,000".into(),
                    "31 ms".into(),
                    "84 ms".into(),
                    "0.02%".into(),
                ],
                vec![
                    "30,000".into(),
                    "64 ms".into(),
                    "178 ms".into(),
                    "0.19%".into(),
                ],
            ],
        ),
        TaskArtifact::doc(
            "event-schema.json",
            "Updated 3d ago · JSON Schema",
            "event-schema.json",
            "{\n  \"version\": 2,\n  \"event_id\": \"ord_01jc8q9z\",\n  \"workspace\": \"northwind\",\n  \"occurred_at\": \"2026-11-05T08:00:00Z\",\n  \"payload\": {\n    \"order_id\": \"NW-88412\",\n    \"total_cents\": 14800\n  }\n}",
        ),
        TaskArtifact::doc(
            "staging",
            "Folder · 3 items",
            "staging/",
            "• deploy-manifest.yaml (2.4 KB)\n• consumer-env.toml (1.1 KB)\n• smoke-replay.sh (890 B)",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        AgentTask, TaskArtifactKind, TaskAsk, TaskAskKind, TaskGroupBy, TaskRingTone, TaskStatus,
        TaskStep, sample_tasks,
    };

    #[test]
    fn sample_tasks_match_project_and_inbox_counts() {
        let tasks = sample_tasks();
        let active: Vec<&AgentTask> = tasks.iter().filter(|t| !t.is_done()).collect();
        assert_eq!(active.len(), 9);

        let nw = active
            .iter()
            .filter(|t| t.project.as_ref() == "Northwind")
            .count();
        let pf = active
            .iter()
            .filter(|t| t.project.as_ref() == "Platform")
            .count();
        let ek = active
            .iter()
            .filter(|t| t.project.as_ref() == "Evalkit")
            .count();
        assert_eq!((nw, pf, ek), (4, 3, 2));

        let total_asks: usize = active.iter().map(|t| t.needs_count()).sum();
        assert_eq!(total_asks, 5);

        let waiting = active.iter().filter(|t| t.needs_count() > 0).count();
        let running = active
            .iter()
            .filter(|t| t.needs_count() == 0 && t.status == TaskStatus::Running)
            .count();
        let done_count = tasks.iter().filter(|t| t.is_done()).count();
        assert_eq!((running, waiting, done_count), (4, 5, 2));
    }

    #[test]
    fn task_progress_and_artifact_kind_inference() {
        let t = AgentTask {
            id: "t1".into(),
            title: "Verify".into(),
            project: "Northwind".into(),
            status: TaskStatus::Running,
            status_text: "Testing".into(),
            ring_tone: TaskRingTone::Teal,
            steps: vec![
                TaskStep::new("One", true),
                TaskStep::new("Two", true),
                TaskStep::new("Three", false),
                TaskStep::new("Four", false),
            ],
            artifacts: Vec::new(),
            asks: vec![TaskAsk::new("a1", "Proceed?", TaskAskKind::Approval)],
            updated_label: "1m".into(),
            done_label: None,
            goal: "Goal".into(),
            started: "Today".into(),
            history: Vec::new(),
            decided: Vec::new(),
            agent: None,
            apps: Vec::new(),
        };
        assert_eq!(t.step_counts(), (2, 4));
        assert!((t.progress() - 0.5).abs() < 1e-5);
        assert_eq!(t.needs_count(), 1);
        assert_eq!(TaskGroupBy::default(), TaskGroupBy::Project);
        assert_eq!(
            TaskArtifactKind::from_filename("load-test-results.csv"),
            TaskArtifactKind::Sheet
        );
        assert_eq!(
            TaskArtifactKind::from_filename("event-schema.json"),
            TaskArtifactKind::Code
        );
        assert_eq!(
            TaskArtifactKind::from_filename("staging"),
            TaskArtifactKind::Folder
        );
    }
}
