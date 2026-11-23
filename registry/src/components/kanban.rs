//! Priority-grouped issue board with list view, filters, inline issue detail, and drag reordering.
//!
//! Renders [`KanbanScreen`] inside [`AppShell`] (`"Acme"` workspace, `"Board"` active),
//! organizing issues across `Low priority`, `Medium priority`, `High priority`, and
//! `Completed` columns (or grouped list rows) with scope tabs, filter and sort menus,
//! pointer and keyboard card reordering, a New issue modal, and an in-place issue
//! inspector with editable title/description, properties, sub-issues, and activity log.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_agent_avatar, pro_btn, pro_eid, pro_empty_state, pro_fold,
            pro_icon_btn, pro_mark_selected, pro_person_avatar, pro_pill_indicator, pro_presence,
            pro_selection_sensor, pro_skeleton_bar, pro_spring_bar, pro_switch,
            pro_view_transition_fill, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, EditorShortcutEvent, InputMode},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const SVG_FUNNEL: &str = r#"<path d="M8.85746 12.5061C6.36901 10.6456 4.59564 8.59915 3.62734 7.44867C3.3276 7.09253 3.22938 6.8319 3.17033 6.3728C2.96811 4.8008 2.86701 4.0148 3.32795 3.5074C3.7889 3 4.60404 3 6.23433 3H17.7657C19.396 3 20.2111 3 20.672 3.5074C21.133 4.0148 21.0319 4.8008 20.8297 6.37281C20.7706 6.83191 20.6724 7.09254 20.3726 7.44867C19.403 8.60062 17.6261 10.6507 15.1326 12.5135C14.907 12.6821 14.7583 12.9567 14.7307 13.2614C14.4837 15.992 14.2559 17.4876 14.1141 18.2442C13.8853 19.4657 12.1532 20.2006 11.226 20.8563C10.6741 21.2466 10.0043 20.782 9.93278 20.1778C9.79643 19.0261 9.53961 16.6864 9.25927 13.2614C9.23409 12.9539 9.08486 12.6761 8.85746 12.5061Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_SORT_AZ: &str = r#"<path d="M15.9367 2L16.417 2.45852C16.8461 2.8682 17.0606 3.07305 16.9851 3.2471C16.9096 3.42115 16.6062 3.42115 15.9993 3.42115H8.56617C5.49108 3.42115 2.88649 5.34451 2 8" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M8.02139 22L7.58291 21.5792C7.15391 21.1674 6.93942 20.9616 7.01491 20.7866C7.0904 20.6117 7.39375 20.6117 8.00043 20.6117H15.432C18.5102 20.6117 21.1169 18.6739 22 16" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M15 9.5H17.9474C18.5675 9.5 18.8775 9.5 18.9601 9.70009C19.0427 9.90019 18.8317 10.1402 18.4098 10.6203L15.9846 13.3797C15.5627 13.8598 15.3517 14.0998 15.4343 14.2999C15.5169 14.5 15.8269 14.5 16.447 14.5H19" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M3 14.5L4.75464 10.5877C5.07987 9.86258 5.24249 9.5 5.5 9.5C5.75751 9.5 5.92013 9.86258 6.24536 10.5877L8 14.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M11 12H12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_STAR_FILLED: &str = r#"<path d="M13.7276 3.44418L15.4874 6.99288C15.7274 7.48687 16.3673 7.9607 16.9073 8.05143L20.0969 8.58575C22.1367 8.92853 22.6167 10.4206 21.1468 11.8925L18.6671 14.3927C18.2471 14.8161 18.0172 15.6327 18.1471 16.2175L18.8571 19.3125C19.417 21.7623 18.1271 22.71 15.9774 21.4296L12.9877 19.6452C12.4478 19.3226 11.5579 19.3226 11.0079 19.6452L8.01827 21.4296C5.8785 22.71 4.57865 21.7522 5.13859 19.3125L5.84851 16.2175C5.97849 15.6327 5.74852 14.8161 5.32856 14.3927L2.84884 11.8925C1.389 10.4206 1.85895 8.92853 3.89872 8.58575L7.08837 8.05143C7.61831 7.9607 8.25824 7.48687 8.49821 6.99288L10.258 3.44418C11.2179 1.51861 12.7777 1.51861 13.7276 3.44418Z" fill="currentColor" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_COMMENT_BUBBLE: &str = r#"<path d="M21.5 12C21.5 17.2467 17.2467 21.5 12 21.5C10.3719 21.5 8.8394 21.0904 7.5 20.3687C5.63177 19.362 4.37462 20.2979 3.26592 20.4658C3.09774 20.4913 2.93024 20.4302 2.80997 20.31C2.62741 20.1274 2.59266 19.8451 2.6935 19.6074C3.12865 18.5818 3.5282 16.6382 2.98341 15C2.6698 14.057 2.5 13.0483 2.5 12C2.5 6.75329 6.75329 2.5 12 2.5C17.2467 2.5 21.5 6.75329 21.5 12Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M12.1257 12H12.0007M8.125 12H8M16.125 12H16" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2.2"></path>"#;

const SVG_PAPERCLIP_META: &str = r#"<path d="M5.82338 12L4.27922 10.4558C2.57359 8.75022 2.57359 5.98485 4.27922 4.27922C5.98485 2.57359 8.75022 2.57359 10.4558 4.27922L19.7208 13.5442C21.4264 15.2498 21.4264 18.0152 19.7208 19.7208C18.0152 21.4264 15.2498 21.4264 13.5442 19.7208L10.0698 16.2464C9.00379 15.1804 9.00379 13.4521 10.0698 12.386C11.1358 11.32 12.8642 11.32 13.9302 12.386L15.8604 14.3162" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_STATUS_REVIEW: &str = r#"<circle cx="7" cy="7" r="5.4" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.3"></circle><path d="M7 3.4A3.6 3.6 0 0 1 7 10.6Z" fill="currentColor"></path>"#;

const SVG_STATUS_PROGRESS: &str = r#"<circle cx="7" cy="7" r="5.4" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.3"></circle><path d="M7 3.4A3.6 3.6 0 0 1 10.6 7H7Z" fill="currentColor"></path>"#;

const SVG_STATUS_ATTENTION: &str = r#"<path d="M7 2.2 12.4 11.4H1.6Z" fill="currentColor" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"></path>"#;

const SVG_STATUS_FIXED: &str = r#"<circle cx="7" cy="7" r="3.5" fill="currentColor"></circle>"#;

const SVG_MARK_DONE: &str = r#"<path d="M3.5 8.5l3 3 6-7" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"></path>"#;

const SVG_CHECK_TICK: &str = r#"<path d="M5 13.5L9 17.5L19 6.5" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round"></path>"#;

const SVG_SUB_DONE: &str = r##"<circle cx="8" cy="8" r="7" fill="currentColor"></circle><path d="M4.8 8.2l2.1 2.1 4.3-4.6" fill="none" stroke="#ffffff" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"></path>"##;

const ASSIGNEES: [(&str, bool); 5] = [
    ("Alex Morgan", false),
    ("Maya Lin", false),
    ("Sam Rivera", false),
    ("Devon Park", false),
    ("Triage Agent", true),
];

const ALL_LABELS: [(&str, u32); 6] = [
    ("Auth", 0x4fb0fe),
    ("Billing", 0xf04438),
    ("Performance", 0xe8b000),
    ("Mobile", 0xa651f0),
    ("Infra", 0x4fc3b4),
    ("Settings", 0x7dcf9f),
];

const PROJECTS: [&str; 4] = ["Overview", "Authentication", "Checkout v2", "Analytics"];

/// Active presentation layout for [`KanbanScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KanbanViewMode {
    /// Multi-column card board grouped by priority/completion stage.
    #[default]
    Board,
    /// Sticky-headed grouped list rows with issue key, title, detail, and status.
    List,
}

/// Top toolbar scope tab on [`KanbanScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KanbanScope {
    /// Show every issue across open and completed columns.
    #[default]
    All,
    /// Show only open issues (`tone != Done`).
    Open,
    /// Show only completed issues (`tone == Done`).
    Completed,
}

/// Ordering rule applied to cards inside each column or list group.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KanbanSort {
    /// Manual / board order as arranged by drag or keyboard move.
    #[default]
    Manual,
    /// Alphabetical by issue title.
    Title,
    /// Descending by comment count.
    Comments,
    /// Descending by status urgency (`Attention` -> `Progress` -> `Review` -> `Fixed`).
    Status,
}

/// Visual priority/completion mark for a [`KanbanColumn`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KanbanTone {
    /// One active blue signal bar (`Low priority`).
    #[default]
    Low,
    /// Two active amber signal bars (`Medium priority`).
    Medium,
    /// Three active red signal bars (`High priority`).
    High,
    /// Green completion checkmark (`Completed`).
    Done,
}

impl KanbanTone {
    /// Return whether this column represents completed issues.
    pub fn is_done(self) -> bool {
        matches!(self, Self::Done)
    }

    fn active_bars(self) -> u8 {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Done => 0,
        }
    }

    fn bar_color(self, p: &ProPalette) -> Hsla {
        match self {
            Self::Low => p.blue,
            Self::Medium => p.amber,
            Self::High => p.red,
            Self::Done => p.green_ink,
        }
    }
}

/// Workflow review/triage state badge shown on the footer of a [`KanbanCard`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KanbanStatus {
    /// Ready for review (blue half-circle).
    #[default]
    Review,
    /// Needs attention (amber warning triangle).
    Attention,
    /// In progress (purple quarter-circle).
    Progress,
    /// Fixed (green dot).
    Fixed,
}

impl KanbanStatus {
    const ALL: [Self; 4] = [Self::Review, Self::Attention, Self::Progress, Self::Fixed];

    /// Localized display label for this status.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Review => tr(cx, "Ready for review", "待评审"),
            Self::Attention => tr(cx, "Needs attention", "需关注"),
            Self::Progress => tr(cx, "In progress", "进行中"),
            Self::Fixed => tr(cx, "Fixed", "已修复"),
        }
    }

    fn color(self, p: &ProPalette) -> Hsla {
        match self {
            Self::Review => p.blue,
            Self::Attention => p.amber,
            Self::Progress => p.purple,
            Self::Fixed => p.green_ink,
        }
    }

    fn urgency_rank(self) -> u8 {
        match self {
            Self::Attention => 0,
            Self::Progress => 1,
            Self::Review => 2,
            Self::Fixed => 3,
        }
    }
}

/// Column definition on [`KanbanScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KanbanColumn {
    /// Stable column identifier referenced by `KanbanCard::column`.
    pub id: SharedString,
    /// Human-readable column heading (`Low priority`, `Medium priority`, `High priority`, `Completed`).
    pub label: SharedString,
    /// Priority signal bar or completion checkmark style.
    pub tone: KanbanTone,
}

impl KanbanColumn {
    /// Construct a board column with `id`, display `label`, and `tone`.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        tone: KanbanTone,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            tone,
        }
    }
}

/// Checkable sub-issue item inside an open [`KanbanCard`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KanbanSubIssue {
    /// Stable sub-issue identifier.
    pub id: SharedString,
    /// Checklist item description.
    pub title: SharedString,
    /// Whether this sub-issue is completed.
    pub done: bool,
}

impl KanbanSubIssue {
    /// Construct a sub-issue checklist item.
    pub fn new(id: impl Into<SharedString>, title: impl Into<SharedString>, done: bool) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            done,
        }
    }
}

/// Entry kind in an issue's activity log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KanbanActivityKind {
    /// Compact one-line status/property change audit event.
    Event,
    /// Elevated comment card with author avatar and paragraph body.
    Comment,
}

/// Activity event or user/agent comment attached to a [`KanbanCard`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KanbanActivity {
    /// Stable activity entry identifier.
    pub id: SharedString,
    /// Person or agent display name.
    pub author: SharedString,
    /// Whether `author` renders with a 3x3 pixel agent avatar.
    pub is_agent: bool,
    /// Event description or comment text.
    pub body: SharedString,
    /// Relative timestamp label (`"14m ago"`, `"Just now"`).
    pub timestamp: SharedString,
    /// Presentation kind (`Event` vs `Comment`).
    pub kind: KanbanActivityKind,
}

impl KanbanActivity {
    /// Construct a one-line audit event entry.
    pub fn event(
        id: impl Into<SharedString>,
        author: impl Into<SharedString>,
        is_agent: bool,
        body: impl Into<SharedString>,
        timestamp: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            author: author.into(),
            is_agent,
            body: body.into(),
            timestamp: timestamp.into(),
            kind: KanbanActivityKind::Event,
        }
    }

    /// Construct a rich comment card entry.
    pub fn comment(
        id: impl Into<SharedString>,
        author: impl Into<SharedString>,
        is_agent: bool,
        body: impl Into<SharedString>,
        timestamp: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            author: author.into(),
            is_agent,
            body: body.into(),
            timestamp: timestamp.into(),
            kind: KanbanActivityKind::Comment,
        }
    }
}

/// Issue card displayed on [`KanbanScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KanbanCard {
    /// Unique card identifier.
    pub id: SharedString,
    /// Short issue key shown in list rows and breadcrumbs (e.g. `"NOR-142"`).
    pub key: SharedString,
    /// Column id this card belongs to (`"low"`, `"medium"`, `"high"`, `"done"`).
    pub column: SharedString,
    /// Concise issue summary.
    pub title: SharedString,
    /// Two-line card preview and editable detail body.
    pub description: SharedString,
    /// Current workflow status mark and label.
    pub status: KanbanStatus,
    /// Assigned team member or agent name (`None` when unassigned).
    pub assignee: Option<SharedString>,
    /// Whether the assignee is an autonomous agent (`pro_agent_avatar`).
    pub assignee_is_agent: bool,
    /// Issue labels (`"Auth"`, `"Billing"`, `"Performance"`, etc.).
    pub labels: Vec<SharedString>,
    /// Parent project name (`"Overview"`, `"Authentication"`, etc.).
    pub project: SharedString,
    /// Comment count badge displayed on the card footer.
    pub comments: u32,
    /// Attachment count badge displayed on the card footer.
    pub attachments: u32,
    /// Sub-issues editable in the issue detail view.
    pub sub_issues: Vec<KanbanSubIssue>,
    /// Chronological activity and comments log.
    pub activity: Vec<KanbanActivity>,
    /// Relative creation timestamp shown in the properties footer.
    pub created_at: SharedString,
    /// Relative update timestamp shown in the properties footer.
    pub updated_at: SharedString,
}

impl KanbanCard {
    /// Construct a card in `column` with `title` and `status`.
    pub fn new(
        id: impl Into<SharedString>,
        key: impl Into<SharedString>,
        column: impl Into<SharedString>,
        title: impl Into<SharedString>,
        status: KanbanStatus,
    ) -> Self {
        Self {
            id: id.into(),
            key: key.into(),
            column: column.into(),
            title: title.into(),
            description: SharedString::default(),
            status,
            assignee: Some("Alex Morgan".into()),
            assignee_is_agent: false,
            labels: Vec::new(),
            project: "Overview".into(),
            comments: 0,
            attachments: 0,
            sub_issues: Vec::new(),
            activity: Vec::new(),
            created_at: "3d ago".into(),
            updated_at: "18m ago".into(),
        }
    }

    /// Attach body description copy.
    pub fn with_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the comment and attachment counts shown on the card footer.
    pub fn with_counts(mut self, comments: u32, attachments: u32) -> Self {
        self.comments = comments;
        self.attachments = attachments;
        self
    }

    /// Assign a person or agent to the card.
    pub fn with_assignee(mut self, name: Option<SharedString>, is_agent: bool) -> Self {
        self.assignee = name;
        self.assignee_is_agent = is_agent;
        self
    }

    /// Attach category labels to the card.
    pub fn with_labels(mut self, labels: Vec<SharedString>) -> Self {
        self.labels = labels;
        self
    }

    /// Set the parent project name.
    pub fn with_project(mut self, project: impl Into<SharedString>) -> Self {
        self.project = project.into();
        self
    }

    /// Populate initial sub-issues.
    pub fn with_sub_issues(mut self, sub_issues: Vec<KanbanSubIssue>) -> Self {
        self.sub_issues = sub_issues;
        self
    }

    /// Populate initial activity events and comments.
    pub fn with_activity(mut self, activity: Vec<KanbanActivity>) -> Self {
        self.activity = activity;
        self
    }
}

/// Events emitted by [`KanbanScreen`].
#[derive(Clone, Debug)]
pub enum KanbanEvent {
    /// User clicked a sidebar navigation item.
    Navigate(SharedString),
    /// User opened an issue card for inspection.
    SelectCard(SharedString),
    /// User closed the open issue detail view back to the board/list.
    CloseCard,
    /// User created a new issue from the composer modal.
    CreateCard(KanbanCard),
    /// User moved a card to another column or reordered it.
    MoveCard {
        id: SharedString,
        from_column: SharedString,
        to_column: SharedString,
    },
    /// User edited a card's title, description, or properties.
    UpdateCard(KanbanCard),
    /// User posted a comment on an open issue.
    CommentAdded {
        card_id: SharedString,
        text: SharedString,
    },
    /// User toggled the board's favorite star.
    FavoriteChanged(bool),
}

