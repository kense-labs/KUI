//! Single-thread orchestrator decision inbox with keyboard triage and workspace task inspector.
//!
//! Presents pending orchestrator choices, visual variant picks, action approvals,
//! and open-ended questions one item at a time alongside a multi-tab task and
//! artifact work area.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_app_tile, pro_btn, pro_eid, pro_empty_state, pro_fold,
            pro_icon_btn, pro_mark_selected, pro_pill_indicator, pro_progress_ring,
            pro_selection_sensor, pro_skeleton_bar, pro_toast_banner, pro_view_transition, tr,
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

const SVG_DECISION: &str = r#"<path d="M21 8.5V6.6C21 4.90294 21 4.05442 20.4728 3.52721C19.9456 3 19.0971 3 17.4 3H15.5M20 4L14.5 9.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M3 8.5V6.6C3 4.90294 3 4.05442 3.52721 3.52721C4.05442 3 4.90294 3 6.6 3H8.5M4 4L9.65686 9.65686C10.813 10.813 11.391 11.391 11.6955 12.1261C12 12.8612 12 13.6787 12 15.3137V21" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_PICK: &str = r#"<path d="M20.1088 3.89124C21.5 5.28249 21.5 7.52166 21.5 12C21.5 16.4783 21.5 18.7175 20.1088 20.1088C18.7175 21.5 16.4783 21.5 12 21.5C7.52166 21.5 5.28249 21.5 3.89124 20.1088C2.5 18.7175 2.5 16.4783 2.5 12C2.5 7.52166 2.5 5.28249 3.89124 3.89124C5.28249 2.5 7.52166 2.5 12 2.5C16.4783 2.5 18.7175 2.5 20.1088 3.89124Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M21.5 12L2.5 12" stroke="currentColor" stroke-linecap="round" stroke-width="1.9"></path><path d="M12 2.5L12 21.5" stroke="currentColor" stroke-linecap="round" stroke-width="1.9"></path>"#;

const SVG_APPROVAL: &str = r#"<path d="M22 12C22 6.47715 17.5228 2 12 2C6.47715 2 2 6.47715 2 12C2 17.5228 6.47715 22 12 22C17.5228 22 22 17.5228 22 12Z" stroke="currentColor" stroke-width="1.9"></path><path d="M8 12.5L10.5 15L16 9" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_QUESTION: &str = r#"<circle cx="12" cy="12" r="10" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></circle><path d="M9.5 9.5C9.5 8.11929 10.61929 7 12 7C13.3807 7 14.5 8.11929 14.5 9.5C14.5 10.3569 14.0689 11.1131 13.4117 11.5636C12.7283 12.0319 12 12.6716 12 13.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M12.125 16.75H12M12.25 16.75C12.25 16.8881 12.1381 17 12 17C11.8619 17 11.75 16.8881 11.75 16.75C11.75 16.6119 11.8619 16.5 12 16.5C12.1381 16.5 12.25 16.6119 12.25 16.75Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_DECLINED: &str = r#"<circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="1.9"></circle><path d="M15 9L9 15M9 9L15 15" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_FILE_DOC: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><path d="M4 6.6h4M4 8.6h2.6" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"></path>"#;

const SVG_FILE_SHEET: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><rect x="3.4" y="5.6" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="6.3" y="5.6" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="3.4" y="7.8" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="6.3" y="7.8" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect>"#;

const SVG_FILE_CODE: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><path d="M4.8 6.4L3.6 7.6L4.8 8.8M7.2 6.4L8.4 7.6L7.2 8.8" stroke="currentColor" stroke-width="1.05" stroke-linecap="round" stroke-linejoin="round"></path>"#;

const SVG_FILE_FOLDER: &str = r#"<path d="M1.4 3.1c0-.72.58-1.3 1.3-1.3h1.72c.38 0 .73.19.94.5l.6.9h3.34c.72 0 1.3.58 1.3 1.3V6H1.4Z" fill="currentColor"></path><path d="M1 6.95C.86 6.3 1.36 5.7 2.02 5.7h7.96c.66 0 1.16.6 1.02 1.25l-.5 2.4a1.3 1.3 0 0 1-1.27 1.05H2.77A1.3 1.3 0 0 1 1.5 9.35Z" fill="currentColor" opacity="0.45"></path>"#;

const SVG_STEP_CHECK: &str = r#"<path d="M5 14L8.5 17.5L19 6.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2.8"></path>"#;

/// Active filter tab in the inbox list header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InboxTab {
    #[default]
    Open,
    Done,
}

/// Prompt category presented by the orchestrator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InboxItemKind {
    #[default]
    Decision,
    Pick,
    Approval,
    Question,
}

/// Wireframe thumbnail preset for [`InboxItemKind::Pick`] variant cards.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InboxWireframe {
    #[default]
    StarterCards,
    SampleChart,
    MinimalNote,
}

/// Selectable option or visual variant inside an [`InboxItem`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InboxOption {
    pub id: SharedString,
    pub title: SharedString,
    pub summary: Option<SharedString>,
    pub recommended: bool,
    pub pros: Vec<SharedString>,
    pub cons: Vec<SharedString>,
    pub wireframe: Option<InboxWireframe>,
}

impl InboxOption {
    /// Construct an option with `id` and display `title`.
    pub fn new(id: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            summary: None,
            recommended: false,
            pros: Vec::new(),
            cons: Vec::new(),
            wireframe: None,
        }
    }

    /// Attach a one-line summary under the option title.
    pub fn with_summary(mut self, summary: impl Into<SharedString>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Mark this option as recommended so it is preselected when opened.
    pub fn with_recommended(mut self, recommended: bool) -> Self {
        self.recommended = recommended;
        self
    }

    /// Set the pros and cons revealed when this option is selected.
    pub fn with_tradeoffs(mut self, pros: Vec<SharedString>, cons: Vec<SharedString>) -> Self {
        self.pros = pros;
        self.cons = cons;
        self
    }

    /// Attach a wireframe preview thumbnail for [`InboxItemKind::Pick`] items.
    pub fn with_wireframe(mut self, wireframe: InboxWireframe) -> Self {
        self.wireframe = Some(wireframe);
        self
    }
}

/// Key-value fact row shown on [`InboxItemKind::Approval`] items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InboxFact {
    pub label: SharedString,
    pub value: SharedString,
}

impl InboxFact {
    /// Create a key-value fact row.
    pub fn new(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

/// User response recorded for an [`InboxItem`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InboxAnswer {
    Choice {
        option: Option<SharedString>,
        text: Option<SharedString>,
    },
    Approve,
    Decline,
}

/// Completion metadata for items under [`InboxTab::Done`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InboxDoneState {
    pub answer: InboxAnswer,
    pub label: SharedString,
    pub at_short: SharedString,
    pub since_steps: Vec<SharedString>,
    pub since_files: Vec<SharedString>,
}

/// One orchestrator prompt in [`AgentInboxScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InboxItem {
    pub id: SharedString,
    pub kind: InboxItemKind,
    pub question: SharedString,
    pub context: Option<SharedString>,
    pub task: SharedString,
    pub at_short: SharedString,
    pub at_long: SharedString,
    pub unread: bool,
    pub app: Option<SharedString>,
    pub options: Vec<InboxOption>,
    pub facts: Vec<InboxFact>,
    pub suggestions: Vec<SharedString>,
    pub done: Option<InboxDoneState>,
}

impl InboxItem {
    /// Construct a new open inbox item.
    pub fn new(
        id: impl Into<SharedString>,
        kind: InboxItemKind,
        question: impl Into<SharedString>,
        task: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            question: question.into(),
            context: None,
            task: task.into(),
            at_short: "now".into(),
            at_long: "just now".into(),
            unread: false,
            app: None,
            options: Vec::new(),
            facts: Vec::new(),
            suggestions: Vec::new(),
            done: None,
        }
    }

    /// Return the default option id (`recommended` first, otherwise first option).
    pub fn default_option_id(&self) -> Option<SharedString> {
        self.options
            .iter()
            .find(|o| o.recommended)
            .or_else(|| self.options.first())
            .map(|o| o.id.clone())
    }
}

/// Execution progress state of a step inside the right-hand Task panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InboxStepState {
    Done,
    WaitingOnYou,
    RunningNow,
    #[default]
    Later,
}

/// One step in [`InboxTaskInfo`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InboxTaskStep {
    pub label: SharedString,
    pub state: InboxStepState,
}

impl InboxTaskStep {
    /// Create a task step with `label` and `state`.
    pub fn new(label: impl Into<SharedString>, state: InboxStepState) -> Self {
        Self {
            label: label.into(),
            state,
        }
    }
}

/// Background task context displayed in the right-hand Task panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InboxTaskInfo {
    pub name: SharedString,
    pub goal: SharedString,
    pub project: SharedString,
    pub status: SharedString,
    pub progress: Option<(u32, u32)>,
    pub started: SharedString,
    pub steps: Vec<InboxTaskStep>,
    pub artifacts: Vec<SharedString>,
}

/// Entry inside a folder artifact viewer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InboxFolderEntry {
    pub name: SharedString,
    pub size: SharedString,
    pub modified: SharedString,
    pub is_folder: bool,
}

/// Typed content payload for an open workspace [`InboxArtifact`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InboxArtifactBody {
    Document {
        title: SharedString,
        paragraphs: Vec<SharedString>,
        checklist: Vec<(SharedString, bool)>,
    },
    Sheet {
        headers: Vec<SharedString>,
        rows: Vec<Vec<SharedString>>,
    },
    Code {
        language: SharedString,
        source: SharedString,
    },
    Folder {
        entries: Vec<InboxFolderEntry>,
    },
}

/// Named workspace artifact inspectable from the right-hand Task panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InboxArtifact {
    pub name: SharedString,
    pub updated: SharedString,
    pub body: InboxArtifactBody,
}

/// Events emitted by [`AgentInboxScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentInboxEvent {
    Navigate(SharedString),
    Select(Option<SharedString>),
    Decide {
        id: SharedString,
        answer: InboxAnswer,
    },
    Later(SharedString),
    Reopen(SharedString),
    OpenArtifact {
        task: SharedString,
        name: SharedString,
    },
    ArtifactChanged {
        name: SharedString,
        artifact: InboxArtifact,
    },
}

impl EventEmitter<AgentInboxEvent> for AgentInboxScreen {}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PanelTab {
    Task,
    Artifact(SharedString),
    NewTab,
}

#[derive(Clone, Debug)]
struct SentBanner {
    item_id: SharedString,
    summary: SharedString,
}

/// Pure helper returning indices of `items` visible under `tab`.
fn visible_item_indices(items: &[InboxItem], tab: InboxTab) -> Vec<usize> {
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| match tab {
            InboxTab::Open => item.done.is_none(),
            InboxTab::Done => item.done.is_some(),
        })
        .map(|(i, _)| i)
        .collect()
}

/// Apply an answer to an open item and advance selection to the next open item.
fn apply_decision(
    items: &mut [InboxItem],
    item_id: &str,
    answer: InboxAnswer,
) -> Option<(SharedString, Option<SharedString>)> {
    let open_before = visible_item_indices(items, InboxTab::Open);
    let pos_in_open = open_before
        .iter()
        .position(|&idx| items[idx].id.as_ref() == item_id)?;
    let item_idx = open_before[pos_in_open];

    let label: SharedString = match &answer {
        InboxAnswer::Approve => "Approved".into(),
        InboxAnswer::Decline => "Declined".into(),
        InboxAnswer::Choice { option, text } => {
            let opt_title = option.as_ref().and_then(|oid| {
                items[item_idx]
                    .options
                    .iter()
                    .find(|o| &o.id == oid)
                    .map(|o| o.title.to_string())
            });
            match (opt_title, text.as_deref().filter(|s| !s.trim().is_empty())) {
                (Some(ot), Some(tx)) => format!("{ot} — {tx}").into(),
                (Some(ot), None) => ot.into(),
                (None, Some(tx)) => tx.to_owned().into(),
                (None, None) => "Answered".into(),
            }
        }
    };

    items[item_idx].unread = false;
    items[item_idx].done = Some(InboxDoneState {
        answer,
        label: label.clone(),
        at_short: "just now".into(),
        since_steps: vec!["Queued orchestrator continuation".into()],
        since_files: Vec::new(),
    });

    let open_after = visible_item_indices(items, InboxTab::Open);
    let next_selected = if open_after.is_empty() {
        None
    } else {
        let next_pos = pos_in_open.min(open_after.len() - 1);
        Some(items[open_after[next_pos]].id.clone())
    };
    Some((label, next_selected))
}

/// Move an open item to the end of the open list and return the next selected id.
fn defer_to_end(items: &mut Vec<InboxItem>, item_id: &str) -> Option<SharedString> {
    let idx = items
        .iter()
        .position(|it| it.id.as_ref() == item_id && it.done.is_none())?;
    let mut item = items.remove(idx);
    item.unread = false;
    items.push(item);
    let open_now = visible_item_indices(items, InboxTab::Open);
    open_now.first().map(|&i| items[i].id.clone())
}

/// Reopen a done item, placing it at the top of the list.
fn reopen_to_top(items: &mut Vec<InboxItem>, item_id: &str) -> bool {
    let Some(idx) = items
        .iter()
        .position(|it| it.id.as_ref() == item_id && it.done.is_some())
    else {
        return false;
    };
    let mut item = items.remove(idx);
    item.done = None;
    items.insert(0, item);
    true
}

/// Single-thread orchestrator inbox screen hosted in [`AppShell`].
pub struct AgentInboxScreen {
    id: ElementId,
    standalone: bool,
    workspace: SharedString,
    user_name: SharedString,
    user_email: SharedString,
    items: Vec<InboxItem>,
    tasks: Vec<InboxTaskInfo>,
    artifacts: Vec<InboxArtifact>,
    running_tasks: u32,
    tab: InboxTab,
    selected_id: Option<SharedString>,
    chosen_options: Vec<(SharedString, SharedString)>,
    open_panel_tabs: Vec<PanelTab>,
    active_panel_tab: usize,
    active_sheet_cell: (usize, usize),
    folder_sort_asc: bool,
    quick_look_entry: Option<SharedString>,
    doc_bold: bool,
    doc_italic: bool,
    doc_code: bool,
    loading: bool,
    empty: bool,
    mobile_reading_open: bool,
    last_sent: Option<SentBanner>,
    toast: Option<(SharedString, Instant)>,
    status_announcement: SharedString,
    reply_editor: Option<Entity<Editor>>,
    answer_editor: Option<Entity<Editor>>,
    folder_search_editor: Option<Entity<Editor>>,
    pending_clear_reply: bool,
    pending_set_answer: Option<String>,
    pending_focus_reply: bool,
    pending_focus_root: bool,
    focus: FocusHandle,
    shell: Entity<AppShell>,
    measured_width: Rc<Cell<f32>>,
    started_at: Instant,
}

