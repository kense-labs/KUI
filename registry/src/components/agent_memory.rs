//! Agent memory browser and provenance inspector for personal and project knowledge.
//!
//! Displays saved and pending-review memories grouped by scope (`About you` and
//! workspace projects), with inline memory creation, recall sheen animation,
//! provenance quotes, task usage history, and reversible forget/keep actions.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_btn, pro_ease, pro_eid, pro_empty_state, pro_fold,
            pro_mark_selected, pro_pill_indicator, pro_presence, pro_selection_sensor,
            pro_skeleton_bar, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        pro_icons::{ProIcon, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashSet, rc::Rc};
use web_time::Instant;

const RECALL_DURATION_MS: f32 = 1600.;

/// Active top-level filter tab in [`AgentMemoryScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MemoryTab {
    /// Confirmed memories actively recalled by the agent.
    #[default]
    Saved,
    /// Candidate memories picked up automatically and awaiting user consent.
    Review,
}

/// Origin category describing how a memory was learned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MemorySourceKind {
    /// Stated explicitly by the user (`Remember` or direct instruction).
    #[default]
    Told,
    /// Extracted from a conversation thread.
    Thread,
    /// Learned from an approval or decision in the agent inbox.
    Decision,
    /// Extracted from a workspace document or data file.
    File,
}

impl MemorySourceKind {
    /// Leading line icon representing this provenance kind.
    pub fn icon(self) -> ProIcon {
        match self {
            Self::Told => ProIcon::User,
            Self::Thread => ProIcon::Thread,
            Self::Decision => ProIcon::Inbox,
            Self::File => ProIcon::PageDoc,
        }
    }
}

/// Provenance record describing where an [`AgentMemoryItem`] came from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemorySource {
    /// Provenance channel (`Told`, `Thread`, `Decision`, or `File`).
    pub kind: MemorySourceKind,
    /// Relative timestamp when the memory was learned (for example `"16 days ago"`).
    pub at: SharedString,
    /// Verbatim user quote or excerpt supporting the memory.
    pub quote: Option<SharedString>,
    /// Prompt or approval question preceding the quote when learned from a decision.
    pub question: Option<SharedString>,
    /// Source file name when `kind` is [`MemorySourceKind::File`].
    pub file: Option<SharedString>,
}

impl MemorySource {
    /// Provenance for a fact explicitly told by the user.
    pub fn told(at: impl Into<SharedString>) -> Self {
        Self {
            kind: MemorySourceKind::Told,
            at: at.into(),
            quote: None,
            question: None,
            file: None,
        }
    }

    /// Provenance for a preference learned in a chat thread.
    pub fn thread(at: impl Into<SharedString>) -> Self {
        Self {
            kind: MemorySourceKind::Thread,
            at: at.into(),
            quote: None,
            question: None,
            file: None,
        }
    }

    /// Provenance for a rule learned from an inbox decision.
    pub fn decision(at: impl Into<SharedString>) -> Self {
        Self {
            kind: MemorySourceKind::Decision,
            at: at.into(),
            quote: None,
            question: None,
            file: None,
        }
    }

    /// Provenance for a fact extracted from a named workspace file.
    pub fn file(name: impl Into<SharedString>, at: impl Into<SharedString>) -> Self {
        Self {
            kind: MemorySourceKind::File,
            at: at.into(),
            quote: None,
            question: None,
            file: Some(name.into()),
        }
    }

    /// Attach a verbatim quote or excerpt to this provenance record.
    pub fn with_quote(mut self, quote: impl Into<SharedString>) -> Self {
        self.quote = Some(quote.into());
        self
    }

    /// Attach the decision question that prompted this memory.
    pub fn with_question(mut self, question: impl Into<SharedString>) -> Self {
        self.question = Some(question.into());
        self
    }

    fn row_label(&self, cx: &App) -> SharedString {
        match self.kind {
            MemorySourceKind::Told => tr(cx, "You", "你告知").into(),
            MemorySourceKind::Thread => tr(cx, "Thread", "会话").into(),
            MemorySourceKind::Decision => tr(cx, "Decision", "决策").into(),
            MemorySourceKind::File => self
                .file
                .clone()
                .unwrap_or_else(|| tr(cx, "File", "文件").into()),
        }
    }

    fn learned_label(&self, cx: &App) -> SharedString {
        match self.kind {
            MemorySourceKind::Told => tr(cx, "You told it", "由你直接告知").into(),
            MemorySourceKind::Thread => tr(cx, "From the Thread", "来自会话").into(),
            MemorySourceKind::Decision => tr(cx, "From a decision", "来自收件箱决策").into(),
            MemorySourceKind::File => {
                if let Some(ref f) = self.file {
                    SharedString::from(format!("{} {f}", tr(cx, "From", "来自")))
                } else {
                    tr(cx, "From a file", "来自文件").into()
                }
            }
        }
    }

    fn foot_label(&self, cx: &App) -> SharedString {
        match self.kind {
            MemorySourceKind::Told => tr(cx, "You told it", "由你直接告知").into(),
            MemorySourceKind::Thread => tr(cx, "You, in the Thread", "你，在会话中").into(),
            MemorySourceKind::Decision => tr(cx, "You, in the Inbox", "你，在收件箱中").into(),
            MemorySourceKind::File => self
                .file
                .clone()
                .unwrap_or_else(|| tr(cx, "Workspace file", "工作区文件").into()),
        }
    }
}

/// Single task invocation where a memory shaped agent behavior.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryUseRecord {
    /// Task or workflow title where the memory was applied.
    pub task: SharedString,
    /// Concise explanation of how the memory changed the outcome.
    pub how: SharedString,
    /// Relative timestamp of use (for example `"2 days ago"`).
    pub at: SharedString,
}

impl MemoryUseRecord {
    /// Create a usage record for `task` describing `how` it was applied `at` a relative time.
    pub fn new(
        task: impl Into<SharedString>,
        how: impl Into<SharedString>,
        at: impl Into<SharedString>,
    ) -> Self {
        Self {
            task: task.into(),
            how: how.into(),
            at: at.into(),
        }
    }
}

/// One personal or project-scoped memory known by the agent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentMemoryItem {
    /// Stable unique identifier.
    pub id: SharedString,
    /// Concise statement remembered by the agent.
    pub text: SharedString,
    /// Optional project name; `None` applies everywhere under `About you`.
    pub project: Option<SharedString>,
    /// Provenance metadata explaining how the memory was acquired.
    pub source: MemorySource,
    /// Total number of times the agent has recalled this memory.
    pub uses: u32,
    /// Compact relative timestamp for the table column (for example `"2d"` or `"26m"`).
    pub used_at: Option<SharedString>,
    /// Expanded relative timestamp for the detail inspector (for example `"2 days ago"`).
    pub used_at_long: Option<SharedString>,
    /// Recent tasks where this memory was applied, newest first.
    pub used_in: Vec<MemoryUseRecord>,
    /// When `true`, waits under `To review` until explicitly kept.
    pub review: bool,
    /// When `true`, stays pinned to the top of its scope group.
    pub pinned: bool,
}

impl AgentMemoryItem {
    /// Construct a saved memory under `About you` (`project: None`) with zero uses.
    pub fn new(
        id: impl Into<SharedString>,
        text: impl Into<SharedString>,
        source: MemorySource,
    ) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            project: None,
            source,
            uses: 0,
            used_at: None,
            used_at_long: None,
            used_in: Vec::new(),
            review: false,
            pinned: false,
        }
    }

    /// Scope this memory to a named project instead of `Everywhere`.
    pub fn with_project(mut self, project: impl Into<SharedString>) -> Self {
        self.project = Some(project.into());
        self
    }

    /// Set the recall count and compact/long last-used timestamps.
    pub fn with_uses(
        mut self,
        uses: u32,
        short: impl Into<SharedString>,
        long: impl Into<SharedString>,
    ) -> Self {
        self.uses = uses;
        self.used_at = Some(short.into());
        self.used_at_long = Some(long.into());
        self
    }

    /// Attach recent task usage records.
    pub fn with_used_in(mut self, used_in: Vec<MemoryUseRecord>) -> Self {
        self.used_in = used_in;
        self
    }

    /// Mark whether this memory awaits user approval under `To review`.
    pub fn with_review(mut self, review: bool) -> Self {
        self.review = review;
        self
    }

    /// Pin or unpin this memory at the top of its group.
    pub fn with_pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }
}

/// Events emitted by [`AgentMemoryScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentMemoryEvent {
    /// User opened a memory in the right-hand inspector.
    Select(SharedString),
    /// User added a new memory via the inline `Remember` composer.
    Create(AgentMemoryItem),
    /// User edited a memory's text, project scope, pin state, or review state.
    Change(AgentMemoryItem),
    /// User forgot a saved memory or dismissed a review candidate.
    Forget(SharedString),
    /// User clicked `Undo` to restore a forgotten memory at its previous index.
    Restore {
        /// Restored memory item.
        memory: AgentMemoryItem,
        /// Original position in the memories list.
        index: usize,
    },
    /// User approved a `To review` memory so the agent uses it going forward.
    Keep(SharedString),
    /// User clicked `Open` on a memory's provenance card.
    OpenSource(AgentMemoryItem),
    /// User selected a sidebar navigation destination.
    Navigate(SharedString),
}

impl EventEmitter<AgentMemoryEvent> for AgentMemoryScreen {}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
enum UndoAction {
    Forgotten {
        memory: AgentMemoryItem,
        index: usize,
    },
    Kept {
        id: SharedString,
    },
}

/// Default sample projects matching the reference `Agent / Memory` workspace.
pub fn default_memory_projects() -> Vec<SharedString> {
    vec!["Northwind".into(), "Platform".into(), "Evalkit".into()]
}