impl EventEmitter<KanbanEvent> for KanbanScreen {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ActivePopover {
    Filter,
    Sort,
    PropStatus,
    PropColumn,
    PropAssignee,
    PropLabels,
    PropProject,
    ModalColumn,
    ModalStatus,
    ModalAssignee,
    ModalLabel,
}

#[derive(Clone)]
struct CardPressState {
    card_id: SharedString,
    start_pos: Point<Pixels>,
    dragging: bool,
}

/// Kanban issue board screen with board/list views, filtering, drag-move, composer modal, and issue detail view.
pub struct KanbanScreen {
    id: ElementId,
    id_name: SharedString,
    shell: Entity<AppShell>,
    standalone: bool,
    workspace: SharedString,
    user_name: SharedString,
    user_email: SharedString,
    board_title: SharedString,
    favorited: bool,
    view_mode: KanbanViewMode,
    scope: KanbanScope,
    sort_by: KanbanSort,
    columns: Vec<KanbanColumn>,
    cards: Vec<KanbanCard>,
    filter_statuses: Vec<KanbanStatus>,
    filter_assignee: Option<SharedString>,
    filter_label: Option<SharedString>,
    search_open: bool,
    search_editor: Option<Entity<Editor>>,
    active_popover: Option<ActivePopover>,
    focused_card_id: Option<SharedString>,
    open_card_id: Option<SharedString>,
    synced_open_card_id: Option<SharedString>,
    issue_title_editor: Option<Entity<Editor>>,
    issue_desc_editor: Option<Entity<Editor>>,
    comment_editor: Option<Entity<Editor>>,
    sub_issue_open: bool,
    sub_issue_editor: Option<Entity<Editor>>,
    modal_open: bool,
    modal_column: SharedString,
    modal_status: KanbanStatus,
    modal_assignee: Option<SharedString>,
    modal_assignee_agent: bool,
    modal_label: Option<SharedString>,
    modal_project: SharedString,
    modal_attachments: u32,
    modal_create_more: bool,
    modal_title_editor: Option<Entity<Editor>>,
    modal_desc_editor: Option<Entity<Editor>>,
    press_state: Option<CardPressState>,
    drag_over_col: Option<(SharedString, usize)>,
    hovered_col: Option<SharedString>,
    next_seq: usize,
    loading: bool,
    empty: bool,
    editor_focused: bool,
    focus: FocusHandle,
    started_at: Instant,
    measured_width: Rc<Cell<f32>>,
}

impl Focusable for KanbanScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl KanbanScreen {
    fn build_shell(
        id_str: &str,
        workspace: SharedString,
        user_name: SharedString,
        user_email: SharedString,
        cx: &mut Context<Self>,
    ) -> Entity<AppShell> {
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let groups = vec![
                ShellNavGroup::primary(vec![
                    ShellNavItem::new("inbox", tr(shell_cx, "Inbox", "收件箱"), ProIcon::Inbox)
                        .with_count(3),
                    ShellNavItem::new("activity", tr(shell_cx, "Activity", "动态"), ProIcon::Bell)
                        .with_count(2),
                    ShellNavItem::new(
                        "my-issues",
                        tr(shell_cx, "My issues", "我的问题"),
                        ProIcon::User,
                    ),
                    ShellNavItem::new("board", tr(shell_cx, "Board", "看板"), ProIcon::Grid),
                    ShellNavItem::new(
                        "projects",
                        tr(shell_cx, "Projects", "项目"),
                        ProIcon::Folder,
                    ),
                ]),
                ShellNavGroup::named(
                    tr(shell_cx, "Problems", "问题专题"),
                    vec![
                        ShellNavItem::new(
                            "prob-reset",
                            "Password reset emails failing",
                            ProIcon::Mail,
                        ),
                        ShellNavItem::new(
                            "prob-pay",
                            "Payments failing for some users",
                            ProIcon::CreditCard,
                        ),
                        ShellNavItem::new("prob-dash", "Dashboard loads slowly", ProIcon::Settings),
                    ],
                ),
            ];

            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Issues, shell_cx)
                .with_workspace(workspace)
                .with_user(user_name, user_email)
                .with_groups(groups)
                .with_active("board")
                .with_side_cards(true)
                .with_work_area(false, false);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s
        });
        cx.subscribe(&shell, |this, _, ev: &AppShellEvent, cx| {
            if let AppShellEvent::Navigate(label) = ev {
                let s = label.as_ref();
                if s == "Password reset emails failing" {
                    this.filter_label = Some("Auth".into());
                    this.open_card_id = None;
                } else if s == "Payments failing for some users" {
                    this.filter_label = Some("Billing".into());
                    this.open_card_id = None;
                } else if s == "Dashboard loads slowly" {
                    this.filter_label = Some("Performance".into());
                    this.open_card_id = None;
                } else if s == "Board" || s == "看板" {
                    this.filter_label = None;
                    this.filter_assignee = None;
                    this.open_card_id = None;
                } else if s == "My issues" || s == "我的问题" {
                    this.filter_assignee = Some(this.user_name.clone());
                    this.open_card_id = None;
                }
                cx.emit(KanbanEvent::Navigate(label.clone()));
                cx.notify();
            }
        })
        .detach();
        shell
    }

    fn sync_shell(&mut self, cx: &mut Context<Self>) {
        self.shell = Self::build_shell(
            &self.id_name,
            self.workspace.clone(),
            self.user_name.clone(),
            self.user_email.clone(),
            cx,
        );
    }

    /// Construct a [`KanbanScreen`] preloaded with the four priority columns and eleven sample issues.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let workspace: SharedString = "Acme".into();
        let user_name: SharedString = "Alex Morgan".into();
        let user_email: SharedString = "alex@acme.ai".into();
        let shell = Self::build_shell(
            &id_str,
            workspace.clone(),
            user_name.clone(),
            user_email.clone(),
            cx,
        );
        let columns = sample_columns();
        let cards = sample_cards();
        let first_card = cards.first().map(|c| c.id.clone());
        let next_seq = 142 + cards.len();

        Self {
            id: ElementId::Name(id_str.clone()),
            id_name: id_str,
            shell,
            standalone: false,
            workspace,
            user_name,
            user_email,
            board_title: "Overview".into(),
            favorited: false,
            view_mode: KanbanViewMode::Board,
            scope: KanbanScope::All,
            sort_by: KanbanSort::Manual,
            columns,
            cards,
            filter_statuses: Vec::new(),
            filter_assignee: None,
            filter_label: None,
            search_open: false,
            search_editor: None,
            active_popover: None,
            focused_card_id: first_card,
            open_card_id: None,
            synced_open_card_id: None,
            issue_title_editor: None,
            issue_desc_editor: None,
            comment_editor: None,
            sub_issue_open: false,
            sub_issue_editor: None,
            modal_open: false,
            modal_column: "low".into(),
            modal_status: KanbanStatus::Review,
            modal_assignee: Some("Alex Morgan".into()),
            modal_assignee_agent: false,
            modal_label: Some("Auth".into()),
            modal_project: "Overview".into(),
            modal_attachments: 0,
            modal_create_more: false,
            modal_title_editor: None,
            modal_desc_editor: None,
            press_state: None,
            drag_over_col: None,
            hovered_col: None,
            next_seq,
            loading: false,
            empty: false,
            editor_focused: false,
            focus: cx.focus_handle(),
            started_at: Instant::now(),
            measured_width: Rc::new(Cell::new(920.)),
        }
    }

    /// Render without the outer [`AppShell`] chrome when embedded inside a custom frame.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Override the workspace name shown in the sidebar header.
    pub fn with_workspace(
        mut self,
        workspace: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        self.workspace = workspace.into();
        self.sync_shell(cx);
        self
    }

    /// Override the account name and email in the sidebar footer.
    pub fn with_user(
        mut self,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        self.user_name = name.into();
        self.user_email = email.into();
        self.sync_shell(cx);
        self
    }

    /// Replace the columns displayed left to right on the board.
    pub fn with_columns(mut self, columns: Vec<KanbanColumn>) -> Self {
        if let Some(first) = columns.first() {
            self.modal_column = first.id.clone();
        }
        self.columns = columns;
        self
    }

    /// Replace the issue cards on the board.
    pub fn with_cards(mut self, cards: Vec<KanbanCard>) -> Self {
        self.focused_card_id = cards.first().map(|c| c.id.clone());
        self.next_seq = 142 + cards.len();
        self.cards = cards;
        self
    }

    /// Set the initial `Board` or `List` presentation mode.
    pub fn with_view_mode(mut self, mode: KanbanViewMode) -> Self {
        self.view_mode = mode;
        self
    }

    /// Set the initial scope tab (`All`, `Open`, or `Completed`).
    pub fn with_scope(mut self, scope: KanbanScope) -> Self {
        self.scope = scope;
        self
    }

    /// Configure skeleton loading placeholders in the shape of real board cards or list rows.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the empty board state (`"No tasks yet"`).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Toggle skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        if empty {
            self.open_card_id = None;
        }
        cx.notify();
    }

    /// Switch between `Board` and `List` views.
    pub fn set_view_mode(&mut self, mode: KanbanViewMode, cx: &mut Context<Self>) {
        if self.view_mode != mode {
            self.view_mode = mode;
            self.active_popover = None;
            cx.notify();
        }
    }

    /// Switch the active scope filter tab (`All`, `Open`, `Completed`).
    pub fn set_scope(&mut self, scope: KanbanScope, cx: &mut Context<Self>) {
        if self.scope != scope {
            self.scope = scope;
            cx.notify();
        }
    }

    /// Open a specific card in the full issue inspector view.
    pub fn open_card(&mut self, card_id: impl Into<SharedString>, cx: &mut Context<Self>) {
        let id: SharedString = card_id.into();
        if self.cards.iter().any(|c| c.id == id) {
            self.focused_card_id = Some(id.clone());
            self.open_card_id = Some(id.clone());
            self.synced_open_card_id = None;
            self.sub_issue_open = false;
            self.active_popover = None;
            cx.emit(KanbanEvent::SelectCard(id));
            cx.notify();
        }
    }

    /// Close any open issue detail view and return to the board or list.
    pub fn close_card(&mut self, cx: &mut Context<Self>) {
        if self.open_card_id.take().is_some() {
            self.synced_open_card_id = None;
            self.active_popover = None;
            self.sub_issue_open = false;
            cx.emit(KanbanEvent::CloseCard);
            cx.notify();
        }
    }

    /// Compute `(all_count, open_count, completed_count)` after active filters and search.
    pub fn scope_counts(&self, query: &str) -> (usize, usize, usize) {
        if self.empty {
            return (0, 0, 0);
        }
        let mut open = 0usize;
        let mut done = 0usize;
        for card in &self.cards {
            if !self.card_matches_filters(card, query) {
                continue;
            }
            if self.column_is_done(&card.column) {
                done += 1;
            } else {
                open += 1;
            }
        }
        (open + done, open, done)
    }

    fn column_is_done(&self, col_id: &str) -> bool {
        self.columns
            .iter()
            .find(|c| c.id.as_ref() == col_id)
            .is_some_and(|c| c.tone.is_done())
    }

    fn column_matches_scope(&self, col: &KanbanColumn) -> bool {
        match self.scope {
            KanbanScope::All => true,
            KanbanScope::Open => !col.tone.is_done(),
            KanbanScope::Completed => col.tone.is_done(),
        }
    }

    fn active_filter_count(&self, query: &str) -> usize {
        self.filter_statuses.len()
            + usize::from(self.filter_assignee.is_some())
            + usize::from(self.filter_label.is_some())
            + usize::from(!query.trim().is_empty())
    }

    fn card_matches_filters(&self, card: &KanbanCard, query: &str) -> bool {
        filter_card_matches(
            card,
            &self.filter_statuses,
            self.filter_assignee.as_deref(),
            self.filter_label.as_deref(),
            query,
        )
    }

    /// Return the visible card indices belonging to `column_id` under the current filters and sort order.
    pub fn visible_card_indices(&self, column_id: &str, query: &str) -> Vec<usize> {
        if self.empty {
            return Vec::new();
        }
        let mut idxs: Vec<usize> = self
            .cards
            .iter()
            .enumerate()
            .filter(|(_, c)| c.column.as_ref() == column_id && self.card_matches_filters(c, query))
            .map(|(i, _)| i)
            .collect();

        match self.sort_by {
            KanbanSort::Manual => {}
            KanbanSort::Title => {
                idxs.sort_by(|&a, &b| self.cards[a].title.cmp(&self.cards[b].title));
            }
            KanbanSort::Comments => {
                idxs.sort_by(|&a, &b| self.cards[b].comments.cmp(&self.cards[a].comments));
            }
            KanbanSort::Status => {
                idxs.sort_by(|&a, &b| {
                    self.cards[a]
                        .status
                        .urgency_rank()
                        .cmp(&self.cards[b].status.urgency_rank())
                });
            }
        }
        idxs
    }

    fn flat_visible_card_ids(&self, query: &str) -> Vec<SharedString> {
        let mut out = Vec::new();
        for col in &self.columns {
            if !self.column_matches_scope(col) {
                continue;
            }
            for idx in self.visible_card_indices(&col.id, query) {
                out.push(self.cards[idx].id.clone());
            }
        }
        out
    }

    /// Move `card_id` into `target_col` at optional `insert_pos` within that column, logging an activity event.
    pub fn move_card(
        &mut self,
        card_id: &str,
        target_col: &str,
        insert_pos: Option<usize>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some((from_col, to_col)) = apply_move_card(
            &mut self.cards,
            &self.columns,
            card_id,
            target_col,
            insert_pos,
            &self.user_name,
        ) else {
            return false;
        };

        cx.emit(KanbanEvent::MoveCard {
            id: card_id.to_owned().into(),
            from_column: from_col,
            to_column: to_col,
        });
        cx.notify();
        true
    }

    /// Create and insert a new issue card, returning the created [`KanbanCard`].
    pub fn create_issue(
        &mut self,
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
        column: impl Into<SharedString>,
        status: KanbanStatus,
        cx: &mut Context<Self>,
    ) -> Option<KanbanCard> {
        let trimmed = title.into().trim().to_owned();
        if trimmed.is_empty() {
            return None;
        }
        let col_id: SharedString = column.into();
        let target_col = if self.columns.iter().any(|c| c.id == col_id) {
            col_id
        } else {
            self.columns
                .first()
                .map(|c| c.id.clone())
                .unwrap_or_else(|| "low".into())
        };
        let key: SharedString = format!("NOR-{}", self.next_seq).into();
        let id: SharedString = format!("iss-{}", self.next_seq).into();
        self.next_seq += 1;

        let mut labels = Vec::new();
        if let Some(ref l) = self.modal_label {
            labels.push(l.clone());
        }
        let final_status = if self.column_is_done(&target_col) {
            KanbanStatus::Fixed
        } else {
            status
        };
        let card = KanbanCard {
            id: id.clone(),
            key,
            column: target_col,
            title: trimmed.into(),
            description: description.into(),
            status: final_status,
            assignee: self.modal_assignee.clone(),
            assignee_is_agent: self.modal_assignee_agent,
            labels,
            project: self.modal_project.clone(),
            comments: 0,
            attachments: self.modal_attachments,
            sub_issues: Vec::new(),
            activity: vec![KanbanActivity::event(
                "ev-create",
                self.user_name.clone(),
                false,
                "created this issue",
                "Just now",
            )],
            created_at: "Just now".into(),
            updated_at: "Just now".into(),
        };

        self.empty = false;
        self.focused_card_id = Some(id);
        self.cards.insert(0, card.clone());
        cx.emit(KanbanEvent::CreateCard(card.clone()));
        cx.notify();
        Some(card)
    }

    /// Append a comment to `card_id` and increment its comment counter.
    pub fn add_comment(
        &mut self,
        card_id: &str,
        text: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        let trimmed = text.into().trim().to_owned();
        if trimmed.is_empty() {
            return false;
        }
        let Some(card) = self.cards.iter_mut().find(|c| c.id.as_ref() == card_id) else {
            return false;
        };
        let cid = format!("cmt-{}", card.activity.len() + 1);
        card.activity.push(KanbanActivity::comment(
            cid,
            self.user_name.clone(),
            false,
            trimmed.clone(),
            "Just now",
        ));
        card.comments = card.comments.saturating_add(1);
        card.updated_at = "Just now".into();
        cx.emit(KanbanEvent::CommentAdded {
            card_id: card_id.to_owned().into(),
            text: trimmed.into(),
        });
        cx.notify();
        true
    }

    fn open_new_modal(
        &mut self,
        column_id: Option<SharedString>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(col) = column_id {
            self.modal_column = col.clone();
            if self.column_is_done(&col) {
                self.modal_status = KanbanStatus::Fixed;
            } else if self.modal_status == KanbanStatus::Fixed {
                self.modal_status = KanbanStatus::Review;
            }
        }
        self.modal_attachments = 0;
        self.modal_open = true;
        self.active_popover = None;
        if let Some(ed) = self.modal_title_editor.clone() {
            ed.update(cx, |e, cx| {
                e.set_value("", w, cx);
                e.focus(w, cx);
            });
        }
        if let Some(ed) = self.modal_desc_editor.clone() {
            ed.update(cx, |e, cx| e.set_value("", w, cx));
        }
        cx.notify();
    }

    fn submit_modal(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let title = self
            .modal_title_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default();
        let desc = self
            .modal_desc_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default();
        let col = self.modal_column.clone();
        let status = self.modal_status;
        if self.create_issue(title, desc, col, status, cx).is_some() {
            if self.modal_create_more {
                self.modal_attachments = 0;
                if let Some(ed) = self.modal_title_editor.clone() {
                    ed.update(cx, |e, cx| {
                        e.set_value("", w, cx);
                        e.focus(w, cx);
                    });
                }
                if let Some(ed) = self.modal_desc_editor.clone() {
                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                }
            } else {
                self.modal_open = false;
            }
            cx.notify();
        }
    }

    fn submit_comment(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(card_id) = self.open_card_id.clone() else {
            return;
        };
        let body = self
            .comment_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default();
        if self.add_comment(&card_id, body, cx)
            && let Some(ed) = self.comment_editor.clone()
        {
            ed.update(cx, |e, cx| e.set_value("", w, cx));
        }
    }

    fn submit_sub_issue(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(ref card_id) = self.open_card_id else {
            return;
        };
        let txt = self
            .sub_issue_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_string())
            .unwrap_or_default();
        if txt.is_empty() {
            self.sub_issue_open = false;
            cx.notify();
            return;
        }
        if let Some(card) = self.cards.iter_mut().find(|c| &c.id == card_id) {
            let sid = format!("sub-{}", card.sub_issues.len() + 1);
            card.sub_issues.push(KanbanSubIssue::new(sid, txt, false));
            card.updated_at = "Just now".into();
            let updated = card.clone();
            cx.emit(KanbanEvent::UpdateCard(updated));
        }
        if let Some(ed) = self.sub_issue_editor.clone() {
            ed.update(cx, |e, cx| e.set_value("", w, cx));
        }
        self.sub_issue_open = false;
        cx.notify();
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.search_editor.is_none() {
            let ph: SharedString = tr(cx, "Filter tasks…", "筛选任务…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
                EditEvent::Focused => this.editor_focused = true,
                EditEvent::Blurred => this.editor_focused = false,
                EditEvent::Escape => {
                    this.search_open = false;
                    cx.notify();
                }
                EditEvent::Changed | EditEvent::Submitted => cx.notify(),
                _ => {}
            })
            .detach();
            self.search_editor = Some(ed);
        }

        if self.modal_title_editor.is_none() {
            let ph: SharedString = tr(cx, "Issue title", "问题标题").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.set_command_submit(true);
                e.font_size = 15.;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
                EditEvent::Focused => this.editor_focused = true,
                EditEvent::Blurred => this.editor_focused = false,
                EditEvent::Escape => {
                    this.modal_open = false;
                    cx.notify();
                }
                EditEvent::Changed => cx.notify(),
                _ => {}
            })
            .detach();
            cx.subscribe_in(&ed, w, |this, _, _: &EditorShortcutEvent, w, cx| {
                this.submit_modal(w, cx);
            })
            .detach();
            self.modal_title_editor = Some(ed);
        }

        if self.modal_desc_editor.is_none() {
            let ph: SharedString = tr(cx, "Add description…", "添加详细描述…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Multi, w, cx);
                e.set_placeholder(ph);
                e.set_command_submit(true);
                e.font_size = 13.5;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
                EditEvent::Focused => this.editor_focused = true,
                EditEvent::Blurred => this.editor_focused = false,
                EditEvent::Escape => {
                    this.modal_open = false;
                    cx.notify();
                }
                EditEvent::Changed => cx.notify(),
                _ => {}
            })
            .detach();
            cx.subscribe_in(&ed, w, |this, _, _: &EditorShortcutEvent, w, cx| {
                this.submit_modal(w, cx);
            })
            .detach();
            self.modal_desc_editor = Some(ed);
        }

        if self.issue_title_editor.is_none() {
            let ph: SharedString = tr(cx, "Issue title", "问题标题").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 22.;
                e
            });
            cx.subscribe(&ed, |this, ed_ent, ev: &EditEvent, cx| match ev {
                EditEvent::Focused => this.editor_focused = true,
                EditEvent::Blurred => this.editor_focused = false,
                EditEvent::Changed | EditEvent::Submitted => {
                    let val = ed_ent.read(cx).value().trim().to_owned();
                    if !val.is_empty()
                        && let Some(ref open_id) = this.open_card_id
                        && let Some(card) = this.cards.iter_mut().find(|c| &c.id == open_id)
                        && card.title.as_ref() != val
                    {
                        card.title = val.into();
                        card.updated_at = "Just now".into();
                        let updated = card.clone();
                        cx.emit(KanbanEvent::UpdateCard(updated));
                    }
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.issue_title_editor = Some(ed);
        }

        if self.issue_desc_editor.is_none() {
            let ph: SharedString = tr(
                cx,
                "Add a description or reproduction steps…",
                "添加问题描述或复现步骤…",
            )
            .into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Multi, w, cx);
                e.set_placeholder(ph);
                e.font_size = 14.;
                e
            });
            cx.subscribe(&ed, |this, ed_ent, ev: &EditEvent, cx| match ev {
                EditEvent::Focused => this.editor_focused = true,
                EditEvent::Blurred => this.editor_focused = false,
                EditEvent::Changed => {
                    let val = ed_ent.read(cx).value().to_owned();
                    if let Some(ref open_id) = this.open_card_id
                        && let Some(card) = this.cards.iter_mut().find(|c| &c.id == open_id)
                        && card.description.as_ref() != val
                    {
                        card.description = val.into();
                        card.updated_at = "Just now".into();
                    }
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.issue_desc_editor = Some(ed);
        }

        if self.comment_editor.is_none() {
            let ph: SharedString =
                tr(cx, "Leave a comment… (⌘↵ to send)", "添加评论…（⌘↵ 发送）").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Multi, w, cx);
                e.set_placeholder(ph);
                e.set_command_submit(true);
                e.font_size = 13.5;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
                EditEvent::Focused => this.editor_focused = true,
                EditEvent::Blurred => this.editor_focused = false,
                EditEvent::Changed => cx.notify(),
                _ => {}
            })
            .detach();
            cx.subscribe_in(&ed, w, |this, _, _: &EditorShortcutEvent, w, cx| {
                this.submit_comment(w, cx);
            })
            .detach();
            self.comment_editor = Some(ed);
        }

        if self.sub_issue_editor.is_none() {
            let ph: SharedString = tr(cx, "Sub-issue title…", "子任务标题…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.5;
                e
            });
            cx.subscribe_in(&ed, w, |this, _, ev: &EditEvent, w, cx| match ev {
                EditEvent::Focused => this.editor_focused = true,
                EditEvent::Blurred => this.editor_focused = false,
                EditEvent::Submitted => this.submit_sub_issue(w, cx),
                EditEvent::Escape => {
                    this.sub_issue_open = false;
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.sub_issue_editor = Some(ed);
        }

        // Sync open card title & description into in-place editors when switching open issues
        if self.open_card_id != self.synced_open_card_id {
            self.synced_open_card_id = self.open_card_id.clone();
            if let Some(ref open_id) = self.open_card_id
                && let Some(card) = self.cards.iter().find(|c| &c.id == open_id).cloned()
            {
                if let Some(ed) = self.issue_title_editor.clone() {
                    ed.update(cx, |e, cx| e.set_value(card.title.to_string(), w, cx));
                }
                if let Some(ed) = self.issue_desc_editor.clone() {
                    ed.update(cx, |e, cx| e.set_value(card.description.to_string(), w, cx));
                }
                if let Some(ed) = self.comment_editor.clone() {
                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                }
            }
        }
    }

    fn current_query(&self, cx: &App) -> String {
        self.search_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default()
    }

    fn handle_key(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let m = ev.keystroke.modifiers;
        let cmd = m.platform || m.control;
        let k = ev.keystroke.key.as_str();

        if cmd && k == "enter" {
            if self.modal_open {
                self.submit_modal(w, cx);
                return;
            }
            if self.open_card_id.is_some() {
                self.submit_comment(w, cx);
                return;
            }
        }

        if k == "escape" {
            if self.active_popover.take().is_some() {
                cx.notify();
                return;
            }
            if self.modal_open {
                self.modal_open = false;
                cx.notify();
                return;
            }
            if self.sub_issue_open {
                self.sub_issue_open = false;
                cx.notify();
                return;
            }
            if self.open_card_id.is_some() {
                self.close_card(cx);
                return;
            }
            if self.search_open {
                self.search_open = false;
                if let Some(ed) = self.search_editor.clone() {
                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                }
                cx.notify();
            }
            return;
        }

        if self.editor_focused || cmd {
            return;
        }

        let query = self.current_query(cx);
        match k {
            "/" => {
                self.search_open = true;
                if let Some(ed) = self.search_editor.clone() {
                    ed.update(cx, |e, cx| e.focus(w, cx));
                }
                cx.notify();
            }
            "c" | "n" if self.open_card_id.is_none() => {
                self.open_new_modal(None, w, cx);
            }
            "v" if self.open_card_id.is_none() => {
                let next = match self.view_mode {
                    KanbanViewMode::Board => KanbanViewMode::List,
                    KanbanViewMode::List => KanbanViewMode::Board,
                };
                self.set_view_mode(next, cx);
            }
            "j" | "down" => {
                if self.open_card_id.is_some() {
                    self.step_open_card(1, &query, cx);
                } else if m.alt {
                    self.nudge_focused_within_column(1, cx);
                } else {
                    self.step_focused_card(1, &query, cx);
                }
            }
            "k" | "up" => {
                if self.open_card_id.is_some() {
                    self.step_open_card(-1, &query, cx);
                } else if m.alt {
                    self.nudge_focused_within_column(-1, cx);
                } else {
                    self.step_focused_card(-1, &query, cx);
                }
            }
            "left" | "[" if m.alt || k == "[" => {
                self.shift_focused_column(-1, cx);
            }
            "right" | "]" if m.alt || k == "]" => {
                self.shift_focused_column(1, cx);
            }
            "enter" if self.open_card_id.is_none() => {
                if let Some(cid) = self.focused_card_id.clone() {
                    self.open_card(cid, cx);
                }
            }
            _ => {}
        }
    }

    fn step_focused_card(&mut self, delta: isize, query: &str, cx: &mut Context<Self>) {
        let ids = self.flat_visible_card_ids(query);
        if ids.is_empty() {
            return;
        }
        let cur = self
            .focused_card_id
            .as_ref()
            .and_then(|id| ids.iter().position(|x| x == id))
            .unwrap_or(0);
        let next = (cur as isize + delta).clamp(0, ids.len() as isize - 1) as usize;
        self.focused_card_id = Some(ids[next].clone());
        cx.notify();
    }

    fn step_open_card(&mut self, delta: isize, query: &str, cx: &mut Context<Self>) {
        let ids = self.flat_visible_card_ids(query);
        if ids.is_empty() {
            return;
        }
        let cur = self
            .open_card_id
            .as_ref()
            .and_then(|id| ids.iter().position(|x| x == id))
            .unwrap_or(0);
        let next = (cur as isize + delta).clamp(0, ids.len() as isize - 1) as usize;
        let target = ids[next].clone();
        if self.open_card_id.as_ref() != Some(&target) {
            self.open_card(target, cx);
        }
    }

    fn shift_focused_column(&mut self, col_delta: isize, cx: &mut Context<Self>) {
        let target_id = self
            .open_card_id
            .clone()
            .or_else(|| self.focused_card_id.clone());
        let Some(cid) = target_id else {
            return;
        };
        let Some(cur_col) = self
            .cards
            .iter()
            .find(|c| c.id == cid)
            .map(|c| c.column.clone())
        else {
            return;
        };
        let Some(col_idx) = self.columns.iter().position(|c| c.id == cur_col) else {
            return;
        };
        let next_idx =
            (col_idx as isize + col_delta).clamp(0, self.columns.len() as isize - 1) as usize;
        if next_idx != col_idx {
            let next_col = self.columns[next_idx].id.clone();
            self.move_card(&cid, &next_col, None, cx);
        }
    }

    fn nudge_focused_within_column(&mut self, row_delta: isize, cx: &mut Context<Self>) {
        let Some(cid) = self.focused_card_id.clone() else {
            return;
        };
        let Some(col_id) = self
            .cards
            .iter()
            .find(|c| c.id == cid)
            .map(|c| c.column.clone())
        else {
            return;
        };
        let col_cards: Vec<SharedString> = self
            .cards
            .iter()
            .filter(|c| c.column == col_id)
            .map(|c| c.id.clone())
            .collect();
        let Some(pos) = col_cards.iter().position(|id| id == &cid) else {
            return;
        };
        let next_pos = (pos as isize + row_delta).clamp(0, col_cards.len() as isize - 1) as usize;
        if next_pos != pos {
            self.move_card(&cid, &col_id, Some(next_pos), cx);
        }
    }

    fn render_priority_mark(tone: KanbanTone, p: &ProPalette) -> AnyElement {
        if tone == KanbanTone::Done {
            return div()
                .size(px(15.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(pro_svg("0 0 16 16", SVG_MARK_DONE, 15., p.green_ink))
                .into_any_element();
        }
        let active = tone.active_bars();
        let on_c = tone.bar_color(p);
        let off_c = p.line_strong;
        let heights: [f32; 3] = [4.0, 7.0, 10.0];
        let mut row = div()
            .size(px(15.))
            .pb(px(2.))
            .flex_none()
            .flex()
            .items_end()
            .justify_center()
            .gap(px(1.5));
        for (i, &h) in heights.iter().enumerate() {
            let lit = (i as u8) < active;
            row = row.child(div().w(px(2.8)).h(px(h)).rounded(px(1.)).bg(if lit {
                on_c
            } else {
                off_c
            }));
        }
        row.into_any_element()
    }

    fn render_status_mark(status: KanbanStatus, p: &ProPalette) -> AnyElement {
        let color = status.color(p);
        let svg = match status {
            KanbanStatus::Review => SVG_STATUS_REVIEW,
            KanbanStatus::Progress => SVG_STATUS_PROGRESS,
            KanbanStatus::Attention => SVG_STATUS_ATTENTION,
            KanbanStatus::Fixed => SVG_STATUS_FIXED,
        };
        pro_svg("0 0 14 14", svg, 13., color)
    }

    fn render_counts(comments: u32, attachments: u32, p: &ProPalette) -> Div {
        let mut wrap = div()
            .flex_none()
            .flex()
            .items_center()
            .text_size(px(12.))
            .text_color(p.faint);
        if comments > 0 {
            wrap = wrap.child(
                div()
                    .px(px(if attachments > 0 { 7. } else { 0. }))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(pro_svg("0 0 24 24", SVG_COMMENT_BUBBLE, 13., p.faint))
                    .child(format!("{comments}")),
            );
        }
        if attachments > 0 {
            if comments > 0 {
                wrap = wrap.child(div().w(px(1.)).h(px(11.)).bg(p.line_strong));
            }
            wrap = wrap.child(
                div()
                    .pl(px(if comments > 0 { 7. } else { 0. }))
                    .flex()
                    .items_center()
                    .gap(px(3.5))
                    .child(pro_svg("0 0 24 24", SVG_PAPERCLIP_META, 13., p.faint))
                    .child(format!("{attachments}")),
            );
        }
        wrap
    }

    fn render_assignee_avatar(
        name: Option<&SharedString>,
        is_agent: bool,
        side: f32,
        elapsed_ms: f32,
        p: &ProPalette,
        cx: &App,
    ) -> AnyElement {
        if let Some(n) = name {
            if is_agent {
                pro_agent_avatar(n, p.purple, side, false, elapsed_ms, cx)
            } else {
                pro_person_avatar(n, side, false, None, cx)
            }
        } else {
            div()
                .size(px(side))
                .rounded_full()
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .items_center()
                .justify_center()
                .child(pro_icon(ProIcon::User, (side * 0.6).max(10.), p.faint))
                .into_any_element()
        }
    }

    fn label_dot_color(label: &str, p: &ProPalette) -> Hsla {
        for (name, hex) in ALL_LABELS {
            if name.eq_ignore_ascii_case(label) {
                return rgb(hex).into();
            }
        }
        p.blue
    }

    fn render_top_bar(&mut self, query: &str, compact: bool, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);

        if let Some(open_id) = self.open_card_id.clone() {
            let visible_ids = self.flat_visible_card_ids(query);
            let pos_idx = visible_ids.iter().position(|id| id == &open_id);
            let pos_txt = pos_idx
                .map(|i| format!("{} / {}", i + 1, visible_ids.len()))
                .unwrap_or_else(|| "1 / 1".to_owned());
            let can_prev = pos_idx.is_some_and(|i| i > 0);
            let can_next = pos_idx.is_some_and(|i| i + 1 < visible_ids.len());
            let (issue_key, issue_title) = self
                .cards
                .iter()
                .find(|c| c.id == open_id)
                .map(|c| (c.key.clone(), c.title.clone()))
                .unwrap_or_else(|| ("NOR-142".into(), "Issue".into()));

            let q_prev = query.to_owned();
            let q_next = query.to_owned();

            return div()
                .h(px(48.))
                .pl(px(if compact { 48. } else { 20. }))
                .pr(px(12.))
                .flex_none()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.))
                .border_b_1()
                .border_color(p.line_soft)
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .text_size(px(13.))
                        .child(div().text_color(p.muted).child(tr(cx, "Inbox", "收件箱")))
                        .child(div().text_color(p.faint).child("/"))
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, "crumb-board"))
                                .role(Role::Button)
                                .tab_index(0)
                                .px(px(5.))
                                .py(px(2.))
                                .rounded(px(6.))
                                .text_color(p.muted)
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover).text_color(p.ink))
                                .child(self.board_title.clone()),
                            cx,
                            |this, _, cx| this.close_card(cx),
                        ))
                        .child(div().text_color(p.faint).child("/"))
                        .child(
                            div()
                                .min_w_0()
                                .max_w(px(if compact { 160. } else { 360. }))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(div().flex_none().text_color(p.faint).child(issue_key))
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(issue_title),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .child(
                            div()
                                .mr(px(4.))
                                .text_size(px(12.))
                                .text_color(p.faint)
                                .child(pos_txt),
                        )
                        .child(
                            div()
                                .when(!can_prev, |d| d.opacity(0.35))
                                .child(pro_icon_btn(
                                    pro_eid(&self.id, "issue-prev"),
                                    tr(cx, "Previous issue", "上一个问题"),
                                    ProIcon::ChevronUp,
                                    28.,
                                    false,
                                    cx,
                                    move |this, _, cx| {
                                        if can_prev {
                                            this.step_open_card(-1, &q_prev, cx);
                                        }
                                    },
                                )),
                        )
                        .child(
                            div()
                                .when(!can_next, |d| d.opacity(0.35))
                                .child(pro_icon_btn(
                                    pro_eid(&self.id, "issue-next"),
                                    tr(cx, "Next issue", "下一个问题"),
                                    ProIcon::ChevronDown,
                                    28.,
                                    false,
                                    cx,
                                    move |this, _, cx| {
                                        if can_next {
                                            this.step_open_card(1, &q_next, cx);
                                        }
                                    },
                                )),
                        )
                        .child(pro_icon_btn(
                            pro_eid(&self.id, "issue-close"),
                            tr(cx, "Back to board", "返回看板"),
                            ProIcon::Close,
                            28.,
                            false,
                            cx,
                            |this, _, cx| this.close_card(cx),
                        )),
                );
        }

        let fav = self.favorited;
        let filter_count = self.active_filter_count(query);
        let filter_open = self.active_popover == Some(ActivePopover::Filter);
        let sort_open = self.active_popover == Some(ActivePopover::Sort);
        let is_board = self.view_mode == KanbanViewMode::Board;

        let star_btn = activate(
            div()
                .id(pro_eid(&self.id, "star-btn"))
                .role(Role::Button)
                .aria_label(tr(cx, "Add to favorites", "添加到收藏"))
                .tab_index(0)
                .size(px(26.))
                .rounded(px(7.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .child(if fav {
                    pro_svg("0 0 24 24", SVG_STAR_FILLED, 15., p.amber)
                } else {
                    pro_icon(ProIcon::Star, 15., p.muted)
                }),
            cx,
            |this, _, cx| {
                this.favorited = !this.favorited;
                cx.emit(KanbanEvent::FavoriteChanged(this.favorited));
                cx.notify();
            },
        );

        let mut tools = div().flex_none().flex().items_center().gap(px(4.));

        // Inline search pill when opened via `/` or search button
        if self.search_open
            && let Some(ref search_ed) = self.search_editor
        {
            tools = tools.child(
                div()
                    .w(px(if compact { 128. } else { 172. }))
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(8.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_icon(ProIcon::Search, 13.5, p.faint))
                    .child(div().flex_1().min_w_0().child(search_ed.clone()))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "search-clear"))
                            .role(Role::Button)
                            .tab_index(0)
                            .size(px(18.))
                            .rounded(px(5.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(move |s| s.bg(p.hover))
                            .child(pro_icon(ProIcon::Close, 12., p.muted)),
                        cx,
                        |this, w, cx| {
                            this.search_open = false;
                            if let Some(ed) = this.search_editor.clone() {
                                ed.update(cx, |e, cx| e.set_value("", w, cx));
                            }
                            cx.notify();
                        },
                    )),
            );
        }

        // Filter trigger + dropdown anchor
        tools = tools.child(
            div()
                .relative()
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "filter-btn"))
                        .role(Role::Button)
                        .aria_expanded(filter_open)
                        .tab_index(0)
                        .h(px(30.))
                        .px(px(10.))
                        .rounded(px(8.))
                        .bg(if filter_open {
                            p.hover
                        } else {
                            transparent_black()
                        })
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(13.))
                        .text_color(if filter_count > 0 || filter_open {
                            p.ink
                        } else {
                            p.muted
                        })
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.hover).text_color(p.ink))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .child(pro_svg(
                            "0 0 24 24",
                            SVG_FUNNEL,
                            15.,
                            if filter_count > 0 || filter_open {
                                p.ink
                            } else {
                                p.muted
                            },
                        ))
                        .child(tr(cx, "Filter", "筛选"))
                        .when(filter_count > 0, |d| {
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
                                    .child(format!("{filter_count}")),
                            )
                        }),
                    cx,
                    |this, _, cx| {
                        this.active_popover = if this.active_popover == Some(ActivePopover::Filter)
                        {
                            None
                        } else {
                            Some(ActivePopover::Filter)
                        };
                        cx.notify();
                    },
                ))
                .child({
                    let flt_menu = self.render_filter_menu(&p, cx).shadow_md();
                    pro_presence(
                        pro_eid(&self.id, "flt-menu-pres"),
                        filter_open,
                        0.18,
                        0.12,
                        move |a| {
                            flt_menu
                                .top(px(34. - 4. * (1. - a)))
                                .opacity(a)
                                .into_any_element()
                        },
                    )
                }),
        );

        // Sort trigger + dropdown anchor
        tools = tools.child(
            div()
                .relative()
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "sort-btn"))
                        .role(Role::Button)
                        .aria_expanded(sort_open)
                        .tab_index(0)
                        .h(px(30.))
                        .px(px(10.))
                        .rounded(px(8.))
                        .bg(if sort_open {
                            p.hover
                        } else {
                            transparent_black()
                        })
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(13.))
                        .text_color(if self.sort_by != KanbanSort::Manual || sort_open {
                            p.ink
                        } else {
                            p.muted
                        })
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.hover).text_color(p.ink))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .child(pro_svg("0 0 24 24", SVG_SORT_AZ, 15., p.muted))
                        .child(tr(cx, "Sort by", "排序"))
                        .child(pro_icon(ProIcon::ChevronDown, 13., p.muted)),
                    cx,
                    |this, _, cx| {
                        this.active_popover = if this.active_popover == Some(ActivePopover::Sort) {
                            None
                        } else {
                            Some(ActivePopover::Sort)
                        };
                        cx.notify();
                    },
                ))
                .child({
                    let srt_menu = self.render_sort_menu(&p, cx).shadow_md();
                    pro_presence(
                        pro_eid(&self.id, "sort-menu-pres"),
                        sort_open,
                        0.18,
                        0.12,
                        move |a| {
                            srt_menu
                                .top(px(34. - 4. * (1. - a)))
                                .opacity(a)
                                .into_any_element()
                        },
                    )
                }),
        );

        // Board | List segmented view switch with sliding pill indicator
        let view_sensor = pro_selection_sensor();
        tools = tools.child(
            div()
                .relative()
                .ml(px(6.))
                .p(px(3.))
                .rounded(px(10.))
                .bg(p.chip)
                .flex()
                .items_center()
                .gap(px(2.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "kb-view-pill"),
                    if is_board { "board" } else { "list" },
                    view_sensor.clone(),
                    7.,
                    p.card,
                    Some(p.line),
                    true,
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "view-board"))
                        .role(Role::RadioButton)
                        .aria_toggled(is_board.into())
                        .tab_index(0)
                        .relative()
                        .h(px(26.))
                        .px(px(12.))
                        .rounded(px(7.))
                        .when(is_board, |d| {
                            d.font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(pro_mark_selected(&view_sensor))
                        })
                        .when(!is_board, |d| {
                            d.text_color(p.muted).hover(move |s| s.text_color(p.ink))
                        })
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(13.))
                        .cursor_pointer()
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .child(tr(cx, "Board", "看板")),
                    cx,
                    |this, _, cx| this.set_view_mode(KanbanViewMode::Board, cx),
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "view-list"))
                        .role(Role::RadioButton)
                        .aria_toggled((!is_board).into())
                        .tab_index(0)
                        .relative()
                        .h(px(26.))
                        .px(px(12.))
                        .rounded(px(7.))
                        .when(!is_board, |d| {
                            d.font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(pro_mark_selected(&view_sensor))
                        })
                        .when(is_board, |d| {
                            d.text_color(p.muted).hover(move |s| s.text_color(p.ink))
                        })
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(13.))
                        .cursor_pointer()
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .child(tr(cx, "List", "列表")),
                    cx,
                    |this, _, cx| this.set_view_mode(KanbanViewMode::List, cx),
                )),
        );

        div()
            .min_h(px(48.))
            .pl(px(if compact { 48. } else { 20. }))
            .pr(px(12.))
            .py(px(if compact { 6. } else { 0. }))
            .flex_none()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(8.))
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .child(div().text_color(p.muted).child(tr(cx, "Inbox", "收件箱")))
                    .child(div().text_color(p.faint).child("/"))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(self.board_title.clone()),
                    )
                    .child(star_btn),
            )
            .child(tools)
    }

    fn render_filter_menu(&self, p: &ProPalette, cx: &mut Context<Self>) -> Div {
        let mut panel = div()
            .occlude()
            .absolute()
            .top(px(34.))
            .right_0()
            .w(px(228.))
            .p(px(5.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_col()
            .gap(px(2.));

        // Search toggle option inside Filter menu
        panel = panel.child(activate(
            div()
                .id(pro_eid(&self.id, "flt-search-toggle"))
                .role(Role::MenuItem)
                .tab_index(0)
                .h(px(30.))
                .px(px(10.))
                .rounded(px(8.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .text_size(px(13.))
                .text_color(p.text)
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover).text_color(p.ink))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_icon(ProIcon::Search, 14., p.muted))
                        .child(tr(cx, "Search text…", "文本搜索…")),
                )
                .child(div().text_size(px(11.5)).text_color(p.faint).child("/")),
            cx,
            |this, w, cx| {
                this.search_open = true;
                this.active_popover = None;
                if let Some(ed) = this.search_editor.clone() {
                    ed.update(cx, |e, cx| e.focus(w, cx));
                }
                cx.notify();
            },
        ));

        panel = panel.child(div().h(px(1.)).mx(px(6.)).my(px(3.)).bg(p.line));
        panel = panel.child(
            div()
                .px(px(10.))
                .py(px(3.))
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.faint)
                .child(tr(cx, "Status", "状态")),
        );

        for st in KanbanStatus::ALL {
            let checked = self.filter_statuses.contains(&st);
            let lbl = st.label(cx);
            panel = panel.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("flt-st-{st:?}")))
                    .role(Role::MenuItem)
                    .aria_toggled(checked.into())
                    .tab_index(0)
                    .h(px(30.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .text_color(if checked { p.ink } else { p.text })
                    .when(checked, |d| d.font_weight(FontWeight::MEDIUM))
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(Self::render_status_mark(st, p))
                            .child(lbl),
                    )
                    .when(checked, |d| {
                        d.child(pro_svg("0 0 24 24", SVG_CHECK_TICK, 14., p.ink))
                    }),
                cx,
                move |this, _, cx| {
                    if let Some(pos) = this.filter_statuses.iter().position(|&x| x == st) {
                        this.filter_statuses.remove(pos);
                    } else {
                        this.filter_statuses.push(st);
                    }
                    cx.notify();
                },
            ));
        }

        panel = panel.child(div().h(px(1.)).mx(px(6.)).my(px(3.)).bg(p.line));
        panel = panel.child(
            div()
                .px(px(10.))
                .py(px(3.))
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.faint)
                .child(tr(cx, "Label", "标签")),
        );

        for (lbl_name, hex) in ALL_LABELS.iter().take(4) {
            let checked = self.filter_label.as_deref() == Some(*lbl_name);
            let dot_c: Hsla = rgb(*hex).into();
            let lbl_owned: SharedString = (*lbl_name).into();
            panel = panel.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("flt-lbl-{lbl_name}")))
                    .role(Role::MenuItem)
                    .aria_toggled(checked.into())
                    .tab_index(0)
                    .h(px(29.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(13.))
                    .text_color(if checked { p.ink } else { p.text })
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(div().size(px(8.)).rounded_full().bg(dot_c))
                            .child(*lbl_name),
                    )
                    .when(checked, |d| {
                        d.child(pro_svg("0 0 24 24", SVG_CHECK_TICK, 14., p.ink))
                    }),
                cx,
                move |this, _, cx| {
                    if this.filter_label.as_ref() == Some(&lbl_owned) {
                        this.filter_label = None;
                    } else {
                        this.filter_label = Some(lbl_owned.clone());
                    }
                    cx.notify();
                },
            ));
        }

        if !self.filter_statuses.is_empty()
            || self.filter_assignee.is_some()
            || self.filter_label.is_some()
        {
            panel = panel
                .child(div().h(px(1.)).mx(px(6.)).my(px(3.)).bg(p.line))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "flt-clear"))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(30.))
                        .px(px(10.))
                        .rounded(px(8.))
                        .flex()
                        .items_center()
                        .text_size(px(12.5))
                        .text_color(p.red)
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.red_soft))
                        .child(tr(cx, "Clear filters", "清除筛选")),
                    cx,
                    |this, _, cx| {
                        this.filter_statuses.clear();
                        this.filter_assignee = None;
                        this.filter_label = None;
                        this.active_popover = None;
                        cx.notify();
                    },
                ));
        }

        panel
    }

    fn render_sort_menu(&self, p: &ProPalette, cx: &mut Context<Self>) -> Div {
        let items = [
            (KanbanSort::Manual, tr(cx, "Manual order", "手动顺序")),
            (KanbanSort::Status, tr(cx, "Status urgency", "按状态优先级")),
            (KanbanSort::Comments, tr(cx, "Most comments", "评论最多")),
            (KanbanSort::Title, tr(cx, "Title A–Z", "标题字母序")),
        ];

        let sort_sensor = pro_selection_sensor();
        let mut panel = div()
            .occlude()
            .absolute()
            .top(px(34.))
            .right_0()
            .w(px(196.))
            .p(px(5.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_col()
            .gap(px(1.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "kb-sort-pill"),
                format!("{:?}", self.sort_by),
                sort_sensor.clone(),
                8.,
                p.select,
                None,
                false,
            ));

        for (mode, text) in items {
            let checked = self.sort_by == mode;
            panel = panel.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("sort-{mode:?}")))
                    .role(Role::MenuItem)
                    .aria_toggled(checked.into())
                    .tab_index(0)
                    .relative()
                    .h(px(32.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .text_color(if checked { p.ink } else { p.text })
                    .when(checked, |d| {
                        d.font_weight(FontWeight::MEDIUM)
                            .child(pro_mark_selected(&sort_sensor))
                    })
                    .cursor_pointer()
                    .when(!checked, |d| {
                        d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                    })
                    .child(text)
                    .when(checked, |d| {
                        d.child(pro_svg("0 0 24 24", SVG_CHECK_TICK, 14., p.ink))
                    }),
                cx,
                move |this, _, cx| {
                    this.sort_by = mode;
                    this.active_popover = None;
                    cx.notify();
                },
            ));
        }

        panel
    }

    fn render_scope_toolbar(&mut self, query: &str, compact: bool, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let (all_n, open_n, done_n) = if self.loading {
            (11, 9, 2)
        } else {
            self.scope_counts(query)
        };

        let tabs = [
            (
                KanbanScope::All,
                tr(cx, "All tasks", "全部任务"),
                all_n,
                "scope-all",
            ),
            (
                KanbanScope::Open,
                tr(cx, "Open", "未完成"),
                open_n,
                "scope-open",
            ),
            (
                KanbanScope::Completed,
                tr(cx, "Completed", "已完成"),
                done_n,
                "scope-done",
            ),
        ];

        let scope_sensor = pro_selection_sensor();
        let scope_key = match self.scope {
            KanbanScope::All => "scope-all",
            KanbanScope::Open => "scope-open",
            KanbanScope::Completed => "scope-done",
        };
        let mut scope_strip =
            div()
                .relative()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "kb-scope-pill"),
                    scope_key,
                    scope_sensor.clone(),
                    8.,
                    p.chip,
                    Some(p.line),
                    false,
                ));

        for (sc, label, count, key) in tabs {
            let selected = self.scope == sc;
            scope_strip = scope_strip.child(activate(
                div()
                    .id(pro_eid(&self.id, key))
                    .role(Role::Tab)
                    .aria_selected(selected)
                    .tab_index(0)
                    .relative()
                    .h(px(28.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(if selected {
                        transparent_black()
                    } else {
                        p.line_strong
                    })
                    .when(selected, |d| {
                        d.font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(pro_mark_selected(&scope_sensor))
                    })
                    .when(!selected, |d| {
                        d.text_color(p.muted).hover(move |s| s.text_color(p.ink))
                    })
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .cursor_pointer()
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .child(label)
                    .when(!compact, |d| {
                        d.child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::NORMAL)
                                .text_color(p.faint)
                                .child(format!("{count}")),
                        )
                    }),
                cx,
                move |this, _, cx| this.set_scope(sc, cx),
            ));
        }

        let mut left = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(4.))
            .child(scope_strip);

        // Show active filter pills if user filtered by label or assignee
        if let Some(ref lbl) = self.filter_label {
            let dot_c = Self::label_dot_color(lbl, &p);
            left = left.child(activate(
                div()
                    .id(pro_eid(&self.id, "pill-clear-label"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(26.))
                    .px(px(8.))
                    .rounded(px(7.))
                    .bg(p.accent_soft)
                    .text_color(p.blue)
                    .text_size(px(12.))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .cursor_pointer()
                    .child(div().size(px(6.5)).rounded_full().bg(dot_c))
                    .child(lbl.clone())
                    .child(pro_icon(ProIcon::Close, 11., p.blue)),
                cx,
                |this, _, cx| {
                    this.filter_label = None;
                    cx.notify();
                },
            ));
        }

        if let Some(ref who) = self.filter_assignee {
            left = left.child(activate(
                div()
                    .id(pro_eid(&self.id, "pill-clear-who"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(26.))
                    .px(px(8.))
                    .rounded(px(7.))
                    .bg(p.accent_soft)
                    .text_color(p.blue)
                    .text_size(px(12.))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .cursor_pointer()
                    .child(who.clone())
                    .child(pro_icon(ProIcon::Close, 11., p.blue)),
                cx,
                |this, _, cx| {
                    this.filter_assignee = None;
                    cx.notify();
                },
            ));
        }

        let new_btn = activate(
            div()
                .id(pro_eid(&self.id, "new-task-btn"))
                .role(Role::Button)
                .tab_index(0)
                .h(px(28.))
                .pl(px(10.))
                .pr(px(12.))
                .rounded(px(8.))
                .bg(p.primary)
                .text_color(p.on_primary)
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .flex_none()
                .flex()
                .items_center()
                .gap(px(6.))
                .cursor_pointer()
                .hover(|s| s.opacity(0.88))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .child(pro_icon(ProIcon::Plus, 14., p.on_primary))
                .child(tr(cx, "New task", "新建任务")),
            cx,
            |this, w, cx| this.open_new_modal(None, w, cx),
        );

        div()
            .pl(px(if compact { 8. } else { 14. }))
            .pr(px(if compact { 8. } else { 12. }))
            .pb(px(10.))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .child(left)
            .child(new_btn)
    }

    fn render_board_view(
        &mut self,
        query: &str,
        phase: f32,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let frame_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.03)
        } else {
            hsla(0., 0., 0., 0.02)
        };
        let active_drag_id = self
            .press_state
            .as_ref()
            .filter(|ps| ps.dragging)
            .map(|ps| ps.card_id.clone());
        let visible_cols: Vec<KanbanColumn> = self
            .columns
            .iter()
            .filter(|c| self.column_matches_scope(c))
            .cloned()
            .collect();

        let mut board = div()
            .id(pro_eid(&self.id, "board-scroll"))
            .flex_1()
            .min_h_0()
            .px(px(if compact { 8. } else { 12. }))
            .pb(px(if compact { 8. } else { 12. }))
            .flex()
            .gap(px(10.))
            .overflow_x_scroll();

        for (col_idx, col) in visible_cols.iter().enumerate() {
            let card_idxs = self.visible_card_indices(&col.id, query);
            let count = if self.loading {
                if col.tone.is_done() { 2 } else { 3 }
            } else {
                card_idxs.len()
            };
            let is_over = self
                .drag_over_col
                .as_ref()
                .is_some_and(|(cid, _)| cid == &col.id);
            let col_hovered = self.hovered_col.as_ref() == Some(&col.id);
            let col_id = col.id.clone();
            let col_id_hover = col.id.clone();
            let col_id_add = col.id.clone();

            let col_head = div()
                .h(px(36.))
                .px(px(6.))
                .flex_none()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(6.))
                .child(Self::render_priority_mark(col.tone, &p))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_center()
                        .truncate()
                        .text_size(px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(col.label.to_uppercase()),
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
                        .child(format!("{count}")),
                );

            let mut cards_list = div()
                .id(pro_eid(&self.id, format!("col-cards-{}", col.id)))
                .flex_1()
                .min_h_0()
                .pb(px(6.))
                .flex()
                .flex_col()
                .gap(px(6.))
                .overflow_y_scroll();

            if self.loading {
                let skel_n = if col.tone.is_done() { 2usize } else { 3usize };
                for si in 0..skel_n {
                    let p_off = (phase + (col_idx * 3 + si) as f32 * 0.08) % 1.0;
                    cards_list = cards_list.child(
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
                                    .child(pro_skeleton_bar(14., 14., 4., p_off, cx))
                                    .child(pro_skeleton_bar(132., 12., 4., p_off, cx)),
                            )
                            .child(
                                div()
                                    .ml(px(22.))
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(pro_skeleton_bar(168., 10., 4., p_off, cx))
                                    .child(pro_skeleton_bar(124., 10., 4., p_off, cx)),
                            )
                            .child(
                                div()
                                    .ml(px(22.))
                                    .mt(px(2.))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(pro_skeleton_bar(78., 11., 4., p_off, cx))
                                    .child(pro_skeleton_bar(42., 11., 4., p_off, cx)),
                            ),
                    );
                }
            } else {
                for (slot_i, &c_idx) in card_idxs.iter().enumerate() {
                    let card = &self.cards[c_idx];
                    let cid = card.id.clone();
                    let cid_move = card.id.clone();
                    let col_for_slot = col.id.clone();
                    let is_dragging = active_drag_id.as_ref() == Some(&cid);
                    let is_focused = self.focused_card_id.as_ref() == Some(&cid);
                    let drop_before = self
                        .drag_over_col
                        .as_ref()
                        .is_some_and(|(dc, di)| dc == &col.id && *di == slot_i)
                        && !is_dragging;

                    if drop_before {
                        cards_list =
                            cards_list.child(div().h(px(2.)).mx(px(6.)).rounded(px(1.)).bg(p.blue));
                    }

                    let status_txt = card.status.label(cx);
                    cards_list = cards_list.child(
                        div()
                            .id(pro_eid(&self.id, format!("card-{cid}")))
                            .role(Role::Button)
                            .aria_label(card.title.clone())
                            .tab_index(0)
                            .pt(px(12.))
                            .pb(px(10.))
                            .px(px(14.))
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(if is_focused { p.line_strong } else { p.line })
                            .when(is_dragging, |d| d.opacity(0.4))
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .when(is_dragging, |d| d.cursor(CursorStyle::ClosedHand))
                            .when(!is_dragging, |d| d.cursor_pointer())
                            .hover(move |s| s.border_color(p.line_strong))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                                    this.focused_card_id = Some(cid.clone());
                                    this.active_popover = None;
                                    this.press_state = Some(CardPressState {
                                        card_id: cid.clone(),
                                        start_pos: ev.position,
                                        dragging: false,
                                    });
                                    cx.stop_propagation();
                                    cx.notify();
                                }),
                            )
                            .on_mouse_move(cx.listener(move |this, _, _, cx| {
                                if this
                                    .press_state
                                    .as_ref()
                                    .is_some_and(|ps| ps.dragging && ps.card_id != cid_move)
                                {
                                    let next_over = (col_for_slot.clone(), slot_i);
                                    if this.drag_over_col.as_ref() != Some(&next_over) {
                                        this.drag_over_col = Some(next_over);
                                        cx.notify();
                                    }
                                }
                            }))
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .items_start()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .mt(px(2.))
                                            .child(Self::render_priority_mark(col.tone, &p)),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .truncate()
                                            .text_size(px(13.))
                                            .line_height(px(20.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(card.title.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .ml(px(23.))
                                    .h(px(38.))
                                    .overflow_hidden()
                                    .text_size(px(12.5))
                                    .line_height(px(19.))
                                    .text_color(if card.description.is_empty() {
                                        p.faint
                                    } else {
                                        p.muted
                                    })
                                    .child(if card.description.is_empty() {
                                        tr(cx, "No description", "暂无描述").into()
                                    } else {
                                        card.description.clone()
                                    }),
                            )
                            .child(
                                div()
                                    .ml(px(23.))
                                    .mt(px(2.))
                                    .min_h(px(22.))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex()
                                            .items_center()
                                            .gap(px(7.))
                                            .text_size(px(12.5))
                                            .text_color(p.text)
                                            .child(Self::render_status_mark(card.status, &p))
                                            .child(div().truncate().child(status_txt)),
                                    )
                                    .child(Self::render_counts(
                                        card.comments,
                                        card.attachments,
                                        &p,
                                    )),
                            ),
                    );
                }

                let drop_at_end = self
                    .drag_over_col
                    .as_ref()
                    .is_some_and(|(dc, di)| dc == &col.id && *di >= card_idxs.len());
                if drop_at_end {
                    cards_list =
                        cards_list.child(div().h(px(2.)).mx(px(6.)).rounded(px(1.)).bg(p.blue));
                }

                let add_aria = format!("New task in {}", col.label);
                cards_list = cards_list.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("col-add-{}", col.id)))
                        .role(Role::Button)
                        .aria_label(add_aria)
                        .tab_index(0)
                        .h(px(26.))
                        .rounded(px(8.))
                        .opacity(if col_hovered || compact { 1.0 } else { 0.0 })
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(move |s| s.opacity(1.0).bg(p.chip))
                        .focus_visible(move |s| s.opacity(1.0).border_1().border_color(p.focus))
                        .child(pro_icon(ProIcon::Plus, 15., p.muted)),
                    cx,
                    move |this, w, cx| this.open_new_modal(Some(col_id_add.clone()), w, cx),
                ));
            }

            let col_tail_len = card_idxs.len();
            board = board.child(
                div()
                    .id(pro_eid(&self.id, format!("col-{}", col.id)))
                    .flex_1()
                    .min_w(px(if compact { 244. } else { 256. }))
                    .max_w(px(if visible_cols.len() == 1 { 460. } else { 340. }))
                    .min_h_0()
                    .p(px(4.))
                    .rounded(px(14.))
                    .bg(if is_over { p.chip } else { frame_bg })
                    .flex()
                    .flex_col()
                    .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                        if *hovered {
                            this.hovered_col = Some(col_id_hover.clone());
                        } else if this.hovered_col.as_ref() == Some(&col_id_hover) {
                            this.hovered_col = None;
                        }
                        cx.notify();
                    }))
                    .on_mouse_move(cx.listener(move |this, _, _, cx| {
                        if this.press_state.as_ref().is_some_and(|ps| ps.dragging) {
                            let already_in_col = this
                                .drag_over_col
                                .as_ref()
                                .is_some_and(|(dc, _)| dc == &col_id);
                            if !already_in_col {
                                this.drag_over_col = Some((col_id.clone(), col_tail_len));
                                cx.notify();
                            }
                        }
                    }))
                    .child(col_head)
                    .child(cards_list),
            );
        }

        board
    }

    fn render_list_view(
        &mut self,
        query: &str,
        phase: f32,
        elapsed_ms: f32,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let frame_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.035)
        } else {
            hsla(0., 0., 0., 0.025)
        };
        let visible_cols: Vec<KanbanColumn> = self
            .columns
            .iter()
            .filter(|c| self.column_matches_scope(c))
            .cloned()
            .collect();

        let mut list = div()
            .id(pro_eid(&self.id, "list-scroll"))
            .flex_1()
            .min_h_0()
            .px(px(if compact { 6. } else { 12. }))
            .pb(px(16.))
            .flex()
            .flex_col()
            .gap(px(14.))
            .overflow_y_scroll();

        for col in visible_cols {
            let card_idxs = self.visible_card_indices(&col.id, query);
            let count = if self.loading {
                if col.tone.is_done() { 2 } else { 3 }
            } else {
                card_idxs.len()
            };
            let col_add_id = col.id.clone();

            let mut group = div().flex().flex_col().gap(px(2.)).child(
                div()
                    .h(px(34.))
                    .mb(px(4.))
                    .pl(px(12.))
                    .pr(px(8.))
                    .rounded(px(10.))
                    .bg(frame_bg)
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(Self::render_priority_mark(col.tone, &p))
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(col.label.clone()),
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
                            .child(format!("{count}")),
                    )
                    .child(div().flex_1())
                    .child(pro_icon_btn(
                        pro_eid(&self.id, format!("list-add-{}", col.id)),
                        tr(cx, "New task", "新建任务"),
                        ProIcon::Plus,
                        24.,
                        false,
                        cx,
                        move |this, w, cx| {
                            this.open_new_modal(Some(col_add_id.clone()), w, cx);
                        },
                    )),
            );

            if self.loading {
                for si in 0..count {
                    let p_off = (phase + si as f32 * 0.11) % 1.0;
                    group = group.child(
                        div()
                            .h(px(42.))
                            .px(px(12.))
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(pro_skeleton_bar(44., 11., 4., p_off, cx))
                            .child(pro_skeleton_bar(168., 12., 4., p_off, cx))
                            .when(!compact, |d| {
                                d.child(
                                    div()
                                        .flex_1()
                                        .child(pro_skeleton_bar(240., 11., 4., p_off, cx)),
                                )
                            })
                            .child(pro_skeleton_bar(86., 12., 4., p_off, cx)),
                    );
                }
            } else if card_idxs.is_empty() {
                group = group.child(
                    div()
                        .px(px(12.))
                        .py(px(6.))
                        .text_size(px(12.5))
                        .text_color(p.faint)
                        .child(tr(
                            cx,
                            "No matching tasks in this group",
                            "该分组下暂无匹配任务",
                        )),
                );
            } else {
                for idx in card_idxs {
                    let card = &self.cards[idx];
                    let cid = card.id.clone();
                    let is_focused = self.focused_card_id.as_ref() == Some(&cid);
                    let status_txt = card.status.label(cx);

                    group = group.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("row-{cid}")))
                            .role(Role::Row)
                            .tab_index(0)
                            .h(px(42.))
                            .px(px(12.))
                            .rounded(px(10.))
                            .when(is_focused, |d| d.bg(p.hover))
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .cursor_pointer()
                            .hover(move |s| s.bg(p.hover))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .child(
                                div()
                                    .w(px(56.))
                                    .flex_none()
                                    .text_size(px(12.))
                                    .text_color(p.faint)
                                    .child(card.key.clone()),
                            )
                            .child(Self::render_priority_mark(col.tone, &p))
                            .child(
                                div()
                                    .when(!compact, |d| d.max_w(px(230.)))
                                    .when(compact, |d| d.flex_1())
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(card.title.clone()),
                            )
                            .when(!compact, |d| {
                                d.child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(13.))
                                        .text_color(p.muted)
                                        .child(card.description.clone()),
                                )
                            })
                            .child(Self::render_assignee_avatar(
                                card.assignee.as_ref(),
                                card.assignee_is_agent,
                                20.,
                                elapsed_ms,
                                &p,
                                cx,
                            ))
                            .child(
                                div()
                                    .when(!compact, |d| d.w(px(208.)))
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(12.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(7.))
                                            .text_size(px(12.5))
                                            .text_color(p.text)
                                            .child(Self::render_status_mark(card.status, &p))
                                            .child(status_txt),
                                    )
                                    .when(!compact, |d| {
                                        d.child(Self::render_counts(
                                            card.comments,
                                            card.attachments,
                                            &p,
                                        ))
                                    }),
                            ),
                        cx,
                        move |this, _, cx| this.open_card(cid.clone(), cx),
                    ));
                }
            }

            list = list.child(group);
        }

        list
    }

    fn render_open_issue(
        &mut self,
        open_id: &SharedString,
        elapsed_ms: f32,
        narrow_issue: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let Some(card) = self.cards.iter().find(|c| &c.id == open_id).cloned() else {
            return div().id(pro_eid(&self.id, "issue-missing"));
        };
        let col_meta = self
            .columns
            .iter()
            .find(|c| c.id == card.column)
            .cloned()
            .unwrap_or_else(|| KanbanColumn::new("low", "Low priority", KanbanTone::Low));

        let done_subs = card.sub_issues.iter().filter(|s| s.done).count();
        let total_subs = card.sub_issues.len();

        let cid_attach = card.id.clone();
        let cid_copy = card.id.clone();
        let cid_delete = card.id.clone();

        // Main left column: editable title, editable description, actions, sub-issues, activity
        let mut main_col = div().flex_1().min_w_0().max_w(px(720.)).flex().flex_col();

        if let Some(ref title_ed) = self.issue_title_editor {
            main_col = main_col.child(
                div()
                    .min_h(px(34.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(title_ed.clone()),
            );
        }
        if let Some(ref desc_ed) = self.issue_desc_editor {
            main_col = main_col.child(
                div()
                    .mt(px(10.))
                    .min_h(px(64.))
                    .text_color(p.text)
                    .child(desc_ed.clone()),
            );
        }

        // Quick action icons under description
        main_col = main_col.child(
            div()
                .mt(px(10.))
                .ml(px(-6.))
                .flex()
                .items_center()
                .gap(px(4.))
                .child(pro_btn(
                    pro_eid(&self.id, "issue-attach"),
                    format!("{} ({})", tr(cx, "Attach", "附件"), card.attachments),
                    Some(ProIcon::Paperclip),
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    move |this, _, cx| {
                        if let Some(c) = this.cards.iter_mut().find(|c| c.id == cid_attach) {
                            c.attachments += 1;
                            c.updated_at = "Just now".into();
                            let ev_id = format!("ev-{}", c.activity.len() + 1);
                            c.activity.push(KanbanActivity::event(
                                ev_id,
                                this.user_name.clone(),
                                false,
                                "attached trace-capture.har",
                                "Just now",
                            ));
                        }
                        cx.notify();
                    },
                ))
                .child(pro_btn(
                    pro_eid(&self.id, "issue-copy-key"),
                    card.key.clone(),
                    Some(ProIcon::Copy),
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    move |_, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(cid_copy.to_string()));
                    },
                ))
                .child(pro_btn(
                    pro_eid(&self.id, "issue-delete"),
                    tr(cx, "Delete", "删除"),
                    Some(ProIcon::Trash),
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    move |this, _, cx| {
                        this.cards.retain(|c| c.id != cid_delete);
                        this.close_card(cx);
                    },
                )),
        );

        // Sub-issues section
        let mut sub_sec = div()
            .mt(px(26.))
            .pt(px(20.))
            .border_t_1()
            .border_color(p.line_soft)
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .mb(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tr(cx, "Sub-issues", "子问题")),
                    )
                    .when(total_subs > 0, |d| {
                        d.child(
                            div()
                                .text_size(px(12.5))
                                .text_color(p.faint)
                                .child(format!("{done_subs}/{total_subs}")),
                        )
                    })
                    .child(div().flex_1())
                    .child(pro_btn(
                        pro_eid(&self.id, "sub-add-top"),
                        tr(cx, "+ Add sub-issue", "+ 添加子问题"),
                        None,
                        ProBtnStyle::Ghost,
                        true,
                        cx,
                        |this, w, cx| {
                            this.sub_issue_open = true;
                            if let Some(ed) = this.sub_issue_editor.clone() {
                                ed.update(cx, |e, cx| {
                                    e.set_value("", w, cx);
                                    e.focus(w, cx);
                                });
                            }
                            cx.notify();
                        },
                    )),
            )
            .when(total_subs > 0, |d| {
                d.child(div().mb(px(8.)).child(pro_spring_bar(
                    pro_eid(&self.id, format!("sub-bar-{}", card.id)),
                    (done_subs as f32 / total_subs as f32).clamp(0.0, 1.0),
                    5.,
                    2.5,
                    p.chip,
                    if p.dark { p.green } else { p.green_ink },
                )))
            });

        for sub in &card.sub_issues {
            let sid = sub.id.clone();
            let cid_sub = card.id.clone();
            let done = sub.done;
            sub_sec = sub_sec.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("sub-{sid}")))
                    .role(Role::CheckBox)
                    .aria_toggled(done.into())
                    .tab_index(0)
                    .h(px(34.))
                    .mx(px(-8.))
                    .px(px(8.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .child(if done {
                        pro_svg("0 0 16 16", SVG_SUB_DONE, 16., p.green_ink)
                    } else {
                        div()
                            .size(px(16.))
                            .rounded_full()
                            .border_1()
                            .border_color(p.line_strong)
                            .into_any_element()
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(13.5))
                            .text_color(if done { p.muted } else { p.ink })
                            .when(done, |d| d.line_through())
                            .child(sub.title.clone()),
                    ),
                cx,
                move |this, _, cx| {
                    if let Some(c) = this.cards.iter_mut().find(|c| c.id == cid_sub)
                        && let Some(s) = c.sub_issues.iter_mut().find(|s| s.id == sid)
                    {
                        s.done = !s.done;
                        c.updated_at = "Just now".into();
                        let updated = c.clone();
                        cx.emit(KanbanEvent::UpdateCard(updated));
                    }
                    cx.notify();
                },
            ));
        }

        if let Some(ref sub_ed) = self.sub_issue_editor {
            let sub_add_row = div()
                .h(px(34.))
                .mx(px(-8.))
                .px(px(10.))
                .rounded(px(8.))
                .bg(p.hover)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .items_center()
                .gap(px(8.))
                .child(div().flex_1().min_w_0().child(sub_ed.clone()))
                .child(pro_btn(
                    pro_eid(&self.id, "sub-save"),
                    tr(cx, "Add", "添加"),
                    None,
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    |this, w, cx| this.submit_sub_issue(w, cx),
                ));
            sub_sec = sub_sec.child(pro_fold(
                pro_eid(&self.id, "sub-add-fold"),
                self.sub_issue_open,
                38.,
                2.,
                move || sub_add_row.into_any_element(),
            ));
        }

        main_col = main_col.child(sub_sec);

        // Activity & Comments section
        let mut act_sec = div()
            .mt(px(26.))
            .pt(px(20.))
            .border_t_1()
            .border_color(p.line_soft)
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(tr(cx, "Activity", "动态记录")),
            );

        for entry in &card.activity {
            let av = if entry.is_agent {
                pro_agent_avatar(&entry.author, p.purple, 22., false, elapsed_ms, cx)
            } else {
                pro_person_avatar(&entry.author, 22., false, None, cx)
            };
            match entry.kind {
                KanbanActivityKind::Event => {
                    act_sec = act_sec.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .text_size(px(13.))
                            .text_color(p.muted)
                            .child(av)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_wrap()
                                    .items_center()
                                    .gap(px(5.))
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(entry.author.clone()),
                                    )
                                    .child(entry.body.clone())
                                    .child(div().text_color(p.faint).child("·"))
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.faint)
                                            .child(entry.timestamp.clone()),
                                    ),
                            ),
                    );
                }
                KanbanActivityKind::Comment => {
                    act_sec = act_sec.child(
                        div()
                            .p(px(12.))
                            .px(px(14.))
                            .rounded(px(12.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .text_size(px(13.))
                                    .child(av)
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(entry.author.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.faint)
                                            .child(entry.timestamp.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .ml(px(30.))
                                    .text_size(px(13.5))
                                    .line_height(px(21.))
                                    .text_color(p.text)
                                    .child(entry.body.clone()),
                            ),
                    );
                }
            }
        }

        // Comment composer
        if let Some(ref cmt_ed) = self.comment_editor {
            let can_send = !cmt_ed.read(cx).value().trim().is_empty();
            act_sec = act_sec.child(
                div()
                    .mt(px(4.))
                    .pt(px(12.))
                    .pb(px(8.))
                    .pl(px(14.))
                    .pr(px(10.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(div().min_h(px(46.)).child(cmt_ed.clone()))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(8.))
                            .child(div().text_size(px(11.5)).text_color(p.faint).child("⌘↵"))
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, "comment-send"))
                                    .role(Role::Button)
                                    .aria_label(tr(cx, "Send comment", "发送评论"))
                                    .tab_index(0)
                                    .size(px(28.))
                                    .rounded_full()
                                    .bg(if can_send { p.primary } else { p.chip })
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .when(can_send, |d| d.cursor_pointer())
                                    .when(!can_send, |d| d.cursor_default())
                                    .child(pro_icon(
                                        ProIcon::ArrowUp,
                                        14.,
                                        if can_send { p.on_primary } else { p.faint },
                                    )),
                                cx,
                                move |this, w, cx| {
                                    if can_send {
                                        this.submit_comment(w, cx);
                                    }
                                },
                            )),
                    ),
            );
        }

        main_col = main_col.child(act_sec);

        // Right 288px Properties Card
        let props_card =
            self.render_properties_card(&card, &col_meta, elapsed_ms, narrow_issue, cx);

        let mut layout = div()
            .id(pro_eid(&self.id, "issue-detail-scroll"))
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .px(px(if narrow_issue { 20. } else { 44. }))
            .pt(px(if narrow_issue { 14. } else { 28. }))
            .pb(px(40.))
            .flex()
            .when(narrow_issue, |d| d.flex_col().gap(px(24.)))
            .when(!narrow_issue, |d| d.items_start().gap(px(40.)));

        if narrow_issue {
            layout = layout.child(props_card).child(main_col);
        } else {
            layout = layout.child(main_col).child(props_card);
        }
        layout
    }

    fn render_properties_card(
        &self,
        card: &KanbanCard,
        col_meta: &KanbanColumn,
        elapsed_ms: f32,
        narrow: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let card_id = card.id.clone();

        let status_open = self.active_popover == Some(ActivePopover::PropStatus);
        let col_open = self.active_popover == Some(ActivePopover::PropColumn);
        let who_open = self.active_popover == Some(ActivePopover::PropAssignee);
        let lbl_open = self.active_popover == Some(ActivePopover::PropLabels);
        let proj_open = self.active_popover == Some(ActivePopover::PropProject);

        let col_idx = self
            .columns
            .iter()
            .position(|c| c.id == card.column)
            .unwrap_or(0);
        let can_left = col_idx > 0;
        let can_right = col_idx + 1 < self.columns.len();

        let mut card_box = div()
            .when(!narrow, |d| d.w(px(288.)).flex_none())
            .when(narrow, |d| d.w_full())
            .pt(px(14.))
            .pb(px(12.))
            .px(px(10.))
            .rounded(px(14.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .px(px(8.))
                    .pb(px(6.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(tr(cx, "Properties", "属性")),
            );

        // 1. Status row
        card_box = card_box.child(
            div()
                .min_h(px(34.))
                .pl(px(8.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(p.muted)
                        .child(tr(cx, "Status", "状态")),
                )
                .child(
                    div()
                        .relative()
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, "prop-status"))
                                .role(Role::Button)
                                .aria_expanded(status_open)
                                .tab_index(0)
                                .h(px(30.))
                                .px(px(8.))
                                .rounded(px(8.))
                                .when(status_open, |d| d.bg(p.hover))
                                .flex()
                                .items_center()
                                .gap(px(7.))
                                .text_size(px(13.))
                                .text_color(p.ink)
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover))
                                .child(Self::render_status_mark(card.status, &p))
                                .child(card.status.label(cx)),
                            cx,
                            |this, _, cx| {
                                this.active_popover =
                                    if this.active_popover == Some(ActivePopover::PropStatus) {
                                        None
                                    } else {
                                        Some(ActivePopover::PropStatus)
                                    };
                                cx.notify();
                            },
                        ))
                        .child({
                            let p_menu = self
                                .render_prop_status_menu(&card_id, card.status, &p, cx)
                                .shadow_md();
                            pro_presence(
                                pro_eid(&self.id, "prop-status-pres"),
                                status_open,
                                0.18,
                                0.12,
                                move |a| {
                                    p_menu
                                        .top(px(34. - 4. * (1. - a)))
                                        .opacity(a)
                                        .into_any_element()
                                },
                            )
                        }),
                ),
        );

        // 2. Priority / Column row
        card_box = card_box.child(
            div()
                .min_h(px(34.))
                .pl(px(8.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(div().text_size(px(13.)).text_color(p.muted).child(tr(
                    cx,
                    "Priority",
                    "优先级",
                )))
                .child(
                    div()
                        .relative()
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, "prop-col"))
                                .role(Role::Button)
                                .aria_expanded(col_open)
                                .tab_index(0)
                                .h(px(30.))
                                .px(px(8.))
                                .rounded(px(8.))
                                .when(col_open, |d| d.bg(p.hover))
                                .flex()
                                .items_center()
                                .gap(px(7.))
                                .text_size(px(13.))
                                .text_color(p.ink)
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover))
                                .child(Self::render_priority_mark(col_meta.tone, &p))
                                .child(col_meta.label.clone()),
                            cx,
                            |this, _, cx| {
                                this.active_popover =
                                    if this.active_popover == Some(ActivePopover::PropColumn) {
                                        None
                                    } else {
                                        Some(ActivePopover::PropColumn)
                                    };
                                cx.notify();
                            },
                        ))
                        .child({
                            let p_menu = self
                                .render_prop_column_menu(&card_id, &card.column, &p, cx)
                                .shadow_md();
                            pro_presence(
                                pro_eid(&self.id, "prop-col-pres"),
                                col_open,
                                0.18,
                                0.12,
                                move |a| {
                                    p_menu
                                        .top(px(34. - 4. * (1. - a)))
                                        .opacity(a)
                                        .into_any_element()
                                },
                            )
                        }),
                ),
        );

        // 3. Assignee row
        let who_label: SharedString = card
            .assignee
            .clone()
            .unwrap_or_else(|| tr(cx, "Unassigned", "未指派").into());
        card_box = card_box.child(
            div()
                .min_h(px(34.))
                .pl(px(8.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(div().text_size(px(13.)).text_color(p.muted).child(tr(
                    cx,
                    "Assignee",
                    "负责人",
                )))
                .child(
                    div()
                        .relative()
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, "prop-who"))
                                .role(Role::Button)
                                .aria_expanded(who_open)
                                .tab_index(0)
                                .h(px(30.))
                                .px(px(8.))
                                .rounded(px(8.))
                                .when(who_open, |d| d.bg(p.hover))
                                .flex()
                                .items_center()
                                .gap(px(7.))
                                .text_size(px(13.))
                                .text_color(if card.assignee.is_some() {
                                    p.ink
                                } else {
                                    p.muted
                                })
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover))
                                .child(Self::render_assignee_avatar(
                                    card.assignee.as_ref(),
                                    card.assignee_is_agent,
                                    20.,
                                    elapsed_ms,
                                    &p,
                                    cx,
                                ))
                                .child(who_label),
                            cx,
                            |this, _, cx| {
                                this.active_popover =
                                    if this.active_popover == Some(ActivePopover::PropAssignee) {
                                        None
                                    } else {
                                        Some(ActivePopover::PropAssignee)
                                    };
                                cx.notify();
                            },
                        ))
                        .child({
                            let p_menu = self
                                .render_prop_assignee_menu(
                                    &card_id,
                                    card.assignee.as_ref(),
                                    elapsed_ms,
                                    &p,
                                    cx,
                                )
                                .shadow_md();
                            pro_presence(
                                pro_eid(&self.id, "prop-who-pres"),
                                who_open,
                                0.18,
                                0.12,
                                move |a| {
                                    p_menu
                                        .top(px(34. - 4. * (1. - a)))
                                        .opacity(a)
                                        .into_any_element()
                                },
                            )
                        }),
                ),
        );

        // 4. Labels row
        let mut labels_wrap = div()
            .py(px(4.))
            .flex()
            .flex_wrap()
            .justify_end()
            .items_center()
            .gap(px(4.));
        for lbl in &card.labels {
            let dot_c = Self::label_dot_color(lbl, &p);
            labels_wrap = labels_wrap.child(
                div()
                    .px(px(7.))
                    .py(px(2.))
                    .rounded(px(6.))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(12.))
                    .text_color(p.text)
                    .child(div().size(px(6.5)).rounded_full().bg(dot_c))
                    .child(lbl.clone()),
            );
        }
        labels_wrap = labels_wrap.child(
            div()
                .relative()
                .child(pro_icon_btn(
                    pro_eid(&self.id, "prop-lbl-plus"),
                    tr(cx, "Edit labels", "编辑标签"),
                    ProIcon::Plus,
                    26.,
                    lbl_open,
                    cx,
                    |this, _, cx| {
                        this.active_popover =
                            if this.active_popover == Some(ActivePopover::PropLabels) {
                                None
                            } else {
                                Some(ActivePopover::PropLabels)
                            };
                        cx.notify();
                    },
                ))
                .child({
                    let p_menu = self
                        .render_prop_labels_menu(&card_id, &card.labels, &p, cx)
                        .shadow_md();
                    pro_presence(
                        pro_eid(&self.id, "prop-lbl-pres"),
                        lbl_open,
                        0.18,
                        0.12,
                        move |a| {
                            p_menu
                                .top(px(30. - 4. * (1. - a)))
                                .opacity(a)
                                .into_any_element()
                        },
                    )
                }),
        );

        card_box = card_box.child(
            div()
                .min_h(px(34.))
                .pl(px(8.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(p.muted)
                        .child(tr(cx, "Labels", "标签")),
                )
                .child(labels_wrap),
        );

        // 5. Project row
        card_box = card_box.child(
            div()
                .min_h(px(34.))
                .pl(px(8.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(div().text_size(px(13.)).text_color(p.muted).child(tr(
                    cx,
                    "Project",
                    "所属项目",
                )))
                .child(
                    div()
                        .relative()
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, "prop-proj"))
                                .role(Role::Button)
                                .aria_expanded(proj_open)
                                .tab_index(0)
                                .h(px(30.))
                                .px(px(8.))
                                .rounded(px(8.))
                                .when(proj_open, |d| d.bg(p.hover))
                                .flex()
                                .items_center()
                                .gap(px(7.))
                                .text_size(px(13.))
                                .text_color(p.ink)
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover))
                                .child(pro_icon(ProIcon::Folder, 14., p.muted))
                                .child(card.project.clone()),
                            cx,
                            |this, _, cx| {
                                this.active_popover =
                                    if this.active_popover == Some(ActivePopover::PropProject) {
                                        None
                                    } else {
                                        Some(ActivePopover::PropProject)
                                    };
                                cx.notify();
                            },
                        ))
                        .child({
                            let p_menu = self
                                .render_prop_project_menu(&card_id, &card.project, &p, cx)
                                .shadow_md();
                            pro_presence(
                                pro_eid(&self.id, "prop-proj-pres"),
                                proj_open,
                                0.18,
                                0.12,
                                move |a| {
                                    p_menu
                                        .top(px(34. - 4. * (1. - a)))
                                        .opacity(a)
                                        .into_any_element()
                                },
                            )
                        }),
                ),
        );

        // Move column quick actions + timestamps footer
        card_box =
            card_box.child(
                div()
                    .mt(px(10.))
                    .pt(px(10.))
                    .px(px(8.))
                    .border_t_1()
                    .border_color(p.line_soft)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(p.faint)
                            .child(format!("{} · {}", card.created_at, card.updated_at)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .child(
                                div()
                                    .when(!can_left, |d| d.opacity(0.35))
                                    .child(pro_icon_btn(
                                        pro_eid(&self.id, "prop-move-left"),
                                        tr(cx, "Move to previous column", "移至前一列"),
                                        ProIcon::ArrowLeft,
                                        24.,
                                        false,
                                        cx,
                                        move |this, _, cx| {
                                            if can_left {
                                                this.shift_focused_column(-1, cx);
                                            }
                                        },
                                    )),
                            )
                            .child(div().when(!can_right, |d| d.opacity(0.35)).child(
                                pro_icon_btn(
                                    pro_eid(&self.id, "prop-move-right"),
                                    tr(cx, "Move to next column", "移至后一列"),
                                    ProIcon::ArrowRight,
                                    24.,
                                    false,
                                    cx,
                                    move |this, _, cx| {
                                        if can_right {
                                            this.shift_focused_column(1, cx);
                                        }
                                    },
                                ),
                            )),
                    ),
            );

        card_box
    }

    fn render_prop_status_menu(
        &self,
        card_id: &SharedString,
        current: KanbanStatus,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut panel = div()
            .occlude()
            .absolute()
            .top(px(34.))
            .right_0()
            .w(px(196.))
            .p(px(5.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_col()
            .gap(px(1.));

        for st in KanbanStatus::ALL {
            let checked = current == st;
            let lbl = st.label(cx);
            let cid = card_id.clone();
            panel = panel.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("pst-{st:?}")))
                    .role(Role::MenuItem)
                    .aria_toggled(checked.into())
                    .tab_index(0)
                    .h(px(30.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .text_color(if checked { p.ink } else { p.text })
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(Self::render_status_mark(st, p))
                            .child(lbl),
                    )
                    .when(checked, |d| {
                        d.child(pro_svg("0 0 24 24", SVG_CHECK_TICK, 14., p.ink))
                    }),
                cx,
                move |this, _, cx| {
                    if let Some(c) = this.cards.iter_mut().find(|c| c.id == cid)
                        && c.status != st
                    {
                        c.status = st;
                        c.updated_at = "Just now".into();
                        let ev_id = format!("ev-{}", c.activity.len() + 1);
                        c.activity.push(KanbanActivity::event(
                            ev_id,
                            this.user_name.clone(),
                            false,
                            format!("changed status to {}", st.label(cx)),
                            "Just now",
                        ));
                        let updated = c.clone();
                        cx.emit(KanbanEvent::UpdateCard(updated));
                    }
                    this.active_popover = None;
                    cx.notify();
                },
            ));
        }
        panel
    }

    fn render_prop_column_menu(
        &self,
        card_id: &SharedString,
        current_col: &SharedString,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut panel = div()
            .occlude()
            .absolute()
            .top(px(34.))
            .right_0()
            .w(px(196.))
            .p(px(5.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_col()
            .gap(px(1.));

        for col in &self.columns {
            let checked = &col.id == current_col;
            let cid = card_id.clone();
            let target_col = col.id.clone();
            panel = panel.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("pcol-{}", col.id)))
                    .role(Role::MenuItem)
                    .aria_toggled(checked.into())
                    .tab_index(0)
                    .h(px(30.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .text_color(if checked { p.ink } else { p.text })
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(Self::render_priority_mark(col.tone, p))
                            .child(col.label.clone()),
                    )
                    .when(checked, |d| {
                        d.child(pro_svg("0 0 24 24", SVG_CHECK_TICK, 14., p.ink))
                    }),
                cx,
                move |this, _, cx| {
                    this.move_card(&cid, &target_col, None, cx);
                    this.active_popover = None;
                    cx.notify();
                },
            ));
        }
        panel
    }

    fn render_prop_assignee_menu(
        &self,
        card_id: &SharedString,
        current: Option<&SharedString>,
        elapsed_ms: f32,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut panel = div()
            .occlude()
            .absolute()
            .top(px(34.))
            .right_0()
            .w(px(208.))
            .p(px(5.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_col()
            .gap(px(1.));

        for (who_name, is_agent) in ASSIGNEES {
            let checked = current.is_some_and(|w| w.as_ref() == who_name);
            let who_shared: SharedString = who_name.into();
            let cid = card_id.clone();
            panel = panel.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("pwho-{who_name}")))
                    .role(Role::MenuItem)
                    .aria_toggled(checked.into())
                    .tab_index(0)
                    .h(px(32.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .text_color(if checked { p.ink } else { p.text })
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(Self::render_assignee_avatar(
                                Some(&who_shared),
                                is_agent,
                                20.,
                                elapsed_ms,
                                p,
                                cx,
                            ))
                            .child(who_name),
                    )
                    .when(checked, |d| {
                        d.child(pro_svg("0 0 24 24", SVG_CHECK_TICK, 14., p.ink))
                    }),
                cx,
                move |this, _, cx| {
                    if let Some(c) = this.cards.iter_mut().find(|c| c.id == cid) {
                        c.assignee = Some(who_shared.clone());
                        c.assignee_is_agent = is_agent;
                        c.updated_at = "Just now".into();
                        let ev_id = format!("ev-{}", c.activity.len() + 1);
                        c.activity.push(KanbanActivity::event(
                            ev_id,
                            this.user_name.clone(),
                            false,
                            format!("assigned to {who_shared}"),
                            "Just now",
                        ));
                        let updated = c.clone();
                        cx.emit(KanbanEvent::UpdateCard(updated));
                    }
                    this.active_popover = None;
                    cx.notify();
                },
            ));
        }
        panel
    }

    fn render_prop_labels_menu(
        &self,
        card_id: &SharedString,
        current: &[SharedString],
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut panel = div()
            .occlude()
            .absolute()
            .top(px(30.))
            .right_0()
            .w(px(184.))
            .p(px(5.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_col()
            .gap(px(1.));

        for (lbl_name, hex) in ALL_LABELS {
            let checked = current.iter().any(|l| l.as_ref() == lbl_name);
            let dot_c: Hsla = rgb(hex).into();
            let lbl_s: SharedString = lbl_name.into();
            let cid = card_id.clone();
            panel = panel.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("plbl-{lbl_name}")))
                    .role(Role::MenuItem)
                    .aria_toggled(checked.into())
                    .tab_index(0)
                    .h(px(30.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(13.))
                    .text_color(if checked { p.ink } else { p.text })
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(div().size(px(8.)).rounded_full().bg(dot_c))
                            .child(lbl_name),
                    )
                    .when(checked, |d| {
                        d.child(pro_svg("0 0 24 24", SVG_CHECK_TICK, 14., p.ink))
                    }),
                cx,
                move |this, _, cx| {
                    if let Some(c) = this.cards.iter_mut().find(|c| c.id == cid) {
                        let action_verb =
                            if let Some(pos) = c.labels.iter().position(|x| x == &lbl_s) {
                                c.labels.remove(pos);
                                "removed label"
                            } else {
                                c.labels.push(lbl_s.clone());
                                "added label"
                            };
                        c.updated_at = "Just now".into();
                        let ev_id = format!("ev-{}", c.activity.len() + 1);
                        c.activity.push(KanbanActivity::event(
                            ev_id,
                            this.user_name.clone(),
                            false,
                            format!("{action_verb} {lbl_s}"),
                            "Just now",
                        ));
                        let updated = c.clone();
                        cx.emit(KanbanEvent::UpdateCard(updated));
                    }
                    cx.notify();
                },
            ));
        }
        panel
    }

    fn render_prop_project_menu(
        &self,
        card_id: &SharedString,
        current: &SharedString,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut panel = div()
            .occlude()
            .absolute()
            .top(px(34.))
            .right_0()
            .w(px(188.))
            .p(px(5.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_col()
            .gap(px(1.));

        for proj_name in PROJECTS {
            let checked = current.as_ref() == proj_name;
            let proj_s: SharedString = proj_name.into();
            let cid = card_id.clone();
            panel = panel.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("pproj-{proj_name}")))
                    .role(Role::MenuItem)
                    .aria_toggled(checked.into())
                    .tab_index(0)
                    .h(px(30.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(13.))
                    .text_color(if checked { p.ink } else { p.text })
                    .cursor_pointer()
                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                    .child(proj_name)
                    .when(checked, |d| {
                        d.child(pro_svg("0 0 24 24", SVG_CHECK_TICK, 14., p.ink))
                    }),
                cx,
                move |this, _, cx| {
                    if let Some(c) = this.cards.iter_mut().find(|c| c.id == cid) {
                        c.project = proj_s.clone();
                        c.updated_at = "Just now".into();
                        let ev_id = format!("ev-{}", c.activity.len() + 1);
                        c.activity.push(KanbanActivity::event(
                            ev_id,
                            this.user_name.clone(),
                            false,
                            format!("moved to project {proj_s}"),
                            "Just now",
                        ));
                        let updated = c.clone();
                        cx.emit(KanbanEvent::UpdateCard(updated));
                    }
                    this.active_popover = None;
                    cx.notify();
                },
            ));
        }
        panel
    }

    fn render_new_modal(
        &mut self,
        elapsed_ms: f32,
        narrow: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let scrim_max = if p.dark { 0.48_f32 } else { 0.16_f32 };
        let frame_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.03)
        } else {
            hsla(0., 0., 0., 0.02)
        };

        let col_meta = self
            .columns
            .iter()
            .find(|c| c.id == self.modal_column)
            .cloned()
            .unwrap_or_else(|| KanbanColumn::new("low", "Low priority", KanbanTone::Low));

        let can_create = self
            .modal_title_editor
            .as_ref()
            .is_some_and(|e| !e.read(cx).value().trim().is_empty());

        let m_col_open = self.active_popover == Some(ActivePopover::ModalColumn);
        let m_st_open = self.active_popover == Some(ActivePopover::ModalStatus);
        let m_who_open = self.active_popover == Some(ActivePopover::ModalAssignee);
        let m_lbl_open = self.active_popover == Some(ActivePopover::ModalLabel);

        let mut side = div()
            .when(!narrow, |d| {
                d.w(px(248.))
                    .flex_none()
                    .border_l_1()
                    .border_color(p.line_soft)
            })
            .when(narrow, |d| {
                d.w_full().border_t_1().border_color(p.line_soft)
            })
            .py(px(14.))
            .px(px(12.))
            .bg(frame_bg)
            .flex()
            .flex_col()
            .gap(px(4.));

        // Modal Priority selector
        side = side.child(
            div()
                .min_h(px(34.))
                .pl(px(6.))
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_size(px(12.5)).text_color(p.muted).child(tr(
                    cx,
                    "Priority",
                    "优先级",
                )))
                .child(
                    div()
                        .relative()
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, "modal-col-btn"))
                                .role(Role::Button)
                                .tab_index(0)
                                .h(px(28.))
                                .px(px(8.))
                                .rounded(px(7.))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(12.5))
                                .text_color(p.ink)
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover))
                                .child(Self::render_priority_mark(col_meta.tone, &p))
                                .child(col_meta.label.clone()),
                            cx,
                            |this, _, cx| {
                                this.active_popover =
                                    if this.active_popover == Some(ActivePopover::ModalColumn) {
                                        None
                                    } else {
                                        Some(ActivePopover::ModalColumn)
                                    };
                                cx.notify();
                            },
                        ))
                        .child({
                            let mut menu = div()
                                .occlude()
                                .absolute()
                                .right_0()
                                .w(px(180.))
                                .p(px(4.))
                                .rounded(px(10.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line_strong)
                                .shadow_md()
                                .flex()
                                .flex_col();
                            for c in &self.columns {
                                let cid = c.id.clone();
                                menu = menu.child(activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("mcol-{cid}")))
                                        .role(Role::MenuItem)
                                        .tab_index(0)
                                        .h(px(28.))
                                        .px(px(8.))
                                        .rounded(px(7.))
                                        .flex()
                                        .items_center()
                                        .gap(px(7.))
                                        .text_size(px(12.5))
                                        .text_color(p.ink)
                                        .cursor_pointer()
                                        .hover(move |s| s.bg(p.hover))
                                        .child(Self::render_priority_mark(c.tone, &p))
                                        .child(c.label.clone()),
                                    cx,
                                    move |this, _, cx| {
                                        this.modal_column = cid.clone();
                                        this.active_popover = None;
                                        cx.notify();
                                    },
                                ));
                            }
                            pro_presence(
                                pro_eid(&self.id, "mcol-pres"),
                                m_col_open,
                                0.18,
                                0.12,
                                move |a| {
                                    menu.top(px(30. - 4. * (1. - a)))
                                        .opacity(a)
                                        .into_any_element()
                                },
                            )
                        }),
                ),
        );

        // Modal Status selector
        side = side.child(
            div()
                .min_h(px(34.))
                .pl(px(6.))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(12.5))
                        .text_color(p.muted)
                        .child(tr(cx, "Status", "状态")),
                )
                .child(
                    div()
                        .relative()
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, "modal-st-btn"))
                                .role(Role::Button)
                                .tab_index(0)
                                .h(px(28.))
                                .px(px(8.))
                                .rounded(px(7.))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(12.5))
                                .text_color(p.ink)
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover))
                                .child(Self::render_status_mark(self.modal_status, &p))
                                .child(self.modal_status.label(cx)),
                            cx,
                            |this, _, cx| {
                                this.active_popover =
                                    if this.active_popover == Some(ActivePopover::ModalStatus) {
                                        None
                                    } else {
                                        Some(ActivePopover::ModalStatus)
                                    };
                                cx.notify();
                            },
                        ))
                        .child({
                            let mut menu = div()
                                .occlude()
                                .absolute()
                                .right_0()
                                .w(px(180.))
                                .p(px(4.))
                                .rounded(px(10.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line_strong)
                                .shadow_md()
                                .flex()
                                .flex_col();
                            for st in KanbanStatus::ALL {
                                menu = menu.child(activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("mst-{st:?}")))
                                        .role(Role::MenuItem)
                                        .tab_index(0)
                                        .h(px(28.))
                                        .px(px(8.))
                                        .rounded(px(7.))
                                        .flex()
                                        .items_center()
                                        .gap(px(7.))
                                        .text_size(px(12.5))
                                        .text_color(p.ink)
                                        .cursor_pointer()
                                        .hover(move |s| s.bg(p.hover))
                                        .child(Self::render_status_mark(st, &p))
                                        .child(st.label(cx)),
                                    cx,
                                    move |this, _, cx| {
                                        this.modal_status = st;
                                        this.active_popover = None;
                                        cx.notify();
                                    },
                                ));
                            }
                            pro_presence(
                                pro_eid(&self.id, "mst-pres"),
                                m_st_open,
                                0.18,
                                0.12,
                                move |a| {
                                    menu.top(px(30. - 4. * (1. - a)))
                                        .opacity(a)
                                        .into_any_element()
                                },
                            )
                        }),
                ),
        );

        // Modal Assignee selector
        let modal_who_lbl: SharedString = self
            .modal_assignee
            .clone()
            .unwrap_or_else(|| tr(cx, "Unassigned", "未指派").into());
        side = side.child(
            div()
                .min_h(px(34.))
                .pl(px(6.))
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_size(px(12.5)).text_color(p.muted).child(tr(
                    cx,
                    "Assignee",
                    "负责人",
                )))
                .child(
                    div()
                        .relative()
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, "modal-who-btn"))
                                .role(Role::Button)
                                .tab_index(0)
                                .h(px(28.))
                                .px(px(8.))
                                .rounded(px(7.))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(12.5))
                                .text_color(p.ink)
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover))
                                .child(Self::render_assignee_avatar(
                                    self.modal_assignee.as_ref(),
                                    self.modal_assignee_agent,
                                    18.,
                                    elapsed_ms,
                                    &p,
                                    cx,
                                ))
                                .child(modal_who_lbl),
                            cx,
                            |this, _, cx| {
                                this.active_popover =
                                    if this.active_popover == Some(ActivePopover::ModalAssignee) {
                                        None
                                    } else {
                                        Some(ActivePopover::ModalAssignee)
                                    };
                                cx.notify();
                            },
                        ))
                        .child({
                            let mut menu = div()
                                .occlude()
                                .absolute()
                                .right_0()
                                .w(px(184.))
                                .p(px(4.))
                                .rounded(px(10.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line_strong)
                                .shadow_md()
                                .flex()
                                .flex_col();
                            for (who_name, is_ag) in ASSIGNEES {
                                let ws: SharedString = who_name.into();
                                menu = menu.child(activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("mwho-{who_name}")))
                                        .role(Role::MenuItem)
                                        .tab_index(0)
                                        .h(px(28.))
                                        .px(px(8.))
                                        .rounded(px(7.))
                                        .flex()
                                        .items_center()
                                        .gap(px(7.))
                                        .text_size(px(12.5))
                                        .text_color(p.ink)
                                        .cursor_pointer()
                                        .hover(move |s| s.bg(p.hover))
                                        .child(Self::render_assignee_avatar(
                                            Some(&ws),
                                            is_ag,
                                            18.,
                                            elapsed_ms,
                                            &p,
                                            cx,
                                        ))
                                        .child(who_name),
                                    cx,
                                    move |this, _, cx| {
                                        this.modal_assignee = Some(ws.clone());
                                        this.modal_assignee_agent = is_ag;
                                        this.active_popover = None;
                                        cx.notify();
                                    },
                                ));
                            }
                            pro_presence(
                                pro_eid(&self.id, "mwho-pres"),
                                m_who_open,
                                0.18,
                                0.12,
                                move |a| {
                                    menu.top(px(30. - 4. * (1. - a)))
                                        .opacity(a)
                                        .into_any_element()
                                },
                            )
                        }),
                ),
        );

        // Modal Label selector
        let modal_lbl_txt: SharedString = self
            .modal_label
            .clone()
            .unwrap_or_else(|| tr(cx, "None", "无").into());
        side = side.child(
            div()
                .min_h(px(34.))
                .pl(px(6.))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(12.5))
                        .text_color(p.muted)
                        .child(tr(cx, "Label", "标签")),
                )
                .child(
                    div()
                        .relative()
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, "modal-lbl-btn"))
                                .role(Role::Button)
                                .tab_index(0)
                                .h(px(28.))
                                .px(px(8.))
                                .rounded(px(7.))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(12.5))
                                .text_color(p.ink)
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover))
                                .child(
                                    div()
                                        .size(px(7.))
                                        .rounded_full()
                                        .bg(Self::label_dot_color(&modal_lbl_txt, &p)),
                                )
                                .child(modal_lbl_txt),
                            cx,
                            |this, _, cx| {
                                this.active_popover =
                                    if this.active_popover == Some(ActivePopover::ModalLabel) {
                                        None
                                    } else {
                                        Some(ActivePopover::ModalLabel)
                                    };
                                cx.notify();
                            },
                        ))
                        .child({
                            let mut menu = div()
                                .occlude()
                                .absolute()
                                .right_0()
                                .w(px(164.))
                                .p(px(4.))
                                .rounded(px(10.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line_strong)
                                .shadow_md()
                                .flex()
                                .flex_col();
                            for (lname, hex) in ALL_LABELS {
                                let ls: SharedString = lname.into();
                                let dc: Hsla = rgb(hex).into();
                                menu = menu.child(activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("mlbl-{lname}")))
                                        .role(Role::MenuItem)
                                        .tab_index(0)
                                        .h(px(28.))
                                        .px(px(8.))
                                        .rounded(px(7.))
                                        .flex()
                                        .items_center()
                                        .gap(px(7.))
                                        .text_size(px(12.5))
                                        .text_color(p.ink)
                                        .cursor_pointer()
                                        .hover(move |s| s.bg(p.hover))
                                        .child(div().size(px(7.)).rounded_full().bg(dc))
                                        .child(lname),
                                    cx,
                                    move |this, _, cx| {
                                        this.modal_label = Some(ls.clone());
                                        this.active_popover = None;
                                        cx.notify();
                                    },
                                ));
                            }
                            pro_presence(
                                pro_eid(&self.id, "mlbl-pres"),
                                m_lbl_open,
                                0.18,
                                0.12,
                                move |a| {
                                    menu.top(px(30. - 4. * (1. - a)))
                                        .opacity(a)
                                        .into_any_element()
                                },
                            )
                        }),
                ),
        );

        let mut main_fields = div()
            .flex_1()
            .min_w_0()
            .p(px(18.))
            .flex()
            .flex_col()
            .gap(px(14.));

        if let Some(ref ted) = self.modal_title_editor {
            main_fields = main_fields.child(
                div()
                    .h(px(34.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .bg(p.subtle)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .items_center()
                    .font_weight(FontWeight::MEDIUM)
                    .child(ted.clone()),
            );
        }
        if let Some(ref ded) = self.modal_desc_editor {
            main_fields = main_fields.child(
                div()
                    .min_h(px(96.))
                    .p(px(10.))
                    .rounded(px(8.))
                    .bg(p.subtle)
                    .border_1()
                    .border_color(p.line)
                    .child(ded.clone()),
            );
        }
        main_fields = main_fields.child(pro_btn(
            pro_eid(&self.id, "modal-attach"),
            if self.modal_attachments > 0 {
                format!(
                    "{} ({})",
                    tr(cx, "Attach file", "添加附件"),
                    self.modal_attachments
                )
            } else {
                tr(cx, "Attach file", "添加附件").to_owned()
            },
            Some(ProIcon::Paperclip),
            ProBtnStyle::Ghost,
            true,
            cx,
            |this, _, cx| {
                this.modal_attachments += 1;
                cx.notify();
            },
        ));

        let dialog = div()
            .occlude()
            .w_full()
            .max_w(px(720.))
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
                    .pl(px(18.))
                    .pr(px(12.))
                    .py(px(14.))
                    .border_b_1()
                    .border_color(p.line_soft)
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .size(px(30.))
                            .rounded(px(9.))
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_icon(ProIcon::Tasks, 15., p.ink)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "New issue", "新建问题")),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(format!("{} · {}", self.workspace, self.board_title)),
                            ),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "modal-close"),
                        tr(cx, "Close", "关闭"),
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
                    .flex()
                    .when(narrow, |d| d.flex_col())
                    .child(main_fields)
                    .child(side),
            )
            .child(
                div()
                    .pl(px(18.))
                    .pr(px(14.))
                    .py(px(12.))
                    .border_t_1()
                    .border_color(p.line_soft)
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .mr_auto()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(pro_switch(
                                pro_eid(&self.id, "modal-more-switch"),
                                tr(cx, "Create more", "继续创建"),
                                self.modal_create_more,
                                cx,
                                |this, _, cx| {
                                    this.modal_create_more = !this.modal_create_more;
                                    cx.notify();
                                },
                            ))
                            .child(div().text_size(px(12.5)).text_color(p.muted).child(tr(
                                cx,
                                "Create more",
                                "继续创建",
                            ))),
                    )
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
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "modal-submit"))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(32.))
                            .pl(px(14.))
                            .pr(px(10.))
                            .rounded(px(9.))
                            .bg(p.primary)
                            .when(!can_create, |d| d.opacity(0.45))
                            .text_color(p.on_primary)
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .when(can_create, |d| d.cursor_pointer())
                            .when(!can_create, |d| d.cursor_default())
                            .child(tr(cx, "Create issue", "创建问题"))
                            .child(
                                div()
                                    .px(px(5.))
                                    .rounded(px(5.))
                                    .bg(hsla(0., 0., 1., 0.16))
                                    .text_size(px(11.))
                                    .child("⌘↵"),
                            ),
                        cx,
                        move |this, w, cx| {
                            if can_create {
                                this.submit_modal(w, cx);
                            }
                        },
                    )),
            );

        let scrim_dismiss = cx.listener(|this, _, _, cx| {
            this.modal_open = false;
            this.active_popover = None;
            cx.notify();
        });
        let modal_open = self.modal_open;
        pro_presence(
            pro_eid(&self.id, "new-modal-pres"),
            modal_open,
            0.22,
            0.15,
            move |a| {
                div()
                    .absolute()
                    .inset_0()
                    .p(px(16.))
                    .bg(hsla(0., 0., 0., scrim_max * a))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(modal_open, |d| {
                        d.on_mouse_down(MouseButton::Left, scrim_dismiss)
                    })
                    .child(
                        dialog
                            .shadow_lg()
                            .relative()
                            .top(px(8. * (1. - a)))
                            .opacity(a),
                    )
                    .into_any_element()
            },
        )
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editors(w, cx);
        let reduced = Theme::global(cx).reduced_motion;
        let elapsed_ms = self.started_at.elapsed().as_secs_f32() * 1000.;
        if self.loading && !reduced {
            w.request_animation_frame();
        }
        let phase = if self.loading && !reduced {
            (elapsed_ms / 1100.) % 1.0
        } else {
            0.35
        };

        let width_cell = self.measured_width.clone();
        let w_px = width_cell.get();
        let compact = w_px < 620.;
        let narrow_issue = w_px < 860.;
        let p = ProPalette::get(cx);
        let query = self.current_query(cx);

        let ruler = canvas(
            move |bounds, _, _| {
                width_cell.set(f32::from(bounds.size.width));
            },
            |_, _, _, _| {},
        )
        .w_full()
        .h(px(0.));

        let top_bar = self.render_top_bar(&query, compact, cx);

        let mut root = div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key))
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| {
                if let Some(ref mut ps) = this.press_state
                    && !ps.dragging
                {
                    let dx = f32::from(ev.position.x - ps.start_pos.x);
                    let dy = f32::from(ev.position.y - ps.start_pos.y);
                    if dx.hypot(dy) > 5.0 {
                        ps.dragging = true;
                        cx.notify();
                    }
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if let Some(ps) = this.press_state.take() {
                        let drop_target = this.drag_over_col.take();
                        if ps.dragging {
                            if let Some((target_col, slot)) = drop_target {
                                this.move_card(&ps.card_id, &target_col, Some(slot), cx);
                            } else {
                                cx.notify();
                            }
                        } else {
                            this.open_card(ps.card_id, cx);
                        }
                    }
                }),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.active_popover.take().is_some() {
                        cx.notify();
                    }
                }),
            )
            .relative()
            .size_full()
            .min_h_0()
            .bg(p.panel)
            .flex()
            .flex_col()
            .child(ruler)
            .child(top_bar);

        if let Some(open_id) = self.open_card_id.clone() {
            let issue_el = self.render_open_issue(&open_id, elapsed_ms, narrow_issue, cx);
            root = root.child(div().flex_1().min_h_0().child(pro_view_transition_fill(
                pro_eid(&self.id, "kb-issue-vtx"),
                open_id,
                issue_el,
            )));
        } else {
            root = root.child(self.render_scope_toolbar(&query, compact, cx));
            let (all_n, _, _) = self.scope_counts(&query);

            let main_content: AnyElement = if !self.loading && (self.empty || all_n == 0) {
                let is_filtered = self.active_filter_count(&query) > 0 && !self.empty;
                let empty_el = if is_filtered {
                    pro_empty_state(
                        ProIcon::Filter,
                        tr(cx, "No matching tasks", "没有匹配的任务"),
                        tr(
                            cx,
                            "Try clearing one of your active filters or search terms.",
                            "可改清除部分筛选条件或搜索词以查看全部任务。",
                        ),
                        Some((
                            pro_eid(&self.id, "empty-clear-flt"),
                            tr(cx, "Clear filters", "清除筛选").into(),
                            Box::new(|this: &mut Self, w, cx| {
                                this.filter_statuses.clear();
                                this.filter_assignee = None;
                                this.filter_label = None;
                                this.search_open = false;
                                if let Some(ed) = this.search_editor.clone() {
                                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                                }
                                cx.notify();
                            }),
                        )),
                        cx,
                    )
                } else {
                    pro_empty_state(
                        ProIcon::Tasks,
                        tr(cx, "No tasks yet", "暂无任务"),
                        tr(
                            cx,
                            "Issues and tasks you create for this workspace will show up here by priority.",
                            "在此工作区创建的问题与任务将按优先级分列展示。",
                        ),
                        Some((
                            pro_eid(&self.id, "empty-new-cta"),
                            tr(cx, "New task", "新建任务").into(),
                            Box::new(|this: &mut Self, w, cx| {
                                this.open_new_modal(None, w, cx);
                            }),
                        )),
                        cx,
                    )
                };
                div()
                    .flex_1()
                    .min_h_0()
                    .px(px(12.))
                    .pb(px(12.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(empty_el)
                    .into_any_element()
            } else if self.view_mode == KanbanViewMode::Board {
                self.render_board_view(&query, phase, compact, cx)
                    .into_any_element()
            } else {
                self.render_list_view(&query, phase, elapsed_ms, compact, cx)
                    .into_any_element()
            };

            let main_vtx_key = format!(
                "{:?}-{:?}-{}-{}",
                self.view_mode, self.scope, self.loading, self.empty
            );
            root = root.child(div().flex_1().min_h_0().child(pro_view_transition_fill(
                pro_eid(&self.id, "kb-main-vtx"),
                main_vtx_key,
                main_content,
            )));
        }

        root = root.child(self.render_new_modal(elapsed_ms, compact, cx));

        root
    }
}