impl AgentInboxScreen {
    /// Construct an [`AgentInboxScreen`] populated with the canonical `Fieldwork` inbox items.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let items = default_inbox_items();
        let tasks = default_inbox_tasks();
        let artifacts = default_inbox_artifacts();
        let selected_id = items
            .iter()
            .find(|it| it.done.is_none())
            .map(|it| it.id.clone());

        let mut chosen_options = Vec::new();
        for item in &items {
            if let Some(opt_id) = item.default_option_id() {
                chosen_options.push((item.id.clone(), opt_id));
            }
        }

        let open_count = items.iter().filter(|it| it.done.is_none()).count() as u32;
        let workspace: SharedString = "Fieldwork".into();
        let user_name: SharedString = "Alex Morgan".into();
        let user_email: SharedString = "alex@fieldwork.co".into();

        let shell = Self::build_shell(
            &id_str,
            workspace.clone(),
            user_name.clone(),
            user_email.clone(),
            Some(open_count),
            true,
            cx,
        );

        Self {
            id: ElementId::Name(id_str),
            standalone: false,
            workspace,
            user_name,
            user_email,
            items,
            tasks,
            artifacts,
            running_tasks: 3,
            tab: InboxTab::Open,
            selected_id,
            chosen_options,
            open_panel_tabs: vec![
                PanelTab::Task,
                PanelTab::Artifact("ingest-design.md".into()),
                PanelTab::Artifact("load-test-results.csv".into()),
            ],
            active_panel_tab: 0,
            active_sheet_cell: (0, 0),
            folder_sort_asc: true,
            quick_look_entry: None,
            doc_bold: false,
            doc_italic: false,
            doc_code: false,
            loading: false,
            empty: false,
            mobile_reading_open: false,
            last_sent: None,
            toast: None,
            status_announcement: SharedString::default(),
            reply_editor: None,
            answer_editor: None,
            folder_search_editor: None,
            pending_clear_reply: false,
            pending_set_answer: None,
            pending_focus_reply: false,
            pending_focus_root: false,
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
                ]),
                ShellNavGroup::named(
                    tr(shell_cx, "Workspace", "工作区"),
                    vec![ShellNavItem::new(
                        "settings",
                        tr(shell_cx, "Settings", "设置"),
                        ProIcon::Settings,
                    )],
                ),
            ];

            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(
                    crate::components::app_shell::AppShellPreset::Agent,
                    shell_cx,
                )
                .with_workspace(workspace)
                .with_user(user_name, user_email)
                .with_groups(groups)
                .with_active("inbox")
                .with_side_cards(false)
                .with_work_area(true, panel_open)
                .with_panel_width(356.);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_inspector(w, cx));
            s
        });
        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                cx.emit(AgentInboxEvent::Navigate(label.clone()));
            }
            AppShellEvent::PanelOpen(_) => cx.notify(),
            _ => {}
        })
        .detach();
        shell
    }

    fn sync_shell_meta(&mut self, cx: &mut Context<Self>) {
        let panel_open = self.shell.read(cx).is_panel_open();
        let count = if self.empty {
            None
        } else {
            Some(self.open_count() as u32)
        };
        let id_str = self.id.to_string();
        self.shell = Self::build_shell(
            &id_str,
            self.workspace.clone(),
            self.user_name.clone(),
            self.user_email.clone(),
            count,
            panel_open,
            cx,
        );
    }

    /// Render without the outer [`AppShell`] frame when embedded in another layout.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Override the sidebar workspace name.
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

    /// Replace the inbox items list.
    pub fn with_items(mut self, items: Vec<InboxItem>, cx: &mut Context<Self>) -> Self {
        self.chosen_options.clear();
        for item in &items {
            if let Some(opt) = item.default_option_id() {
                self.chosen_options.push((item.id.clone(), opt));
            }
        }
        self.selected_id = items
            .iter()
            .find(|it| it.done.is_none())
            .or_else(|| items.first())
            .map(|it| it.id.clone());
        self.items = items;
        self.sync_shell_meta(cx);
        self
    }

    /// Replace the task definitions displayed in the right-hand Task panel.
    pub fn with_tasks(mut self, tasks: Vec<InboxTaskInfo>) -> Self {
        self.tasks = tasks;
        self
    }

    /// Replace the workspace artifacts inspectable in the right-hand Task panel.
    pub fn with_artifacts(mut self, artifacts: Vec<InboxArtifact>) -> Self {
        self.artifacts = artifacts;
        self
    }

    /// Configure which artifact tabs start open beside the primary `Task` tab.
    pub fn with_open_artifacts(mut self, names: Vec<SharedString>) -> Self {
        self.open_panel_tabs = std::iter::once(PanelTab::Task)
            .chain(names.into_iter().map(PanelTab::Artifact))
            .collect();
        self.active_panel_tab = 0;
        self
    }

    /// Configure how many background tasks continue running in the caught-up state.
    pub fn with_running(mut self, running: u32) -> Self {
        self.running_tasks = running;
        self
    }

    /// Choose the initial list tab (`Open` or `Done`).
    pub fn with_tab(mut self, tab: InboxTab) -> Self {
        self.tab = tab;
        let vis = visible_item_indices(&self.items, tab);
        self.selected_id = vis.first().map(|&i| self.items[i].id.clone());
        self
    }

    /// Deep-link to a specific item by `id`, switching tabs automatically if needed.
    pub fn with_selected(mut self, item_id: impl Into<SharedString>) -> Self {
        let target: SharedString = item_id.into();
        if let Some(item) = self.items.iter().find(|it| it.id == target) {
            self.tab = if item.done.is_some() {
                InboxTab::Done
            } else {
                InboxTab::Open
            };
            self.selected_id = Some(target);
            self.mobile_reading_open = true;
        }
        self
    }

    /// Enable or disable the skeleton loading placeholder state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Enable or disable the empty inbox state.
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

    /// Mutably toggle the empty inbox state.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        self.sync_shell_meta(cx);
        cx.notify();
    }

    /// Return all inbox items.
    pub fn items(&self) -> &[InboxItem] {
        &self.items
    }

    /// Count items currently waiting in [`InboxTab::Open`].
    pub fn open_count(&self) -> usize {
        self.items.iter().filter(|it| it.done.is_none()).count()
    }

    /// Count completed items in [`InboxTab::Done`].
    pub fn done_count(&self) -> usize {
        self.items.iter().filter(|it| it.done.is_some()).count()
    }

    /// Return the currently selected item id, if any.
    pub fn selected_id(&self) -> Option<&SharedString> {
        self.selected_id.as_ref()
    }

    /// Latest polite screen-reader announcement produced by triage actions.
    pub fn status_announcement(&self) -> &str {
        self.status_announcement.as_ref()
    }

    /// Access the underlying [`AppShell`] entity.
    pub fn shell(&self) -> &Entity<AppShell> {
        &self.shell
    }

    fn chosen_option_for(&self, item: &InboxItem) -> Option<SharedString> {
        self.chosen_options
            .iter()
            .find(|(id, _)| id == &item.id)
            .map(|(_, opt)| opt.clone())
            .or_else(|| item.default_option_id())
    }

    fn set_chosen_option(&mut self, item_id: SharedString, opt_id: SharedString) {
        if let Some(slot) = self
            .chosen_options
            .iter_mut()
            .find(|(id, _)| id == &item_id)
        {
            slot.1 = opt_id;
        } else {
            self.chosen_options.push((item_id, opt_id));
        }
    }

    fn select_item(&mut self, id: SharedString, open_mobile: bool, cx: &mut Context<Self>) {
        if let Some(it) = self.items.iter_mut().find(|it| it.id == id) {
            it.unread = false;
        }
        self.selected_id = Some(id.clone());
        self.mobile_reading_open = open_mobile;
        self.pending_clear_reply = true;
        self.pending_set_answer = Some(String::new());
        if !self
            .open_panel_tabs
            .iter()
            .any(|t| matches!(t, PanelTab::Task))
        {
            self.open_panel_tabs.insert(0, PanelTab::Task);
            self.active_panel_tab = 0;
        }
        cx.emit(AgentInboxEvent::Select(Some(id)));
        cx.notify();
    }

    fn step_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let vis = visible_item_indices(&self.items, self.tab);
        if vis.is_empty() {
            return;
        }
        let cur_pos = self
            .selected_id
            .as_ref()
            .and_then(|sid| vis.iter().position(|&idx| &self.items[idx].id == sid))
            .unwrap_or(0);
        let next_pos = (cur_pos as isize + delta).clamp(0, vis.len() as isize - 1) as usize;
        let next_id = self.items[vis[next_pos]].id.clone();
        self.select_item(next_id, self.mobile_reading_open, cx);
    }

    fn submit_current(&mut self, explicit_approval: Option<bool>, cx: &mut Context<Self>) {
        if self.tab != InboxTab::Open || self.empty || self.loading {
            return;
        }
        let Some(cur_id) = self.selected_id.clone() else {
            return;
        };
        let Some(cur_item) = self.items.iter().find(|it| it.id == cur_id).cloned() else {
            return;
        };

        let reply_text = self
            .reply_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_owned())
            .unwrap_or_default();
        let area_text = self
            .answer_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_owned())
            .unwrap_or_default();

        let answer = match cur_item.kind {
            InboxItemKind::Approval => {
                if let Some(approved) = explicit_approval {
                    if approved {
                        InboxAnswer::Approve
                    } else {
                        InboxAnswer::Decline
                    }
                } else if !reply_text.is_empty() {
                    InboxAnswer::Choice {
                        option: None,
                        text: Some(reply_text.into()),
                    }
                } else {
                    InboxAnswer::Approve
                }
            }
            InboxItemKind::Question => {
                if area_text.is_empty() {
                    return;
                }
                InboxAnswer::Choice {
                    option: None,
                    text: Some(area_text.into()),
                }
            }
            InboxItemKind::Decision | InboxItemKind::Pick => {
                let chosen = self.chosen_option_for(&cur_item);
                let extra = (!reply_text.is_empty()).then(|| reply_text.into());
                InboxAnswer::Choice {
                    option: chosen,
                    text: extra,
                }
            }
        };

        if let Some((label, next_sel)) =
            apply_decision(&mut self.items, cur_id.as_ref(), answer.clone())
        {
            let left = self.open_count();
            let toast_msg: SharedString = format!("{} · {label}", tr(cx, "Sent", "已发送")).into();
            self.toast = Some((toast_msg, Instant::now()));
            self.last_sent = Some(SentBanner {
                item_id: cur_id.clone(),
                summary: label,
            });
            self.status_announcement = format!("Sent. {left} left.").into();
            self.selected_id = next_sel.clone();
            if let Some(ref nid) = self.selected_id
                && let Some(it) = self.items.iter_mut().find(|i| &i.id == nid)
            {
                it.unread = false;
            }
            self.pending_clear_reply = true;
            self.pending_set_answer = Some(String::new());
            self.pending_focus_root = true;
            cx.emit(AgentInboxEvent::Decide { id: cur_id, answer });
            cx.emit(AgentInboxEvent::Select(next_sel));
            self.sync_shell_meta(cx);
            cx.notify();
        }
    }

    fn snooze_current_later(&mut self, cx: &mut Context<Self>) {
        if self.tab != InboxTab::Open || self.empty || self.loading {
            return;
        }
        let Some(cur_id) = self.selected_id.clone() else {
            return;
        };
        if let Some(next_id) = defer_to_end(&mut self.items, cur_id.as_ref()) {
            self.toast = Some((
                tr(cx, "Saved for later", "已移至稍后处理").into(),
                Instant::now(),
            ));
            self.selected_id = Some(next_id.clone());
            if let Some(it) = self.items.iter_mut().find(|i| i.id == next_id) {
                it.unread = false;
                self.status_announcement = format!("Later. Next: {}", it.question).into();
            }
            self.pending_clear_reply = true;
            cx.emit(AgentInboxEvent::Later(cur_id));
            cx.emit(AgentInboxEvent::Select(Some(next_id)));
            cx.notify();
        }
    }

    fn reopen_by_id(&mut self, item_id: SharedString, cx: &mut Context<Self>) {
        if reopen_to_top(&mut self.items, item_id.as_ref()) {
            self.tab = InboxTab::Open;
            self.selected_id = Some(item_id.clone());
            self.last_sent = None;
            self.toast = Some((
                tr(cx, "Reopened in inbox", "已重新放回收件箱").into(),
                Instant::now(),
            ));
            let open = self.open_count();
            self.status_announcement = format!("Reopened. {open} open.").into();
            cx.emit(AgentInboxEvent::Reopen(item_id.clone()));
            cx.emit(AgentInboxEvent::Select(Some(item_id)));
            self.sync_shell_meta(cx);
            cx.notify();
        }
    }

    fn open_artifact_tab(&mut self, name: SharedString, cx: &mut Context<Self>) {
        let task_name = self
            .current_task()
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "Ingest pipeline".into());
        if let Some(pos) = self
            .open_panel_tabs
            .iter()
            .position(|t| matches!(t, PanelTab::Artifact(n) if n == &name))
        {
            self.active_panel_tab = pos;
        } else if self
            .open_panel_tabs
            .get(self.active_panel_tab)
            .is_some_and(|t| matches!(t, PanelTab::NewTab))
        {
            self.open_panel_tabs[self.active_panel_tab] = PanelTab::Artifact(name.clone());
        } else {
            self.open_panel_tabs.push(PanelTab::Artifact(name.clone()));
            self.active_panel_tab = self.open_panel_tabs.len() - 1;
        }
        self.shell
            .update(cx, |s, s_cx| s.set_panel_open(true, s_cx));
        cx.emit(AgentInboxEvent::OpenArtifact {
            task: task_name,
            name,
        });
        cx.notify();
    }

    fn close_panel_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.open_panel_tabs.len() {
            return;
        }
        self.open_panel_tabs.remove(index);
        if self.open_panel_tabs.is_empty() {
            self.open_panel_tabs.push(PanelTab::Task);
            self.active_panel_tab = 0;
            self.shell
                .update(cx, |s, s_cx| s.set_panel_open(false, s_cx));
        } else if self.active_panel_tab >= self.open_panel_tabs.len() {
            self.active_panel_tab = self.open_panel_tabs.len() - 1;
        }
        cx.notify();
    }

    fn current_item(&self) -> Option<&InboxItem> {
        let vis = visible_item_indices(&self.items, self.tab);
        if let Some(ref sid) = self.selected_id
            && let Some(&idx) = vis.iter().find(|&&i| &self.items[i].id == sid)
        {
            return Some(&self.items[idx]);
        }
        vis.first().map(|&i| &self.items[i])
    }

    fn current_task(&self) -> Option<&InboxTaskInfo> {
        if self.empty {
            return None;
        }
        let task_name = self
            .current_item()
            .map(|it| it.task.as_ref())
            .or_else(|| self.items.first().map(|it| it.task.as_ref()))?;
        self.tasks
            .iter()
            .find(|t| t.name.as_ref() == task_name)
            .or_else(|| self.tasks.first())
    }

    fn is_typing(&self, w: &Window, cx: &App) -> bool {
        self.reply_editor
            .as_ref()
            .is_some_and(|e| e.read(cx).focus.is_focused(w))
            || self
                .answer_editor
                .as_ref()
                .is_some_and(|e| e.read(cx).focus.is_focused(w))
            || self
                .folder_search_editor
                .as_ref()
                .is_some_and(|e| e.read(cx).focus.is_focused(w))
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.reply_editor.is_none() {
            let ph: SharedString = tr(cx, "Or tell it something else…", "或者补充其他说明…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => cx.notify(),
                EditEvent::Submitted => this.submit_current(None, cx),
                EditEvent::Escape => {
                    this.pending_focus_root = true;
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.reply_editor = Some(ed);
        }

        if self.answer_editor.is_none() {
            let ph: SharedString = tr(cx, "Write your answer…", "输入你的回答…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::InlineMulti, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => cx.notify(),
                EditEvent::Submitted => this.submit_current(None, cx),
                EditEvent::Escape => {
                    this.pending_focus_root = true;
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.answer_editor = Some(ed);
        }

        if self.folder_search_editor.is_none() {
            let ph: SharedString = tr(cx, "Filter files…", "筛选文件…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 12.5;
                e
            });
            cx.subscribe(&ed, |_, _, ev: &EditEvent, cx| {
                if matches!(ev, EditEvent::Changed | EditEvent::Submitted) {
                    cx.notify();
                }
            })
            .detach();
            self.folder_search_editor = Some(ed);
        }

        if self.pending_clear_reply {
            self.pending_clear_reply = false;
            if let Some(ed) = &self.reply_editor {
                ed.update(cx, |e, cx| e.set_value("", w, cx));
            }
        }
        if let Some(next_val) = self.pending_set_answer.take()
            && let Some(ed) = &self.answer_editor
        {
            ed.update(cx, |e, cx| e.set_value(next_val, w, cx));
        }
        if self.pending_focus_reply {
            self.pending_focus_reply = false;
            let is_question = self
                .current_item()
                .is_some_and(|it| it.kind == InboxItemKind::Question);
            let target = if is_question {
                self.answer_editor.as_ref()
            } else {
                self.reply_editor.as_ref()
            };
            if let Some(ed) = target {
                let fh = ed.read(cx).focus.clone();
                fh.focus(w, cx);
            }
        }
        if self.pending_focus_root {
            self.pending_focus_root = false;
            self.focus.focus(w, cx);
        }
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if self.is_typing(w, cx) {
            return;
        }
        let m = &ev.keystroke.modifiers;
        if m.platform || m.control || m.alt {
            return;
        }
        let key = ev.keystroke.key.as_str();
        match key {
            "j" | "J" | "down" => {
                self.step_selection(1, cx);
            }
            "k" | "K" | "up" => {
                self.step_selection(-1, cx);
            }
            "home" => {
                let vis = visible_item_indices(&self.items, self.tab);
                if let Some(&first) = vis.first() {
                    let id = self.items[first].id.clone();
                    self.select_item(id, self.mobile_reading_open, cx);
                }
            }
            "end" => {
                let vis = visible_item_indices(&self.items, self.tab);
                if let Some(&last) = vis.last() {
                    let id = self.items[last].id.clone();
                    self.select_item(id, self.mobile_reading_open, cx);
                }
            }
            "left" => {
                if self.tab != InboxTab::Open {
                    self.tab = InboxTab::Open;
                    let vis = visible_item_indices(&self.items, InboxTab::Open);
                    self.selected_id = vis.first().map(|&i| self.items[i].id.clone());
                    cx.notify();
                }
            }
            "right" => {
                if self.tab != InboxTab::Done {
                    self.tab = InboxTab::Done;
                    let vis = visible_item_indices(&self.items, InboxTab::Done);
                    self.selected_id = vis.first().map(|&i| self.items[i].id.clone());
                    cx.notify();
                }
            }
            "1" | "2" | "3" => {
                let idx = match key {
                    "1" => 0usize,
                    "2" => 1usize,
                    _ => 2usize,
                };
                if self.tab == InboxTab::Open
                    && let Some(cur) = self.current_item().cloned()
                    && let Some(opt) = cur.options.get(idx)
                {
                    self.set_chosen_option(cur.id, opt.id.clone());
                    cx.notify();
                }
            }
            "enter" => {
                if self.tab == InboxTab::Open {
                    self.submit_current(None, cx);
                }
            }
            "a" | "A" => {
                if self.tab == InboxTab::Open
                    && self
                        .current_item()
                        .is_some_and(|it| it.kind == InboxItemKind::Approval)
                {
                    self.submit_current(Some(true), cx);
                }
            }
            "d" | "D" => {
                if self.tab == InboxTab::Open
                    && self
                        .current_item()
                        .is_some_and(|it| it.kind == InboxItemKind::Approval)
                {
                    self.submit_current(Some(false), cx);
                }
            }
            "l" | "L" => {
                self.snooze_current_later(cx);
            }
            "r" | "R" | "/" => {
                if self.tab == InboxTab::Open && self.current_item().is_some() {
                    self.pending_focus_reply = true;
                    cx.notify();
                }
            }
            "u" | "U" => {
                if let Some(sent) = self.last_sent.clone() {
                    self.reopen_by_id(sent.item_id, cx);
                } else if self.tab == InboxTab::Done
                    && let Some(cur) = self.current_item().cloned()
                {
                    self.reopen_by_id(cur.id, cx);
                }
            }
            "escape" => {
                if self.quick_look_entry.is_some() {
                    self.quick_look_entry = None;
                    cx.notify();
                } else if self.mobile_reading_open {
                    self.mobile_reading_open = false;
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    fn render_kind_glyph(
        kind: InboxItemKind,
        done: Option<&InboxDoneState>,
        side: f32,
        p: &ProPalette,
    ) -> AnyElement {
        if let Some(done_state) = done {
            if matches!(done_state.answer, InboxAnswer::Decline) {
                return pro_svg("0 0 24 24", SVG_DECLINED, side, p.muted);
            }
            return pro_svg("0 0 24 24", SVG_APPROVAL, side, p.green);
        }
        match kind {
            InboxItemKind::Decision => pro_svg("0 0 24 24", SVG_DECISION, side, p.purple),
            InboxItemKind::Pick => pro_svg("0 0 24 24", SVG_PICK, side, p.blue),
            InboxItemKind::Approval => pro_svg("0 0 24 24", SVG_APPROVAL, side, p.green),
            InboxItemKind::Question => pro_svg("0 0 24 24", SVG_QUESTION, side, p.orange),
        }
    }

    fn render_file_glyph(name: &str, side: f32, p: &ProPalette) -> AnyElement {
        let lower = name.to_ascii_lowercase();
        if !lower.contains('.') {
            pro_svg("0 0 12 12", SVG_FILE_FOLDER, side, p.amber)
        } else if lower.ends_with(".csv") || lower.ends_with(".xlsx") {
            pro_svg("0 0 12 12", SVG_FILE_SHEET, side, p.green)
        } else if lower.ends_with(".json")
            || lower.ends_with(".ts")
            || lower.ends_with(".rs")
            || lower.ends_with(".sql")
        {
            pro_svg("0 0 12 12", SVG_FILE_CODE, side, p.orange)
        } else {
            pro_svg("0 0 12 12", SVG_FILE_DOC, side, p.blue)
        }
    }

    fn render_wireframe_thumb(preset: InboxWireframe, height: f32, p: &ProPalette) -> Div {
        let wire_ink = if p.dark {
            hsla(0., 0., 1., 0.50)
        } else {
            hsla(0., 0., 0., 0.42)
        };
        let wire_mid = if p.dark {
            hsla(0., 0., 1., 0.20)
        } else {
            hsla(0., 0., 0., 0.16)
        };
        let wire_soft = if p.dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.07)
        };

        let inner = match preset {
            InboxWireframe::StarterCards => div()
                .size_full()
                .p(px(10.))
                .flex()
                .flex_col()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(div().w(px(52.)).h(px(5.)).rounded(px(2.)).bg(wire_ink))
                        .child(div().w(px(78.)).h(px(3.5)).rounded(px(2.)).bg(wire_mid)),
                )
                .child(div().flex().gap(px(5.)).children((0..3).map(|_| {
                    div()
                        .flex_1()
                        .h(px(34.))
                        .rounded(px(4.))
                        .bg(wire_soft)
                        .p(px(5.))
                        .flex()
                        .flex_col()
                        .justify_between()
                        .child(div().size(px(7.)).rounded(px(2.)).bg(wire_mid))
                        .child(div().w_full().h(px(3.5)).rounded(px(1.5)).bg(wire_ink))
                }))),
            InboxWireframe::SampleChart => div()
                .size_full()
                .p(px(10.))
                .flex()
                .flex_col()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(div().w(px(44.)).h(px(5.)).rounded(px(2.)).bg(wire_ink))
                        .child(div().w(px(26.)).h(px(9.)).rounded(px(3.)).bg(wire_mid)),
                )
                .child(div().h(px(40.)).flex().items_end().gap(px(4.)).children(
                    [14., 22., 18., 30., 25., 36., 28.].into_iter().map(|bh| {
                        div().flex_1().h(px(bh)).rounded(px(2.)).bg(if bh > 28. {
                            wire_mid
                        } else {
                            wire_soft
                        })
                    }),
                )),
            InboxWireframe::MinimalNote => div()
                .size_full()
                .p(px(10.))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(5.))
                .child(div().size(px(14.)).rounded(px(4.)).bg(wire_soft))
                .child(div().w(px(64.)).h(px(4.5)).rounded(px(2.)).bg(wire_ink))
                .child(div().w(px(82.)).h(px(3.5)).rounded(px(2.)).bg(wire_mid))
                .child(
                    div()
                        .mt(px(2.))
                        .w(px(38.))
                        .h(px(10.))
                        .rounded(px(3.))
                        .bg(wire_ink),
                ),
        };

        div()
            .w_full()
            .h(px(height))
            .rounded(px(6.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .overflow_hidden()
            .child(inner)
    }

    fn render_list_column(&self, narrow: bool, phase: f32, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let open_n = self.open_count();
        let vis = visible_item_indices(&self.items, self.tab);
        let active_sid = self.current_item().map(|it| it.id.clone());

        let tab_sensor = pro_selection_sensor();
        let selected_tab_key = match self.tab {
            InboxTab::Open => "open",
            InboxTab::Done => "done",
        };
        let seg_btn = |tab: InboxTab,
                       label: SharedString,
                       suffix: &'static str,
                       sensor: crate::components::pro_chrome::ProSelectionSensor,
                       cx: &mut Context<Self>| {
            let selected = self.tab == tab;
            activate(
                div()
                    .id(pro_eid(&self.id, suffix))
                    .role(Role::Tab)
                    .tab_index(0)
                    .relative()
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .when(!selected, |d| {
                        d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                    })
                    .when(selected, |d| d.child(pro_mark_selected(&sensor)))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .text_size(px(13.))
                    .font_weight(if selected {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if selected { p.ink } else { p.muted })
                    .cursor_pointer()
                    .child(label),
                cx,
                move |this, _, cx| {
                    this.tab = tab;
                    let next_vis = visible_item_indices(&this.items, tab);
                    this.selected_id = next_vis.first().map(|&i| this.items[i].id.clone());
                    cx.notify();
                },
            )
        };

        let open_lbl: SharedString = tr(cx, "Open", "待处理").into();
        let done_lbl: SharedString = tr(cx, "Done", "已完成").into();
        let btn_open = seg_btn(InboxTab::Open, open_lbl, "tab-open", tab_sensor.clone(), cx);
        let btn_done = seg_btn(InboxTab::Done, done_lbl, "tab-done", tab_sensor.clone(), cx);

        let header = div()
            .h(px(48.))
            .pl(px(20.))
            .pr(px(12.))
            .flex_none()
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(px(7.))
                    .min_w_0()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tr(cx, "Inbox", "收件箱")),
                    )
                    .when(!self.empty && !self.loading && open_n > 0, |d| {
                        d.child(
                            div()
                                .text_size(px(13.))
                                .text_color(p.faint)
                                .child(open_n.to_string()),
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
                        pro_eid(&self.id, "inbox-tabs"),
                        selected_tab_key,
                        tab_sensor,
                        6.,
                        p.chip,
                        None,
                        false,
                    ))
                    .child(btn_open)
                    .child(btn_done),
            );

        let mut rows_inner = div().w_full().flex().flex_col().gap(px(2.));

        if self.loading {
            for (w_q, w_m) in [
                (196., 104.),
                (180., 116.),
                (188., 98.),
                (164., 88.),
                (192., 108.),
            ] {
                rows_inner = rows_inner.child(
                    div()
                        .h(px(56.))
                        .px(px(12.))
                        .py(px(10.))
                        .rounded(px(8.))
                        .flex()
                        .items_start()
                        .gap(px(12.))
                        .child(
                            div()
                                .pt(px(2.))
                                .child(pro_skeleton_bar(15., 15., 5., phase, cx)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(7.))
                                .child(pro_skeleton_bar(w_q, 12., 4., phase, cx))
                                .child(pro_skeleton_bar(w_m, 10., 4., phase, cx)),
                        ),
                );
            }
        } else if self.empty || vis.is_empty() {
            let empty_note = if self.empty {
                tr(cx, "No items yet", "暂无待办事项")
            } else if self.tab == InboxTab::Open {
                tr(cx, "Nothing waiting on you", "没有等待你决定的事项")
            } else {
                tr(cx, "Nothing answered yet", "暂无已处理记录")
            };
            rows_inner = rows_inner.child(
                div()
                    .px(px(12.))
                    .py(px(10.))
                    .text_size(px(13.))
                    .text_color(p.muted)
                    .child(empty_note),
            );
        } else {
            for idx in vis {
                let item = &self.items[idx];
                let is_sel = !narrow && active_sid.as_ref() == Some(&item.id);
                let item_id = item.id.clone();
                let aria = format!(
                    "{}: {}, {}",
                    item.question,
                    item.task,
                    if item.unread { "unread" } else { "read" }
                );
                let sub_time = item
                    .done
                    .as_ref()
                    .map(|d| d.at_short.clone())
                    .unwrap_or_else(|| item.at_short.clone());

                rows_inner = rows_inner.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("row-{item_id}")))
                        .role(Role::ListBoxOption)
                        .aria_label(aria)
                        .tab_index(if is_sel { 0 } else { -1 })
                        .h(px(56.))
                        .px(px(12.))
                        .py(px(10.))
                        .rounded(px(8.))
                        .bg(if is_sel { p.chip } else { transparent_black() })
                        .hover(move |s| s.bg(if is_sel { p.chip } else { p.hover }))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_start()
                        .gap(px(12.))
                        .cursor_pointer()
                        .child(
                            div()
                                .h(px(20.))
                                .w(px(16.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(Self::render_kind_glyph(
                                    item.kind,
                                    item.done.as_ref(),
                                    16.,
                                    &p,
                                )),
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
                                        .line_height(px(20.))
                                        .text_color(p.ink)
                                        .truncate()
                                        .child(item.question.clone()),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(5.))
                                        .min_w_0()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .child(div().min_w_0().truncate().child(item.task.clone()))
                                        .child(div().flex_none().text_color(p.faint).child("·"))
                                        .child(div().flex_none().child(sub_time)),
                                ),
                        )
                        .child(
                            div()
                                .w(px(6.))
                                .pt(px(7.))
                                .flex_none()
                                .when(item.unread && item.done.is_none(), |d| {
                                    d.child(div().size(px(6.)).rounded_full().bg(p.blue))
                                }),
                        ),
                    cx,
                    move |this, _, cx| {
                        this.select_item(item_id.clone(), true, cx);
                    },
                ));
            }
        }

        let list_body: AnyElement = if self.loading {
            rows_inner.into_any_element()
        } else {
            pro_view_transition(
                pro_eid(&self.id, "inbox-list-vtx"),
                format!("{:?}-{}", self.tab, self.empty),
                rows_inner,
            )
        };

        let rows_col = div()
            .id(pro_eid(&self.id, "rows-scroll"))
            .role(Role::ListBox)
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .pt(px(6.))
            .pb(px(16.))
            .px(px(8.))
            .flex()
            .flex_col()
            .child(list_body);

        let main_w = self.measured_width.get();
        let list_w = if main_w < 740. { 268. } else { 320. };

        div()
            .when(narrow, |d| d.w_full())
            .when(!narrow, |d| {
                d.w(px(list_w))
                    .flex_none()
                    .border_r_1()
                    .border_color(p.line)
            })
            .h_full()
            .min_h_0()
            .flex()
            .flex_col()
            .child(header)
            .child(rows_col)
    }

    fn render_reading_column(&self, narrow: bool, phase: f32, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let back_bar = narrow.then(|| {
            div()
                .h(px(48.))
                .px(px(12.))
                .flex_none()
                .border_b_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .child(pro_btn(
                    pro_eid(&self.id, "mobile-back"),
                    tr(cx, "Inbox", "返回收件箱"),
                    Some(ProIcon::ChevronLeft),
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    |this, _, cx| {
                        this.mobile_reading_open = false;
                        cx.notify();
                    },
                ))
        });

        let body: AnyElement = if self.loading {
            div()
                .w_full()
                .max_w(px(580.))
                .mx_auto()
                .pt(px(if narrow { 24. } else { 72. }))
                .pb(px(56.))
                .px(px(if narrow { 20. } else { 36. }))
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(pro_skeleton_bar(148., 11., 4., phase, cx))
                .child(pro_skeleton_bar(320., 24., 6., phase, cx))
                .child(pro_skeleton_bar(260., 13., 4., phase, cx))
                .child(pro_skeleton_bar(220., 13., 4., phase, cx))
                .child(
                    div()
                        .mt(px(18.))
                        .p(px(12.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(240., 16., 4., phase, cx))
                        .child(pro_skeleton_bar(210., 16., 4., phase, cx))
                        .child(pro_skeleton_bar(190., 16., 4., phase, cx)),
                )
                .child(
                    div()
                        .mt(px(6.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex_1()
                                .child(pro_skeleton_bar(180., 30., 7., phase, cx)),
                        )
                        .child(pro_skeleton_bar(56., 28., 6., phase, cx))
                        .child(pro_skeleton_bar(56., 28., 6., phase, cx)),
                )
                .into_any_element()
        } else if self.empty {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child(pro_empty_state(
                    ProIcon::Inbox,
                    tr(cx, "Nothing in your inbox yet", "收件箱中暂无内容"),
                    tr(
                        cx,
                        "When your orchestrator needs a choice, variant or approval, it shows up here.",
                        "当编排智能体需要你做出选择、挑选方案或审批操作时，会显示在这里。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-go-thread"),
                        tr(cx, "Go to Thread", "前往会话").into(),
                        Box::new(|_, _, cx| {
                            cx.emit(AgentInboxEvent::Navigate("Thread".into()));
                        }),
                    )),
                    cx,
                ))
                .into_any_element()
        } else if let Some(item) = self.current_item().cloned() {
            if let Some(done_state) = item.done.clone() {
                self.render_done_detail(&item, &done_state, narrow, cx)
                    .into_any_element()
            } else {
                self.render_open_detail(&item, narrow, cx)
                    .into_any_element()
            }
        } else if self.tab == InboxTab::Open {
            self.render_caught_up(cx).into_any_element()
        } else {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child(pro_empty_state(
                    ProIcon::Check,
                    tr(cx, "No answered items yet", "暂无已处理记录"),
                    tr(
                        cx,
                        "Decisions and approvals you send will appear here so you can review or reopen them.",
                        "你提交的决策与审批会保存在此处，方便随时查看或重新打开。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element()
        };

        let body: AnyElement = if self.loading {
            body
        } else {
            let active_key = self
                .current_item()
                .map(|it| it.id.as_ref())
                .unwrap_or("caught-up");
            let detail_key = format!("{:?}-{active_key}-{}", self.tab, self.empty);
            pro_view_transition(pro_eid(&self.id, "reading-vtx"), detail_key, body)
        };

        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .min_h_0()
            .flex()
            .flex_col()
            .when_some(back_bar, |d, b| d.child(b))
            .child(
                div()
                    .id(pro_eid(&self.id, "detail-scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .child(body),
            )
    }

    fn render_caught_up(&self, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let running = self.running_tasks;
        let sub_text = if running == 1 {
            tr(
                cx,
                "1 task keeps running in the background",
                "1 个任务仍在后台运行",
            )
            .to_owned()
        } else {
            format!(
                "{running} {}",
                tr(
                    cx,
                    "tasks keep running in the background",
                    "个任务仍在后台运行"
                )
            )
        };
        let undo_id = self.last_sent.as_ref().map(|s| s.item_id.clone());

        div()
            .flex_1()
            .py(px(64.))
            .px(px(24.))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .child(
                div()
                    .size(px(36.))
                    .rounded_full()
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_icon(ProIcon::Check, 17., p.ink)),
            )
            .child(
                div()
                    .mt(px(16.))
                    .text_size(px(15.))
                    .line_height(px(24.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(tr(cx, "You're all caught up", "已全部处理完毕")),
            )
            .child(
                div()
                    .mt(px(2.))
                    .mb(px(20.))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.muted)
                    .child(sub_text),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(pro_btn(
                        pro_eid(&self.id, "caught-tasks"),
                        tr(cx, "Open Tasks", "查看任务"),
                        None,
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |_, _, cx| {
                            cx.emit(AgentInboxEvent::Navigate("Tasks".into()));
                        },
                    ))
                    .when_some(undo_id, |d, uid| {
                        d.child(pro_btn(
                            pro_eid(&self.id, "caught-undo"),
                            tr(cx, "Undo last", "撤销上一条"),
                            Some(ProIcon::Undo),
                            ProBtnStyle::Ghost,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.reopen_by_id(uid.clone(), cx);
                            },
                        ))
                    }),
            )
    }

    fn render_open_detail(&self, item: &InboxItem, narrow: bool, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let rec_bg = if p.dark {
            hsla(212. / 360., 1.0, 0.50, 0.22)
        } else {
            hsla(205. / 360., 1.0, 0.48, 0.10)
        };
        let rec_fg = if p.dark { p.blue } else { rgb(0x006dcb).into() };

        // Sent banner with quick Undo if the user just answered the previous prompt
        let sent_toast = self.last_sent.as_ref().map(|sent| {
            let undo_id = sent.item_id.clone();
            div()
                .mb(px(18.))
                .px(px(12.))
                .h(px(34.))
                .rounded(px(8.))
                .bg(p.chip)
                .flex()
                .items_center()
                .justify_between()
                .gap(px(10.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .min_w_0()
                        .child(
                            div()
                                .size(px(18.))
                                .rounded_full()
                                .bg(p.green_soft)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(pro_icon(ProIcon::Check, 11., p.green)),
                        )
                        .child(
                            div()
                                .text_size(px(12.5))
                                .text_color(p.text)
                                .truncate()
                                .child(format!(
                                    "{} · {}",
                                    tr(
                                        cx,
                                        "Sent – it'll take it from here",
                                        "已发送，智能体将继续执行"
                                    ),
                                    sent.summary
                                )),
                        ),
                )
                .child(pro_btn(
                    pro_eid(&self.id, "undo-sent-btn"),
                    tr(cx, "Undo", "撤销"),
                    Some(ProIcon::Undo),
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    move |this, _, cx| {
                        this.reopen_by_id(undo_id.clone(), cx);
                    },
                ))
        });

        let meta_row = div()
            .flex()
            .items_center()
            .gap(px(7.))
            .min_w_0()
            .text_size(px(12.))
            .line_height(px(16.))
            .text_color(p.muted)
            .child(Self::render_kind_glyph(item.kind, None, 14., &p))
            .when_some(item.app.as_ref(), |d, app| {
                d.child(pro_app_tile(app.as_ref(), 16., cx))
            })
            .child(div().min_w_0().truncate().child(item.task.clone()))
            .child(div().flex_none().text_color(p.faint).child("·"))
            .child(div().flex_none().child(item.at_long.clone()));

        let chosen_opt = self.chosen_option_for(item);

        let answer_section: AnyElement = match item.kind {
            InboxItemKind::Decision => {
                let mut opts_card = div()
                    .p(px(4.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(2.));

                for (oi, opt) in item.options.iter().enumerate() {
                    let checked = chosen_opt.as_ref() == Some(&opt.id);
                    let iid = item.id.clone();
                    let oid = opt.id.clone();
                    let num_badge = div()
                        .size(px(20.))
                        .flex_none()
                        .rounded(px(5.))
                        .when(checked, |d| d.bg(p.primary).text_color(p.on_primary))
                        .when(!checked, |d| {
                            d.border_1().border_color(p.line_strong).text_color(p.muted)
                        })
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .child((oi + 1).to_string());

                    let mut body_col = div().flex_1().min_w_0().flex().flex_col().child(
                        div()
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .truncate()
                                    .child(opt.title.clone()),
                            )
                            .when(opt.recommended, |d| {
                                d.child(
                                    div()
                                        .h(px(18.))
                                        .px(px(6.))
                                        .rounded(px(4.))
                                        .bg(rec_bg)
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(rec_fg)
                                        .flex()
                                        .items_center()
                                        .child(tr(cx, "Recommended", "推荐")),
                                )
                            }),
                    );

                    if let Some(ref sum) = opt.summary {
                        body_col = body_col.child(
                            div()
                                .text_size(px(13.))
                                .line_height(px(20.))
                                .text_color(p.muted)
                                .truncate()
                                .child(sum.clone()),
                        );
                    }

                    if !opt.pros.is_empty() || !opt.cons.is_empty() {
                        let est_h = ((opt.pros.len() + opt.cons.len()) as f32) * 20. + 8.;
                        let mut lines = div().pt(px(8.)).flex().flex_col().gap(px(2.));
                        for pro in &opt.pros {
                            lines = lines.child(
                                div()
                                    .flex()
                                    .items_start()
                                    .gap(px(8.))
                                    .text_size(px(12.))
                                    .line_height(px(18.))
                                    .text_color(p.muted)
                                    .child(
                                        div()
                                            .w(px(8.))
                                            .flex_none()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.faint)
                                            .child("+"),
                                    )
                                    .child(div().flex_1().min_w_0().child(pro.clone())),
                            );
                        }
                        for con in &opt.cons {
                            lines = lines.child(
                                div()
                                    .flex()
                                    .items_start()
                                    .gap(px(8.))
                                    .text_size(px(12.))
                                    .line_height(px(18.))
                                    .text_color(p.muted)
                                    .child(
                                        div()
                                            .w(px(8.))
                                            .flex_none()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.faint)
                                            .child("–"),
                                    )
                                    .child(div().flex_1().min_w_0().child(con.clone())),
                            );
                        }
                        let lines_el = lines.into_any_element();
                        body_col = body_col.child(pro_fold(
                            pro_eid(&self.id, format!("opt-fold-{iid}-{oid}")),
                            checked,
                            est_h,
                            0.,
                            move || lines_el,
                        ));
                    }

                    opts_card = opts_card.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("opt-{iid}-{oid}")))
                            .role(Role::RadioButton)
                            .aria_label(opt.title.clone())
                            .tab_index(0)
                            .min_h(px(56.))
                            .px(px(12.))
                            .py(px(10.))
                            .rounded(px(6.))
                            .bg(if checked { p.chip } else { transparent_black() })
                            .hover(move |s| s.bg(if checked { p.chip } else { p.hover }))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_start()
                            .gap(px(12.))
                            .cursor_pointer()
                            .child(num_badge)
                            .child(body_col),
                        cx,
                        move |this, _, cx| {
                            this.set_chosen_option(iid.clone(), oid.clone());
                            cx.notify();
                        },
                    ));
                }
                opts_card.into_any_element()
            }
            InboxItemKind::Pick => {
                let pick_sensor = pro_selection_sensor();
                let pick_key = chosen_opt.clone().unwrap_or_else(|| "none".into());
                let mut grid = div()
                    .relative()
                    .flex()
                    .gap(px(if narrow { 4. } else { 8. }))
                    .child(pro_pill_indicator(
                        pro_eid(&self.id, format!("pick-ind-{}", item.id)),
                        pick_key.clone(),
                        pick_sensor.clone(),
                        10.,
                        p.chip,
                        None,
                        false,
                    ));
                let mut selected_opt_ref: Option<&InboxOption> = None;

                for (oi, opt) in item.options.iter().enumerate() {
                    let checked = chosen_opt.as_ref() == Some(&opt.id);
                    if checked {
                        selected_opt_ref = Some(opt);
                    }
                    let iid = item.id.clone();
                    let oid = opt.id.clone();
                    let wf = opt.wireframe.unwrap_or_default();

                    grid = grid.child(
                        div().flex_1().min_w_0().child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("var-{iid}-{oid}")))
                                .role(Role::RadioButton)
                                .aria_label(opt.title.clone())
                                .tab_index(0)
                                .relative()
                                .p(px(6.))
                                .pb(px(8.))
                                .rounded(px(10.))
                                .when(!checked, |d| d.hover(move |s| s.bg(p.hover)))
                                .when(checked, |d| d.child(pro_mark_selected(&pick_sensor)))
                                .focus_visible(move |s| s.border_1().border_color(p.focus))
                                .flex()
                                .flex_col()
                                .gap(px(8.))
                                .cursor_pointer()
                                .child(Self::render_wireframe_thumb(wf, 84., &p))
                                .child(
                                    div()
                                        .px(px(2.))
                                        .flex()
                                        .items_center()
                                        .gap(px(7.))
                                        .min_w_0()
                                        .when(!narrow, |d| {
                                            d.child(
                                                div()
                                                    .size(px(18.))
                                                    .flex_none()
                                                    .rounded(px(5.))
                                                    .when(checked, |b| {
                                                        b.bg(p.primary).text_color(p.on_primary)
                                                    })
                                                    .when(!checked, |b| {
                                                        b.border_1()
                                                            .border_color(p.line_strong)
                                                            .text_color(p.muted)
                                                    })
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .text_size(px(11.))
                                                    .child((oi + 1).to_string()),
                                            )
                                        })
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .truncate()
                                                .child(opt.title.clone()),
                                        ),
                                ),
                            cx,
                            move |this, _, cx| {
                                this.set_chosen_option(iid.clone(), oid.clone());
                                cx.notify();
                            },
                        )),
                    );
                }

                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(grid)
                    .when_some(selected_opt_ref, |d, sel| {
                        let summary_card = div()
                            .px(px(12.))
                            .py(px(9.))
                            .rounded(px(8.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .gap(px(3.))
                            .when_some(sel.summary.as_ref(), |b, s| {
                                b.child(
                                    div()
                                        .text_size(px(12.5))
                                        .text_color(p.text)
                                        .child(s.clone()),
                                )
                            })
                            .children(sel.pros.iter().map(|pro| {
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(format!("+ {pro}"))
                            }))
                            .children(sel.cons.iter().map(|con| {
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(format!("– {con}"))
                            }));
                        d.child(pro_view_transition(
                            pro_eid(&self.id, format!("pick-sum-{}", item.id)),
                            pick_key,
                            summary_card,
                        ))
                    })
                    .into_any_element()
            }
            InboxItemKind::Approval => div()
                .px(px(16.))
                .py(px(4.))
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .children(item.facts.iter().enumerate().map(|(fi, fact)| {
                    div()
                        .min_h(px(40.))
                        .py(px(8.))
                        .when(fi > 0, |d| d.border_t_1().border_color(p.line))
                        .flex()
                        .items_start()
                        .gap(px(12.))
                        .child(
                            div()
                                .w(px(if narrow { 60. } else { 76. }))
                                .flex_none()
                                .text_size(px(13.))
                                .line_height(px(20.))
                                .text_color(p.muted)
                                .child(fact.label.clone()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(13.))
                                .line_height(px(20.))
                                .text_color(p.ink)
                                .child(fact.value.clone()),
                        )
                }))
                .into_any_element(),
            InboxItemKind::Question => {
                let current_val = self
                    .answer_editor
                    .as_ref()
                    .map(|e| e.read(cx).value().to_owned())
                    .unwrap_or_default();
                let answer_ed = self.answer_editor.clone();

                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .when(!item.suggestions.is_empty(), |d| {
                        d.child(div().flex().flex_wrap().gap(px(6.)).children(
                            item.suggestions.iter().enumerate().map(|(si, sug)| {
                                let active_chip = current_val.trim() == sug.as_ref();
                                let sug_txt = sug.to_string();
                                activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("sug-{si}")))
                                        .role(Role::Button)
                                        .tab_index(0)
                                        .h(px(28.))
                                        .px(px(10.))
                                        .rounded(px(6.))
                                        .bg(p.chip)
                                        .when(active_chip, |b| {
                                            b.border_1()
                                                .border_color(p.line_strong)
                                                .text_color(p.ink)
                                        })
                                        .when(!active_chip, |b| b.text_color(p.text))
                                        .hover(move |s| s.bg(p.chip_hover).text_color(p.ink))
                                        .flex()
                                        .items_center()
                                        .text_size(px(13.))
                                        .cursor_pointer()
                                        .child(sug.clone()),
                                    cx,
                                    move |this, _, cx| {
                                        this.pending_set_answer = Some(sug_txt.clone());
                                        cx.notify();
                                    },
                                )
                            }),
                        ))
                    })
                    .child(
                        div()
                            .h(px(88.))
                            .p(px(10.))
                            .px(px(12.))
                            .rounded(px(8.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .hover(move |s| s.border_color(p.line_strong))
                            .when_some(answer_ed, |d, ed| d.child(ed)),
                    )
                    .into_any_element()
            }
        };

        // Footer bar (.ui-ib-foot)
        let reply_str = self
            .reply_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_owned())
            .unwrap_or_default();
        let area_str = self
            .answer_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_owned())
            .unwrap_or_default();
        let reply_ed = self.reply_editor.clone();

        let footer = if item.kind == InboxItemKind::Question {
            let can_send = !area_str.is_empty();
            div()
                .mt(px(16.))
                .flex()
                .items_center()
                .justify_end()
                .gap(px(8.))
                .child(pro_btn(
                    pro_eid(&self.id, "btn-later"),
                    tr(cx, "Later", "稍后"),
                    None,
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    |this, _, cx| this.snooze_current_later(cx),
                ))
                .child(div().when(!can_send, |d| d.opacity(0.42)).child(pro_btn(
                    pro_eid(&self.id, "btn-send-q"),
                    tr(cx, "Send", "发送"),
                    None,
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    |this, _, cx| this.submit_current(None, cx),
                )))
        } else {
            let is_approval = item.kind == InboxItemKind::Approval;
            let primary_label = if is_approval && reply_str.is_empty() {
                tr(cx, "Approve", "批准")
            } else {
                tr(cx, "Send", "发送")
            };
            div()
                .mt(px(16.))
                .flex()
                .when(narrow, |d| d.flex_col().items_stretch())
                .when(!narrow, |d| d.items_center())
                .gap(px(8.))
                .child(
                    div()
                        .h(px(32.))
                        .px(px(12.))
                        .when(!narrow, |d| d.flex_1().min_w_0())
                        .rounded(px(8.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .hover(move |s| s.border_color(p.line_strong))
                        .flex()
                        .items_center()
                        .when_some(reply_ed, |d, ed| {
                            d.child(div().flex_1().min_w_0().child(ed))
                        }),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(px(8.))
                        .child(pro_btn(
                            pro_eid(&self.id, "btn-later"),
                            tr(cx, "Later", "稍后"),
                            None,
                            ProBtnStyle::Ghost,
                            true,
                            cx,
                            |this, _, cx| this.snooze_current_later(cx),
                        ))
                        .when(is_approval && reply_str.is_empty(), |d| {
                            d.child(pro_btn(
                                pro_eid(&self.id, "btn-decline"),
                                tr(cx, "Decline", "拒绝"),
                                None,
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                |this, _, cx| this.submit_current(Some(false), cx),
                            ))
                        })
                        .child(pro_btn(
                            pro_eid(&self.id, "btn-send"),
                            primary_label,
                            None,
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            move |this, _, cx| {
                                if is_approval && reply_str.is_empty() {
                                    this.submit_current(Some(true), cx);
                                } else {
                                    this.submit_current(None, cx);
                                }
                            },
                        )),
                )
        };

        div()
            .w_full()
            .max_w(px(580.))
            .mx_auto()
            .pt(px(if narrow { 18. } else { 68. }))
            .pb(px(64.))
            .px(px(if narrow { 20. } else { 36. }))
            .flex()
            .flex_col()
            .when_some(sent_toast, |d, t| d.child(t))
            .child(meta_row)
            .child(
                div()
                    .mt(px(10.))
                    .text_size(px(if narrow { 20. } else { 24. }))
                    .line_height(px(if narrow { 28. } else { 32. }))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(item.question.clone()),
            )
            .when_some(item.context.as_ref(), |d, ctx_txt| {
                d.child(
                    div()
                        .mt(px(12.))
                        .max_h(px(66.))
                        .overflow_hidden()
                        .text_size(px(14.))
                        .line_height(px(22.))
                        .text_color(p.text)
                        .child(ctx_txt.clone()),
                )
            })
            .child(div().mt(px(28.)).child(answer_section))
            .child(footer)
    }

    fn render_done_detail(
        &self,
        item: &InboxItem,
        done: &InboxDoneState,
        narrow: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let declined = matches!(done.answer, InboxAnswer::Decline);
        let reopen_id = item.id.clone();

        div()
            .w_full()
            .max_w(px(580.))
            .mx_auto()
            .pt(px(if narrow { 18. } else { 68. }))
            .pb(px(64.))
            .px(px(if narrow { 20. } else { 36. }))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(Self::render_kind_glyph(item.kind, Some(done), 14., &p))
                    .child(item.task.clone())
                    .child(div().text_color(p.faint).child("·"))
                    .child(item.at_long.clone()),
            )
            .child(
                div()
                    .mt(px(10.))
                    .text_size(px(if narrow { 20. } else { 24. }))
                    .line_height(px(if narrow { 28. } else { 32. }))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(item.question.clone()),
            )
            .when_some(item.context.as_ref(), |d, ctx_txt| {
                d.child(
                    div()
                        .mt(px(10.))
                        .text_size(px(14.))
                        .line_height(px(22.))
                        .text_color(p.text)
                        .child(ctx_txt.clone()),
                )
            })
            .child(
                div()
                    .mt(px(24.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .px(px(14.))
                    .py(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(if declined {
                        pro_svg("0 0 24 24", SVG_DECLINED, 16., p.muted)
                    } else {
                        pro_svg("0 0 24 24", SVG_APPROVAL, 16., p.green)
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .truncate()
                            .child(done.label.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(done.at_short.clone()),
                    )
                    .child(pro_btn(
                        pro_eid(&self.id, "btn-reopen"),
                        tr(cx, "Reopen", "重新打开"),
                        Some(ProIcon::Undo),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.reopen_by_id(reopen_id.clone(), cx);
                        },
                    )),
            )
            .when(
                !done.since_steps.is_empty() || !done.since_files.is_empty(),
                |d| {
                    d.child(
                        div()
                            .mt(px(20.))
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "Since then", "后续进展")),
                            )
                            .children(done.since_steps.iter().map(|st| {
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .text_size(px(13.))
                                    .line_height(px(20.))
                                    .text_color(p.text)
                                    .child(div().size(px(4.)).rounded_full().bg(p.faint))
                                    .child(st.clone())
                            }))
                            .when(!done.since_files.is_empty(), |b| {
                                b.child(div().mt(px(4.)).flex().flex_wrap().gap(px(6.)).children(
                                    done.since_files.iter().map(|fname| {
                                        let target = fname.clone();
                                        activate(
                                            div()
                                                .id(pro_eid(&self.id, format!("done-file-{fname}")))
                                                .role(Role::Button)
                                                .tab_index(0)
                                                .h(px(26.))
                                                .pl(px(7.))
                                                .pr(px(9.))
                                                .rounded(px(6.))
                                                .bg(p.chip)
                                                .hover(move |s| s.bg(p.chip_hover))
                                                .flex()
                                                .items_center()
                                                .gap(px(6.))
                                                .text_size(px(12.))
                                                .text_color(p.ink)
                                                .cursor_pointer()
                                                .child(Self::render_file_glyph(fname, 12., &p))
                                                .child(fname.clone()),
                                            cx,
                                            move |this, _, cx| {
                                                this.open_artifact_tab(target.clone(), cx);
                                            },
                                        )
                                    }),
                                ))
                            }),
                    )
                },
            )
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let phase = if self.loading && !reduced {
            w.request_animation_frame();
            (self.started_at.elapsed().as_secs_f32() * 0.9) % 1.0
        } else {
            0.35
        };

        let width = self.measured_width.get();
        let narrow = width < 620.;
        let measure = self.measured_width.clone();

        let content = if narrow {
            if self.mobile_reading_open && !self.empty {
                self.render_reading_column(true, phase, cx)
            } else {
                self.render_list_column(true, phase, cx)
            }
        } else {
            div()
                .flex_1()
                .min_w_0()
                .h_full()
                .min_h_0()
                .flex()
                .child(self.render_list_column(false, phase, cx))
                .child(self.render_reading_column(false, phase, cx))
        };

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
            .child(
                div()
                    .id(pro_eid(&self.id, "live-status"))
                    .role(Role::Status)
                    .aria_label(self.status_announcement.clone())
                    .absolute()
                    .size_0()
                    .overflow_hidden(),
            )
            .child(content)
            .child(pro_toast_banner(self.toast.clone()))
    }

    fn render_inspector(
        &mut self,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SharedString, ProIcon, AnyElement)> {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let phase = if self.loading && !Theme::global(cx).reduced_motion {
            (self.started_at.elapsed().as_secs_f32() * 0.9) % 1.0
        } else {
            0.35
        };

        if self.loading {
            let skel = div()
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(pro_skeleton_bar(220., 12., 4., phase, cx))
                .child(pro_skeleton_bar(180., 12., 4., phase, cx))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(160., 12., 4., phase, cx))
                        .child(pro_skeleton_bar(140., 12., 4., phase, cx))
                        .child(pro_skeleton_bar(150., 12., 4., phase, cx)),
                )
                .child(
                    div()
                        .p(px(10.))
                        .rounded(px(10.))
                        .bg(p.chip)
                        .flex()
                        .flex_col()
                        .gap(px(9.))
                        .child(pro_skeleton_bar(170., 12., 4., phase, cx))
                        .child(pro_skeleton_bar(185., 12., 4., phase, cx))
                        .child(pro_skeleton_bar(155., 12., 4., phase, cx)),
                )
                .into_any_element();
            return Some((tr(cx, "Task", "任务").into(), ProIcon::Tasks, skel));
        }

        if self.empty {
            let empty_panel = div()
                .p(px(24.))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .text_color(p.muted)
                .child(tr(cx, "No task is open", "当前没有打开的任务"))
                .into_any_element();
            return Some((tr(cx, "Task", "任务").into(), ProIcon::Tasks, empty_panel));
        }

        let task_name = self
            .current_task()
            .map(|t| t.name.clone())
            .unwrap_or_else(|| tr(cx, "Task", "任务").into());

        let active_tab = self
            .open_panel_tabs
            .get(self.active_panel_tab)
            .cloned()
            .unwrap_or(PanelTab::Task);

        let (head_title, head_icon) = match &active_tab {
            PanelTab::Task => (task_name.clone(), ProIcon::Tasks),
            PanelTab::Artifact(name) => {
                let ic = if !name.contains('.') {
                    ProIcon::Folder
                } else if name.ends_with(".csv") {
                    ProIcon::Filter
                } else if name.ends_with(".json") {
                    ProIcon::Code
                } else {
                    ProIcon::PageDoc
                };
                (name.clone(), ic)
            }
            PanelTab::NewTab => (tr(cx, "New tab", "新标签页").into(), ProIcon::Plus),
        };

        // Compact multi-tab strip matching .ui-asw-ptabs (Ingest pipeline, ingest-design.md, load-test-results.csv, +)
        let ptab_sensor = pro_selection_sensor();
        let selected_ptab_key = format!("ptab-{}", self.active_panel_tab);
        let mut tab_strip = div()
            .relative()
            .px(px(10.))
            .py(px(6.))
            .border_b_1()
            .border_color(p.line_soft)
            .flex()
            .items_center()
            .gap(px(3.))
            .overflow_hidden()
            .child(pro_pill_indicator(
                pro_eid(&self.id, "panel-tabs"),
                selected_ptab_key,
                ptab_sensor.clone(),
                6.,
                p.chip,
                None,
                false,
            ));

        for (ti, ptab) in self.open_panel_tabs.iter().enumerate() {
            let selected = ti == self.active_panel_tab;
            let (lbl, ic): (SharedString, ProIcon) = match ptab {
                PanelTab::Task => (task_name.clone(), ProIcon::Tasks),
                PanelTab::Artifact(n) => {
                    let icon = if !n.contains('.') {
                        ProIcon::Folder
                    } else if n.ends_with(".csv") {
                        ProIcon::Filter
                    } else if n.ends_with(".json") {
                        ProIcon::Code
                    } else {
                        ProIcon::PageDoc
                    };
                    (n.clone(), icon)
                }
                PanelTab::NewTab => (tr(cx, "New tab", "新标签页").into(), ProIcon::Globe),
            };
            tab_strip = tab_strip.child(
                div()
                    .relative()
                    .h(px(26.))
                    .pl(px(7.))
                    .pr(px(4.))
                    .rounded(px(6.))
                    .when(!selected, |d| d.hover(move |s| s.bg(p.hover)))
                    .when(selected, |d| d.child(pro_mark_selected(&ptab_sensor)))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .min_w_0()
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("ptab-{ti}")))
                            .role(Role::Tab)
                            .tab_index(0)
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .min_w_0()
                            .cursor_pointer()
                            .child(pro_icon(ic, 12.5, if selected { p.ink } else { p.muted }))
                            .child(
                                div()
                                    .max_w(px(78.))
                                    .truncate()
                                    .text_size(px(12.))
                                    .font_weight(if selected {
                                        FontWeight::MEDIUM
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .text_color(if selected { p.ink } else { p.muted })
                                    .child(lbl),
                            ),
                        cx,
                        move |this, _, cx| {
                            this.active_panel_tab = ti;
                            cx.notify();
                        },
                    ))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("ptab-x-{ti}")))
                            .role(Role::Button)
                            .aria_label("Close tab")
                            .tab_index(0)
                            .size(px(15.))
                            .rounded(px(4.))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .child(pro_icon(ProIcon::Close, 9.5, p.faint)),
                        cx,
                        move |this, _, cx| {
                            this.close_panel_tab(ti, cx);
                        },
                    )),
            );
        }

        tab_strip = tab_strip.child(pro_icon_btn(
            pro_eid(&self.id, "ptab-new"),
            tr(cx, "New tab", "新标签页"),
            ProIcon::Plus,
            24.,
            false,
            cx,
            |this, _, cx| {
                this.open_panel_tabs.push(PanelTab::NewTab);
                this.active_panel_tab = this.open_panel_tabs.len() - 1;
                cx.notify();
            },
        ));

        let pane_key = format!("{active_tab:?}-{task_name}");
        let raw_pane_body: AnyElement = match active_tab {
            PanelTab::Task => self.render_task_pane(cx).into_any_element(),
            PanelTab::Artifact(ref art_name) => self
                .render_artifact_pane(art_name.clone(), cx)
                .into_any_element(),
            PanelTab::NewTab => self.render_suggested_pane(cx).into_any_element(),
        };
        let pane_body =
            pro_view_transition(pro_eid(&self.id, "panel-pane-vtx"), pane_key, raw_pane_body);

        let full_panel = div()
            .flex()
            .flex_col()
            .child(tab_strip)
            .child(div().p(px(14.)).child(pane_body))
            .into_any_element();

        Some((head_title, head_icon, full_panel))
    }

    fn render_task_pane(&self, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let Some(task) = self.current_task().cloned() else {
            return div().text_size(px(13.)).text_color(p.muted).child(tr(
                cx,
                "No task selected",
                "未选择任务",
            ));
        };

        let wait_bg = if p.dark {
            hsla(25. / 360., 0.98, 0.49, 0.15)
        } else {
            hsla(37. / 360., 1.0, 0.50, 0.16)
        };
        let wait_fg = p.amber_ink;
        let tray_bg = if p.dark {
            hsla(0., 0., 1., 0.03)
        } else {
            hsla(0., 0., 0., 0.025)
        };

        let (done_steps, total_steps) = task.progress.unwrap_or_else(|| {
            let d = task
                .steps
                .iter()
                .filter(|s| s.state == InboxStepState::Done)
                .count() as u32;
            (d, task.steps.len().max(1) as u32)
        });
        let ring_pct = done_steps as f32 / total_steps.max(1) as f32;

        let decided_for_task: Vec<(SharedString, SharedString, SharedString)> = self
            .items
            .iter()
            .filter(|it| it.task == task.name)
            .filter_map(|it| {
                it.done
                    .as_ref()
                    .map(|d| (it.id.clone(), it.question.clone(), d.label.clone()))
            })
            .collect();

        let step_count = task.steps.len();

        div()
            .flex()
            .flex_col()
            .gap(px(18.))
            .child(
                div()
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.text)
                    .child(task.goal),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(
                        div()
                            .min_h(px(26.))
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
                                    .text_size(px(13.))
                                    .text_color(p.ink)
                                    .child(pro_progress_ring(
                                        ring_pct,
                                        14.,
                                        1.6,
                                        p.muted,
                                        p.line_strong,
                                    ))
                                    .child(div().truncate().child(task.status))
                                    .child(
                                        div()
                                            .flex_none()
                                            .text_color(p.faint)
                                            .child(format!("{done_steps} / {total_steps}")),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .min_h(px(26.))
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
                                    .child(task.project),
                            ),
                    )
                    .child(
                        div()
                            .min_h(px(26.))
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .w(px(76.))
                                    .flex_none()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(tr(cx, "Started", "开始时间")),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(13.))
                                    .text_color(p.ink)
                                    .truncate()
                                    .child(task.started),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Steps", "执行步骤")),
                    )
                    .child(
                        div()
                            .p(px(8.))
                            .px(px(10.))
                            .rounded(px(10.))
                            .bg(tray_bg)
                            .flex()
                            .flex_col()
                            .children(task.steps.into_iter().enumerate().map(|(si, step)| {
                                let has_next = si + 1 < step_count;
                                let mark = match step.state {
                                    InboxStepState::Done => div()
                                        .size(px(15.))
                                        .rounded_full()
                                        .bg(p.blue)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(pro_svg("0 0 24 24", SVG_STEP_CHECK, 10., p.panel)),
                                    InboxStepState::WaitingOnYou => div()
                                        .size(px(15.))
                                        .rounded_full()
                                        .bg(p.orange.opacity(0.18))
                                        .border_1()
                                        .border_color(p.orange),
                                    InboxStepState::RunningNow => div()
                                        .size(px(15.))
                                        .rounded_full()
                                        .bg(p.blue.opacity(0.18))
                                        .border_1()
                                        .border_color(p.blue),
                                    InboxStepState::Later => div()
                                        .size(px(15.))
                                        .rounded_full()
                                        .border_1()
                                        .border_color(p.line_strong),
                                };
                                let label_c = match step.state {
                                    InboxStepState::WaitingOnYou | InboxStepState::RunningNow => {
                                        p.ink
                                    }
                                    InboxStepState::Done | InboxStepState::Later => p.muted,
                                };
                                div()
                                    .relative()
                                    .min_h(px(28.))
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .when(has_next, |d| {
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
                                            .text_size(px(13.))
                                            .text_color(label_c)
                                            .truncate()
                                            .child(step.label),
                                    )
                                    .when(step.state == InboxStepState::WaitingOnYou, |d| {
                                        d.child(
                                            div()
                                                .flex_none()
                                                .h(px(18.))
                                                .px(px(6.))
                                                .rounded(px(4.))
                                                .bg(wait_bg)
                                                .text_size(px(11.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(wait_fg)
                                                .flex()
                                                .items_center()
                                                .child(tr(cx, "Needs you", "待你决定")),
                                        )
                                    })
                            })),
                    ),
            )
            .when(!task.artifacts.is_empty(), |d| {
                let art_len = task.artifacts.len();
                d.child(
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
                                .child(tr(cx, "Artifacts", "产物文件"))
                                .child(div().text_color(p.faint).child(art_len.to_string())),
                        )
                        .child(div().flex().flex_wrap().gap(px(6.)).children(
                            task.artifacts.into_iter().map(|fname| {
                                let target = fname.clone();
                                activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("task-art-{fname}")))
                                        .role(Role::Button)
                                        .tab_index(0)
                                        .h(px(26.))
                                        .pl(px(7.))
                                        .pr(px(9.))
                                        .rounded(px(6.))
                                        .bg(p.chip)
                                        .hover(move |s| s.bg(p.chip_hover))
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .text_size(px(12.))
                                        .text_color(p.ink)
                                        .cursor_pointer()
                                        .child(Self::render_file_glyph(&fname, 12., &p))
                                        .child(fname),
                                    cx,
                                    move |this, _, cx| {
                                        this.open_artifact_tab(target.clone(), cx);
                                    },
                                )
                            }),
                        )),
                )
            })
            .when(!decided_for_task.is_empty(), |d| {
                d.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .child(tr(cx, "Decided", "已决定")),
                        )
                        .child(
                            div()
                                .p(px(3.))
                                .rounded(px(10.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line)
                                .flex()
                                .flex_col()
                                .children(decided_for_task.into_iter().map(|(did, dq, da)| {
                                    activate(
                                        div()
                                            .id(pro_eid(&self.id, format!("decided-{did}")))
                                            .role(Role::Button)
                                            .tab_index(0)
                                            .p(px(8.))
                                            .rounded(px(7.))
                                            .hover(move |s| s.bg(p.hover))
                                            .flex()
                                            .items_center()
                                            .gap(px(9.))
                                            .cursor_pointer()
                                            .child(pro_svg("0 0 24 24", SVG_APPROVAL, 14., p.green))
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .flex()
                                                    .flex_col()
                                                    .child(
                                                        div()
                                                            .text_size(px(12.5))
                                                            .text_color(p.ink)
                                                            .truncate()
                                                            .child(dq),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.5))
                                                            .text_color(p.muted)
                                                            .truncate()
                                                            .child(da),
                                                    ),
                                            ),
                                        cx,
                                        move |this, _, cx| {
                                            this.tab = InboxTab::Done;
                                            this.select_item(did.clone(), true, cx);
                                        },
                                    )
                                })),
                        ),
                )
            })
    }

    fn render_artifact_pane(&self, art_name: SharedString, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let Some(art_idx) = self.artifacts.iter().position(|a| a.name == art_name) else {
            return div().text_size(px(13.)).text_color(p.muted).child(tr(
                cx,
                "Artifact not found",
                "未找到该产物",
            ));
        };
        let art = self.artifacts[art_idx].clone();

        let head = div()
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
                    .child(Self::render_file_glyph(&art.name, 14., &p)),
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
                            .child(art.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(p.muted)
                            .child(art.updated.clone()),
                    ),
            );

        let content: AnyElement = match art.body {
            InboxArtifactBody::Document {
                title,
                paragraphs,
                checklist,
            } => {
                let bold_on = self.doc_bold;
                let italic_on = self.doc_italic;
                let code_on = self.doc_code;
                let toolbar = div()
                    .pb(px(8.))
                    .border_b_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(3.))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "doc-tb-b"))
                            .role(Role::Button)
                            .tab_index(0)
                            .size(px(26.))
                            .rounded(px(5.))
                            .bg(if bold_on { p.chip } else { transparent_black() })
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(12.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(if bold_on { p.ink } else { p.muted })
                            .cursor_pointer()
                            .child("B"),
                        cx,
                        |this, _, cx| {
                            this.doc_bold = !this.doc_bold;
                            cx.notify();
                        },
                    ))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "doc-tb-i"))
                            .role(Role::Button)
                            .tab_index(0)
                            .size(px(26.))
                            .rounded(px(5.))
                            .bg(if italic_on {
                                p.chip
                            } else {
                                transparent_black()
                            })
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(12.))
                            .text_color(if italic_on { p.ink } else { p.muted })
                            .cursor_pointer()
                            .child("I"),
                        cx,
                        |this, _, cx| {
                            this.doc_italic = !this.doc_italic;
                            cx.notify();
                        },
                    ))
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "doc-tb-code"),
                        "Inline code",
                        ProIcon::Code,
                        26.,
                        code_on,
                        cx,
                        |this, _, cx| {
                            this.doc_code = !this.doc_code;
                            cx.notify();
                        },
                    ));

                let word_count: usize = paragraphs
                    .iter()
                    .map(|s| s.split_whitespace().count())
                    .sum::<usize>()
                    + checklist
                        .iter()
                        .map(|(s, _)| s.split_whitespace().count())
                        .sum::<usize>();

                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(toolbar)
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(title),
                    )
                    .children(paragraphs.into_iter().map(|para| {
                        div()
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .when(bold_on, |d| d.font_weight(FontWeight::SEMIBOLD))
                            .when(code_on, |d| d.font_family("Fira Code"))
                            .text_color(p.text)
                            .child(para)
                    }))
                    .children(checklist.into_iter().enumerate().map(|(ci, (line, done))| {
                        let aname = art_name.clone();
                        activate(
                            div()
                                .id(pro_eid(&self.id, format!("chk-{art_idx}-{ci}")))
                                .role(Role::CheckBox)
                                .tab_index(0)
                                .py(px(2.))
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .cursor_pointer()
                                .child(
                                    div()
                                        .size(px(14.))
                                        .flex_none()
                                        .rounded(px(4.))
                                        .when(done, |d| d.bg(p.ink))
                                        .when(!done, |d| {
                                            d.bg(p.card).border_1().border_color(p.line_strong)
                                        })
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .when(done, |d| {
                                            d.child(pro_icon(ProIcon::Check, 10., p.card))
                                        }),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(19.))
                                        .text_color(if done { p.muted } else { p.ink })
                                        .child(line),
                                ),
                            cx,
                            move |this, _, cx| {
                                if let Some(art_mut) = this.artifacts.get_mut(art_idx)
                                    && let InboxArtifactBody::Document {
                                        ref mut checklist, ..
                                    } = art_mut.body
                                    && let Some(entry) = checklist.get_mut(ci)
                                {
                                    entry.1 = !entry.1;
                                    let snapshot = art_mut.clone();
                                    cx.emit(AgentInboxEvent::ArtifactChanged {
                                        name: aname.clone(),
                                        artifact: snapshot,
                                    });
                                    cx.notify();
                                }
                            },
                        )
                    }))
                    .child(
                        div()
                            .pt(px(8.))
                            .border_t_1()
                            .border_color(p.line)
                            .text_size(px(11.5))
                            .text_color(p.faint)
                            .child(format!("{word_count} words")),
                    )
                    .into_any_element()
            }
            InboxArtifactBody::Sheet { headers, rows } => {
                let (sel_r, sel_c) = self.active_sheet_cell;
                let col_letter = (b'A' + (sel_c.min(25) as u8)) as char;
                let cell_ref = format!("{col_letter}{}", sel_r + 1);
                let active_val = rows
                    .get(sel_r)
                    .and_then(|r| r.get(sel_c))
                    .cloned()
                    .unwrap_or_default();

                let mut numeric_vals = Vec::new();
                for row in &rows {
                    for cell in row {
                        let cleaned = cell
                            .trim_end_matches('%')
                            .trim_end_matches("ms")
                            .replace(',', "");
                        if let Ok(v) = cleaned.trim().parse::<f64>() {
                            numeric_vals.push(v);
                        }
                    }
                }
                let count_n = numeric_vals.len();
                let sum_v: f64 = numeric_vals.iter().sum();

                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .h(px(28.))
                            .rounded(px(6.))
                            .border_1()
                            .border_color(p.line_strong)
                            .flex()
                            .items_center()
                            .text_size(px(12.))
                            .child(
                                div()
                                    .w(px(46.))
                                    .h_full()
                                    .border_r_1()
                                    .border_color(p.line_strong)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_color(p.ink)
                                    .child(cell_ref),
                            )
                            .child(
                                div()
                                    .w(px(24.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_color(p.faint)
                                    .child("fx"),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .px(px(6.))
                                    .text_color(p.ink)
                                    .truncate()
                                    .child(active_val),
                            ),
                    )
                    .child(
                        div()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(p.line)
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .h(px(24.))
                                    .bg(p.chip)
                                    .px(px(6.))
                                    .flex()
                                    .items_center()
                                    .children(headers.iter().map(|h| {
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .px(px(4.))
                                            .text_size(px(11.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(h.clone())
                                    })),
                            )
                            .children(rows.into_iter().enumerate().map(|(ri, row)| {
                                div()
                                    .h(px(26.))
                                    .px(px(6.))
                                    .border_t_1()
                                    .border_color(p.line_soft)
                                    .flex()
                                    .items_center()
                                    .children(row.into_iter().enumerate().map(|(ci, cell)| {
                                        let is_active = ri == sel_r && ci == sel_c;
                                        activate(
                                            div()
                                                .id(pro_eid(&self.id, format!("cell-{ri}-{ci}")))
                                                .role(Role::Cell)
                                                .tab_index(0)
                                                .flex_1()
                                                .min_w_0()
                                                .h_full()
                                                .px(px(4.))
                                                .when(is_active, |d| {
                                                    d.bg(p.accent_soft)
                                                        .border_1()
                                                        .border_color(p.blue)
                                                })
                                                .flex()
                                                .items_center()
                                                .text_size(px(11.5))
                                                .text_color(if ci == 0 { p.ink } else { p.text })
                                                .truncate()
                                                .cursor_pointer()
                                                .child(cell),
                                            cx,
                                            move |this, _, cx| {
                                                this.active_sheet_cell = (ri, ci);
                                                cx.notify();
                                            },
                                        )
                                    }))
                            })),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(12.))
                            .text_size(px(11.5))
                            .text_color(p.muted)
                            .child(format!("Count {count_n}"))
                            .child(format!("Sum {sum_v:.0}")),
                    )
                    .into_any_element()
            }
            InboxArtifactBody::Code { language, source } => div()
                .rounded(px(10.))
                .bg(p.chip)
                .border_1()
                .border_color(p.line_soft)
                .overflow_hidden()
                .flex()
                .flex_col()
                .child(
                    div()
                        .h(px(24.))
                        .px(px(10.))
                        .border_b_1()
                        .border_color(p.line_soft)
                        .flex()
                        .items_center()
                        .text_size(px(10.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(language.to_uppercase()),
                )
                .child(
                    div()
                        .p(px(12.))
                        .font_family("Fira Code")
                        .text_size(px(11.5))
                        .line_height(px(18.))
                        .text_color(p.ink)
                        .child(source),
                )
                .into_any_element(),
            InboxArtifactBody::Folder { mut entries } => {
                let query = self
                    .folder_search_editor
                    .as_ref()
                    .map(|e| e.read(cx).value().to_ascii_lowercase())
                    .unwrap_or_default();
                if self.folder_sort_asc {
                    entries.sort_by(|a, b| a.name.cmp(&b.name));
                } else {
                    entries.sort_by(|a, b| b.name.cmp(&a.name));
                }
                let filtered: Vec<InboxFolderEntry> = entries
                    .into_iter()
                    .filter(|e| query.is_empty() || e.name.to_ascii_lowercase().contains(&query))
                    .collect();
                let search_ed = self.folder_search_editor.clone();
                let ql = self.quick_look_entry.clone();

                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .h(px(28.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .border_1()
                            .border_color(p.line_strong)
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(pro_icon(ProIcon::Search, 13., p.faint))
                            .when_some(search_ed, |d, ed| {
                                d.child(div().flex_1().min_w_0().child(ed))
                            }),
                    )
                    .child(
                        div()
                            .h(px(24.))
                            .px(px(6.))
                            .border_b_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, "folder-sort"))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .cursor_pointer()
                                    .child(tr(cx, "Name", "名称"))
                                    .child(pro_icon(
                                        if self.folder_sort_asc {
                                            ProIcon::ChevronUp
                                        } else {
                                            ProIcon::ChevronDown
                                        },
                                        11.,
                                        p.faint,
                                    )),
                                cx,
                                |this, _, cx| {
                                    this.folder_sort_asc = !this.folder_sort_asc;
                                    cx.notify();
                                },
                            ))
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .child(tr(cx, "Size", "大小")),
                            ),
                    )
                    .children(filtered.into_iter().map(|ent| {
                        let ename = ent.name.clone();
                        let is_ql = ql.as_ref() == Some(&ename);
                        activate(
                            div()
                                .id(pro_eid(&self.id, format!("fd-ent-{ename}")))
                                .role(Role::ListBoxOption)
                                .tab_index(0)
                                .h(px(30.))
                                .px(px(6.))
                                .rounded(px(6.))
                                .bg(if is_ql {
                                    p.accent_soft
                                } else {
                                    transparent_black()
                                })
                                .hover(move |s| s.bg(p.hover))
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(8.))
                                .cursor_pointer()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(7.))
                                        .min_w_0()
                                        .child(if ent.is_folder {
                                            pro_svg("0 0 12 12", SVG_FILE_FOLDER, 13., p.amber)
                                        } else {
                                            Self::render_file_glyph(&ent.name, 13., &p)
                                        })
                                        .child(
                                            div()
                                                .text_size(px(12.5))
                                                .text_color(p.ink)
                                                .truncate()
                                                .child(ent.name),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .text_size(px(11.5))
                                        .text_color(p.muted)
                                        .child(div().text_color(p.faint).child(ent.modified))
                                        .child(ent.size),
                                ),
                            cx,
                            move |this, _, cx| {
                                if this.quick_look_entry.as_ref() == Some(&ename) {
                                    this.quick_look_entry = None;
                                } else {
                                    this.quick_look_entry = Some(ename.clone());
                                }
                                cx.notify();
                            },
                        )
                    }))
                    .when_some(ql, |d, qname| {
                        d.child(
                            div()
                                .mt(px(4.))
                                .p(px(10.))
                                .rounded(px(8.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line_strong)
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(p.text)
                                        .truncate()
                                        .child(format!("Quick Look · {qname}")),
                                )
                                .child(pro_icon_btn(
                                    pro_eid(&self.id, "ql-close"),
                                    "Close Quick Look",
                                    ProIcon::Close,
                                    22.,
                                    false,
                                    cx,
                                    |this, _, cx| {
                                        this.quick_look_entry = None;
                                        cx.notify();
                                    },
                                )),
                        )
                    })
                    .into_any_element()
            }
        };

        div()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(head)
            .child(content)
    }

    fn render_suggested_pane(&self, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(tr(cx, "Suggested artifacts", "推荐产物")),
            )
            .child(
                div()
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .children(self.artifacts.iter().enumerate().map(|(ai, art)| {
                        let target = art.name.clone();
                        activate(
                            div()
                                .id(pro_eid(&self.id, format!("sug-art-{ai}")))
                                .role(Role::Button)
                                .tab_index(0)
                                .px(px(10.))
                                .py(px(8.))
                                .when(ai > 0, |d| d.border_t_1().border_color(p.line_soft))
                                .hover(move |s| s.bg(p.hover))
                                .flex()
                                .items_center()
                                .gap(px(9.))
                                .cursor_pointer()
                                .child(Self::render_file_glyph(&art.name, 14., &p))
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
                                                .text_color(p.ink)
                                                .truncate()
                                                .child(art.name.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.5))
                                                .text_color(p.muted)
                                                .child(art.updated.clone()),
                                        ),
                                ),
                            cx,
                            move |this, _, cx| {
                                this.open_artifact_tab(target.clone(), cx);
                            },
                        )
                    })),
            )
    }
}