/// Default sample memories (11 saved + 2 awaiting review) matching the reference specification.
pub fn default_agent_memories() -> Vec<AgentMemoryItem> {
    vec![
        AgentMemoryItem::new(
            "lead-decision",
            "Lead with the decision, then the reasons – three lines or fewer",
            MemorySource::thread("22 days ago").with_quote(
                "Give me the answer on line one, then two or three lines on why – never a wall of setup first.",
            ),
        )
        .with_uses(34, "26m", "26 minutes ago")
        .with_used_in(vec![
            MemoryUseRecord::new(
                "Q3 board brief",
                "Opened with the recommendation in one sentence",
                "26 minutes ago",
            ),
            MemoryUseRecord::new(
                "Vendor comparison",
                "Trimmed rationale to three short lines",
                "yesterday",
            ),
        ]),
        AgentMemoryItem::new(
            "vienna-cet",
            "Works from Vienna – times in CET, nothing booked before 9:00",
            MemorySource::told("1 month ago")
                .with_quote("I work from Vienna in CET – please keep mornings before 9:00 clear."),
        )
        .with_uses(12, "5h", "5 hours ago")
        .with_used_in(vec![
            MemoryUseRecord::new(
                "Partner sync invite",
                "Proposed 10:00 CET instead of 08:30 CET",
                "5 hours ago",
            ),
            MemoryUseRecord::new(
                "Design review",
                "Converted Pacific slots to CET",
                "4 days ago",
            ),
        ]),
        AgentMemoryItem::new(
            "friday",
            "Nothing goes out to clients on a Friday afternoon",
            MemorySource::decision("16 days ago")
                .with_question("Send the weekly update to Northwind now?")
                .with_quote("Not on a Friday afternoon – Monday at 9:00."),
        )
        .with_uses(6, "2d", "2 days ago")
        .with_used_in(vec![
            MemoryUseRecord::new(
                "Weekly update",
                "Held it until Monday 9:00",
                "2 days ago",
            ),
            MemoryUseRecord::new(
                "Client dashboard",
                "Moved the launch email to Monday",
                "9 days ago",
            ),
        ]),
        AgentMemoryItem::new(
            "euros-sign",
            "Prices in euros with the sign after the number: 1 240 €",
            MemorySource::file("writing-style.md", "19 days ago")
                .with_quote("Format euro figures with a space before the symbol: 1 240 €."),
        )
        .with_uses(9, "1d", "yesterday")
        .with_used_in(vec![MemoryUseRecord::new(
            "Renewal proposal",
            "Formatted line items as 1 240 €",
            "yesterday",
        )]),
        AgentMemoryItem::new(
            "british-english",
            "Writes in British English – colour, organise, analyse",
            MemorySource::told("1 month ago")
                .with_quote("Use British spelling in everything you draft for me – colour, organise, analyse."),
        )
        .with_uses(21, "1h", "1 hour ago")
        .with_used_in(vec![MemoryUseRecord::new(
            "Northwind status note",
            "Drafted summary in British English",
            "1 hour ago",
        )]),
        AgentMemoryItem::new(
            "priya-signoff",
            "Priya Shah signs off on anything that reaches Northwind's customers",
            MemorySource::thread("14 days ago")
                .with_quote("Make sure Priya Shah signs off before anything customer-facing goes out to Northwind."),
        )
        .with_project("Northwind")
        .with_uses(8, "42m", "42 minutes ago")
        .with_used_in(vec![MemoryUseRecord::new(
            "Migration notice",
            "Routed draft to Priya Shah for sign-off",
            "42 minutes ago",
        )]),
        AgentMemoryItem::new(
            "order-events-json",
            "Order events are JSON with a version field",
            MemorySource::decision("18 days ago")
                .with_question("Publish v2 order events without a schema envelope?")
                .with_quote("Keep order events as JSON and always include a top-level version field."),
        )
        .with_project("Northwind")
        .with_uses(5, "4h", "4 hours ago")
        .with_used_in(vec![MemoryUseRecord::new(
            "Webhook contract check",
            "Verified top-level version field on payload",
            "4 hours ago",
        )]),
        AgentMemoryItem::new(
            "dashboard-830",
            "Their team opens the dashboard at 8:30 – the numbers are ready by 8:00",
            MemorySource::thread("12 days ago")
                .with_quote("Northwind's standup opens the dashboard at 8:30, so have the numbers refreshed by 8:00."),
        )
        .with_project("Northwind")
        .with_uses(3, "3d", "3 days ago")
        .with_used_in(vec![MemoryUseRecord::new(
            "Daily rollup schedule",
            "Finished warehouse sync at 07:48 CET",
            "3 days ago",
        )]),
        AgentMemoryItem::new(
            "staging-tests",
            "Staging gets a deploy only once every test passes – no skipping, not even for a hotfix",
            MemorySource::told("3 weeks ago")
                .with_quote("Never deploy to staging with a failing check – not even for an urgent hotfix."),
        )
        .with_project("Platform")
        .with_uses(14, "6m", "6 minutes ago")
        .with_used_in(vec![MemoryUseRecord::new(
            "Hotfix #418",
            "Waited for integration suite before deploying",
            "6 minutes ago",
        )]),
        AgentMemoryItem::new(
            "api-v1-clients",
            "Two clients still call API v1 – write to them before anything changes",
            MemorySource::file("v1-traffic.csv", "9 days ago")
                .with_quote("lumen-prod and hartmann-sync still send daily traffic to /v1/orders."),
        )
        .with_project("Platform")
        .with_uses(2, "3h", "3 hours ago")
        .with_used_in(vec![MemoryUseRecord::new(
            "API deprecation plan",
            "Added advance notice task for two v1 callers",
            "3 hours ago",
        )]),
        AgentMemoryItem::new(
            "scores-one-decimal",
            "Scores show one decimal – 87.4, never 87.38",
            MemorySource::decision("11 days ago")
                .with_question("Show two decimals in benchmark score tables?")
                .with_quote("Stick to one decimal – 87.4, never 87.38."),
        )
        .with_project("Evalkit")
        .with_uses(4, "2d", "2 days ago")
        .with_used_in(vec![MemoryUseRecord::new(
            "Weekly benchmark report",
            "Rounded pass rates to one decimal place",
            "2 days ago",
        )]),
        // Two candidate memories waiting under "To review"
        AgentMemoryItem::new(
            "review-invoices-pdf",
            "Invoices for Northwind go to finance@northwind.co as PDF attachments, never links",
            MemorySource::thread("18 minutes ago").with_quote(
                "Send Northwind's invoices straight to finance@northwind.co as attached PDFs – links get stripped.",
            ),
        )
        .with_project("Northwind")
        .with_review(true),
        AgentMemoryItem::new(
            "review-pr-bullets",
            "Pull request summaries stay under five bullets with a short test section",
            MemorySource::thread("2 hours ago")
                .with_quote("Keep PR descriptions under five bullets and end with how you tested it."),
        )
        .with_review(true),
    ]
}

fn format_compact_uses(uses: u32) -> String {
    if uses >= 1_000_000 {
        format!("{:.1}M ×", uses as f32 / 1_000_000.)
    } else if uses >= 1_000 {
        format!("{:.1}K ×", uses as f32 / 1_000.)
    } else {
        format!("{uses} ×")
    }
}

fn filter_and_group_memories(
    memories: &[AgentMemoryItem],
    projects: &[SharedString],
    tab: MemoryTab,
    query: &str,
) -> Vec<(Option<SharedString>, Vec<AgentMemoryItem>)> {
    let want_review = tab == MemoryTab::Review;
    let q = query.trim().to_ascii_lowercase();
    let matches_query = |m: &AgentMemoryItem| -> bool {
        if m.review != want_review {
            return false;
        }
        if q.is_empty() {
            return true;
        }
        m.text.to_ascii_lowercase().contains(&q)
            || m.project
                .as_ref()
                .is_some_and(|p| p.to_ascii_lowercase().contains(&q))
            || m.source
                .file
                .as_ref()
                .is_some_and(|f| f.to_ascii_lowercase().contains(&q))
            || m.source
                .quote
                .as_ref()
                .is_some_and(|qt| qt.to_ascii_lowercase().contains(&q))
    };

    let mut groups: Vec<(Option<SharedString>, Vec<AgentMemoryItem>)> = Vec::new();

    // 1. "About you" (project == None)
    let mut about_you: Vec<AgentMemoryItem> = memories
        .iter()
        .filter(|m| m.project.is_none() && matches_query(m))
        .cloned()
        .collect();
    about_you.sort_by_key(|m| !m.pinned);
    if !about_you.is_empty() {
        groups.push((None, about_you));
    }

    // 2. Ordered projects, plus any extra projects present on items
    let mut ordered_projects: Vec<SharedString> = projects.to_vec();
    for m in memories {
        if let Some(ref proj) = m.project
            && !ordered_projects.contains(proj)
        {
            ordered_projects.push(proj.clone());
        }
    }

    for proj in ordered_projects {
        let mut items: Vec<AgentMemoryItem> = memories
            .iter()
            .filter(|m| m.project.as_ref() == Some(&proj) && matches_query(m))
            .cloned()
            .collect();
        items.sort_by_key(|m| !m.pinned);
        if !items.is_empty() {
            groups.push((Some(proj), items));
        }
    }

    groups
}

/// Interactive `Agent / Memory` governance screen hosted in [`AppShell`].
pub struct AgentMemoryScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    standalone: bool,
    loading: bool,
    empty: bool,
    tab: MemoryTab,
    projects: Vec<SharedString>,
    memories: Vec<AgentMemoryItem>,
    selected_id: Option<SharedString>,
    collapsed_groups: HashSet<SharedString>,
    composer_open: bool,
    composer_scope: Option<SharedString>,
    composer_scope_menu: bool,
    panel_scope_menu: bool,
    search_editor: Option<Entity<Editor>>,
    composer_editor: Option<Entity<Editor>>,
    panel_editor: Option<Entity<Editor>>,
    synced_panel_id: Option<SharedString>,
    recalled: Option<(SharedString, Instant)>,
    undo_state: Option<(UndoAction, SharedString)>,
    last_undo_msg: Option<SharedString>,
    next_seq: usize,
    focus: FocusHandle,
    measured_width: Rc<Cell<f32>>,
    boot_at: Instant,
}