impl Render for KanbanScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

fn sample_columns() -> Vec<KanbanColumn> {
    vec![
        KanbanColumn::new("low", "Low priority", KanbanTone::Low),
        KanbanColumn::new("medium", "Medium priority", KanbanTone::Medium),
        KanbanColumn::new("high", "High priority", KanbanTone::High),
        KanbanColumn::new("done", "Completed", KanbanTone::Done),
    ]
}

fn sample_cards() -> Vec<KanbanCard> {
    vec![
        // Low priority (3)
        KanbanCard::new(
            "c1",
            "NOR-142",
            "low",
            "Reset password flow issue",
            KanbanStatus::Review,
        )
        .with_description(
            "Users who signed up with Google but later added a password can't reset it.",
        )
        .with_counts(32, 1)
        .with_assignee(Some("Alex Morgan".into()), false)
        .with_labels(vec!["Auth".into()])
        .with_sub_issues(vec![
            KanbanSubIssue::new("s1", "Verify Google OAuth + password credential merge", true),
            KanbanSubIssue::new("s2", "Allow password reset token for hybrid accounts", true),
            KanbanSubIssue::new("s3", "Add integration test for reset confirmation", false),
        ])
        .with_activity(vec![
            KanbanActivity::event("e1", "Triage Agent", true, "linked Sentry issue #4812", "2d ago"),
            KanbanActivity::comment(
                "e2",
                "Maya Lin",
                false,
                "Reproduced on staging — the identity lookup stops at the OAuth provider record before checking password_hash.",
                "3h ago",
            ),
        ]),
        KanbanCard::new(
            "c2",
            "NOR-143",
            "low",
            "Avatar upload ignores crop",
            KanbanStatus::Progress,
        )
        .with_description("The cropped area is shown in the editor, but the full image is saved.")
        .with_counts(8, 2)
        .with_assignee(Some("Maya Lin".into()), false)
        .with_labels(vec!["Settings".into()]),
        KanbanCard::new(
            "c3",
            "NOR-144",
            "low",
            "Export settings to CSV",
            KanbanStatus::Progress,
        )
        .with_description(
            "Admins want to download the workspace settings as a CSV for their audits.",
        )
        .with_counts(6, 1)
        .with_assignee(Some("Sam Rivera".into()), false)
        .with_labels(vec!["Settings".into()]),
        // Medium priority (3)
        KanbanCard::new(
            "c4",
            "NOR-145",
            "medium",
            "Session tokens expire too early",
            KanbanStatus::Attention,
        )
        .with_description(
            "Sessions end after 20 minutes instead of the expected 24 hours, so people sign in again and again.",
        )
        .with_counts(32, 1)
        .with_assignee(Some("Devon Park".into()), false)
        .with_labels(vec!["Auth".into()]),
        KanbanCard::new(
            "c5",
            "NOR-146",
            "medium",
            "Broken password reset link",
            KanbanStatus::Review,
        )
        .with_description("Reset emails go out, but the magic link opens a 404 page.")
        .with_counts(32, 1)
        .with_assignee(Some("Alex Morgan".into()), false)
        .with_labels(vec!["Auth".into()]),
        KanbanCard::new(
            "c6",
            "NOR-147",
            "medium",
            "Slow dashboard load",
            KanbanStatus::Review,
        )
        .with_description("The analytics dashboard takes 6–8 seconds to render.")
        .with_counts(14, 1)
        .with_assignee(Some("Triage Agent".into()), true)
        .with_labels(vec!["Performance".into()]),
        // High priority (3)
        KanbanCard::new(
            "c7",
            "NOR-148",
            "high",
            "Checkout fails on Safari",
            KanbanStatus::Attention,
        )
        .with_description(
            "Customers on Safari 17 can't complete a purchase in the checkout flow.",
        )
        .with_counts(32, 1)
        .with_assignee(Some("Sam Rivera".into()), false)
        .with_labels(vec!["Billing".into()]),
        KanbanCard::new(
            "c8",
            "NOR-149",
            "high",
            "Video calls drop after 10 minutes",
            KanbanStatus::Attention,
        )
        .with_description(
            "Calls with more than four people end without a warning after about ten minutes.",
        )
        .with_counts(18, 2)
        .with_assignee(Some("Devon Park".into()), false)
        .with_labels(vec!["Infra".into()]),
        KanbanCard::new(
            "c9",
            "NOR-150",
            "high",
            "Webhook retries pile up",
            KanbanStatus::Review,
        )
        .with_description(
            "Failed deliveries retry every second instead of backing off, and the queue keeps growing.",
        )
        .with_counts(9, 1)
        .with_assignee(Some("Triage Agent".into()), true)
        .with_labels(vec!["Infra".into(), "Billing".into()]),
        // Completed (2)
        KanbanCard::new(
            "c10",
            "NOR-151",
            "done",
            "WebSocket disconnects",
            KanbanStatus::Fixed,
        )
        .with_description("Live updates stopped after the connection idled for a minute.")
        .with_counts(32, 0)
        .with_assignee(Some("Maya Lin".into()), false)
        .with_labels(vec!["Infra".into()]),
        KanbanCard::new(
            "c11",
            "NOR-152",
            "done",
            "Mobile app crash on login",
            KanbanStatus::Fixed,
        )
        .with_description(
            "Order creation failed now and then, leaving people with lost carts.",
        )
        .with_counts(32, 0)
        .with_assignee(Some("Alex Morgan".into()), false)
        .with_labels(vec!["Mobile".into(), "Auth".into()]),
    ]
}