impl Render for AgentInboxScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

fn default_inbox_items() -> Vec<InboxItem> {
    vec![
        InboxItem {
            id: "queue".into(),
            kind: InboxItemKind::Decision,
            question: "Which queue should the ingest service use?".into(),
            context: Some(
                "Events arrive in bursts of up to 20k a minute. The service needs retries, and a way to replay a day of events after a bad deploy."
                    .into(),
            ),
            task: "Ingest pipeline".into(),
            at_short: "4m".into(),
            at_long: "4 minutes ago".into(),
            unread: false,
            app: None,
            options: vec![
                InboxOption::new("kafka", "Kafka")
                    .with_summary("A durable log – replay is built in")
                    .with_recommended(true)
                    .with_tradeoffs(
                        vec![
                            "Replays any day after a bad deploy".into(),
                            "Takes the bursts with room to spare".into(),
                        ],
                        vec![
                            "One more cluster to run".into(),
                            "Overkill below a thousand events a minute".into(),
                        ],
                    ),
                InboxOption::new("sqs", "SQS")
                    .with_summary("Managed, nothing to operate")
                    .with_tradeoffs(
                        vec![
                            "Zero broker maintenance or paging".into(),
                            "Absorbs burst spikes automatically".into(),
                        ],
                        vec![
                            "Replay requires separate S3 archive job".into(),
                            "14-day retention ceiling".into(),
                        ],
                    ),
                InboxOption::new("postgres", "Postgres queue")
                    .with_summary("Use the database you already have")
                    .with_tradeoffs(
                        vec![
                            "Transactional with existing order writes".into(),
                            "Nothing new to provision".into(),
                        ],
                        vec![
                            "20k/min bursts contend with primary OLTP".into(),
                            "Manual vacuum tuning for day-long replay".into(),
                        ],
                    ),
            ],
            facts: Vec::new(),
            suggestions: Vec::new(),
            done: None,
        },
        InboxItem {
            id: "empty-state".into(),
            kind: InboxItemKind::Pick,
            question: "Which empty state for the dashboard?".into(),
            context: Some(
                "Shown the first time a client opens their workspace before any events arrive. All three fit the current grid."
                    .into(),
            ),
            task: "Client dashboard".into(),
            at_short: "18m".into(),
            at_long: "18 minutes ago".into(),
            unread: true,
            app: None,
            options: vec![
                InboxOption::new("starter", "Starter steps")
                    .with_summary("Three quick actions to connect their first event source")
                    .with_recommended(true)
                    .with_wireframe(InboxWireframe::StarterCards)
                    .with_tradeoffs(
                        vec!["Gives new admins an obvious first click".into()],
                        vec!["Takes more vertical height on mobile".into()],
                    ),
                InboxOption::new("sample", "Sample chart")
                    .with_summary("Ghost throughput bars with a Connect source action")
                    .with_wireframe(InboxWireframe::SampleChart)
                    .with_tradeoffs(
                        vec!["Previews how live metrics will read".into()],
                        vec!["Can look like real traffic at first glance".into()],
                    ),
                InboxOption::new("quiet", "Quiet note")
                    .with_summary("One short line and a single primary button")
                    .with_wireframe(InboxWireframe::MinimalNote)
                    .with_tradeoffs(
                        vec!["Calm and quick to scan".into()],
                        vec!["Less setup guidance".into()],
                    ),
            ],
            facts: Vec::new(),
            suggestions: Vec::new(),
            done: None,
        },
        InboxItem {
            id: "weekly-update".into(),
            kind: InboxItemKind::Approval,
            question: "Send the weekly update to Northwind?".into(),
            context: Some(
                "Pulled milestones from 14 merged PRs and drafted the Friday customer note. Ready to send from your Gmail account."
                    .into(),
            ),
            task: "Weekly update".into(),
            at_short: "42m".into(),
            at_long: "42 minutes ago".into(),
            unread: true,
            app: Some("Gmail".into()),
            options: Vec::new(),
            facts: vec![
                InboxFact::new("To", "maya@northwind.co, eng-leads@northwind.co"),
                InboxFact::new("Subject", "Northwind × Fieldwork – Week 44 progress"),
                InboxFact::new(
                    "Highlights",
                    "Ingest schema landed, 20k/min load test passed, staging cutover Monday",
                ),
                InboxFact::new("From", "alex@fieldwork.co via Gmail"),
            ],
            suggestions: Vec::new(),
            done: None,
        },
        InboxItem {
            id: "launch-tone".into(),
            kind: InboxItemKind::Question,
            question: "What tone should the launch post have?".into(),
            context: Some(
                "The draft covers the new ingest pipeline and replay guarantees. Pick a direction or write a note on how it should read."
                    .into(),
            ),
            task: "Launch post".into(),
            at_short: "1h".into(),
            at_long: "1 hour ago".into(),
            unread: false,
            app: None,
            options: Vec::new(),
            facts: Vec::new(),
            suggestions: vec![
                "Technical & matter-of-fact".into(),
                "Short changelog note".into(),
                "Customer story first".into(),
                "Benchmarks up front".into(),
            ],
            done: None,
        },
        InboxItem {
            id: "old-api".into(),
            kind: InboxItemKind::Decision,
            question: "Keep the old API until March or drop it now?".into(),
            context: Some(
                "Only two staging tokens still call /v1/events, both owned by internal test suites. Keeping the shim adds one translation hop."
                    .into(),
            ),
            task: "API migration".into(),
            at_short: "3h".into(),
            at_long: "3 hours ago".into(),
            unread: false,
            app: None,
            options: vec![
                InboxOption::new("march", "Keep shim until March 1")
                    .with_summary("Translate /v1 calls at the edge and log remaining callers")
                    .with_recommended(true)
                    .with_tradeoffs(
                        vec![
                            "Zero breakage for unmigrated scripts".into(),
                            "Deprecation headers warn remaining callers".into(),
                        ],
                        vec!["Maintains dual schema tests for one quarter".into()],
                    ),
                InboxOption::new("drop-now", "Drop /v1 in this release")
                    .with_summary("Update the two internal suites and remove the adapter")
                    .with_tradeoffs(
                        vec!["Deletes 1,400 lines of translation code now".into()],
                        vec!["Any undocumented caller gets 410 Gone".into()],
                    ),
            ],
            facts: Vec::new(),
            suggestions: Vec::new(),
            done: None,
        },
        InboxItem {
            id: "schema-format".into(),
            kind: InboxItemKind::Decision,
            question: "Avro or JSON Schema for order events?".into(),
            context: Some(
                "Producers are TypeScript and Go services; analysts query replays in DuckDB."
                    .into(),
            ),
            task: "Ingest pipeline".into(),
            at_short: "5h".into(),
            at_long: "5 hours ago".into(),
            unread: false,
            app: None,
            options: vec![
                InboxOption::new("json-schema", "JSON Schema (draft 2020-12)"),
                InboxOption::new("avro", "Avro with registry"),
            ],
            facts: Vec::new(),
            suggestions: Vec::new(),
            done: Some(InboxDoneState {
                answer: InboxAnswer::Choice {
                    option: Some("json-schema".into()),
                    text: None,
                },
                label: "JSON Schema (draft 2020-12)".into(),
                at_short: "4h ago".into(),
                since_steps: vec![
                    "Generated Go and TypeScript validators".into(),
                    "Validated 50k historical Northwind payloads".into(),
                ],
                since_files: vec!["event-schema.json".into()],
            }),
        },
        InboxItem {
            id: "staging-pr".into(),
            kind: InboxItemKind::Approval,
            question: "Open staging PR #479 for webhook retry backoff?".into(),
            context: Some("Adds jittered exponential backoff and dead-letter tags.".into()),
            task: "API migration".into(),
            at_short: "1d".into(),
            at_long: "Yesterday".into(),
            unread: false,
            app: Some("GitHub".into()),
            options: Vec::new(),
            facts: vec![
                InboxFact::new("Repo", "fieldwork/ingest-gateway"),
                InboxFact::new("Branch", "feat/webhook-backoff -> main"),
            ],
            suggestions: Vec::new(),
            done: Some(InboxDoneState {
                answer: InboxAnswer::Approve,
                label: "Approved".into(),
                at_short: "Yesterday".into(),
                since_steps: vec![
                    "CI test suite passed in 2m 14s".into(),
                    "Deployed to staging environment".into(),
                ],
                since_files: vec!["staging".into()],
            }),
        },
    ]
}