impl AgentMemoryScreen {
    /// Construct a new [`AgentMemoryScreen`] preloaded with [`default_agent_memories`] and `"friday"` selected.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str = id.into();
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Agent, shell_cx)
                .with_active("Memory")
                .with_work_area(true, true)
                .with_panel_width(352.);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_inspector(w, cx));
            s
        });
        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => cx.emit(AgentMemoryEvent::Navigate(label.clone())),
            AppShellEvent::PanelOpen(_) => cx.notify(),
            _ => {}
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            shell,
            standalone: false,
            loading: false,
            empty: false,
            tab: MemoryTab::Saved,
            projects: default_memory_projects(),
            memories: default_agent_memories(),
            selected_id: Some("friday".into()),
            collapsed_groups: HashSet::new(),
            composer_open: false,
            composer_scope: None,
            composer_scope_menu: false,
            panel_scope_menu: false,
            search_editor: None,
            composer_editor: None,
            panel_editor: None,
            synced_panel_id: None,
            recalled: None,
            undo_state: None,
            last_undo_msg: None,
            next_seq: 1,
            focus: cx.focus_handle(),
            measured_width: Rc::new(Cell::new(920.)),
            boot_at: Instant::now(),
        }
    }

    /// Replace the memories list.
    pub fn with_memories(mut self, memories: Vec<AgentMemoryItem>) -> Self {
        self.memories = memories;
        self
    }

    /// Override the ordered project scope list.
    pub fn with_projects(mut self, projects: Vec<SharedString>) -> Self {
        self.projects = projects;
        self
    }

    /// Choose the initial tab (`Saved` or `Review`).
    pub fn with_default_tab(mut self, tab: MemoryTab) -> Self {
        self.tab = tab;
        self
    }

    /// Set the initially selected memory id (or `None` to close the right inspector).
    pub fn with_selected(mut self, id: Option<SharedString>, cx: &mut Context<Self>) -> Self {
        let open = id.is_some();
        self.selected_id = id;
        self.synced_panel_id = None;
        self.shell.update(cx, |s, cx| s.set_panel_open(open, cx));
        self
    }

    /// Update the `Inbox` badge count in the [`AppShell`] sidebar.
    pub fn with_inbox_count(self, count: u32, cx: &mut Context<Self>) -> Self {
        self.shell.update(cx, |s, cx| {
            let groups = vec![
                ShellNavGroup::primary(vec![
                    ShellNavItem::new("thread", tr(cx, "Thread", "会话"), ProIcon::Thread),
                    ShellNavItem::new("inbox", tr(cx, "Inbox", "收件箱"), ProIcon::Inbox)
                        .with_count(count),
                    ShellNavItem::new("tasks", tr(cx, "Tasks", "任务"), ProIcon::Tasks),
                    ShellNavItem::new("memory", tr(cx, "Memory", "记忆"), ProIcon::Memory),
                ]),
                ShellNavGroup::named(
                    tr(cx, "Workspace", "工作区"),
                    vec![
                        ShellNavItem::new(
                            "schedules",
                            tr(cx, "Schedules", "定时计划"),
                            ProIcon::Calendar,
                        ),
                        ShellNavItem::new(
                            "activity",
                            tr(cx, "Activity", "活动记录"),
                            ProIcon::Activity,
                        ),
                        ShellNavItem::new(
                            "usage",
                            tr(cx, "Usage", "用量统计"),
                            ProIcon::UsageChart,
                        ),
                        ShellNavItem::new(
                            "permissions",
                            tr(cx, "Permissions", "权限策略"),
                            ProIcon::Shield,
                        ),
                        ShellNavItem::new(
                            "settings",
                            tr(cx, "Settings", "设置"),
                            ProIcon::Settings,
                        ),
                    ],
                ),
            ];
            *s = std::mem::replace(s, AppShell::new("tmp", cx)).with_groups(groups);
            cx.notify();
        });
        self
    }

    /// Override the workspace title shown in the left sidebar.
    pub fn with_workspace(
        self,
        workspace: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let ws = workspace.into();
        self.shell.update(cx, |s, cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", cx)).with_workspace(ws);
            cx.notify();
        });
        self
    }

    /// Override the user account displayed in the sidebar footer.
    pub fn with_user(
        self,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = name.into();
        let email = email.into();
        self.shell.update(cx, |s, cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", cx)).with_user(name, email);
            cx.notify();
        });
        self
    }

    /// Render without the surrounding [`AppShell`] chrome when `standalone` is `true`.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Configure the skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the empty state and fold the inspector when `empty` is `true`.
    pub fn with_empty(mut self, empty: bool, cx: &mut Context<Self>) -> Self {
        self.empty = empty;
        if empty {
            self.shell.update(cx, |s, cx| s.set_panel_open(false, cx));
        }
        self
    }

    /// Toggle the skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Toggle the empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            if empty {
                self.shell.update(cx, |s, cx| s.set_panel_open(false, cx));
            }
            cx.notify();
        }
    }

    /// Access the underlying [`AppShell`] entity.
    pub fn shell(&self) -> &Entity<AppShell> {
        &self.shell
    }

    /// Trigger a live recall on `id`, incrementing its use count and sweeping a recall sheen across its row.
    pub fn recall(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(item) = self.memories.iter_mut().find(|m| m.id.as_ref() == id) {
            item.uses = item.uses.saturating_add(1);
            item.used_at = Some(tr(cx, "now", "刚刚").into());
            item.used_at_long = Some(tr(cx, "just now", "刚刚").into());
            self.recalled = Some((item.id.clone(), Instant::now()));
            cx.notify();
        }
    }

    fn project_color(&self, project: &str, p: &ProPalette) -> Hsla {
        let palette = [p.blue, p.green, p.orange, p.teal, p.amber, p.pink];
        let idx = self
            .projects
            .iter()
            .position(|name| name.as_ref() == project)
            .unwrap_or(0);
        palette[idx % palette.len()]
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.search_editor.is_none() {
            let ph: SharedString = tr(cx, "Search memory", "搜索记忆").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed
                | EditEvent::Submitted
                | EditEvent::Focused
                | EditEvent::Blurred => {
                    cx.notify();
                }
                EditEvent::Escape => {
                    ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.search_editor = Some(ed);
        }

        if self.composer_editor.is_none() {
            let ph: SharedString = tr(
                cx,
                "Remember something about you or a project…",
                "记录一条关于你或项目的记忆…",
            )
            .into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |this, _, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => cx.notify(),
                EditEvent::Submitted => this.submit_composer(w, cx),
                EditEvent::Escape => {
                    this.composer_open = false;
                    this.composer_scope_menu = false;
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.composer_editor = Some(ed);
        }

        if self.panel_editor.is_none() {
            let ph: SharedString = tr(cx, "What it remembers", "记忆内容").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::InlineMulti, w, cx);
                e.set_placeholder(ph);
                e.font_size = 15.;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Submitted => {
                    let next_text = ed_ent.read(cx).value().trim().to_string();
                    if !next_text.is_empty()
                        && let Some(ref sel_id) = this.selected_id
                        && let Some(item) = this.memories.iter_mut().find(|m| &m.id == sel_id)
                        && item.text.as_ref() != next_text
                    {
                        item.text = next_text.into();
                        let updated = item.clone();
                        cx.emit(AgentMemoryEvent::Change(updated));
                    }
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                EditEvent::Escape => {
                    if let Some(ref sel_id) = this.selected_id
                        && let Some(item) = this.memories.iter().find(|m| &m.id == sel_id)
                    {
                        let orig = item.text.to_string();
                        ed_ent.update(cx, |e, cx| e.set_value(orig, w, cx));
                    }
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                EditEvent::Changed | EditEvent::Focused | EditEvent::Blurred => cx.notify(),
                _ => {}
            })
            .detach();
            self.panel_editor = Some(ed);
        }

        // Keep panel editor synchronized when selection changes
        if self.synced_panel_id != self.selected_id {
            self.synced_panel_id = self.selected_id.clone();
            if let Some(ref ed) = self.panel_editor {
                let txt = self
                    .selected_id
                    .as_ref()
                    .and_then(|id| self.memories.iter().find(|m| &m.id == id))
                    .map(|m| m.text.to_string())
                    .unwrap_or_default();
                ed.update(cx, |e, cx| e.set_value(txt, w, cx));
            }
        }
    }

    fn current_query(&self, cx: &App) -> String {
        self.search_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default()
    }

    fn visible_ids(&self, cx: &App) -> Vec<SharedString> {
        if self.empty {
            return Vec::new();
        }
        let q = self.current_query(cx);
        let grouped = filter_and_group_memories(&self.memories, &self.projects, self.tab, &q);
        let mut out = Vec::new();
        for (proj, items) in grouped {
            let gkey: SharedString = proj.unwrap_or_else(|| "__about_you__".into());
            if !self.collapsed_groups.contains(&gkey) {
                for item in items {
                    out.push(item.id);
                }
            }
        }
        out
    }

    fn select_memory(&mut self, id: SharedString, open_panel: bool, cx: &mut Context<Self>) {
        self.selected_id = Some(id.clone());
        self.synced_panel_id = None;
        self.panel_scope_menu = false;
        if open_panel {
            self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        }
        cx.emit(AgentMemoryEvent::Select(id));
        cx.notify();
    }

    fn submit_composer(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(ed) = self.composer_editor.clone() else {
            return;
        };
        let raw = ed.read(cx).value().trim().to_string();
        if raw.is_empty() {
            return;
        }
        let id = SharedString::from(format!("mem-new-{}", self.next_seq));
        self.next_seq += 1;
        let mut item = AgentMemoryItem::new(
            id.clone(),
            raw.clone(),
            MemorySource::told(tr(cx, "just now", "刚刚")).with_quote(raw),
        );
        item.project = self.composer_scope.clone();
        self.memories.insert(0, item.clone());
        self.tab = MemoryTab::Saved;
        self.empty = false;
        self.selected_id = Some(id);
        self.synced_panel_id = None;
        self.composer_scope_menu = false;
        ed.update(cx, |e, cx| e.set_value("", w, cx));
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        cx.emit(AgentMemoryEvent::Create(item));
        cx.notify();
    }

    fn forget_memory(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(pos) = self.memories.iter().position(|m| m.id.as_ref() == id) else {
            return;
        };
        let removed = self.memories.remove(pos);
        let msg = if removed.review {
            SharedString::from(format!(
                "{}: {}",
                tr(cx, "Dismissed", "已跳过"),
                removed.text
            ))
        } else {
            SharedString::from(format!(
                "{}: {}",
                tr(cx, "Forgotten", "已遗忘"),
                removed.text
            ))
        };
        self.last_undo_msg = Some(msg.clone());
        self.undo_state = Some((
            UndoAction::Forgotten {
                memory: removed.clone(),
                index: pos,
            },
            msg,
        ));
        if self.selected_id.as_ref().is_some_and(|s| s.as_ref() == id) {
            let vis = self.visible_ids(cx);
            self.selected_id = vis.first().cloned();
            self.synced_panel_id = None;
        }
        cx.emit(AgentMemoryEvent::Forget(removed.id));
        cx.notify();
    }

    fn keep_memory(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(item) = self.memories.iter_mut().find(|m| m.id.as_ref() == id) else {
            return;
        };
        if !item.review {
            return;
        }
        item.review = false;
        let kept_id = item.id.clone();
        let updated = item.clone();
        let kept_msg: SharedString = tr(
            cx,
            "Kept – it uses this from now on",
            "已保留——智能体此后将使用该记忆",
        )
        .into();
        self.last_undo_msg = Some(kept_msg.clone());
        self.undo_state = Some((
            UndoAction::Kept {
                id: kept_id.clone(),
            },
            kept_msg,
        ));
        cx.emit(AgentMemoryEvent::Keep(kept_id));
        cx.emit(AgentMemoryEvent::Change(updated));
        cx.notify();
    }

    fn toggle_pin(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(item) = self.memories.iter_mut().find(|m| m.id.as_ref() == id) {
            item.pinned = !item.pinned;
            let updated = item.clone();
            cx.emit(AgentMemoryEvent::Change(updated));
            cx.notify();
        }
    }

    fn set_memory_scope(&mut self, id: &str, scope: Option<SharedString>, cx: &mut Context<Self>) {
        if let Some(item) = self.memories.iter_mut().find(|m| m.id.as_ref() == id)
            && item.project != scope
        {
            item.project = scope;
            let updated = item.clone();
            cx.emit(AgentMemoryEvent::Change(updated));
        }
        self.panel_scope_menu = false;
        cx.notify();
    }

    fn apply_undo(&mut self, cx: &mut Context<Self>) {
        let Some((action, _)) = self.undo_state.take() else {
            return;
        };
        match action {
            UndoAction::Forgotten { memory, index } => {
                let idx = index.min(self.memories.len());
                let mid = memory.id.clone();
                self.memories.insert(idx, memory.clone());
                self.selected_id = Some(mid);
                self.synced_panel_id = None;
                cx.emit(AgentMemoryEvent::Restore { memory, index: idx });
            }
            UndoAction::Kept { id } => {
                if let Some(item) = self.memories.iter_mut().find(|m| m.id == id) {
                    item.review = true;
                    let updated = item.clone();
                    cx.emit(AgentMemoryEvent::Change(updated));
                }
            }
        }
        cx.notify();
    }

    fn handle_key(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if ev.keystroke.modifiers.platform || ev.keystroke.modifiers.control {
            return;
        }
        let key = ev.keystroke.key.as_str();
        match key {
            "/" => {
                if let Some(ref ed) = self.search_editor {
                    {
                        let fh = ed.read(cx).focus.clone();
                        fh.focus(w, cx);
                    }
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }
            "n" | "N" => {
                self.composer_open = true;
                self.composer_scope_menu = false;
                if let Some(ref ed) = self.composer_editor {
                    {
                        let fh = ed.read(cx).focus.clone();
                        fh.focus(w, cx);
                    }
                }
                w.prevent_default();
                cx.stop_propagation();
                cx.notify();
            }
            "j" | "J" | "down" => {
                let ids = self.visible_ids(cx);
                if !ids.is_empty() {
                    let next = self
                        .selected_id
                        .as_ref()
                        .and_then(|cur| ids.iter().position(|x| x == cur))
                        .map(|i| (i + 1).min(ids.len() - 1))
                        .unwrap_or(0);
                    self.select_memory(ids[next].clone(), true, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }
            "k" | "K" | "up" => {
                let ids = self.visible_ids(cx);
                if !ids.is_empty() {
                    let prev = self
                        .selected_id
                        .as_ref()
                        .and_then(|cur| ids.iter().position(|x| x == cur))
                        .map(|i| i.saturating_sub(1))
                        .unwrap_or(0);
                    self.select_memory(ids[prev].clone(), true, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }
            "home" => {
                let ids = self.visible_ids(cx);
                if let Some(first) = ids.first() {
                    self.select_memory(first.clone(), true, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }
            "end" => {
                let ids = self.visible_ids(cx);
                if let Some(last) = ids.last() {
                    self.select_memory(last.clone(), true, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }
            "enter" => {
                if let Some(sel) = self.selected_id.clone() {
                    self.select_memory(sel, true, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }
            "p" | "P" => {
                if let Some(sel) = self.selected_id.clone() {
                    self.toggle_pin(&sel, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }
            "r" | "R" => {
                if let Some(sel) = self.selected_id.clone() {
                    self.recall(&sel, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }
            "backspace" | "delete" => {
                if let Some(sel) = self.selected_id.clone() {
                    self.forget_memory(&sel, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }
            "escape" => {
                if self.composer_scope_menu || self.panel_scope_menu || self.composer_open {
                    self.composer_scope_menu = false;
                    self.panel_scope_menu = false;
                    self.composer_open = false;
                    cx.notify();
                } else if !self.current_query(cx).is_empty()
                    && let Some(ref ed) = self.search_editor
                {
                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    fn render_loading_list(&self, phase: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let group_specs: [(f32, &[f32]); 3] = [
            (84., &[260., 224., 242., 196.]),
            (76., &[248., 184., 230.]),
            (68., &[272., 208.]),
        ];
        let mut tree = div().p(px(8.)).pb(px(56.)).flex().flex_col().gap(px(14.));
        for (gi, (g_w, rows)) in group_specs.iter().enumerate() {
            let mut grp = div().flex().flex_col().gap(px(2.));
            grp = grp.child(
                div()
                    .h(px(34.))
                    .mb(px(4.))
                    .px(px(12.))
                    .rounded(px(10.))
                    .bg(p.ink.opacity(0.045))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(pro_skeleton_bar(14., 14., 4., phase, cx))
                    .child(pro_skeleton_bar(
                        *g_w,
                        11.,
                        4.,
                        phase + 0.08 * gi as f32,
                        cx,
                    ))
                    .child(pro_skeleton_bar(20., 16., 5., phase, cx)),
            );
            for (ri, &rw) in rows.iter().enumerate() {
                grp = grp.child(
                    div()
                        .h(px(40.))
                        .px(px(12.))
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(14., 14., 4., phase, cx))
                        .child(div().flex_1().min_w_0().child(pro_skeleton_bar(
                            rw,
                            11.,
                            4.,
                            phase + 0.05 * ri as f32,
                            cx,
                        )))
                        .child(pro_skeleton_bar(48., 10., 4., phase, cx))
                        .child(pro_skeleton_bar(26., 10., 4., phase, cx))
                        .child(pro_skeleton_bar(20., 10., 4., phase, cx)),
                );
            }
            tree = tree.child(grp);
        }
        tree
    }

    fn render_composer(&mut self, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let composer_ed = self.composer_editor.clone();
        let can_save = composer_ed
            .as_ref()
            .is_some_and(|e| !e.read(cx).value().trim().is_empty());
        let scope_label: SharedString = self
            .composer_scope
            .clone()
            .unwrap_or_else(|| tr(cx, "Everywhere", "全局适用").into());
        let (scope_icon, scope_ic_color) = if let Some(ref proj) = self.composer_scope {
            (ProIcon::Folder, self.project_color(proj, &p))
        } else {
            (ProIcon::Globe, p.muted)
        };

        let mut menu_list = div()
            .w(px(184.))
            .p(px(4.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_col()
            .gap(px(1.));
        let is_everywhere = self.composer_scope.is_none();
        menu_list = menu_list.child(activate(
            div()
                .id(pro_eid(&self.id, "comp-scope-everywhere"))
                .role(Role::MenuItem)
                .tab_index(0)
                .h(px(30.))
                .px(px(8.))
                .rounded(px(7.))
                .hover(move |s| s.bg(p.hover))
                .flex()
                .items_center()
                .gap(px(8.))
                .text_size(px(12.5))
                .text_color(if is_everywhere { p.ink } else { p.text })
                .cursor_pointer()
                .child(pro_icon(ProIcon::Globe, 14., p.muted))
                .child(div().flex_1().child(tr(cx, "Everywhere", "全局适用")))
                .when(is_everywhere, |d| {
                    d.child(pro_icon(ProIcon::Check, 13., p.ink))
                }),
            cx,
            |this, _, cx| {
                this.composer_scope = None;
                this.composer_scope_menu = false;
                cx.notify();
            },
        ));
        for (pi, proj) in self.projects.clone().into_iter().enumerate() {
            let checked = self.composer_scope.as_ref() == Some(&proj);
            let p_color = self.project_color(&proj, &p);
            let proj_choose = proj.clone();
            menu_list = menu_list.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("comp-scope-p-{pi}")))
                    .role(Role::MenuItem)
                    .tab_index(0)
                    .h(px(30.))
                    .px(px(8.))
                    .rounded(px(7.))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.5))
                    .text_color(if checked { p.ink } else { p.text })
                    .cursor_pointer()
                    .child(pro_icon(ProIcon::Folder, 14., p_color))
                    .child(div().flex_1().truncate().child(proj))
                    .when(checked, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink))),
                cx,
                move |this, _, cx| {
                    this.composer_scope = Some(proj_choose.clone());
                    this.composer_scope_menu = false;
                    cx.notify();
                },
            ));
        }
        let scope_menu = pro_presence(
            pro_eid(&self.id, "comp-scope-pres"),
            self.composer_scope_menu,
            0.16,
            0.12,
            move |amount| {
                div()
                    .absolute()
                    .top(px(34. - 5. * (1. - amount)))
                    .right(px(0.))
                    .opacity(amount)
                    .child(menu_list)
                    .into_any_element()
            },
        );

        div()
            .mx(px(8.))
            .mt(px(8.))
            .py(px(8.))
            .pl(px(12.))
            .pr(px(8.))
            .rounded(px(8.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .items_center()
            .gap(px(8.))
            .child(pro_icon(ProIcon::User, 14., p.muted))
            .when_some(composer_ed, |d, ed| {
                d.child(div().flex_1().min_w_0().child(ed))
            })
            .child(
                div()
                    .relative()
                    .flex_none()
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "comp-scope-btn"))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(28.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .hover(move |s| s.bg(p.chip))
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .text_size(px(12.5))
                            .text_color(p.text)
                            .cursor_pointer()
                            .child(pro_icon(scope_icon, 13., scope_ic_color))
                            .child(div().max_w(px(110.)).truncate().child(scope_label))
                            .child(pro_icon(ProIcon::ChevronDown, 12., p.faint)),
                        cx,
                        |this, _, cx| {
                            this.composer_scope_menu = !this.composer_scope_menu;
                            cx.notify();
                        },
                    ))
                    .child(scope_menu),
            )
            .child(pro_btn(
                pro_eid(&self.id, "comp-cancel"),
                tr(cx, "Cancel", "取消"),
                None,
                ProBtnStyle::Ghost,
                true,
                cx,
                |this, w, cx| {
                    this.composer_open = false;
                    this.composer_scope_menu = false;
                    this.focus.focus(w, cx);
                    cx.notify();
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "comp-save"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(10.))
                    .rounded(px(6.))
                    .bg(p.ink)
                    .text_color(p.card)
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .opacity(if can_save { 1.0 } else { 0.4 })
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .child(tr(cx, "Save", "保存")),
                cx,
                |this, w, cx| this.submit_composer(w, cx),
            ))
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let reduced_motion = Theme::global(cx).reduced_motion;
        let width = self.measured_width.get();
        let hide_source = width < 720.;
        let compact_row = width < 560.;
        let hide_title_count = width < 560.;
        let short_review = width < 340.;
        let panel_open = self.standalone || self.shell.read(cx).is_panel_open();

        // Check active recall sheen animation
        let mut active_sheen: Option<(SharedString, f32)> = None;
        if let Some((ref rid, started)) = self.recalled {
            let elapsed = started.elapsed().as_secs_f32() * 1000.;
            if !reduced_motion && elapsed < RECALL_DURATION_MS {
                active_sheen = Some((rid.clone(), (elapsed / RECALL_DURATION_MS).clamp(0., 1.)));
                w.request_animation_frame();
            } else {
                self.recalled = None;
            }
        }

        if self.loading && !reduced_motion {
            w.request_animation_frame();
        }
        let skel_phase = (self.boot_at.elapsed().as_secs_f32() * 0.85) % 1.0;

        let saved_count = if self.empty {
            0
        } else {
            self.memories.iter().filter(|m| !m.review).count()
        };
        let review_count = if self.empty {
            0
        } else {
            self.memories.iter().filter(|m| m.review).count()
        };

        let query = self.current_query(cx);
        let search_focused = self
            .search_editor
            .as_ref()
            .is_some_and(|e| e.read(cx).focus.is_focused(w));
        let search_ed = self.search_editor.clone();

        let tab_sensor = pro_selection_sensor();

        // Top header (.ui-am-head)
        let head = div()
            .h(px(48.))
            .flex_none()
            .pl(px(20.))
            .pr(px(if panel_open { 12. } else { 48. }))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(if compact_row { 8. } else { 12. }))
            .when(width >= 460., |d| {
                d.child(
                    div()
                        .flex_none()
                        .flex()
                        .items_baseline()
                        .gap(px(7.))
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(tr(cx, "Memory", "记忆")),
                        )
                        .when(!hide_title_count, |d| {
                            d.child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(p.faint)
                                    .child(format!("{saved_count}")),
                            )
                        }),
                )
            })
            .child(
                div()
                    .relative()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .child(pro_pill_indicator(
                        pro_eid(&self.id, "tab-ind"),
                        format!("{:?}", self.tab),
                        tab_sensor.clone(),
                        6.,
                        p.chip,
                        None,
                        false,
                    ))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "tab-saved"))
                            .role(Role::Tab)
                            .tab_index(0)
                            .relative()
                            .h(px(28.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .when(self.tab != MemoryTab::Saved, |d| {
                                d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                            })
                            .flex()
                            .items_center()
                            .text_size(px(13.))
                            .text_color(if self.tab == MemoryTab::Saved {
                                p.ink
                            } else {
                                p.muted
                            })
                            .cursor_pointer()
                            .child(tr(cx, "Saved", "已保存"))
                            .when(self.tab == MemoryTab::Saved, |d| {
                                d.child(pro_mark_selected(&tab_sensor))
                            }),
                        cx,
                        |this, _, cx| {
                            this.tab = MemoryTab::Saved;
                            let vis = this.visible_ids(cx);
                            if !vis.iter().any(|id| this.selected_id.as_ref() == Some(id)) {
                                this.selected_id = vis.first().cloned();
                                this.synced_panel_id = None;
                            }
                            cx.notify();
                        },
                    ))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "tab-review"))
                            .role(Role::Tab)
                            .tab_index(0)
                            .relative()
                            .h(px(28.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .when(self.tab != MemoryTab::Review, |d| {
                                d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                            })
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(13.))
                            .text_color(if self.tab == MemoryTab::Review {
                                p.ink
                            } else {
                                p.muted
                            })
                            .cursor_pointer()
                            .child(if short_review {
                                tr(cx, "Review", "待审")
                            } else {
                                tr(cx, "To review", "待审核")
                            })
                            .when(review_count > 0, |d| {
                                d.child(
                                    div()
                                        .min_w(px(18.))
                                        .h(px(18.))
                                        .px(px(5.))
                                        .rounded(px(4.))
                                        .bg(p.amber_soft)
                                        .text_color(p.amber_ink)
                                        .text_size(px(11.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(format!("{review_count}")),
                                )
                            })
                            .when(self.tab == MemoryTab::Review, |d| {
                                d.child(pro_mark_selected(&tab_sensor))
                            }),
                        cx,
                        |this, _, cx| {
                            this.tab = MemoryTab::Review;
                            let vis = this.visible_ids(cx);
                            if !vis.iter().any(|id| this.selected_id.as_ref() == Some(id)) {
                                this.selected_id = vis.first().cloned();
                                this.synced_panel_id = None;
                            }
                            cx.notify();
                        },
                    )),
            )
            .child(
                div()
                    .ml_auto()
                    .w(px(if compact_row { 132. } else { 204. }))
                    .min_w(px(96.))
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .bg(if search_focused { p.card } else { p.chip })
                    .border_1()
                    .border_color(if search_focused {
                        p.line_strong
                    } else {
                        transparent_black()
                    })
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_icon(ProIcon::Search, 14., p.faint))
                    .when_some(search_ed, |d, ed| {
                        d.child(div().flex_1().min_w_0().child(ed))
                    })
                    .when(!query.is_empty(), |d| {
                        d.child(activate(
                            div()
                                .id(pro_eid(&self.id, "clear-search"))
                                .role(Role::Button)
                                .aria_label(tr(cx, "Clear search", "清除搜索"))
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
                            |this, w, cx| {
                                if let Some(ref ed) = this.search_editor {
                                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                                }
                                cx.notify();
                            },
                        ))
                    }),
            )
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "remember-btn"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Remember", "添加记忆"))
                    .tab_index(0)
                    .h(px(28.))
                    .when(compact_row, |d| d.w(px(28.)).justify_center())
                    .when(!compact_row, |d| d.pl(px(8.)).pr(px(10.)).gap(px(6.)))
                    .flex_none()
                    .rounded(px(6.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .hover(move |s| s.border_color(p.line_strong))
                    .flex()
                    .items_center()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .cursor_pointer()
                    .child(pro_icon(ProIcon::Plus, 14., p.muted))
                    .when(!compact_row, |d| d.child(tr(cx, "Remember", "记住"))),
                cx,
                |this, w, cx| {
                    this.composer_open = !this.composer_open;
                    this.composer_scope_menu = false;
                    if this.composer_open
                        && let Some(ref ed) = this.composer_editor
                    {
                        {
                            let fh = ed.read(cx).focus.clone();
                            fh.focus(w, cx);
                        }
                    }
                    cx.notify();
                },
            ));

        let composer_body = self.render_composer(cx).into_any_element();
        let composer = pro_fold(
            pro_eid(&self.id, "composer-fold"),
            self.composer_open,
            48.,
            0.,
            move || composer_body,
        );

        // Body content: loading | empty | grouped tree
        let body: AnyElement = if self.loading {
            self.render_loading_list(skel_phase, cx).into_any_element()
        } else if self.empty {
            let empty_view = if self.tab == MemoryTab::Saved {
                pro_empty_state(
                    ProIcon::Memory,
                    tr(cx, "Nothing remembered yet", "尚无已保存记忆"),
                    tr(
                        cx,
                        "Memories come from your chats, decisions in the Inbox, files you share, or things you tell it directly.",
                        "记忆来自你的对话、收件箱决策、共享文件，或你直接告知智能体的偏好。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-remember"),
                        tr(cx, "Remember", "添加记忆").into(),
                        Box::new(|this: &mut Self, w: &mut Window, cx: &mut Context<Self>| {
                            this.composer_open = true;
                            if let Some(ref ed) = this.composer_editor {
                                {
                                    let fh = ed.read(cx).focus.clone();
                                    fh.focus(w, cx);
                                }
                            }
                            cx.notify();
                        }),
                    )),
                    cx,
                )
            } else {
                pro_empty_state(
                    ProIcon::Memory,
                    tr(cx, "Nothing to review", "暂无待审核记忆"),
                    tr(
                        cx,
                        "When your agent picks up something new on its own, it waits here for your OK.",
                        "当智能体在工作中自行习得新信息时，会在此等待你的确认。",
                    ),
                    None,
                    cx,
                )
            };
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child(empty_view)
                .into_any_element()
        } else {
            let grouped =
                filter_and_group_memories(&self.memories, &self.projects, self.tab, &query);
            if grouped.is_empty() {
                let empty_view = if !query.trim().is_empty() {
                    pro_empty_state(
                        ProIcon::Search,
                        tr(cx, "No matching memories", "未找到匹配的记忆"),
                        tr(
                            cx,
                            "Try another keyword or clear the search filter.",
                            "请可改其他关键词或清除搜索条件。",
                        ),
                        Some((
                            pro_eid(&self.id, "empty-clear-search"),
                            tr(cx, "Clear search", "清除搜索").into(),
                            Box::new(|this: &mut Self, w: &mut Window, cx: &mut Context<Self>| {
                                if let Some(ref ed) = this.search_editor {
                                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                                }
                                cx.notify();
                            }),
                        )),
                        cx,
                    )
                } else if self.tab == MemoryTab::Review {
                    pro_empty_state(
                        ProIcon::Check,
                        tr(cx, "Nothing to review", "全部审核完毕"),
                        tr(
                            cx,
                            "Everything your agent picked up on its own has been reviewed.",
                            "智能体自行习得的全部内容均已审核。",
                        ),
                        None,
                        cx,
                    )
                } else {
                    pro_empty_state(
                        ProIcon::Memory,
                        tr(cx, "Nothing remembered yet", "尚无已保存记忆"),
                        tr(
                            cx,
                            "Tell your agent what matters about your work and projects.",
                            "告诉智能体关于你的工作方式与项目约定。",
                        ),
                        None,
                        cx,
                    )
                };
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(empty_view)
                    .into_any_element()
            } else {
                let mut tree = div().p(px(8.)).pb(px(72.)).flex().flex_col().gap(px(14.));

                for (gi, (proj, items)) in grouped.into_iter().enumerate() {
                    let gkey: SharedString = proj.clone().unwrap_or_else(|| "__about_you__".into());
                    let gname: SharedString = proj
                        .clone()
                        .unwrap_or_else(|| tr(cx, "About you", "关于你").into());
                    let (g_icon, g_color) = if let Some(ref pname) = proj {
                        (ProIcon::Folder, self.project_color(pname, &p))
                    } else {
                        (ProIcon::User, p.purple)
                    };
                    let collapsed = self.collapsed_groups.contains(&gkey);
                    let gcount = items.len();
                    let gkey_toggle = gkey.clone();

                    let mut group_el = div().flex().flex_col().gap(px(2.)).child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("ghead-{gi}")))
                            .role(Role::Button)
                            .aria_label(format!("{gname}, {gcount}"))
                            .tab_index(0)
                            .h(px(34.))
                            .mb(px(4.))
                            .pl(px(12.))
                            .pr(px(8.))
                            .rounded(px(10.))
                            .bg(p.ink.opacity(0.045))
                            .hover(move |s| s.bg(p.ink.opacity(0.07)))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .cursor_pointer()
                            .child(pro_icon(g_icon, 14., g_color))
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(gname),
                            )
                            .child(
                                div()
                                    .min_w(px(20.))
                                    .h(px(18.))
                                    .px(px(5.))
                                    .rounded(px(6.))
                                    .bg(p.card)
                                    .border_1()
                                    .border_color(p.line)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .child(format!("{gcount}")),
                            )
                            .child(
                                div()
                                    .ml_auto()
                                    .w(px(20.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(pro_icon(
                                        if collapsed {
                                            ProIcon::ChevronRight
                                        } else {
                                            ProIcon::ChevronDown
                                        },
                                        14.,
                                        p.faint,
                                    )),
                            ),
                        cx,
                        move |this, _, cx| {
                            if !this.collapsed_groups.remove(&gkey_toggle) {
                                this.collapsed_groups.insert(gkey_toggle.clone());
                            }
                            cx.notify();
                        },
                    ));

                    let mut group_rows = div().flex().flex_col().gap(px(2.));
                    for item in items {
                        let selected = self.selected_id.as_ref() == Some(&item.id);
                        let sheen_t = active_sheen
                            .as_ref()
                            .filter(|(id, _)| id == &item.id)
                            .map(|(_, t)| *t);
                        let is_recalled = sheen_t.is_some();
                        let glyph_color = if let Some(t) = sheen_t {
                            let warm = (1.0 - (t - 0.42).abs() * 2.2).clamp(0., 1.);
                            if warm > 0.25 { p.purple } else { p.muted }
                        } else {
                            p.muted
                        };
                        let src_lbl = item.source.row_label(cx);
                        let uses_lbl = format_compact_uses(item.uses);
                        let time_lbl = item
                            .used_at
                            .clone()
                            .unwrap_or_else(|| item.source.at.clone());
                        let row_id = item.id.clone();
                        let keep_id = item.id.clone();
                        let drop_id = item.id.clone();
                        let purple_c = p.purple;

                        group_rows = group_rows.child(
                            div()
                                .relative()
                                .h(px(if compact_row { 52. } else { 40. }))
                                .rounded(px(8.))
                                .overflow_hidden()
                                .when(selected, |d| d.bg(p.chip))
                                .when_some(sheen_t, |d, t| {
                                    let eased = pro_ease(t);
                                    let fade = if t > 0.72 {
                                        ((1. - t) / 0.28).clamp(0., 1.)
                                    } else {
                                        1.0
                                    };
                                    d.child(
                                        canvas(
                                            |_, _, _| (),
                                            move |bounds, _, window, _| {
                                                let bw = f32::from(bounds.size.width);
                                                let bh = f32::from(bounds.size.height);
                                                let band_w = (bw * 0.44).max(80.);
                                                let x = -band_w + eased * (bw + band_w * 1.25);
                                                let left = bounds.left() + px(x);
                                                let right = left + px(band_w);
                                                let top = bounds.top();
                                                let bottom = top + px(bh);
                                                let mut pb = PathBuilder::fill();
                                                pb.move_to(point(left, top));
                                                pb.line_to(point(right, top));
                                                pb.line_to(point(right, bottom));
                                                pb.line_to(point(left, bottom));
                                                pb.close();
                                                if let Ok(path) = pb.build() {
                                                    window.paint_path(
                                                        path,
                                                        linear_gradient(
                                                            90.,
                                                            linear_color_stop(
                                                                purple_c.opacity(0.0),
                                                                0.0,
                                                            ),
                                                            linear_color_stop(
                                                                purple_c.opacity(0.24 * fade),
                                                                1.0,
                                                            ),
                                                        ),
                                                    );
                                                }
                                            },
                                        )
                                        .absolute()
                                        .inset_0(),
                                    )
                                })
                                .child(
                                    div()
                                        .size_full()
                                        .px(px(12.))
                                        .flex()
                                        .items_center()
                                        .gap(px(12.))
                                        .when(!selected, |d| d.hover(move |s| s.bg(p.hover)))
                                        .child(activate(
                                            div()
                                                .id(pro_eid(&self.id, format!("row-{}", item.id)))
                                                .role(Role::ListBoxOption)
                                                .aria_label(item.text.clone())
                                                .tab_index(0)
                                                .flex_1()
                                                .min_w_0()
                                                .h_full()
                                                .flex()
                                                .items_center()
                                                .gap(px(12.))
                                                .cursor_pointer()
                                                .child(
                                                    div()
                                                        .w(px(16.))
                                                        .flex_none()
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .child(pro_icon(
                                                            item.source.kind.icon(),
                                                            14.,
                                                            glyph_color,
                                                        )),
                                                )
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .flex()
                                                        .items_center()
                                                        .gap(px(6.))
                                                        .when(item.pinned, |d| {
                                                            d.child(pro_icon(
                                                                ProIcon::Pin,
                                                                12.,
                                                                p.blue,
                                                            ))
                                                        })
                                                        .child(
                                                            div()
                                                                .flex_1()
                                                                .min_w_0()
                                                                .truncate()
                                                                .text_size(px(13.))
                                                                .text_color(if item.review {
                                                                    p.text
                                                                } else {
                                                                    p.ink
                                                                })
                                                                .child(item.text.clone()),
                                                        ),
                                                )
                                                .when(!hide_source, |d| {
                                                    d.child(
                                                        div()
                                                            .w(px(92.))
                                                            .flex_none()
                                                            .truncate()
                                                            .text_right()
                                                            .text_size(px(12.))
                                                            .text_color(p.faint)
                                                            .child(src_lbl),
                                                    )
                                                })
                                                .when(!item.review && !compact_row, |d| {
                                                    d.child(
                                                        div()
                                                            .w(px(46.))
                                                            .flex_none()
                                                            .text_right()
                                                            .text_size(px(12.))
                                                            .text_color(if is_recalled {
                                                                p.ink
                                                            } else {
                                                                p.faint
                                                            })
                                                            .child(uses_lbl),
                                                    )
                                                })
                                                .when(!item.review, |d| {
                                                    d.child(
                                                        div()
                                                            .w(px(30.))
                                                            .flex_none()
                                                            .text_right()
                                                            .text_size(px(12.))
                                                            .text_color(if is_recalled {
                                                                p.ink
                                                            } else {
                                                                p.muted
                                                            })
                                                            .child(time_lbl),
                                                    )
                                                }),
                                            cx,
                                            move |this, _, cx| {
                                                this.select_memory(row_id.clone(), true, cx);
                                            },
                                        ))
                                        .when(item.review, |d| {
                                            d.child(
                                                div()
                                                    .flex_none()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(4.))
                                                    .child(activate(
                                                        div()
                                                            .id(pro_eid(
                                                                &self.id,
                                                                format!("row-drop-{}", item.id),
                                                            ))
                                                            .role(Role::Button)
                                                            .tab_index(0)
                                                            .h(px(24.))
                                                            .px(px(8.))
                                                            .rounded(px(6.))
                                                            .hover(move |s| {
                                                                s.bg(p.chip).text_color(p.ink)
                                                            })
                                                            .flex()
                                                            .items_center()
                                                            .text_size(px(12.))
                                                            .text_color(p.text)
                                                            .cursor_pointer()
                                                            .child(tr(cx, "Don't keep", "不保留")),
                                                        cx,
                                                        move |this, _, cx| {
                                                            this.forget_memory(&drop_id, cx);
                                                        },
                                                    ))
                                                    .child(activate(
                                                        div()
                                                            .id(pro_eid(
                                                                &self.id,
                                                                format!("row-keep-{}", item.id),
                                                            ))
                                                            .role(Role::Button)
                                                            .tab_index(0)
                                                            .h(px(24.))
                                                            .px(px(8.))
                                                            .rounded(px(6.))
                                                            .bg(p.card)
                                                            .border_1()
                                                            .border_color(p.line_strong)
                                                            .hover(move |s| s.bg(p.hover))
                                                            .flex()
                                                            .items_center()
                                                            .text_size(px(12.))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(p.ink)
                                                            .cursor_pointer()
                                                            .child(tr(cx, "Keep", "保留")),
                                                        cx,
                                                        move |this, _, cx| {
                                                            this.keep_memory(&keep_id, cx);
                                                        },
                                                    )),
                                            )
                                        }),
                                ),
                        );
                    }
                    let row_h = if compact_row { 52. } else { 40. };
                    let est_h = (gcount as f32 * (row_h + 2.) - 2.).max(row_h);
                    let rows_el = group_rows.into_any_element();
                    group_el = group_el.child(pro_fold(
                        pro_eid(&self.id, format!("grp-fold-{gkey}")),
                        !collapsed,
                        est_h,
                        2.,
                        move || rows_el,
                    ));

                    tree = tree.child(group_el);
                }

                tree.into_any_element()
            }
        };
        let body = if self.loading {
            body
        } else {
            pro_view_transition(
                pro_eid(&self.id, "mem-list-vtx"),
                format!("{:?}-{}", self.tab, self.empty),
                body,
            )
        };

        // Floating undo banner (.ui-am-undo)
        let undo_open = self.undo_state.is_some();
        let undo_msg = self
            .undo_state
            .as_ref()
            .map(|(_, m)| m.clone())
            .or_else(|| self.last_undo_msg.clone())
            .unwrap_or_default();
        let undo_btn_el = activate(
            div()
                .id(pro_eid(&self.id, "undo-btn"))
                .role(Role::Button)
                .tab_index(0)
                .h(px(28.))
                .px(px(10.))
                .rounded(px(6.))
                .hover(move |s| s.bg(p.card.opacity(0.14)))
                .flex()
                .items_center()
                .font_weight(FontWeight::MEDIUM)
                .cursor_pointer()
                .child(tr(cx, "Undo", "撤销")),
            cx,
            |this, _, cx| this.apply_undo(cx),
        );
        let undo_bar = pro_presence(
            pro_eid(&self.id, "undo-bar-pres"),
            undo_open,
            0.20,
            0.16,
            move |amount| {
                div()
                    .absolute()
                    .bottom(px(16. - 8. * (1. - amount)))
                    .left_0()
                    .right_0()
                    .opacity(amount)
                    .flex()
                    .justify_center()
                    .px(px(16.))
                    .child(
                        div()
                            .max_w(px(420.))
                            .h(px(36.))
                            .pl(px(12.))
                            .pr(px(4.))
                            .rounded(px(8.))
                            .bg(p.ink)
                            .text_color(p.card)
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(13.))
                            .child(div().flex_1().min_w_0().truncate().child(undo_msg))
                            .child(undo_btn_el),
                    )
                    .into_any_element()
            },
        );

        let measure_cell = self.measured_width.clone();

        div()
            .id(pro_eid(&self.id, "page"))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| this.handle_key(ev, w, cx)))
            .relative()
            .size_full()
            .bg(p.panel)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                canvas(
                    move |b, window, _| {
                        let nw = f32::from(b.size.width);
                        if nw > 10. && (measure_cell.get() - nw).abs() > 1.0 {
                            measure_cell.set(nw);
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| {},
                )
                .w_full()
                .h(px(0.)),
            )
            .child(head)
            .child(composer)
            .child(
                div()
                    .id(pro_eid(&self.id, "scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .child(body),
            )
            .child(undo_bar)
    }

    fn render_inspector(
        &mut self,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SharedString, ProIcon, AnyElement)> {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);

        if self.loading {
            let phase = (self.boot_at.elapsed().as_secs_f32() * 0.85) % 1.0;
            let skel_body = div()
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(20.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(pro_skeleton_bar(220., 14., 4., phase, cx))
                        .child(pro_skeleton_bar(140., 14., 4., phase + 0.1, cx)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(pro_skeleton_bar(180., 12., 4., phase, cx))
                        .child(pro_skeleton_bar(210., 12., 4., phase + 0.08, cx))
                        .child(pro_skeleton_bar(168., 12., 4., phase + 0.16, cx)),
                )
                .child(pro_skeleton_bar(240., 68., 10., phase, cx))
                .child(pro_skeleton_bar(240., 84., 10., phase + 0.12, cx))
                .into_any_element();
            return Some((tr(cx, "Memory", "记忆").into(), ProIcon::Memory, skel_body));
        }

        if self.empty {
            return None;
        }

        let sel_id = self.selected_id.clone()?;
        let item = self.memories.iter().find(|m| m.id == sel_id)?.clone();

        let panel_ed = self.panel_editor.clone();
        let panel_ed_focused = panel_ed
            .as_ref()
            .is_some_and(|e| e.read(cx).focus.is_focused(w));

        // Scope picker button + optional inline dropdown
        let scope_name: SharedString = item
            .project
            .clone()
            .unwrap_or_else(|| tr(cx, "Everywhere", "全局适用").into());
        let (scope_ic, scope_ic_color) = if let Some(ref proj) = item.project {
            (ProIcon::Folder, self.project_color(proj, &p))
        } else {
            (ProIcon::Globe, p.muted)
        };

        let scope_dropdown = {
            let mem_id = item.id.clone();
            let is_everywhere = item.project.is_none();
            let mut list = div()
                .mt(px(4.))
                .p(px(4.))
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .flex_col()
                .gap(px(1.));
            let mid_ev = mem_id.clone();
            list = list.child(activate(
                div()
                    .id(pro_eid(&self.id, "panel-scope-everywhere"))
                    .role(Role::MenuItem)
                    .tab_index(0)
                    .h(px(30.))
                    .px(px(8.))
                    .rounded(px(7.))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.5))
                    .text_color(if is_everywhere { p.ink } else { p.text })
                    .cursor_pointer()
                    .child(pro_icon(ProIcon::Globe, 14., p.muted))
                    .child(div().flex_1().child(tr(cx, "Everywhere", "全局适用")))
                    .when(is_everywhere, |d| {
                        d.child(pro_icon(ProIcon::Check, 13., p.ink))
                    }),
                cx,
                move |this, _, cx| this.set_memory_scope(&mid_ev, None, cx),
            ));
            for (pi, proj) in self.projects.clone().into_iter().enumerate() {
                let checked = item.project.as_ref() == Some(&proj);
                let p_color = self.project_color(&proj, &p);
                let mid_p = mem_id.clone();
                let proj_c = proj.clone();
                list = list.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("panel-scope-{pi}")))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(30.))
                        .px(px(8.))
                        .rounded(px(7.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(12.5))
                        .text_color(if checked { p.ink } else { p.text })
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Folder, 14., p_color))
                        .child(div().flex_1().truncate().child(proj))
                        .when(checked, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink))),
                    cx,
                    move |this, _, cx| this.set_memory_scope(&mid_p, Some(proj_c.clone()), cx),
                ));
            }
            let est_scope_h = ((1 + self.projects.len()) as f32 * 31. + 12.).max(68.);
            let list_el = list.into_any_element();
            pro_fold(
                pro_eid(&self.id, format!("panel-scope-fold-{}", item.id)),
                self.panel_scope_menu,
                est_scope_h,
                2.,
                move || list_el,
            )
        };

        let learned_txt = item.source.learned_label(cx);
        let learned_at = SharedString::from(format!("· {}", item.source.at));

        let used_primary: SharedString = if item.uses == 0 {
            tr(cx, "Not used yet", "尚未使用").into()
        } else if item.uses == 1 {
            tr(cx, "1 time", "1 次").into()
        } else {
            SharedString::from(format!("{} {}", item.uses, tr(cx, "times", "次")))
        };
        let used_secondary: Option<SharedString> = item
            .used_at_long
            .as_ref()
            .map(|last| SharedString::from(format!("· {} {last}", tr(cx, "last", "最近"))));

        let props_dl = div()
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
                            .w(px(84.))
                            .flex_none()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(tr(cx, "Applies to", "适用范围")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .ml(px(-8.))
                            .flex()
                            .items_center()
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, "panel-scope-btn"))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .h(px(28.))
                                    .px(px(8.))
                                    .rounded(px(6.))
                                    .hover(move |s| s.bg(p.chip))
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(13.))
                                    .text_color(p.ink)
                                    .cursor_pointer()
                                    .child(pro_icon(scope_ic, 14., scope_ic_color))
                                    .child(div().truncate().child(scope_name))
                                    .child(pro_icon(ProIcon::ChevronDown, 12., p.faint)),
                                cx,
                                |this, _, cx| {
                                    this.panel_scope_menu = !this.panel_scope_menu;
                                    cx.notify();
                                },
                            )),
                    ),
            )
            .child(scope_dropdown)
            .child(
                div()
                    .min_h(px(28.))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w(px(84.))
                            .flex_none()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(tr(cx, "Learned", "学习来源")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(13.))
                            .child(pro_icon(item.source.kind.icon(), 14., p.muted))
                            .child(div().truncate().text_color(p.ink).child(learned_txt))
                            .child(div().truncate().text_color(p.faint).child(learned_at)),
                    ),
            )
            .child(
                div()
                    .min_h(px(28.))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w(px(84.))
                            .flex_none()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(tr(cx, "Used", "使用次数")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .text_size(px(13.))
                            .child(
                                div()
                                    .text_color(if item.uses == 0 { p.muted } else { p.ink })
                                    .child(used_primary),
                            )
                            .when_some(used_secondary, |d, sec| {
                                d.child(div().truncate().text_color(p.faint).child(sec))
                            }),
                    ),
            );

        // Where it came from (.ui-am-quote)
        let open_item = item.clone();
        let foot_lbl = item.source.foot_label(cx);
        let foot_at = SharedString::from(format!("· {}", item.source.at));
        let quote_section = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(tr(cx, "Where it came from", "记忆出处")),
            )
            .child(
                div()
                    .mx(px(-6.))
                    .pt(px(10.))
                    .px(px(10.))
                    .pb(px(6.))
                    .rounded(px(10.))
                    .bg(p.ink.opacity(0.03))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .when_some(item.source.question.clone(), |d, q| {
                        d.child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(q),
                        )
                    })
                    .when_some(item.source.quote.clone(), |d, qt| {
                        d.child(
                            div()
                                .flex()
                                .items_stretch()
                                .gap(px(9.))
                                .child(
                                    div()
                                        .w(px(2.))
                                        .flex_none()
                                        .rounded(px(1.))
                                        .bg(p.line_strong),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_size(px(13.))
                                        .line_height(px(20.))
                                        .text_color(p.ink)
                                        .child(qt),
                                ),
                        )
                    })
                    .child(
                        div()
                            .min_h(px(26.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .child(pro_icon(item.source.kind.icon(), 13., p.muted))
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .text_color(p.muted)
                                    .child(foot_lbl),
                            )
                            .child(div().flex_none().text_color(p.faint).child(foot_at))
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, "open-source"))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .ml_auto()
                                    .mr(px(-4.))
                                    .h(px(24.))
                                    .px(px(6.))
                                    .rounded(px(6.))
                                    .hover(move |s| s.bg(p.chip).text_color(p.ink))
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .gap(px(3.))
                                    .text_color(p.muted)
                                    .cursor_pointer()
                                    .child(tr(cx, "Open", "打开"))
                                    .child(pro_icon(ProIcon::ChevronRight, 12., p.faint)),
                                cx,
                                move |_, _, cx| {
                                    cx.emit(AgentMemoryEvent::OpenSource(open_item.clone()));
                                },
                            )),
                    ),
            );

        // Where it was used (.ui-am-uses)
        let uses_count = item.used_in.len();
        let used_section = div()
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
                    .child(tr(cx, "Where it was used", "使用记录"))
                    .when(uses_count > 0, |d| {
                        d.child(div().text_color(p.faint).child(format!("{uses_count}")))
                    }),
            )
            .child(if item.used_in.is_empty() {
                div()
                    .text_size(px(13.))
                    .text_color(p.muted)
                    .child(tr(
                        cx,
                        "Hasn't been recalled in a task yet.",
                        "尚未在具体任务中被调用。",
                    ))
                    .into_any_element()
            } else {
                div()
                    .mx(px(-6.))
                    .px(px(10.))
                    .py(px(2.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .children(item.used_in.iter().enumerate().map(|(ui, rec)| {
                        div()
                            .py(px(8.))
                            .when(ui > 0, |d| d.border_t_1().border_color(p.line_soft))
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(
                                div()
                                    .w(px(16.))
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(pro_icon(ProIcon::Tasks, 13., p.muted)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .truncate()
                                            .text_size(px(13.))
                                            .line_height(px(20.))
                                            .text_color(p.ink)
                                            .child(rec.task.clone()),
                                    )
                                    .child(
                                        div()
                                            .truncate()
                                            .text_size(px(12.))
                                            .line_height(px(16.))
                                            .text_color(p.muted)
                                            .child(format!("{} · {}", rec.how, rec.at)),
                                    ),
                            )
                    }))
                    .into_any_element()
            });

        // Panel bottom actions (.ui-am-panel__acts)
        let forget_id = item.id.clone();
        let keep_id = item.id.clone();
        let pin_id = item.id.clone();
        let is_pinned = item.pinned;
        let acts = if item.review {
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(6.))
                .child(pro_btn(
                    pro_eid(&self.id, "panel-dont-keep"),
                    tr(cx, "Don't keep", "不保留"),
                    None,
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    move |this, _, cx| this.forget_memory(&forget_id, cx),
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "panel-keep"))
                        .role(Role::Button)
                        .tab_index(0)
                        .h(px(28.))
                        .px(px(12.))
                        .rounded(px(6.))
                        .bg(p.ink)
                        .text_color(p.card)
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .child(tr(cx, "Keep", "保留")),
                    cx,
                    move |this, _, cx| this.keep_memory(&keep_id, cx),
                ))
        } else {
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(6.))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "panel-forget"))
                        .role(Role::Button)
                        .tab_index(0)
                        .ml(px(-8.))
                        .h(px(28.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .hover(move |s| s.bg(p.chip).text_color(p.red))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.text)
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Trash, 14., p.muted))
                        .child(tr(cx, "Forget", "遗忘")),
                    cx,
                    move |this, _, cx| this.forget_memory(&forget_id, cx),
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "panel-pin"))
                        .role(Role::Button)
                        .aria_label(if is_pinned {
                            tr(cx, "Unpin memory", "取消置顶")
                        } else {
                            tr(cx, "Pin memory", "置顶记忆")
                        })
                        .tab_index(0)
                        .h(px(28.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .when(is_pinned, |d| d.bg(p.chip))
                        .hover(move |s| s.bg(p.hover).text_color(p.ink))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(12.5))
                        .text_color(if is_pinned { p.ink } else { p.muted })
                        .cursor_pointer()
                        .child(pro_icon(
                            ProIcon::Pin,
                            13.,
                            if is_pinned { p.blue } else { p.faint },
                        ))
                        .child(if is_pinned {
                            tr(cx, "Pinned", "已置顶")
                        } else {
                            tr(cx, "Pin", "置顶")
                        }),
                    cx,
                    move |this, _, cx| this.toggle_pin(&pin_id, cx),
                ))
        };

        let body = div()
            .pt(px(12.))
            .px(px(14.))
            .pb(px(20.))
            .flex()
            .flex_col()
            .gap(px(20.))
            .when(item.review, |d| {
                d.child(
                    div()
                        .flex()
                        .items_start()
                        .gap(px(8.))
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(p.muted)
                        .child(pro_icon(ProIcon::Sparkles, 13., p.amber_ink))
                        .child(tr(
                            cx,
                            "Picked up while working – waits for your OK before it uses this.",
                            "在工作中自行习得——经你确认后才会生效。",
                        )),
                )
            })
            .child(
                div()
                    .mx(px(-8.))
                    .px(px(8.))
                    .py(px(6.))
                    .rounded(px(8.))
                    .bg(if panel_ed_focused {
                        p.card
                    } else {
                        transparent_black()
                    })
                    .border_1()
                    .border_color(if panel_ed_focused {
                        p.line_strong
                    } else {
                        transparent_black()
                    })
                    .when(!panel_ed_focused, |d| d.hover(move |s| s.bg(p.hover)))
                    .text_color(p.ink)
                    .font_weight(FontWeight::MEDIUM)
                    .when_some(panel_ed, |d, ed| d.child(ed)),
            )
            .child(props_dl)
            .child(quote_section)
            .child(used_section)
            .child(acts);
        let body = pro_view_transition(pro_eid(&self.id, "mem-insp-vtx"), sel_id, body);

        Some((item.text, ProIcon::Memory, body))
    }
}