fn filter_card_matches(
    card: &KanbanCard,
    statuses: &[KanbanStatus],
    assignee: Option<&str>,
    label: Option<&str>,
    query: &str,
) -> bool {
    if !statuses.is_empty() && !statuses.contains(&card.status) {
        return false;
    }
    if let Some(who) = assignee
        && card.assignee.as_deref() != Some(who)
    {
        return false;
    }
    if let Some(lbl) = label
        && !card.labels.iter().any(|l| l.as_ref() == lbl)
    {
        return false;
    }
    let q = query.trim().to_ascii_lowercase();
    if !q.is_empty() {
        let hit = card.title.to_ascii_lowercase().contains(&q)
            || card.description.to_ascii_lowercase().contains(&q)
            || card.key.to_ascii_lowercase().contains(&q)
            || card
                .labels
                .iter()
                .any(|l| l.to_ascii_lowercase().contains(&q));
        if !hit {
            return false;
        }
    }
    true
}

fn apply_move_card(
    cards: &mut Vec<KanbanCard>,
    columns: &[KanbanColumn],
    card_id: &str,
    target_col: &str,
    insert_pos: Option<usize>,
    actor_name: &SharedString,
) -> Option<(SharedString, SharedString)> {
    let src_idx = cards.iter().position(|c| c.id.as_ref() == card_id)?;
    let target_meta = columns.iter().find(|c| c.id.as_ref() == target_col)?;

    let mut card = cards.remove(src_idx);
    let from_col = card.column.clone();
    let to_col: SharedString = target_col.to_owned().into();

    if from_col != to_col {
        card.column = to_col.clone();
        if target_meta.tone.is_done() {
            card.status = KanbanStatus::Fixed;
        } else if card.status == KanbanStatus::Fixed {
            card.status = KanbanStatus::Progress;
        }
        card.updated_at = "Just now".into();
        let ev_id = format!("ev-{}", card.activity.len() + 1);
        card.activity.push(KanbanActivity::event(
            ev_id,
            actor_name.clone(),
            false,
            format!("moved this issue to {}", target_meta.label),
            "Just now",
        ));
    }

    let target_positions: Vec<usize> = cards
        .iter()
        .enumerate()
        .filter(|(_, c)| c.column == to_col)
        .map(|(i, _)| i)
        .collect();

    let dest_vec_idx = match insert_pos {
        Some(pos) if pos < target_positions.len() => target_positions[pos],
        _ => target_positions
            .last()
            .map_or(cards.len(), |&last| last + 1),
    };
    cards.insert(dest_vec_idx, card);
    Some((from_col, to_col))
}