fn default_inbox_tasks() -> Vec<InboxTaskInfo> {
    vec![
        InboxTaskInfo {
            name: "Ingest pipeline".into(),
            goal: "Take in Northwind's order events and keep a day of them to replay after a bad deploy.".into(),
            project: "Northwind".into(),
            status: "Waiting on you".into(),
            progress: Some((2, 4)),
            started: "Mon, Nov 3".into(),
            steps: vec![
                InboxTaskStep::new("Draft the event schema", InboxStepState::Done),
                InboxTaskStep::new("Load test at 20k a minute", InboxStepState::Done),
                InboxTaskStep::new("Pick a queue", InboxStepState::WaitingOnYou),
                InboxTaskStep::new("Deploy to staging", InboxStepState::Later),
            ],
            artifacts: vec![
                "ingest-design.md".into(),
                "load-test-results.csv".into(),
                "event-schema.json".into(),
                "staging".into(),
            ],
        },
        InboxTaskInfo {
            name: "Client dashboard".into(),
            goal: "Ship the first-run overview for Northwind workspace admins.".into(),
            project: "Northwind".into(),
            status: "Waiting on you".into(),
            progress: Some((2, 3)),
            started: "Tue, Nov 4".into(),
            steps: vec![
                InboxTaskStep::new("Build metric cards and filters", InboxStepState::Done),
                InboxTaskStep::new("Wire live WebSocket feed", InboxStepState::Done),
                InboxTaskStep::new("Pick empty state variant", InboxStepState::WaitingOnYou),
            ],
            artifacts: vec!["ingest-design.md".into(), "staging".into()],
        },
        InboxTaskInfo {
            name: "Weekly update".into(),
            goal: "Compile Friday engineering highlights for Northwind stakeholders.".into(),
            project: "Northwind".into(),
            status: "Waiting on you".into(),
            progress: Some((3, 4)),
            started: "Fri, Nov 7".into(),
            steps: vec![
                InboxTaskStep::new("Collect 14 merged pull requests", InboxStepState::Done),
                InboxTaskStep::new("Summarize load test numbers", InboxStepState::Done),
                InboxTaskStep::new("Draft customer email", InboxStepState::Done),
                InboxTaskStep::new("Approve Gmail send", InboxStepState::WaitingOnYou),
            ],
            artifacts: vec!["load-test-results.csv".into()],
        },
        InboxTaskInfo {
            name: "Launch post".into(),
            goal: "Write the public release note for replayable event ingestion.".into(),
            project: "Fieldwork Core".into(),
            status: "Waiting on you".into(),
            progress: Some((1, 3)),
            started: "Thu, Nov 6".into(),
            steps: vec![
                InboxTaskStep::new("Outline architecture and benchmarks", InboxStepState::Done),
                InboxTaskStep::new("Set voice and tone", InboxStepState::WaitingOnYou),
                InboxTaskStep::new("Publish changelog draft", InboxStepState::Later),
            ],
            artifacts: vec!["ingest-design.md".into(), "load-test-results.csv".into()],
        },
        InboxTaskInfo {
            name: "API migration".into(),
            goal: "Retire legacy /v1/events endpoints cleanly across staging and production.".into(),
            project: "Fieldwork Core".into(),
            status: "Waiting on you".into(),
            progress: Some((2, 4)),
            started: "Mon, Nov 3".into(),
            steps: vec![
                InboxTaskStep::new("Audit active API tokens", InboxStepState::Done),
                InboxTaskStep::new("Land retry backoff PR #479", InboxStepState::Done),
                InboxTaskStep::new("Decide /v1 sunset date", InboxStepState::WaitingOnYou),
                InboxTaskStep::new("Roll out gateway config", InboxStepState::Later),
            ],
            artifacts: vec!["event-schema.json".into(), "staging".into()],
        },
    ]
}