impl Render for AgentMemoryScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgentMemoryItem, MemorySource, MemoryTab, default_agent_memories, default_memory_projects,
        filter_and_group_memories, format_compact_uses,
    };

    #[test]
    fn default_memories_partition_into_eleven_saved_and_two_review() {
        let memories = default_agent_memories();
        let projects = default_memory_projects();

        let saved_groups = filter_and_group_memories(&memories, &projects, MemoryTab::Saved, "");
        let saved_total: usize = saved_groups.iter().map(|(_, v)| v.len()).sum();
        assert_eq!(saved_total, 11);
        assert_eq!(saved_groups.len(), 4);
        assert_eq!(saved_groups[0].0, None);
        assert_eq!(saved_groups[0].1.len(), 5);
        assert_eq!(saved_groups[1].0.as_deref(), Some("Northwind"));
        assert_eq!(saved_groups[1].1.len(), 3);

        let review_groups = filter_and_group_memories(&memories, &projects, MemoryTab::Review, "");
        let review_total: usize = review_groups.iter().map(|(_, v)| v.len()).sum();
        assert_eq!(review_total, 2);
    }

    #[test]
    fn search_and_pinning_filter_and_order_group_entries() {
        let mut memories = default_agent_memories();
        let projects = default_memory_projects();

        // Pin the last "About you" item and verify it rises to index 0 of its group.
        if let Some(m) = memories
            .iter_mut()
            .find(|m| m.id.as_ref() == "british-english")
        {
            m.pinned = true;
        }
        let grouped = filter_and_group_memories(&memories, &projects, MemoryTab::Saved, "");
        assert_eq!(grouped[0].1[0].id.as_ref(), "british-english");

        // Search by source file name matches "v1-traffic.csv" under Platform.
        let by_file =
            filter_and_group_memories(&memories, &projects, MemoryTab::Saved, "v1-traffic");
        assert_eq!(by_file.len(), 1);
        assert_eq!(by_file[0].0.as_deref(), Some("Platform"));
        assert_eq!(by_file[0].1[0].id.as_ref(), "api-v1-clients");

        // Compact use count formatting
        assert_eq!(format_compact_uses(6), "6 ×");
        assert_eq!(format_compact_uses(12_500), "12.5K ×");
        let custom = AgentMemoryItem::new("x", "Test fact", MemorySource::told("now"));
        assert!(!custom.review);
    }
}