#[cfg(test)]
mod tests {
    use super::{
        KanbanActivityKind, KanbanCard, KanbanStatus, KanbanSubIssue, KanbanTone, apply_move_card,
        filter_card_matches, sample_cards, sample_columns,
    };

    #[test]
    fn sample_board_filtering_and_column_counts_reconcile() {
        let cols = sample_columns();
        let cards = sample_cards();
        assert_eq!(cols.len(), 4);
        assert_eq!(cards.len(), 11);

        let low = cards.iter().filter(|c| c.column.as_ref() == "low").count();
        let med = cards
            .iter()
            .filter(|c| c.column.as_ref() == "medium")
            .count();
        let high = cards.iter().filter(|c| c.column.as_ref() == "high").count();
        let done = cards.iter().filter(|c| c.column.as_ref() == "done").count();
        assert_eq!((low, med, high, done), (3, 3, 3, 2));

        let auth_hits: Vec<&str> = cards
            .iter()
            .filter(|c| filter_card_matches(c, &[], None, Some("Auth"), ""))
            .map(|c| c.key.as_ref())
            .collect();
        assert_eq!(auth_hits, vec!["NOR-142", "NOR-145", "NOR-146", "NOR-152"]);

        let attention_only = cards
            .iter()
            .filter(|c| filter_card_matches(c, &[KanbanStatus::Attention], None, None, ""))
            .count();
        assert_eq!(attention_only, 3);

        let search_hits = cards
            .iter()
            .filter(|c| filter_card_matches(c, &[], None, None, "checkout"))
            .count();
        assert_eq!(search_hits, 1);
    }