fn default_inbox_artifacts() -> Vec<InboxArtifact> {
    vec![
        InboxArtifact {
            name: "ingest-design.md".into(),
            updated: "Updated 6m ago · Markdown".into(),
            body: InboxArtifactBody::Document {
                title: "Ingest pipeline – queue & replay".into(),
                paragraphs: vec![
                    "Northwind peaks at 20,000 order events per minute during flash sales. Every event must be persisted before acknowledging the webhook.".into(),
                    "Keeping a 24-hour replay window lets on-call engineers re-run downstream projections after a schema or pricing fix.".into(),
                ],
                checklist: vec![
                    ("Validate JSON Schema v2 against 50k payloads".into(), true),
                    ("Benchmark 20k/min sustained burst".into(), true),
                    ("Confirm production queue retention".into(), false),
                    ("Cut over staging traffic".into(), false),
                ],
            },
        },
        InboxArtifact {
            name: "load-test-results.csv".into(),
            updated: "Updated 14m ago · Sheet".into(),
            body: InboxArtifactBody::Sheet {
                headers: vec![
                    "Backend".into(),
                    "Burst/min".into(),
                    "p99".into(),
                    "Replay".into(),
                ],
                rows: vec![
                    vec![
                        "Kafka".into(),
                        "25,000".into(),
                        "14ms".into(),
                        "7 days".into(),
                    ],
                    vec![
                        "SQS".into(),
                        "20,000".into(),
                        "29ms".into(),
                        "Via S3".into(),
                    ],
                    vec![
                        "Postgres".into(),
                        "8,500".into(),
                        "92ms".into(),
                        "Manual".into(),
                    ],
                ],
            },
        },
        InboxArtifact {
            name: "event-schema.json".into(),
            updated: "Updated 1h ago · JSON".into(),
            body: InboxArtifactBody::Code {
                language: "json".into(),
                source: "{\n  \"$schema\": \"https://json-schema.org/draft/2020-12/schema\",\n  \"title\": \"OrderEvent\",\n  \"required\": [\"event_id\", \"tenant\", \"occurred_at\", \"payload\"],\n  \"properties\": {\n    \"event_id\": { \"type\": \"string\" },\n    \"tenant\": { \"type\": \"string\" },\n    \"occurred_at\": { \"type\": \"string\", \"format\": \"date-time\" }\n  }\n}".into(),
            },
        },
        InboxArtifact {
            name: "staging".into(),
            updated: "4 items · Folder".into(),
            body: InboxArtifactBody::Folder {
                entries: vec![
                    InboxFolderEntry {
                        name: "migrations".into(),
                        size: "--".into(),
                        modified: "2h ago".into(),
                        is_folder: true,
                    },
                    InboxFolderEntry {
                        name: "docker-compose.yml".into(),
                        size: "3.4 KB".into(),
                        modified: "22m ago".into(),
                        is_folder: false,
                    },
                    InboxFolderEntry {
                        name: "replay-worker.toml".into(),
                        size: "1.8 KB".into(),
                        modified: "11m ago".into(),
                        is_folder: false,
                    },
                    InboxFolderEntry {
                        name: "smoke-report.txt".into(),
                        size: "6.1 KB".into(),
                        modified: "5m ago".into(),
                        is_folder: false,
                    },
                ],
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        InboxAnswer, InboxItemKind, InboxTab, apply_decision, default_inbox_items, defer_to_end,
        reopen_to_top, visible_item_indices,
    };

    #[test]
    fn default_items_separate_open_and_done_counts() {
        let items = default_inbox_items();
        let open = visible_item_indices(&items, InboxTab::Open);
        let done = visible_item_indices(&items, InboxTab::Done);
        assert_eq!(open.len(), 5);
        assert_eq!(done.len(), 2);
        assert_eq!(items[open[0]].id.as_ref(), "queue");
        assert_eq!(items[open[0]].default_option_id().as_deref(), Some("kafka"));
    }

    #[test]
    fn decision_later_and_reopen_transitions_preserve_order() {
        let mut items = default_inbox_items();
        let next = defer_to_end(&mut items, "queue");
        assert_eq!(next.as_deref(), Some("empty-state"));
        let open_after_later = visible_item_indices(&items, InboxTab::Open);
        assert_eq!(
            items[*open_after_later.last().expect("non-empty open list")]
                .id
                .as_ref(),
            "queue"
        );

        let (label, next_sel) = apply_decision(&mut items, "weekly-update", InboxAnswer::Approve)
            .expect("weekly-update is open");
        assert_eq!(label.as_ref(), "Approved");
        assert_eq!(next_sel.as_deref(), Some("launch-tone"));
        assert_eq!(visible_item_indices(&items, InboxTab::Open).len(), 4);
        assert_eq!(visible_item_indices(&items, InboxTab::Done).len(), 3);

        assert!(reopen_to_top(&mut items, "weekly-update"));
        assert_eq!(items[0].id.as_ref(), "weekly-update");
        assert_eq!(items[0].kind, InboxItemKind::Approval);
        assert!(items[0].done.is_none());
    }
}