    #[test]
    fn move_card_updates_status_activity_and_column_slot_order() {
        let cols = sample_columns();
        let mut cards = sample_cards();
        let actor = "Alex Morgan".into();

        let moved = apply_move_card(&mut cards, &cols, "c1", "done", Some(0), &actor);
        assert!(moved.is_some_and(|(from, to)| from.as_ref() == "low" && to.as_ref() == "done"));

        let c1 = cards
            .iter()
            .find(|c| c.id.as_ref() == "c1")
            .expect("c1 exists");
        assert_eq!(c1.status, KanbanStatus::Fixed);
        assert_eq!(
            c1.activity.last().expect("activity logged").kind,
            KanbanActivityKind::Event
        );

        let done_ids: Vec<&str> = cards
            .iter()
            .filter(|c| c.column.as_ref() == "done")
            .map(|c| c.id.as_ref())
            .collect();
        assert_eq!(done_ids, vec!["c1", "c10", "c11"]);

        apply_move_card(&mut cards, &cols, "c1", "high", None, &actor);
        let reopened = cards
            .iter()
            .find(|c| c.id.as_ref() == "c1")
            .expect("c1 exists");
        assert_eq!(reopened.status, KanbanStatus::Progress);

        let card = KanbanCard::new(
            "custom-1",
            "NOR-200",
            "high",
            "Payment webhook timeout",
            KanbanStatus::Attention,
        )
        .with_counts(5, 2)
        .with_sub_issues(vec![
            KanbanSubIssue::new("s1", "Queue payload before verification", true),
            KanbanSubIssue::new("s2", "Add retry backoff jitter", false),
        ]);
        assert!(!KanbanTone::High.is_done());
        assert!(KanbanTone::Done.is_done());
        assert_eq!(card.sub_issues.iter().filter(|s| s.done).count(), 1);
    }
}
