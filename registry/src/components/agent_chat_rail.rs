//! Double-sidebar AI search chat with icon rail, project switcher, and spreadsheet artifacts.
//!
//! Renders a four-column workspace layout: a `52px` section icon rail on the tinted stage,
//! a drag-resizable project chat list inside the main card, a web-grounded conversation
//! thread with collapsible tool traces and multi-line composer, and a tabbed artifact panel.

use crate::{
    components::pro_chrome::{
        ProPalette, ProSelectionSensor, pro_eid, pro_empty_state, pro_fold, pro_icon_btn,
        pro_mark_selected, pro_person_avatar, pro_pill_indicator, pro_presence,
        pro_selection_sensor, pro_skeleton_bar, pro_spring_f32, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, EditorShortcutEvent, InputMode},
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const SVG_RAIL_CHATS: &str = r#"<path d="M21.5 12C21.5 17.2467 17.2467 21.5 12 21.5C10.3719 21.5 8.8394 21.0904 7.5 20.3687C5.63177 19.362 4.37462 20.2979 3.26592 20.4658C3.09774 20.4913 2.93024 20.4302 2.80997 20.31C2.62741 20.1274 2.59266 19.8451 2.6935 19.6074C3.12865 18.5818 3.5282 16.6382 2.98341 15C2.6698 14.057 2.5 13.0483 2.5 12C2.5 6.75329 6.75329 2.5 12 2.5C17.2467 2.5 21.5 6.75329 21.5 12Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><circle cx="8" cy="12" r="1" fill="currentColor"></circle><circle cx="12" cy="12" r="1" fill="currentColor"></circle><circle cx="16" cy="12" r="1" fill="currentColor"></circle>"#;

const SVG_RAIL_RUNS: &str = r#"<circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="1.9"></circle><path d="M15.9453 12.3948C15.7686 13.0215 14.9333 13.4644 13.2629 14.3502C11.648 15.2064 10.8406 15.6346 10.1899 15.4625C9.9209 15.3913 9.6758 15.2562 9.47812 15.0701C9 14.6198 9 13.7465 9 12C9 10.2535 9 9.38018 9.47812 8.92995C9.6758 8.74381 9.9209 8.60868 10.1899 8.53753C10.8406 8.36544 11.648 8.79357 13.2629 9.64983C14.9333 10.5356 15.7686 10.9785 15.9453 11.6052C16.0182 11.8639 16.0182 12.1361 15.9453 12.3948Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_RAIL_PLUG: &str = r#"<path d="M15.5 2V6M8.5 6V2" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M6.00446 7.61331C5.93719 6.74273 6.63957 6 7.53014 6H16.4699C17.3604 6 18.0628 6.74273 17.9955 7.61331L17.8117 9.99197C17.6796 11.7019 17.1011 13.3498 16.132 14.7773L15.5312 15.6622C14.9638 16.4979 14.0077 17 12.9838 17H11.0162C9.99228 17 9.03617 16.4979 8.46881 15.6622L7.86803 14.7773C6.89885 13.3498 6.32041 11.7019 6.18827 9.99197L6.00446 7.61331Z" stroke="currentColor" stroke-width="1.9"></path><path d="M12 17V22" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M11 9H13" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_HAND_APPROVAL: &str = r#"<path d="M52.5305 32.6557V24.8463C52.5305 22.1016 54.7555 19.8766 57.5001 19.8766C60.2448 19.8766 62.4698 22.1016 62.4698 24.8463V42.027C62.4698 53.3194 53.3154 62.4735 42.0233 62.4735H34.2388M22.7127 39.7552L15.94 31.8537C14.8314 30.5602 13.2129 29.8159 11.5094 29.8159H11.1235C8.11379 29.8159 5.67395 35.2655 5.67395 35.2655C5.67395 36.3412 5.99241 37.393 6.58919 38.2881L16.4118 53.0221C20.3482 58.9269 26.9753 62.4735 34.0719 62.4735" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M52.534 24.1378V16.3284C52.534 13.5837 50.3091 11.3588 47.5644 11.3588C44.8197 11.3588 42.5948 13.5837 42.5948 16.3284V29.8174" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M42.5958 29.8198V10.6513C42.5958 7.90661 40.3708 5.68163 37.6262 5.68163C34.8815 5.68163 32.6565 7.90661 32.6565 10.6513V28.4" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M32.6576 28.3975V16.3284C32.6576 13.5837 30.4326 11.3588 27.6879 11.3588C24.9433 11.3588 22.7183 13.5837 22.7183 16.3284V39.7567" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path>"#;

const SVG_FILE_DOC: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><path d="M4 6.6h4M4 8.6h2.6" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"></path>"#;

const SVG_FILE_SHEET: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><rect x="3.4" y="5.6" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="6.3" y="5.6" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="3.4" y="7.8" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="6.3" y="7.8" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect>"#;

const SVG_REDO: &str = r#"<path d="M13 6H8.5C6.01472 6 4 8.01472 4 10.5C4 12.9853 6.01472 15 8.5 15H20" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M17 12C17 12 20 14.2095 20 15C20 15.7906 17 18 17 18" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_THUMB_UP: &str = r#"<path d="M7 11V19C7 19.5523 6.55228 20 6 20H4C3.44772 20 3 19.5523 3 19V12C3 11.4477 3.44772 11 4 11H7ZM7 11L10.6 4.7C11.1 3.8 12.3 3.6 13.1 4.2C13.7 4.7 13.9 5.5 13.7 6.3L12.8 10H18.4C19.8 10 20.8 11.3 20.4 12.7L18.9 18.2C18.6 19.3 17.6 20 16.5 20H9C7.9 20 7 19.1 7 18" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"></path>"#;

const SVG_THUMB_DOWN: &str = r#"<path d="M7 13V5C7 4.44772 6.55228 4 6 4H4C3.44772 4 3 4.44772 3 5V12C3 12.5523 3.44772 13 4 13H7ZM7 13L10.6 19.3C11.1 20.2 12.3 20.4 13.1 19.8C13.7 19.3 13.9 18.5 13.7 17.7L12.8 14H18.4C19.8 14 20.8 12.7 20.4 11.3L18.9 5.8C18.6 4.7 17.6 4 16.5 4H9C7.9 4 7 4.9 7 6" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"></path>"#;

const SVG_INSERT_ROW: &str = r#"<rect x="3" y="4" width="18" height="16" rx="2.5" stroke="currentColor" stroke-width="1.8"></rect><path d="M3 10H21M12 13V17M10 15H14" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"></path>"#;

const SVG_INSERT_COL: &str = r#"<rect x="3" y="4" width="18" height="16" rx="2.5" stroke="currentColor" stroke-width="1.8"></rect><path d="M9 4V20M15 10V14M13 12H17" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"></path>"#;

/// Top-level product section selected in the `52px` left icon rail.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RailSection {
    #[default]
    Chats,
    Tasks,
    Runs,
    Folders,
    Integrations,
}

impl RailSection {
    /// Human-readable navigation label emitted on click and read by screen readers.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Chats => tr(cx, "Chats", "对话"),
            Self::Tasks => tr(cx, "Tasks", "任务"),
            Self::Runs => tr(cx, "Runs", "运行"),
            Self::Folders => tr(cx, "Folders", "文件夹"),
            Self::Integrations => tr(cx, "Integrations", "集成"),
        }
    }

    fn all() -> [Self; 5] {
        [
            Self::Chats,
            Self::Tasks,
            Self::Runs,
            Self::Folders,
            Self::Integrations,
        ]
    }
}

/// Project scope selected in the chat column switcher.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProjectFilter {
    #[default]
    All,
    SummerCollection,
    Brand,
    Hiring,
    Research,
}

impl ProjectFilter {
    /// Display title shown in the switcher button and menu.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::All => tr(cx, "All projects", "全部项目"),
            Self::SummerCollection => "Summer collection",
            Self::Brand => "Brand",
            Self::Hiring => "Hiring",
            Self::Research => "Research",
        }
    }

    fn all() -> [Self; 5] {
        [
            Self::All,
            Self::SummerCollection,
            Self::Brand,
            Self::Hiring,
            Self::Research,
        ]
    }
}

/// Sender role for a conversation turn in [`AgentChatRailScreen`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentChatRailRole {
    User,
    Assistant,
}

/// Helpful/unhelpful vote recorded on an assistant turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentChatRailFeedback {
    Up,
    Down,
}

/// Action approval policy chosen in the bottom composer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RailApprovalMode {
    #[default]
    RequestApproval,
    ApproveEdits,
    AutoRun,
}

impl RailApprovalMode {
    /// Display label shown on the composer button and menu.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::RequestApproval => tr(cx, "Request approval", "请求审批"),
            Self::ApproveEdits => tr(cx, "Approve safe edits", "批准安全编辑"),
            Self::AutoRun => tr(cx, "Auto-run searches", "自动执行检索"),
        }
    }
}

/// Visual type of an artifact linked from an assistant turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RailArtifactKind {
    Sheet,
    Doc,
}

/// One completed or running tool invocation folded inside an assistant turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RailToolStep {
    pub id: SharedString,
    pub icon: ProIcon,
    pub label: SharedString,
    pub duration: SharedString,
    pub done: bool,
}

impl RailToolStep {
    /// Construct a tool trace row.
    pub fn new(
        id: impl Into<SharedString>,
        icon: ProIcon,
        label: impl Into<SharedString>,
        duration: impl Into<SharedString>,
        done: bool,
    ) -> Self {
        Self {
            id: id.into(),
            icon,
            label: label.into(),
            duration: duration.into(),
            done,
        }
    }
}

/// Clickable artifact chip that opens or focuses a tab in the right-hand panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RailArtifactChip {
    pub id: SharedString,
    pub name: SharedString,
    pub kind: RailArtifactKind,
}

impl RailArtifactChip {
    /// Construct an artifact chip reference.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        kind: RailArtifactKind,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
        }
    }
}

/// One user or assistant message turn in [`AgentChatRailScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RailChatMessage {
    pub id: SharedString,
    pub role: AgentChatRailRole,
    pub timestamp: Option<SharedString>,
    pub tool_summary: Option<SharedString>,
    pub tools_open: bool,
    pub tools: Vec<RailToolStep>,
    pub paragraphs: Vec<SharedString>,
    pub artifacts: Vec<RailArtifactChip>,
    pub feedback: Option<AgentChatRailFeedback>,
}

impl RailChatMessage {
    /// Construct a right-aligned user bubble turn.
    pub fn user(id: impl Into<SharedString>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            role: AgentChatRailRole::User,
            timestamp: None,
            tool_summary: None,
            tools_open: false,
            tools: Vec::new(),
            paragraphs: vec![text.into()],
            artifacts: Vec::new(),
            feedback: None,
        }
    }

    /// Construct a timestamped assistant turn.
    pub fn assistant(
        id: impl Into<SharedString>,
        timestamp: impl Into<SharedString>,
        paragraphs: Vec<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            role: AgentChatRailRole::Assistant,
            timestamp: Some(timestamp.into()),
            tool_summary: None,
            tools_open: false,
            tools: Vec::new(),
            paragraphs,
            artifacts: Vec::new(),
            feedback: None,
        }
    }
}

/// Sidebar chat entry grouped under `Pinned`, a `ProjectFilter`, or ungrouped `Chats`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RailThreadEntry {
    pub id: SharedString,
    pub title: SharedString,
    pub project: Option<ProjectFilter>,
    pub pinned: bool,
}

/// Events emitted by [`AgentChatRailScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentChatRailEvent {
    Navigate(SharedString),
    SelectProject(ProjectFilter),
    SelectThread(SharedString),
    SendMessage {
        text: SharedString,
        model: SharedString,
        attachments: Vec<SharedString>,
    },
    StopGeneration,
    SelectModel(SharedString),
    SelectApproval(RailApprovalMode),
    OpenArtifact(SharedString),
    CopyMessage(SharedString),
    Regenerate(SharedString),
    Feedback {
        message_id: SharedString,
        feedback: Option<AgentChatRailFeedback>,
    },
}

impl EventEmitter<AgentChatRailEvent> for AgentChatRailScreen {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DragColumn {
    ChatSidebar,
    ArtifactPanel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ComposerPopover {
    Add,
    Approval,
    Model,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArtifactSlot {
    FabricCsv,
    SourcesMd,
    NewTab,
}

impl ArtifactSlot {
    fn id(self) -> &'static str {
        match self {
            Self::FabricCsv => "fabric-comparison.csv",
            Self::SourcesMd => "sources.md",
            Self::NewTab => "new-tab",
        }
    }

    fn title(self, cx: &App) -> SharedString {
        match self {
            Self::FabricCsv => "fabric-comparison.csv".into(),
            Self::SourcesMd => "sources.md".into(),
            Self::NewTab => tr(cx, "New tab", "新标签页").into(),
        }
    }

    fn icon(self) -> ProIcon {
        match self {
            Self::FabricCsv => ProIcon::ClipboardDoc,
            Self::SourcesMd => ProIcon::PageDoc,
            Self::NewTab => ProIcon::Globe,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SheetSnapshot {
    rows: Vec<[SharedString; 4]>,
    bold_cells: Vec<(usize, usize)>,
}

/// Double-sidebar AI search chat screen with project switcher and live spreadsheet artifacts.
pub struct AgentChatRailScreen {
    id: ElementId,
    focus: FocusHandle,
    standalone: bool,
    workspace: SharedString,
    user_name: SharedString,
    loading: bool,
    empty: bool,
    rail_section: RailSection,
    unread_notifications: bool,
    ws_menu_open: bool,
    ws_cursor: usize,
    notif_open: bool,
    account_menu_open: bool,
    project_filter: ProjectFilter,
    project_menu_open: bool,
    project_cursor: usize,
    expanded_projects: [bool; 4],
    threads: Vec<RailThreadEntry>,
    active_thread_id: SharedString,
    messages: Vec<RailChatMessage>,
    copied_msg_id: Option<SharedString>,
    composer_editor: Option<Entity<Editor>>,
    composer_popover: Option<ComposerPopover>,
    last_composer_popover: Cell<ComposerPopover>,
    active_model: SharedString,
    approval_mode: RailApprovalMode,
    attachments: Vec<SharedString>,
    streaming: bool,
    dictating: bool,
    search_open: bool,
    search_cursor: usize,
    search_editor: Option<Entity<Editor>>,
    chat_col_width: f32,
    chat_col_collapsed: bool,
    panel_open: bool,
    panel_width: f32,
    artifact_tabs: Vec<ArtifactSlot>,
    active_artifact_tab: usize,
    sheet_rows: Vec<[SharedString; 4]>,
    sheet_bold: Vec<(usize, usize)>,
    sheet_undo_stack: Vec<SheetSnapshot>,
    sheet_redo_stack: Vec<SheetSnapshot>,
    selected_cell: (usize, usize),
    source_checks: [bool; 3],
    dragging: Option<(DragColumn, f32, f32)>,
    mobile_drawer_open: bool,
    started_at: Instant,
    measured_width: Rc<Cell<f32>>,
}

impl AgentChatRailScreen {
    /// Construct an [`AgentChatRailScreen`] with the default double-sidebar layout and fabric study.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: ElementId::Name(id.into()),
            focus: cx.focus_handle(),
            standalone: false,
            workspace: "Fieldwork".into(),
            user_name: "Alex Morgan".into(),
            loading: false,
            empty: false,
            rail_section: RailSection::Chats,
            unread_notifications: true,
            ws_menu_open: false,
            ws_cursor: 0,
            notif_open: false,
            account_menu_open: false,
            project_filter: ProjectFilter::All,
            project_menu_open: false,
            project_cursor: 0,
            expanded_projects: [true, false, false, false],
            threads: Self::default_threads(),
            active_thread_id: "best-fabric".into(),
            messages: Self::default_fabric_messages(),
            copied_msg_id: None,
            composer_editor: None,
            composer_popover: None,
            last_composer_popover: Cell::new(ComposerPopover::Model),
            active_model: "Auto".into(),
            approval_mode: RailApprovalMode::RequestApproval,
            attachments: Vec::new(),
            streaming: false,
            dictating: false,
            search_open: false,
            search_cursor: 0,
            search_editor: None,
            chat_col_width: 226.,
            chat_col_collapsed: false,
            panel_open: true,
            panel_width: 318.,
            artifact_tabs: vec![ArtifactSlot::FabricCsv, ArtifactSlot::SourcesMd],
            active_artifact_tab: 0,
            sheet_rows: Self::default_sheet_rows(),
            sheet_bold: vec![(0, 0), (0, 1), (0, 2), (0, 3)],
            sheet_undo_stack: Vec::new(),
            sheet_redo_stack: Vec::new(),
            selected_cell: (1, 0),
            source_checks: [true, true, false],
            dragging: None,
            mobile_drawer_open: false,
            started_at: Instant::now(),
            measured_width: Rc::new(Cell::new(1120.)),
        }
    }

    fn default_threads() -> Vec<RailThreadEntry> {
        vec![
            RailThreadEntry {
                id: "tone-of-voice".into(),
                title: "Tone of voice, in five rules".into(),
                project: None,
                pinned: true,
            },
            RailThreadEntry {
                id: "best-fabric".into(),
                title: "Best fabric for a summer overshirt".into(),
                project: Some(ProjectFilter::SummerCollection),
                pinned: false,
            },
            RailThreadEntry {
                id: "colour-names".into(),
                title: "Colour names for the linen line".into(),
                project: Some(ProjectFilter::SummerCollection),
                pinned: false,
            },
            RailThreadEntry {
                id: "size-chart".into(),
                title: "Size chart for relaxed fits".into(),
                project: Some(ProjectFilter::SummerCollection),
                pinned: false,
            },
            RailThreadEntry {
                id: "brand-typography".into(),
                title: "Tone & typography guide".into(),
                project: Some(ProjectFilter::Brand),
                pinned: false,
            },
            RailThreadEntry {
                id: "textile-sourcer".into(),
                title: "Senior textile sourcer".into(),
                project: Some(ProjectFilter::Hiring),
                pinned: false,
            },
            RailThreadEntry {
                id: "mill-lead-times".into(),
                title: "Portuguese mill lead times".into(),
                project: Some(ProjectFilter::Research),
                pinned: false,
            },
            RailThreadEntry {
                id: "packing-lisbon".into(),
                title: "Packing list for Lisbon".into(),
                project: None,
                pinned: false,
            },
            RailThreadEntry {
                id: "reply-agency".into(),
                title: "Reply to the agency".into(),
                project: None,
                pinned: false,
            },
        ]
    }

    fn default_fabric_messages() -> Vec<RailChatMessage> {
        let mut a1 = RailChatMessage::assistant(
            "rail-a-1",
            "9:04 AM",
            vec![
                "Linen breathes best – it pulls moisture off the skin and dries fast, which is why it feels cool on a hot day. The trade-off is creasing: a pure linen overshirt looks lived-in after an hour.".into(),
                "For an overshirt I'd take a linen-cotton blend around 55/45 at 160–180 gsm: it keeps most of the coolness, creases far less and is soft from the first wear. Pure linen if the relaxed look is the point; seersucker if it has to look sharp all day.".into(),
            ],
        );
        a1.tool_summary = Some("Ran 3 tools · 3.1s".into());
        a1.tools = vec![
            RailToolStep::new(
                "tool-1",
                ProIcon::Globe,
                "Searched web: linen vs cotton blend gsm summer overshirt",
                "1.1s",
                true,
            ),
            RailToolStep::new(
                "tool-2",
                ProIcon::PageDoc,
                "Read 4 textile mill specification sheets",
                "1.2s",
                true,
            ),
            RailToolStep::new(
                "tool-3",
                ProIcon::ClipboardDoc,
                "Wrote fabric-comparison.csv and sources.md",
                "0.8s",
                true,
            ),
        ];
        a1.artifacts = vec![
            RailArtifactChip::new(
                "fabric-comparison.csv",
                "fabric-comparison.csv",
                RailArtifactKind::Sheet,
            ),
            RailArtifactChip::new("sources.md", "sources.md", RailArtifactKind::Doc),
        ];

        let mut a2 = RailChatMessage::assistant(
            "rail-a-2",
            "9:08 AM",
            vec![
                "Go a little heavier than a shirt: 180–200 gsm. It hangs straight when it's open, doesn't cling to the T-shirt underneath and still breathes – under 160 gsm it starts to look like a shirt you forgot to button.".into(),
            ],
        );
        a2.tool_summary = Some("Ran 1 tool · 1.2s".into());
        a2.tools = vec![RailToolStep::new(
            "tool-4",
            ProIcon::Globe,
            "Searched web: overshirt gsm open drape over t-shirt",
            "1.2s",
            true,
        )];

        vec![
            RailChatMessage::user(
                "rail-u-1",
                "What's the best fabric for a summer overshirt – linen, cotton or a blend?",
            ),
            a1,
            RailChatMessage::user(
                "rail-u-2",
                "Which weight for a shirt you can wear open over a T-shirt?",
            ),
            a2,
        ]
    }

    fn default_sheet_rows() -> Vec<[SharedString; 4]> {
        vec![
            [
                "Fabric".into(),
                "Weight (gsm)".into(),
                "Breathes".into(),
                "Creases".into(),
            ],
            [
                "Linen".into(),
                "150–190".into(),
                "Best".into(),
                "A lot".into(),
            ],
            [
                "Linen-cotton 55/45".into(),
                "160–180".into(),
                "Very well".into(),
                "Some".into(),
            ],
            [
                "Cotton poplin".into(),
                "110–130".into(),
                "Well".into(),
                "Little".into(),
            ],
            [
                "Cotton seersucker".into(),
                "140–160".into(),
                "Very well".into(),
                "Hardly".into(),
            ],
            ["".into(), "".into(), "".into(), "".into()],
            ["".into(), "".into(), "".into(), "".into()],
            ["".into(), "".into(), "".into(), "".into()],
            ["".into(), "".into(), "".into(), "".into()],
        ]
    }

    /// Render only the conversation + composer column without the outer rail and sidebars.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Override the workspace name whose initial tops the left icon rail (`"Fieldwork"`, `"Acme"`, etc.).
    pub fn with_workspace(mut self, workspace: impl Into<SharedString>) -> Self {
        self.workspace = workspace.into();
        self
    }

    /// Override the signed-in user name whose initials appear at the bottom of the icon rail.
    pub fn with_user(mut self, name: impl Into<SharedString>) -> Self {
        self.user_name = name.into();
        self
    }

    /// Enable or disable the skeleton loading placeholder state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Enable or disable the empty state (`loading` wins when both are `true`).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Replace the initial conversation messages.
    pub fn with_messages(mut self, messages: Vec<RailChatMessage>) -> Self {
        self.messages = messages;
        self
    }

    /// Set the active model label in the composer.
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.active_model = model.into();
        self
    }

    /// Return whether the loading skeleton state is active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Return whether the empty state is active (`false` while `loading` is `true`).
    pub fn is_empty(&self) -> bool {
        self.empty && !self.loading
    }

    /// Return the active project filter in the chat sidebar switcher.
    pub fn project_filter(&self) -> ProjectFilter {
        self.project_filter
    }

    /// Return the active chat thread identifier.
    pub fn active_thread_id(&self) -> &str {
        self.active_thread_id.as_ref()
    }

    /// Return the messages currently displayed in the conversation thread.
    pub fn messages(&self) -> &[RailChatMessage] {
        &self.messages
    }

    /// Return the threads visible under the active [`ProjectFilter`].
    pub fn visible_threads(&self) -> Vec<&RailThreadEntry> {
        if self.empty && !self.loading {
            return Vec::new();
        }
        match self.project_filter {
            ProjectFilter::All => self.threads.iter().collect(),
            scoped => self
                .threads
                .iter()
                .filter(|t| t.project == Some(scoped))
                .collect(),
        }
    }

    /// Return the Excel-style reference (`"A2"`) and text contents of the selected spreadsheet cell.
    pub fn selected_sheet_cell(&self) -> (String, SharedString) {
        let (r, c) = self.selected_cell;
        let col_ch = (b'A' + (c.min(3) as u8)) as char;
        let cell_ref = format!("{col_ch}{}", r + 1);
        let value = self
            .sheet_rows
            .get(r)
            .and_then(|row| row.get(c))
            .cloned()
            .unwrap_or_default();
        (cell_ref, value)
    }

    /// Update the loading skeleton state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Update the empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    /// Change the active [`ProjectFilter`] in the chat column switcher.
    pub fn select_project(&mut self, filter: ProjectFilter, cx: &mut Context<Self>) {
        self.project_filter = filter;
        self.project_menu_open = false;
        cx.emit(AgentChatRailEvent::SelectProject(filter));
        cx.emit(AgentChatRailEvent::Navigate(filter.label(cx).into()));
        cx.notify();
    }

    /// Switch to a chat thread by id and populate its conversation turns.
    pub fn select_thread(&mut self, thread_id: &str, cx: &mut Context<Self>) {
        let Some(entry) = self
            .threads
            .iter()
            .find(|t| t.id.as_ref() == thread_id)
            .cloned()
        else {
            return;
        };
        self.empty = false;
        self.active_thread_id = entry.id.clone();
        if entry.id.as_ref() == "best-fabric" {
            self.messages = Self::default_fabric_messages();
        } else {
            let mut reply = RailChatMessage::assistant(
                format!("msg-a-{}", entry.id),
                "9:14 AM",
                vec![
                    format!(
                        "Pulled the latest project notes and web references for \"{}\".",
                        entry.title
                    )
                    .into(),
                ],
            );
            reply.tool_summary = Some("Ran 2 tools · 1.8s".into());
            reply.tools = vec![
                RailToolStep::new(
                    "switch-t1",
                    ProIcon::Globe,
                    format!("Searched web context for {}", entry.title),
                    "1.1s",
                    true,
                ),
                RailToolStep::new(
                    "switch-t2",
                    ProIcon::PageDoc,
                    "Synced sources.md in workspace",
                    "0.7s",
                    true,
                ),
            ];
            reply.artifacts = vec![RailArtifactChip::new(
                "sources.md",
                "sources.md",
                RailArtifactKind::Doc,
            )];
            self.messages = vec![
                RailChatMessage::user(format!("msg-u-{}", entry.id), entry.title.clone()),
                reply,
            ];
        }
        self.mobile_drawer_open = false;
        cx.emit(AgentChatRailEvent::SelectThread(entry.id));
        cx.emit(AgentChatRailEvent::Navigate(entry.title));
        cx.notify();
    }

    /// Open or focus an artifact tab (`"fabric-comparison.csv"`, `"sources.md"`, `"new-tab"`).
    pub fn open_artifact(&mut self, key: &str, cx: &mut Context<Self>) {
        let slot = match key {
            "fabric-comparison.csv" => ArtifactSlot::FabricCsv,
            "sources.md" => ArtifactSlot::SourcesMd,
            _ => ArtifactSlot::NewTab,
        };
        if let Some(pos) = self.artifact_tabs.iter().position(|&t| t == slot) {
            self.active_artifact_tab = pos;
        } else if self
            .artifact_tabs
            .get(self.active_artifact_tab)
            .is_some_and(|&t| t == ArtifactSlot::NewTab)
            && slot != ArtifactSlot::NewTab
        {
            self.artifact_tabs[self.active_artifact_tab] = slot;
        } else {
            self.artifact_tabs.push(slot);
            self.active_artifact_tab = self.artifact_tabs.len() - 1;
        }
        self.panel_open = true;
        cx.emit(AgentChatRailEvent::OpenArtifact(slot.id().into()));
        cx.notify();
    }

    /// Close an artifact tab by index; closing the last tab folds the artifacts panel.
    pub fn close_artifact_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.artifact_tabs.len() {
            return;
        }
        self.artifact_tabs.remove(index);
        if self.artifact_tabs.is_empty() {
            self.artifact_tabs.push(ArtifactSlot::FabricCsv);
            self.active_artifact_tab = 0;
            self.panel_open = false;
        } else if self.active_artifact_tab >= self.artifact_tabs.len() {
            self.active_artifact_tab = self.artifact_tabs.len() - 1;
        }
        cx.notify();
    }

    /// Sort non-empty data rows (preserving header row `1`) in `fabric-comparison.csv` by the selected column.
    pub fn sort_sheet(&mut self, ascending: bool, cx: &mut Context<Self>) {
        if self.sheet_rows.len() <= 2 {
            return;
        }
        self.push_sheet_undo();
        let col = self.selected_cell.1.min(3);
        let header = self.sheet_rows[0].clone();
        let mut non_empty: Vec<[SharedString; 4]> = Vec::new();
        let mut blanks: Vec<[SharedString; 4]> = Vec::new();
        for row in self.sheet_rows.iter().skip(1) {
            if row.iter().all(|c| c.is_empty()) {
                blanks.push(row.clone());
            } else {
                non_empty.push(row.clone());
            }
        }
        non_empty.sort_by(|a, b| {
            let ord = a[col].as_ref().cmp(b[col].as_ref());
            if ascending { ord } else { ord.reverse() }
        });
        let mut rebuilt = Vec::with_capacity(1 + non_empty.len() + blanks.len());
        rebuilt.push(header);
        rebuilt.extend(non_empty);
        rebuilt.extend(blanks);
        self.sheet_rows = rebuilt;
        cx.notify();
    }

    /// Toggle bold formatting on the currently selected spreadsheet cell.
    pub fn toggle_cell_bold(&mut self, cx: &mut Context<Self>) {
        self.push_sheet_undo();
        let target = self.selected_cell;
        if let Some(pos) = self.sheet_bold.iter().position(|&c| c == target) {
            self.sheet_bold.remove(pos);
        } else {
            self.sheet_bold.push(target);
        }
        cx.notify();
    }

    /// Undo the most recent spreadsheet edit or sort.
    pub fn sheet_undo(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(prev) = self.sheet_undo_stack.pop() else {
            return false;
        };
        self.sheet_redo_stack.push(SheetSnapshot {
            rows: self.sheet_rows.clone(),
            bold_cells: self.sheet_bold.clone(),
        });
        self.sheet_rows = prev.rows;
        self.sheet_bold = prev.bold_cells;
        cx.notify();
        true
    }

    /// Redo a previously undone spreadsheet edit or sort.
    pub fn sheet_redo(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(next) = self.sheet_redo_stack.pop() else {
            return false;
        };
        self.sheet_undo_stack.push(SheetSnapshot {
            rows: self.sheet_rows.clone(),
            bold_cells: self.sheet_bold.clone(),
        });
        self.sheet_rows = next.rows;
        self.sheet_bold = next.bold_cells;
        cx.notify();
        true
    }

    fn push_sheet_undo(&mut self) {
        self.sheet_undo_stack.push(SheetSnapshot {
            rows: self.sheet_rows.clone(),
            bold_cells: self.sheet_bold.clone(),
        });
        self.sheet_redo_stack.clear();
    }

    /// Submit a prompt string into the active thread (`false` when `loading` or `empty` blocks sending, or draft is blank).
    pub fn submit_prompt(&mut self, raw: &str, cx: &mut Context<Self>) -> bool {
        let trimmed = raw.trim();
        if self.loading || self.empty || (trimmed.is_empty() && self.attachments.is_empty()) {
            return false;
        }
        let prompt_text: SharedString = if trimmed.is_empty() {
            format!("Attached {} file(s)", self.attachments.len()).into()
        } else {
            trimmed.to_owned().into()
        };
        let idx = self.messages.len() + 1;
        self.messages.push(RailChatMessage::user(
            format!("rail-u-{idx}"),
            prompt_text.clone(),
        ));

        let mut reply = RailChatMessage::assistant(
            format!("rail-a-{idx}"),
            "9:12 AM",
            vec![
                format!(
                    "Checked current textile specs using {}: {} holds shape cleanly at 175 gsm with enzyme-washed pre-shrink.",
                    self.active_model, prompt_text
                )
                .into(),
            ],
        );
        reply.tool_summary = Some("Ran 2 tools · 1.6s".into());
        reply.tools = vec![
            RailToolStep::new(
                format!("t-web-{idx}"),
                ProIcon::Globe,
                format!("Searched web: {prompt_text}"),
                "1.0s",
                true,
            ),
            RailToolStep::new(
                format!("t-sheet-{idx}"),
                ProIcon::ClipboardDoc,
                "Updated fabric-comparison.csv",
                "0.6s",
                true,
            ),
        ];
        reply.artifacts = vec![RailArtifactChip::new(
            "fabric-comparison.csv",
            "fabric-comparison.csv",
            RailArtifactKind::Sheet,
        )];
        self.messages.push(reply);

        let sent_att = std::mem::take(&mut self.attachments);
        self.streaming = false;
        cx.emit(AgentChatRailEvent::SendMessage {
            text: prompt_text,
            model: self.active_model.clone(),
            attachments: sent_att,
        });
        cx.notify();
        true
    }

    /// Record or toggle thumbs-up/down feedback on an assistant message.
    pub fn toggle_feedback(
        &mut self,
        message_id: &str,
        vote: AgentChatRailFeedback,
        cx: &mut Context<Self>,
    ) {
        if let Some(msg) = self
            .messages
            .iter_mut()
            .find(|m| m.id.as_ref() == message_id)
        {
            msg.feedback = if msg.feedback == Some(vote) {
                None
            } else {
                Some(vote)
            };
            let fb = msg.feedback;
            let mid = msg.id.clone();
            cx.emit(AgentChatRailEvent::Feedback {
                message_id: mid,
                feedback: fb,
            });
            cx.notify();
        }
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.composer_editor.is_none() {
            let ph: SharedString = tr(
                cx,
                "Ask anything – it searches the web",
                "提问任意内容——自动联网检索",
            )
            .into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new("Message", InputMode::InlineMulti, w, cx);
                e.set_placeholder(ph);
                e.set_command_submit(true);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => cx.notify(),
                EditEvent::Submitted => {
                    let val = ed_ent.read(cx).value().to_string();
                    if this.submit_prompt(&val, cx) {
                        ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                    }
                }
                EditEvent::Escape => {
                    if this.streaming {
                        this.streaming = false;
                        cx.emit(AgentChatRailEvent::StopGeneration);
                        cx.notify();
                    } else if this.dictating || this.composer_popover.is_some() {
                        this.dictating = false;
                        this.composer_popover = None;
                        cx.notify();
                    }
                }
                _ => {}
            })
            .detach();
            cx.subscribe_in(
                &ed,
                w,
                |this, ed_ent, ev: &EditorShortcutEvent, w, cx| match ev {
                    EditorShortcutEvent::CommandSubmit => {
                        let val = ed_ent.read(cx).value().to_string();
                        if this.submit_prompt(&val, cx) {
                            ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                        }
                    }
                },
            )
            .detach();
            self.composer_editor = Some(ed);
        }

        if self.search_open && self.search_editor.is_none() {
            let ph: SharedString = tr(cx, "Search chats and projects...", "搜索对话与项目…").into();
            let sed = cx.new(|cx| {
                let mut e = Editor::new("Search", InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&sed, |this, _, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => {
                    this.search_cursor = 0;
                    cx.notify();
                }
                EditEvent::Escape => {
                    this.search_open = false;
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            {
                let fh = sed.read(cx).focus.clone();
                fh.focus(w, cx);
            }
            self.search_editor = Some(sed);
        }
    }

    fn skeleton_phase(&self, w: &mut Window, cx: &App) -> f32 {
        let reduced = Theme::global(cx).reduced_motion;
        if (self.loading || self.dictating || self.streaming) && !reduced {
            w.request_animation_frame();
            (self.started_at.elapsed().as_secs_f32() * 0.9).fract()
        } else {
            0.25
        }
    }

    fn render_purple_pixel_avatar(p: &ProPalette) -> Div {
        let cells = [true, false, true, false, true, false, true, false, true];
        let mut col = div()
            .size(px(14.))
            .flex_none()
            .flex()
            .flex_col()
            .justify_center()
            .gap(px(1.5));
        for row_cells in cells.as_chunks::<3>().0 {
            let mut row = div().flex().items_center().justify_center().gap(px(1.5));
            for &on in row_cells {
                row = row.child(div().size(px(2.8)).rounded(px(0.8)).bg(if on {
                    p.purple
                } else {
                    transparent_black()
                }));
            }
            col = col.child(row);
        }
        col
    }

    fn render_artifact_glyph(kind: RailArtifactKind, size_px: f32, p: &ProPalette) -> AnyElement {
        match kind {
            RailArtifactKind::Sheet => pro_svg("0 0 12 12", SVG_FILE_SHEET, size_px, p.green),
            RailArtifactKind::Doc => pro_svg("0 0 12 12", SVG_FILE_DOC, size_px, p.blue),
        }
    }

    fn render_rail_icon(section: RailSection, color: Hsla) -> AnyElement {
        match section {
            RailSection::Chats => pro_svg("0 0 24 24", SVG_RAIL_CHATS, 18., color),
            RailSection::Tasks => pro_icon(ProIcon::Tasks, 18., color),
            RailSection::Runs => pro_svg("0 0 24 24", SVG_RAIL_RUNS, 18., color),
            RailSection::Folders => pro_icon(ProIcon::Folder, 18., color),
            RailSection::Integrations => pro_svg("0 0 24 24", SVG_RAIL_PLUG, 18., color),
        }
    }

    fn render_icon_rail(&self, p: &ProPalette, cx: &mut Context<Self>) -> Div {
        let ws_initial: String = self
            .workspace
            .chars()
            .find(|c| !c.is_whitespace())
            .map(|c| c.to_ascii_uppercase().to_string())
            .unwrap_or_else(|| "F".to_string());

        let mut top = div().flex().flex_col().items_center().gap(px(4.));
        top = top
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "rail-ws-btn"))
                    .role(Role::Button)
                    .aria_label(self.workspace.clone())
                    .aria_expanded(self.ws_menu_open)
                    .tab_index(0)
                    .size(px(30.))
                    .rounded(px(9.))
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line_strong)
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.ink)
                    .cursor_pointer()
                    .child(ws_initial),
                cx,
                |this, _, cx| {
                    this.ws_menu_open = !this.ws_menu_open;
                    this.project_menu_open = false;
                    this.notif_open = false;
                    this.account_menu_open = false;
                    cx.notify();
                },
            ))
            .child(div().w(px(18.)).h(px(1.)).my(px(4.)).bg(p.line));

        let rail_sensor = pro_selection_sensor();
        let mut sec_strip = div()
            .relative()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(4.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "rail-sec-pill"),
                format!("{:?}", self.rail_section),
                rail_sensor.clone(),
                9.,
                p.select,
                None,
                false,
            ));

        for (ri, sec) in RailSection::all().into_iter().enumerate() {
            let active = self.rail_section == sec;
            let lbl = sec.label(cx);
            sec_strip = sec_strip.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("rail-sec-{ri}")))
                    .role(Role::Button)
                    .aria_label(lbl)
                    .tab_index(0)
                    .relative()
                    .size(px(34.))
                    .rounded(px(9.))
                    .when(active, |d| d.child(pro_mark_selected(&rail_sensor)))
                    .when(!active, |d| d.hover(move |s| s.bg(p.hover)))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(Self::render_rail_icon(
                        sec,
                        if active { p.ink } else { p.muted },
                    )),
                cx,
                move |this, _, cx| {
                    if this.rail_section == sec {
                        this.chat_col_collapsed = !this.chat_col_collapsed;
                    } else {
                        this.rail_section = sec;
                        this.chat_col_collapsed = false;
                    }
                    cx.emit(AgentChatRailEvent::Navigate(sec.label(cx).into()));
                    cx.notify();
                },
            ));
        }
        top = top.child(sec_strip);

        let bottom = div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(4.))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "rail-notif"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Notifications", "通知"))
                    .tab_index(0)
                    .relative()
                    .size(px(34.))
                    .rounded(px(9.))
                    .bg(if self.notif_open {
                        p.select
                    } else {
                        transparent_black()
                    })
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(pro_icon(
                        ProIcon::Bell,
                        18.,
                        if self.notif_open { p.ink } else { p.muted },
                    ))
                    .when(self.unread_notifications, |d| {
                        d.child(
                            div()
                                .absolute()
                                .top(px(7.))
                                .right(px(7.))
                                .size(px(6.))
                                .rounded_full()
                                .bg(p.blue)
                                .border_1()
                                .border_color(p.page),
                        )
                    }),
                cx,
                |this, _, cx| {
                    this.notif_open = !this.notif_open;
                    this.unread_notifications = false;
                    this.ws_menu_open = false;
                    this.account_menu_open = false;
                    cx.notify();
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "rail-settings"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Settings", "设置"))
                    .tab_index(0)
                    .size(px(34.))
                    .rounded(px(9.))
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(pro_icon(ProIcon::Settings, 18., p.muted)),
                cx,
                |_, _, cx| {
                    cx.emit(AgentChatRailEvent::Navigate("Settings".into()));
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "rail-account"))
                    .role(Role::Button)
                    .aria_label(self.user_name.clone())
                    .tab_index(0)
                    .size(px(34.))
                    .rounded_full()
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(pro_person_avatar(&self.user_name, 24., false, None, cx)),
                cx,
                |this, _, cx| {
                    this.account_menu_open = !this.account_menu_open;
                    this.ws_menu_open = false;
                    this.notif_open = false;
                    cx.notify();
                },
            ));

        div()
            .w(px(52.))
            .h_full()
            .py(px(10.))
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .justify_between()
            .child(top)
            .child(bottom)
    }

    fn render_thread_row(
        &self,
        id_suffix: String,
        entry: &RailThreadEntry,
        indented: bool,
        sensor: &ProSelectionSensor,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let tid = entry.id.clone();
        let active = !self.is_empty() && self.active_thread_id == entry.id;
        activate(
            div()
                .id(pro_eid(&self.id, id_suffix))
                .role(Role::Button)
                .tab_index(0)
                .relative()
                .h(px(29.))
                .pl(px(if indented { 24. } else { 8. }))
                .pr(px(8.))
                .rounded(px(7.))
                .when(active, |d| d.child(pro_mark_selected(sensor)))
                .when(!active, |d| d.hover(move |s| s.bg(p.hover)))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .flex()
                .items_center()
                .text_size(px(12.5))
                .font_weight(if active {
                    FontWeight::MEDIUM
                } else {
                    FontWeight::NORMAL
                })
                .text_color(if active { p.ink } else { p.text })
                .cursor_pointer()
                .child(div().min_w_0().truncate().child(entry.title.clone())),
            cx,
            move |this, _, cx| {
                this.select_thread(&tid, cx);
            },
        )
    }

    fn render_chat_column(&self, width: f32, p: &ProPalette, cx: &mut Context<Self>) -> Div {
        let switcher_label = self.project_filter.label(cx);
        let head = div()
            .h(px(30.))
            .px(px(2.))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(6.))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "proj-switcher"))
                    .role(Role::Button)
                    .aria_expanded(self.project_menu_open)
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(6.))
                    .rounded(px(7.))
                    .when(self.project_menu_open, |d| d.bg(p.hover))
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .cursor_pointer()
                    .child(div().max_w(px(150.)).truncate().child(switcher_label))
                    .child(pro_icon(ProIcon::ChevronDown, 12., p.muted)),
                cx,
                |this, _, cx| {
                    this.project_menu_open = !this.project_menu_open;
                    this.ws_menu_open = false;
                    cx.notify();
                },
            ))
            .child(pro_icon_btn(
                pro_eid(&self.id, "side-search-btn"),
                tr(cx, "Search chats", "搜索对话"),
                ProIcon::Search,
                26.,
                self.search_open,
                cx,
                |this, _, cx| {
                    this.search_open = true;
                    this.project_menu_open = false;
                    cx.notify();
                },
            ));

        let ed_for_new = self.composer_editor.clone();
        let new_chat_btn = activate(
            div()
                .id(pro_eid(&self.id, "side-new-chat"))
                .role(Role::Button)
                .tab_index(0)
                .h(px(30.))
                .px(px(8.))
                .rounded(px(7.))
                .hover(move |s| s.bg(p.hover))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .flex()
                .items_center()
                .gap(px(8.))
                .text_size(px(13.))
                .text_color(p.ink)
                .cursor_pointer()
                .child(pro_icon(ProIcon::NewChat, 15., p.muted))
                .child(tr(cx, "New chat", "新建对话")),
            cx,
            move |this, w, cx| {
                this.empty = false;
                this.active_thread_id = "new-chat".into();
                this.messages.clear();
                if let Some(ref ed) = ed_for_new {
                    {
                        let fh = ed.read(cx).focus.clone();
                        fh.focus(w, cx);
                    }
                }
                cx.emit(AgentChatRailEvent::Navigate("New chat".into()));
                cx.notify();
            },
        );

        let thread_sensor = pro_selection_sensor();
        let mut inner_groups =
            div()
                .relative()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "rail-thread-pill"),
                    format!("{:?}:{}", self.project_filter, self.active_thread_id),
                    thread_sensor.clone(),
                    7.,
                    p.select,
                    None,
                    false,
                ));

        if self.is_empty() {
            inner_groups = inner_groups.child(
                div()
                    .px(px(8.))
                    .py(px(6.))
                    .text_size(px(12.5))
                    .text_color(p.faint)
                    .child(tr(cx, "No chats yet", "暂无对话")),
            );
        } else if self.project_filter == ProjectFilter::All {
            let pinned: Vec<RailThreadEntry> =
                self.threads.iter().filter(|t| t.pinned).cloned().collect();
            if !pinned.is_empty() {
                let mut sec = div().flex().flex_col().gap(px(2.)).child(
                    div()
                        .px(px(8.))
                        .pb(px(2.))
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.faint)
                        .child(tr(cx, "Pinned", "已置顶")),
                );
                for (pi, item) in pinned.iter().enumerate() {
                    sec = sec.child(self.render_thread_row(
                        format!("pin-{pi}"),
                        item,
                        false,
                        &thread_sensor,
                        p,
                        cx,
                    ));
                }
                inner_groups = inner_groups.child(sec);
            }

            let projects = [
                (
                    ProjectFilter::SummerCollection,
                    ProIcon::ProjectFolder,
                    0usize,
                ),
                (ProjectFilter::Brand, ProIcon::Folder, 1usize),
                (ProjectFilter::Hiring, ProIcon::Folder, 2usize),
                (ProjectFilter::Research, ProIcon::Folder, 3usize),
            ];
            let mut proj_sec = div().flex().flex_col().gap(px(2.)).child(
                div()
                    .px(px(8.))
                    .pb(px(2.))
                    .text_size(px(11.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.faint)
                    .child(tr(cx, "Projects", "项目")),
            );
            for (proj, folder_ic, slot_idx) in projects {
                let open = self.expanded_projects[slot_idx];
                let plabel = proj.label(cx);
                proj_sec = proj_sec.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("proj-folder-{slot_idx}")))
                        .role(Role::Button)
                        .aria_expanded(open)
                        .tab_index(0)
                        .h(px(29.))
                        .px(px(8.))
                        .rounded(px(7.))
                        .hover(move |s| s.bg(p.hover))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(12.5))
                        .text_color(if open { p.ink } else { p.text })
                        .cursor_pointer()
                        .child(pro_icon(folder_ic, 15., if open { p.ink } else { p.muted }))
                        .child(div().flex_1().min_w_0().truncate().child(plabel)),
                    cx,
                    move |this, _, cx| {
                        this.expanded_projects[slot_idx] = !this.expanded_projects[slot_idx];
                        cx.emit(AgentChatRailEvent::Navigate(proj.label(cx).into()));
                        cx.notify();
                    },
                ));

                let children: Vec<RailThreadEntry> = self
                    .threads
                    .iter()
                    .filter(|t| t.project == Some(proj))
                    .cloned()
                    .collect();
                let mut child_col = div().flex().flex_col().gap(px(2.));
                for (ci, child) in children.iter().enumerate() {
                    child_col = child_col.child(self.render_thread_row(
                        format!("pchat-{slot_idx}-{ci}"),
                        child,
                        true,
                        &thread_sensor,
                        p,
                        cx,
                    ));
                }
                let est_h = (children.len() as f32) * 31.;
                proj_sec = proj_sec.child(pro_fold(
                    pro_eid(&self.id, format!("proj-fold-{slot_idx}")),
                    open && !children.is_empty(),
                    est_h,
                    2.,
                    move || child_col.into_any_element(),
                ));
            }
            inner_groups = inner_groups.child(proj_sec);

            let loose: Vec<RailThreadEntry> = self
                .threads
                .iter()
                .filter(|t| !t.pinned && t.project.is_none())
                .cloned()
                .collect();
            if !loose.is_empty() {
                let mut loose_sec = div().flex().flex_col().gap(px(2.)).child(
                    div()
                        .px(px(8.))
                        .pb(px(2.))
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.faint)
                        .child(tr(cx, "Chats", "对话")),
                );
                for (li, item) in loose.iter().enumerate() {
                    loose_sec = loose_sec.child(self.render_thread_row(
                        format!("loose-{li}"),
                        item,
                        false,
                        &thread_sensor,
                        p,
                        cx,
                    ));
                }
                inner_groups = inner_groups.child(loose_sec);
            }
        } else {
            let scoped = self.project_filter;
            let scoped_items: Vec<RailThreadEntry> = self
                .threads
                .iter()
                .filter(|t| t.project == Some(scoped))
                .cloned()
                .collect();
            let mut scoped_sec = div().flex().flex_col().gap(px(2.)).child(
                div()
                    .px(px(8.))
                    .pb(px(2.))
                    .text_size(px(11.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.faint)
                    .child(scoped.label(cx)),
            );
            for (si, item) in scoped_items.iter().enumerate() {
                scoped_sec = scoped_sec.child(self.render_thread_row(
                    format!("scoped-{si}"),
                    item,
                    false,
                    &thread_sensor,
                    p,
                    cx,
                ));
            }
            inner_groups = inner_groups.child(scoped_sec);
        }

        let list = div()
            .id(pro_eid(&self.id, "chat-col-scroll"))
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .child(pro_view_transition(
                pro_eid(&self.id, "chat-col-filter-vtx"),
                format!("{:?}:{}", self.project_filter, self.is_empty()),
                inner_groups,
            ));

        div()
            .w(px(width))
            .h_full()
            .py(px(10.))
            .px(px(8.))
            .flex_none()
            .bg(p.side_tint)
            .border_r_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(head)
            .child(new_chat_btn)
            .child(list)
    }

    fn render_thread_loading(&self, phase: f32, p: &ProPalette, cx: &Context<Self>) -> Div {
        div()
            .w_full()
            .max_w(px(680.))
            .mx_auto()
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(
                div()
                    .self_end()
                    .px(px(14.))
                    .py(px(9.))
                    .rounded(px(16.))
                    .bg(p.chip)
                    .child(pro_skeleton_bar(270., 14., 5., phase, cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(pro_skeleton_bar(14., 14., 4., phase, cx))
                            .child(pro_skeleton_bar(92., 12., 4., phase, cx)),
                    )
                    .child(pro_skeleton_bar(116., 12., 4., phase, cx))
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(7.))
                            .child(pro_skeleton_bar(380., 13., 5., phase, cx))
                            .child(pro_skeleton_bar(340., 13., 5., phase, cx)),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(7.))
                            .child(pro_skeleton_bar(395., 13., 5., phase, cx))
                            .child(pro_skeleton_bar(280., 13., 5., phase, cx)),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(6.))
                            .child(pro_skeleton_bar(140., 24., 6., phase, cx))
                            .child(pro_skeleton_bar(96., 24., 6., phase, cx)),
                    ),
            )
            .child(
                div()
                    .self_end()
                    .px(px(14.))
                    .py(px(9.))
                    .rounded(px(16.))
                    .bg(p.chip)
                    .child(pro_skeleton_bar(230., 14., 5., phase, cx)),
            )
    }

    fn render_message(
        &self,
        mi: usize,
        msg: &RailChatMessage,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if msg.role == AgentChatRailRole::User {
            let body = msg.paragraphs.join("\n");
            return div()
                .w_full()
                .flex()
                .justify_end()
                .child(
                    div()
                        .max_w(relative(0.82))
                        .px(px(14.))
                        .py(px(8.))
                        .rounded(px(16.))
                        .bg(p.chip)
                        .text_size(px(13.5))
                        .line_height(px(20.5))
                        .text_color(p.ink)
                        .child(body),
                )
                .into_any_element();
        }

        let mut col = div()
            .w_full()
            .flex()
            .flex_col()
            .items_start()
            .gap(px(11.))
            .text_size(px(13.5))
            .line_height(px(21.6))
            .text_color(p.ink);

        // Who line: [purple 3x3 avatar] Assistant 9:04 AM
        let mut who = div()
            .h(px(20.))
            .flex()
            .items_center()
            .gap(px(8.))
            .child(Self::render_purple_pixel_avatar(p))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child("Assistant"),
            );
        if let Some(ref ts) = msg.timestamp {
            who = who.child(
                div()
                    .text_size(px(12.))
                    .text_color(p.faint)
                    .child(ts.clone()),
            );
        }
        col = col.child(who);

        // Collapsible tool trace group
        if let Some(ref summary) = msg.tool_summary {
            let open = msg.tools_open;
            let mid = msg.id.clone();
            col = col.child(
                div()
                    .w_full()
                    .my(px(-2.))
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap(px(4.))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("tools-head-{mi}")))
                            .role(Role::Button)
                            .aria_expanded(open)
                            .tab_index(0)
                            .h(px(26.))
                            .ml(px(-6.))
                            .px(px(6.))
                            .rounded(px(6.))
                            .hover(move |s| s.bg(p.hover))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .cursor_pointer()
                            .child(pro_icon(ProIcon::Check, 13., p.green))
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .text_color(p.muted)
                                    .child(summary.clone()),
                            )
                            .when(!msg.tools.is_empty(), |d| {
                                d.child(pro_icon(
                                    if open {
                                        ProIcon::ChevronDown
                                    } else {
                                        ProIcon::ChevronRight
                                    },
                                    11.,
                                    p.faint,
                                ))
                            }),
                        cx,
                        move |this, _, cx| {
                            if let Some(target) = this.messages.iter_mut().find(|m| m.id == mid) {
                                target.tools_open = !target.tools_open;
                                cx.notify();
                            }
                        },
                    ))
                    .child({
                        let tools_snapshot = msg.tools.clone();
                        let est_h = (tools_snapshot.len() as f32) * 29.;
                        let line_c = p.line;
                        let green_c = p.green;
                        let amber_c = p.amber;
                        let faint_c = p.faint;
                        let text_c = p.text;
                        pro_fold(
                            pro_eid(&self.id, format!("rail-tools-fold-{}", msg.id)),
                            open && !msg.tools.is_empty(),
                            est_h,
                            4.,
                            move || {
                                let mut list = div()
                                    .ml(px(6.))
                                    .pl(px(12.))
                                    .border_l_1()
                                    .border_color(line_c)
                                    .flex()
                                    .flex_col()
                                    .gap(px(3.));
                                for step in tools_snapshot {
                                    list = list.child(
                                        div()
                                            .h(px(26.))
                                            .flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .text_size(px(12.5))
                                            .child(pro_icon(
                                                if step.done {
                                                    ProIcon::Check
                                                } else {
                                                    ProIcon::Clock
                                                },
                                                12.,
                                                if step.done { green_c } else { amber_c },
                                            ))
                                            .child(pro_icon(step.icon, 13., faint_c))
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .truncate()
                                                    .text_color(text_c)
                                                    .child(step.label),
                                            )
                                            .child(
                                                div()
                                                    .flex_none()
                                                    .text_size(px(11.5))
                                                    .text_color(faint_c)
                                                    .child(step.duration),
                                            ),
                                    );
                                }
                                list.into_any_element()
                            },
                        )
                    }),
            );
        }

        for para in &msg.paragraphs {
            col = col.child(div().w_full().child(para.clone()));
        }

        if !msg.artifacts.is_empty() {
            let mut chips = div().flex().flex_wrap().gap(px(6.));
            for (ai, chip) in msg.artifacts.iter().enumerate() {
                let key = chip.id.clone();
                let name = chip.name.clone();
                let glyph = Self::render_artifact_glyph(chip.kind, 12., p);
                chips = chips.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("msg-chip-{mi}-{ai}")))
                        .role(Role::Button)
                        .aria_label(name.clone())
                        .tab_index(0)
                        .h(px(25.))
                        .pl(px(7.))
                        .pr(px(9.))
                        .rounded(px(6.))
                        .bg(p.chip)
                        .hover(move |s| s.bg(p.chip_hover))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .cursor_pointer()
                        .child(glyph)
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.ink)
                                .truncate()
                                .child(name),
                        ),
                    cx,
                    move |this, _, cx| {
                        this.open_artifact(key.as_ref(), cx);
                    },
                ));
            }
            col = col.child(chips);
        }

        // Copy / Regenerate / Feedback row
        let copy_id = msg.id.clone();
        let copy_body: String = msg
            .paragraphs
            .iter()
            .map(SharedString::as_ref)
            .collect::<Vec<_>>()
            .join("\n\n");
        let regen_id = msg.id.clone();
        let up_id = msg.id.clone();
        let down_id = msg.id.clone();
        let is_copied = self.copied_msg_id.as_ref() == Some(&msg.id);
        let fb = msg.feedback;

        let actions = div()
            .mt(px(-3.))
            .ml(px(-4.))
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_icon_btn(
                pro_eid(&self.id, format!("r-copy-{mi}")),
                tr(cx, "Copy response", "复制回复"),
                if is_copied {
                    ProIcon::Check
                } else {
                    ProIcon::Copy
                },
                24.,
                is_copied,
                cx,
                move |this, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(copy_body.clone()));
                    this.copied_msg_id = Some(copy_id.clone());
                    cx.emit(AgentChatRailEvent::CopyMessage(copy_id.clone()));
                    cx.notify();
                },
            ))
            .child(pro_icon_btn(
                pro_eid(&self.id, format!("r-regen-{mi}")),
                tr(cx, "Regenerate response", "重新生成"),
                ProIcon::Refresh,
                24.,
                false,
                cx,
                move |this, _, cx| {
                    if let Some(target) = this.messages.iter_mut().find(|m| m.id == regen_id) {
                        target.tools_open = false;
                    }
                    cx.emit(AgentChatRailEvent::Regenerate(regen_id.clone()));
                    cx.notify();
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, format!("r-up-{mi}")))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Helpful response", "有帮助"))
                    .tab_index(0)
                    .size(px(24.))
                    .rounded(px(6.))
                    .bg(if fb == Some(AgentChatRailFeedback::Up) {
                        p.chip
                    } else {
                        transparent_black()
                    })
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(pro_svg(
                        "0 0 24 24",
                        SVG_THUMB_UP,
                        13.,
                        if fb == Some(AgentChatRailFeedback::Up) {
                            p.green_ink
                        } else {
                            p.muted
                        },
                    )),
                cx,
                move |this, _, cx| {
                    this.toggle_feedback(&up_id, AgentChatRailFeedback::Up, cx);
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, format!("r-down-{mi}")))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Unhelpful response", "无帮助"))
                    .tab_index(0)
                    .size(px(24.))
                    .rounded(px(6.))
                    .bg(if fb == Some(AgentChatRailFeedback::Down) {
                        p.chip
                    } else {
                        transparent_black()
                    })
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(pro_svg(
                        "0 0 24 24",
                        SVG_THUMB_DOWN,
                        13.,
                        if fb == Some(AgentChatRailFeedback::Down) {
                            p.red_ink
                        } else {
                            p.muted
                        },
                    )),
                cx,
                move |this, _, cx| {
                    this.toggle_feedback(&down_id, AgentChatRailFeedback::Down, cx);
                },
            ));

        col.child(actions).into_any_element()
    }

    fn render_composer(
        &self,
        compact: bool,
        phase: f32,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let draft = self
            .composer_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default();
        let lines = draft
            .split('\n')
            .count()
            .max((draft.chars().count() / 60) + 1)
            .clamp(2, 7);
        let field_h = (lines as f32) * 20.;
        let can_send = !self.loading
            && !self.empty
            && (!draft.trim().is_empty() || !self.attachments.is_empty());

        let mut foot = div()
            .w_full()
            .max_w(px(728.))
            .mx_auto()
            .pt(px(6.))
            .pb(px(if compact { 10. } else { 18. }))
            .px(px(if compact { 12. } else { 24. }))
            .flex()
            .flex_col()
            .gap(px(8.));

        {
            let mut att_row = div().flex().flex_wrap().gap(px(6.));
            let hover_c = p.hover;
            for (ai, name) in self.attachments.iter().enumerate() {
                att_row = att_row.child(
                    div()
                        .h(px(25.))
                        .pl(px(8.))
                        .pr(px(5.))
                        .rounded(px(7.))
                        .bg(p.chip)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(pro_icon(ProIcon::Paperclip, 12., p.muted))
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.ink)
                                .child(name.clone()),
                        )
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("rail-rm-att-{ai}")))
                                .role(Role::Button)
                                .aria_label(tr(cx, "Remove attachment", "移除附件"))
                                .tab_index(0)
                                .size(px(18.))
                                .rounded(px(4.))
                                .hover(move |s| s.bg(hover_c))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .child(pro_icon(ProIcon::Close, 10., p.muted)),
                            cx,
                            move |this, _, cx| {
                                if ai < this.attachments.len() {
                                    this.attachments.remove(ai);
                                    cx.notify();
                                }
                            },
                        )),
                );
            }
            foot = foot.child(pro_fold(
                pro_eid(&self.id, "rcmp-att-fold"),
                !self.attachments.is_empty(),
                31.,
                8.,
                move || att_row.into_any_element(),
            ));
        }

        if let Some(pop) = self.composer_popover {
            self.last_composer_popover.set(pop);
        }
        {
            let pop = self
                .composer_popover
                .unwrap_or_else(|| self.last_composer_popover.get());
            let pop_open = self.composer_popover.is_some();
            let align_end = matches!(pop, ComposerPopover::Model);
            let hover_c = p.hover;
            let menu = match pop {
                ComposerPopover::Add => div()
                    .w(px(238.))
                    .p(px(5.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .px(px(8.))
                            .py(px(4.))
                            .text_size(px(11.5))
                            .text_color(p.faint)
                            .child(tr(cx, "Add to the chat", "添加到对话")),
                    )
                    .children(
                        [
                            (
                                "fabric-comparison.csv",
                                "Attach fabric-comparison.csv",
                                ProIcon::ClipboardDoc,
                            ),
                            ("sources.md", "Attach sources.md", ProIcon::PageDoc),
                            ("mill-swatch.jpg", "Upload fabric photo", ProIcon::Upload),
                        ]
                        .into_iter()
                        .enumerate()
                        .map(|(idx, (fname, lbl, ic))| {
                            activate(
                                div()
                                    .id(pro_eid(&self.id, format!("r-add-{idx}")))
                                    .role(Role::MenuItem)
                                    .tab_index(0)
                                    .h(px(30.))
                                    .px(px(8.))
                                    .rounded(px(7.))
                                    .hover(move |s| s.bg(hover_c))
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .text_size(px(12.5))
                                    .text_color(p.ink)
                                    .cursor_pointer()
                                    .child(pro_icon(ic, 14., p.muted))
                                    .child(lbl),
                                cx,
                                move |this, _, cx| {
                                    let f: SharedString = fname.into();
                                    if !this.attachments.contains(&f) {
                                        this.attachments.push(f);
                                    }
                                    this.composer_popover = None;
                                    cx.notify();
                                },
                            )
                        }),
                    ),
                ComposerPopover::Approval => {
                    let appr_sensor = pro_selection_sensor();
                    div()
                        .w(px(246.))
                        .p(px(5.))
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line_strong)
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .px(px(8.))
                                .py(px(4.))
                                .text_size(px(11.5))
                                .text_color(p.faint)
                                .child(tr(cx, "How should actions be approved?", "操作审批策略")),
                        )
                        .child(
                            div()
                                .relative()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(pro_pill_indicator(
                                    pro_eid(&self.id, "rcmp-appr-pill"),
                                    format!("{:?}", self.approval_mode),
                                    appr_sensor.clone(),
                                    7.,
                                    p.chip,
                                    None,
                                    false,
                                ))
                                .children(
                                    [
                                        RailApprovalMode::RequestApproval,
                                        RailApprovalMode::ApproveEdits,
                                        RailApprovalMode::AutoRun,
                                    ]
                                    .into_iter()
                                    .enumerate()
                                    .map(|(idx, mode)| {
                                        let selected = self.approval_mode == mode;
                                        activate(
                                            div()
                                                .id(pro_eid(&self.id, format!("r-appr-{idx}")))
                                                .role(Role::ListBoxOption)
                                                .tab_index(0)
                                                .relative()
                                                .h(px(30.))
                                                .px(px(8.))
                                                .rounded(px(7.))
                                                .when(selected, |d| {
                                                    d.child(pro_mark_selected(&appr_sensor))
                                                })
                                                .when(!selected, |d| {
                                                    d.hover(move |s| s.bg(hover_c))
                                                })
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .text_size(px(12.5))
                                                .text_color(p.ink)
                                                .cursor_pointer()
                                                .child(mode.label(cx))
                                                .when(selected, |d| {
                                                    d.child(pro_icon(ProIcon::Check, 13., p.ink))
                                                }),
                                            cx,
                                            move |this, _, cx| {
                                                this.approval_mode = mode;
                                                this.composer_popover = None;
                                                cx.emit(AgentChatRailEvent::SelectApproval(mode));
                                                cx.notify();
                                            },
                                        )
                                    }),
                                ),
                        )
                }
                ComposerPopover::Model => {
                    let mdl_sensor = pro_selection_sensor();
                    div()
                        .w(px(252.))
                        .p(px(5.))
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line_strong)
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .px(px(8.))
                                .py(px(4.))
                                .text_size(px(11.5))
                                .text_color(p.faint)
                                .child(tr(cx, "Select a model", "选择模型")),
                        )
                        .child(
                            div()
                                .relative()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(pro_pill_indicator(
                                    pro_eid(&self.id, "rcmp-mdl-pill"),
                                    self.active_model.clone(),
                                    mdl_sensor.clone(),
                                    7.,
                                    p.chip,
                                    None,
                                    false,
                                ))
                                .children(
                                    [
                                        ("Auto", "sparkles", "Fast web synthesis"),
                                        ("Claude Sonnet 4.5", "claude", "Clear editorial prose"),
                                        ("GPT-5", "openai", "Deep multi-hop search"),
                                        ("Gemini 2.5 Pro", "gemini", "Large spec comparison"),
                                    ]
                                    .into_iter()
                                    .enumerate()
                                    .map(
                                        |(idx, (m_name, brand, desc))| {
                                            let selected = self.active_model.as_ref() == m_name;
                                            let lead = if brand == "sparkles" {
                                                pro_icon(ProIcon::Sparkles, 14., p.muted)
                                            } else {
                                                pro_app_artwork(brand, 14., cx).unwrap_or_else(
                                                    || pro_icon(ProIcon::Sparkles, 14., p.muted),
                                                )
                                            };
                                            activate(
                                                div()
                                                    .id(pro_eid(&self.id, format!("r-mdl-{idx}")))
                                                    .role(Role::ListBoxOption)
                                                    .tab_index(0)
                                                    .relative()
                                                    .py(px(6.))
                                                    .px(px(8.))
                                                    .rounded(px(7.))
                                                    .when(selected, |d| {
                                                        d.child(pro_mark_selected(&mdl_sensor))
                                                    })
                                                    .when(!selected, |d| {
                                                        d.hover(move |s| s.bg(hover_c))
                                                    })
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(9.))
                                                    .cursor_pointer()
                                                    .child(lead)
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .min_w_0()
                                                            .flex()
                                                            .flex_col()
                                                            .child(
                                                                div()
                                                                    .text_size(px(12.5))
                                                                    .font_weight(if selected {
                                                                        FontWeight::MEDIUM
                                                                    } else {
                                                                        FontWeight::NORMAL
                                                                    })
                                                                    .text_color(p.ink)
                                                                    .child(m_name),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_size(px(11.5))
                                                                    .text_color(p.muted)
                                                                    .child(desc),
                                                            ),
                                                    )
                                                    .when(selected, |d| {
                                                        d.child(pro_icon(
                                                            ProIcon::Check,
                                                            13.,
                                                            p.ink,
                                                        ))
                                                    }),
                                                cx,
                                                move |this, _, cx| {
                                                    let chosen: SharedString = m_name.into();
                                                    this.active_model = chosen.clone();
                                                    this.composer_popover = None;
                                                    cx.emit(AgentChatRailEvent::SelectModel(
                                                        chosen,
                                                    ));
                                                    cx.notify();
                                                },
                                            )
                                        },
                                    ),
                                ),
                        )
                }
            };
            let pop_vtx_id = pro_eid(&self.id, "rcmp-pop-vtx");
            let pop_key: SharedString = format!("{pop:?}").into();
            foot = foot.child(pro_presence(
                pro_eid(&self.id, "rcmp-pop-pres"),
                pop_open,
                0.18,
                0.12,
                move |a| {
                    div()
                        .relative()
                        .top(px(5. * (1. - a)))
                        .opacity(a)
                        .w_full()
                        .flex()
                        .when(align_end, |d| d.justify_end())
                        .child(pro_view_transition(pop_vtx_id, pop_key, menu))
                        .into_any_element()
                },
            ));
        }

        let ed_for_voice = self.composer_editor.clone();
        let ed_for_send = self.composer_editor.clone();

        let left_tools = div()
            .flex()
            .items_center()
            .gap(px(4.))
            .child(pro_icon_btn(
                pro_eid(&self.id, "rcmp-plus"),
                tr(cx, "Add to the chat", "添加到对话"),
                ProIcon::Plus,
                28.,
                self.composer_popover == Some(ComposerPopover::Add),
                cx,
                |this, _, cx| {
                    this.composer_popover = if this.composer_popover == Some(ComposerPopover::Add) {
                        None
                    } else {
                        Some(ComposerPopover::Add)
                    };
                    cx.notify();
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "rcmp-approval"))
                    .role(Role::Button)
                    .aria_expanded(self.composer_popover == Some(ComposerPopover::Approval))
                    .tab_index(0)
                    .h(px(28.))
                    .pl(px(8.))
                    .pr(px(10.))
                    .rounded(px(8.))
                    .when(
                        self.composer_popover == Some(ComposerPopover::Approval),
                        |d| d.bg(p.hover),
                    )
                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .text_color(p.muted)
                    .cursor_pointer()
                    .child(pro_svg(
                        "0 0 68.155 68.155",
                        SVG_HAND_APPROVAL,
                        15.,
                        p.muted,
                    ))
                    .when(!compact, |d| d.child(self.approval_mode.label(cx))),
                cx,
                |this, _, cx| {
                    this.composer_popover =
                        if this.composer_popover == Some(ComposerPopover::Approval) {
                            None
                        } else {
                            Some(ComposerPopover::Approval)
                        };
                    cx.notify();
                },
            ));

        let right_tools = if self.dictating {
            let mut wave = div()
                .mr(px(4.))
                .h(px(20.))
                .flex()
                .items_center()
                .gap(px(3.));
            for i in 0..7usize {
                let bh = 5. + 11. * ((phase * std::f32::consts::TAU + i as f32 * 0.7).sin().abs());
                wave = wave.child(div().w(px(2.)).h(px(bh)).rounded(px(1.)).bg(p.blue));
            }
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(wave)
                .child(pro_icon_btn(
                    pro_eid(&self.id, "rvoice-cancel"),
                    tr(cx, "Cancel dictation", "取消语音输入"),
                    ProIcon::Close,
                    28.,
                    false,
                    cx,
                    |this, _, cx| {
                        this.dictating = false;
                        cx.notify();
                    },
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "rvoice-commit"))
                        .role(Role::Button)
                        .aria_label(tr(cx, "Finish dictation", "完成语音输入"))
                        .tab_index(0)
                        .size(px(28.))
                        .rounded(px(8.))
                        .bg(p.primary)
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Check, 15., p.on_primary)),
                    cx,
                    move |this, w, cx| {
                        this.dictating = false;
                        if let Some(ref ed) = ed_for_voice {
                            ed.update(cx, |e, cx| {
                                e.set_value(
                                    "Compare enzyme-washed linen against Japanese seersucker",
                                    w,
                                    cx,
                                );
                            });
                        }
                        cx.notify();
                    },
                ))
        } else {
            let model_lead = match self.active_model.as_ref() {
                "Claude Sonnet 4.5" => pro_app_artwork("claude", 13., cx),
                "GPT-5" => pro_app_artwork("openai", 13., cx),
                "Gemini 2.5 Pro" => pro_app_artwork("gemini", 13., cx),
                _ => None,
            };
            let submit_btn = if self.streaming {
                activate(
                    div()
                        .id(pro_eid(&self.id, "rcmp-stop"))
                        .role(Role::Button)
                        .aria_label(tr(cx, "Stop response", "停止生成"))
                        .tab_index(0)
                        .size(px(28.))
                        .ml(px(2.))
                        .rounded(px(8.))
                        .bg(p.chip_hover)
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .child(div().size(px(9.)).rounded(px(2.)).bg(p.ink)),
                    cx,
                    |this, _, cx| {
                        this.streaming = false;
                        cx.emit(AgentChatRailEvent::StopGeneration);
                        cx.notify();
                    },
                )
                .into_any_element()
            } else {
                activate(
                    div()
                        .id(pro_eid(&self.id, "rcmp-send"))
                        .role(Role::Button)
                        .aria_label(tr(cx, "Send message", "发送消息"))
                        .tab_index(0)
                        .size(px(28.))
                        .ml(px(2.))
                        .rounded(px(8.))
                        .bg(if can_send { p.primary } else { p.chip })
                        .flex()
                        .items_center()
                        .justify_center()
                        .when(can_send, |d| d.cursor_pointer())
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .child(pro_icon(
                            ProIcon::ArrowUp,
                            16.,
                            if can_send { p.on_primary } else { p.faint },
                        )),
                    cx,
                    move |this, w, cx| {
                        if let Some(ref ed) = ed_for_send {
                            let val = ed.read(cx).value().to_string();
                            if this.submit_prompt(&val, cx) {
                                ed.update(cx, |e, cx| e.set_value("", w, cx));
                            }
                        }
                    },
                )
                .into_any_element()
            };

            div()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "rcmp-model"))
                        .role(Role::Button)
                        .aria_expanded(self.composer_popover == Some(ComposerPopover::Model))
                        .tab_index(0)
                        .h(px(28.))
                        .pl(px(8.))
                        .pr(px(6.))
                        .rounded(px(8.))
                        .when(self.composer_popover == Some(ComposerPopover::Model), |d| {
                            d.bg(p.hover)
                        })
                        .hover(move |s| s.bg(p.hover))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .cursor_pointer()
                        .when_some(model_lead, |d, ic| d.child(ic))
                        .child(
                            div()
                                .max_w(px(132.))
                                .truncate()
                                .child(self.active_model.clone()),
                        )
                        .child(pro_icon(ProIcon::ChevronDown, 13., p.muted)),
                    cx,
                    |this, _, cx| {
                        this.composer_popover =
                            if this.composer_popover == Some(ComposerPopover::Model) {
                                None
                            } else {
                                Some(ComposerPopover::Model)
                            };
                        cx.notify();
                    },
                ))
                .child(pro_icon_btn(
                    pro_eid(&self.id, "rcmp-mic"),
                    tr(cx, "Dictate message", "语音输入"),
                    ProIcon::Mic,
                    28.,
                    false,
                    cx,
                    |this, _, cx| {
                        this.dictating = true;
                        this.composer_popover = None;
                        cx.notify();
                    },
                ))
                .child(submit_btn)
        };

        let card_bg = if p.dark { rgb(0x1c1c1e).into() } else { p.card };
        foot.child(
            div()
                .w_full()
                .rounded(px(16.))
                .bg(card_bg)
                .border_1()
                .border_color(p.line)
                .pt(px(12.))
                .px(px(10.))
                .pb(px(10.))
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(
                    div()
                        .w_full()
                        .h(px(field_h))
                        .px(px(4.))
                        .when_some(self.composer_editor.clone(), |d, ed| d.child(ed)),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.))
                        .child(left_tools)
                        .child(right_tools),
                ),
        )
    }

    fn render_main_column(
        &self,
        compact: bool,
        show_inline_panel: bool,
        phase: f32,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let top_chrome = div()
            .h(px(36.))
            .px(px(12.))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .child(if !self.standalone {
                if compact {
                    pro_icon_btn(
                        pro_eid(&self.id, "open-drawer-btn"),
                        tr(cx, "Open sidebar", "打开侧边栏"),
                        ProIcon::SidebarLeft,
                        28.,
                        self.mobile_drawer_open,
                        cx,
                        |this, _, cx| {
                            this.mobile_drawer_open = true;
                            cx.notify();
                        },
                    )
                    .into_any_element()
                } else {
                    pro_icon_btn(
                        pro_eid(&self.id, "toggle-chat-rail-btn"),
                        tr(cx, "Toggle chat rail", "切换会话栏"),
                        ProIcon::SidebarLeft,
                        28.,
                        !self.chat_col_collapsed,
                        cx,
                        |this, _, cx| {
                            this.chat_col_collapsed = !this.chat_col_collapsed;
                            cx.notify();
                        },
                    )
                    .into_any_element()
                }
            } else {
                div().into_any_element()
            })
            .child(if !show_inline_panel && !self.standalone {
                pro_icon_btn(
                    pro_eid(&self.id, "open-artifacts-btn"),
                    tr(cx, "Open artifacts", "打开产物面板"),
                    ProIcon::SidebarRight,
                    28.,
                    false,
                    cx,
                    |this, _, cx| {
                        this.panel_open = true;
                        cx.notify();
                    },
                )
                .into_any_element()
            } else {
                div().into_any_element()
            });

        let body: AnyElement = if self.loading {
            self.render_thread_loading(phase, p, cx).into_any_element()
        } else if self.empty || self.messages.is_empty() {
            div()
                .w_full()
                .min_h(px(280.))
                .flex()
                .items_center()
                .justify_center()
                .when(!self.empty, |d| {
                    d.child(pro_empty_state(
                        ProIcon::Thread,
                        tr(cx, "Start a new search chat", "开始新检索对话"),
                        tr(
                            cx,
                            "Ask a question below — web searches fold into one line and tables open on the right.",
                            "在下方输入问题——联网检索会自动折叠为单行摘要，对比表格在右侧打开。",
                        ),
                        None,
                        cx,
                    ))
                })
                .into_any_element()
        } else {
            let msgs_snapshot = self.messages.clone();
            let mut col = div()
                .w_full()
                .max_w(px(680.))
                .mx_auto()
                .flex()
                .flex_col()
                .gap(px(24.));
            for (mi, msg) in msgs_snapshot.iter().enumerate() {
                col = col.child(self.render_message(mi, msg, p, cx));
            }
            col.into_any_element()
        };

        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(p.panel)
            .flex()
            .flex_col()
            .child(top_chrome)
            .child(
                div()
                    .id(pro_eid(&self.id, "rail-thread-scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .pt(px(8.))
                    .px(px(if compact { 14. } else { 24. }))
                    .pb(px(24.))
                    .child(pro_view_transition(
                        pro_eid(&self.id, "rail-thread-vtx"),
                        format!("{}:{}:{}", self.active_thread_id, self.loading, self.empty),
                        body,
                    )),
            )
            .child(self.render_composer(compact, phase, p, cx))
    }

    fn render_sheet_view(&self, p: &ProPalette, cx: &mut Context<Self>) -> Div {
        let (cell_ref, cell_val) = self.selected_sheet_cell();
        let (sel_r, sel_c) = self.selected_cell;
        let is_bold = self.sheet_bold.contains(&(sel_r, sel_c));
        let can_undo = !self.sheet_undo_stack.is_empty();
        let can_redo = !self.sheet_redo_stack.is_empty();

        // Artifact header: [green sheet icon] fabric-comparison.csv / Just now
        let head = div()
            .px(px(14.))
            .pt(px(14.))
            .pb(px(10.))
            .flex()
            .items_center()
            .gap(px(10.))
            .child(
                div()
                    .size(px(26.))
                    .rounded(px(7.))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(Self::render_artifact_glyph(RailArtifactKind::Sheet, 12., p)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child("fabric-comparison.csv"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(p.muted)
                            .child(tr(cx, "Just now", "刚刚")),
                    ),
            );

        // Sheet toolbar matching screenshot: Undo, Redo | B, Aa v | A↓, Z↓ | +Row, +Col, Trash v
        let toolbar = div()
            .px(px(10.))
            .pb(px(8.))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(4.))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "sh-undo"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Undo", "撤销"))
                    .tab_index(0)
                    .size(px(25.))
                    .rounded(px(6.))
                    .when(!can_undo, |d| d.opacity(0.4))
                    .when(can_undo, |d| {
                        d.hover(move |s| s.bg(p.hover)).cursor_pointer()
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_icon(ProIcon::Undo, 13., p.muted)),
                cx,
                |this, _, cx| {
                    this.sheet_undo(cx);
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "sh-redo"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Redo", "重做"))
                    .tab_index(0)
                    .size(px(25.))
                    .rounded(px(6.))
                    .when(!can_redo, |d| d.opacity(0.4))
                    .when(can_redo, |d| {
                        d.hover(move |s| s.bg(p.hover)).cursor_pointer()
                    })
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_svg("0 0 24 24", SVG_REDO, 13., p.muted)),
                cx,
                |this, _, cx| {
                    this.sheet_redo(cx);
                },
            ))
            .child(div().w(px(1.)).h(px(14.)).mx(px(2.)).bg(p.line))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "sh-bold"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Bold", "加粗"))
                    .tab_index(0)
                    .size(px(25.))
                    .rounded(px(6.))
                    .bg(if is_bold { p.chip } else { transparent_black() })
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::BOLD)
                    .text_color(if is_bold { p.ink } else { p.muted })
                    .cursor_pointer()
                    .child("B"),
                cx,
                |this, _, cx| {
                    this.toggle_cell_bold(cx);
                },
            ))
            .child(
                div()
                    .h(px(25.))
                    .px(px(5.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child("Aa")
                    .child(pro_icon(ProIcon::ChevronDown, 10., p.faint)),
            )
            .child(div().w(px(1.)).h(px(14.)).mx(px(2.)).bg(p.line))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "sh-sort-asc"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Sort A to Z", "升序排列"))
                    .tab_index(0)
                    .h(px(25.))
                    .px(px(5.))
                    .rounded(px(6.))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .cursor_pointer()
                    .child("A↓"),
                cx,
                |this, _, cx| {
                    this.sort_sheet(true, cx);
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "sh-sort-desc"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Sort Z to A", "降序排列"))
                    .tab_index(0)
                    .h(px(25.))
                    .px(px(5.))
                    .rounded(px(6.))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .cursor_pointer()
                    .child("Z↓"),
                cx,
                |this, _, cx| {
                    this.sort_sheet(false, cx);
                },
            ))
            .child(div().w(px(1.)).h(px(14.)).mx(px(2.)).bg(p.line))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "sh-ins-row"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Insert row above", "在上方插入行"))
                    .tab_index(0)
                    .size(px(25.))
                    .rounded(px(6.))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(pro_svg("0 0 24 24", SVG_INSERT_ROW, 13., p.muted)),
                cx,
                |this, _, cx| {
                    this.push_sheet_undo();
                    let insert_at = this.selected_cell.0.max(1).min(this.sheet_rows.len());
                    this.sheet_rows
                        .insert(insert_at, ["".into(), "".into(), "".into(), "".into()]);
                    cx.notify();
                },
            ))
            .child(
                div()
                    .size(px(25.))
                    .rounded(px(6.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_svg("0 0 24 24", SVG_INSERT_COL, 13., p.muted)),
            )
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "sh-del-row"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Clear selected cell", "清空选中单元格"))
                    .tab_index(0)
                    .size(px(25.))
                    .rounded(px(6.))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(pro_icon(ProIcon::Trash, 13., p.muted)),
                cx,
                |this, _, cx| {
                    let (r, c) = this.selected_cell;
                    if r > 0
                        && let Some(row) = this.sheet_rows.get(r)
                        && !row[c].is_empty()
                    {
                        this.push_sheet_undo();
                        this.sheet_rows[r][c] = "".into();
                        cx.notify();
                    }
                },
            ));

        // Formula bar: [A2] | fx | [Linen]
        let formula_bar = div()
            .mx(px(14.))
            .my(px(10.))
            .h(px(28.))
            .rounded(px(6.))
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .items_center()
            .text_size(px(12.))
            .child(
                div()
                    .w(px(44.))
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
                    .min_w_0()
                    .px(px(6.))
                    .truncate()
                    .text_color(p.ink)
                    .child(cell_val),
            );

        // Spreadsheet grid (A..D x rows 1..N)
        let col_widths = [110.0f32, 82.0, 68.0, 64.0];
        let rim_bg = if p.dark {
            hsla(0., 0., 1., 0.025)
        } else {
            hsla(0., 0., 0., 0.02)
        };
        let active_rim_bg = if p.dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.055)
        };

        let mut col_head = div()
            .h(px(22.))
            .bg(rim_bg)
            .border_t_1()
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .child(
                div()
                    .w(px(24.))
                    .h_full()
                    .flex_none()
                    .border_r_1()
                    .border_color(p.line),
            );
        for (ci, &cw) in col_widths.iter().enumerate() {
            let letter = ((b'A' + ci as u8) as char).to_string();
            let active_col = ci == sel_c;
            col_head = col_head.child(
                div()
                    .w(px(cw))
                    .h_full()
                    .flex_none()
                    .when(active_col, |d| d.bg(active_rim_bg))
                    .border_r_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(11.))
                    .font_weight(if active_col {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if active_col { p.ink } else { p.faint })
                    .child(letter),
            );
        }

        let mut grid_rows = div().flex().flex_col();
        for (ri, row) in self.sheet_rows.iter().enumerate() {
            let active_row = ri == sel_r;
            let mut row_el = div()
                .h(px(26.))
                .border_b_1()
                .border_color(p.line_soft)
                .flex()
                .items_center()
                .child(
                    div()
                        .w(px(24.))
                        .h_full()
                        .flex_none()
                        .bg(if active_row { active_rim_bg } else { rim_bg })
                        .border_r_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(11.))
                        .text_color(if active_row { p.ink } else { p.faint })
                        .child(format!("{}", ri + 1)),
                );

            for (ci, cell_text) in row.iter().enumerate() {
                let cw = col_widths[ci];
                let selected = (ri, ci) == (sel_r, sel_c);
                let cell_bold = self.sheet_bold.contains(&(ri, ci));
                row_el = row_el.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("sh-cell-{ri}-{ci}")))
                        .role(Role::Cell)
                        .tab_index(0)
                        .relative()
                        .w(px(cw))
                        .h_full()
                        .flex_none()
                        .px(px(7.))
                        .border_r_1()
                        .border_color(p.line_soft)
                        .when(selected, |d| {
                            d.bg(p.chip).shadow(vec![BoxShadow {
                                color: p.muted,
                                offset: point(px(0.), px(0.)),
                                blur_radius: px(0.),
                                spread_radius: px(1.5),
                                inset: true,
                            }])
                        })
                        .hover(move |s| if selected { s } else { s.bg(p.hover) })
                        .flex()
                        .items_center()
                        .text_size(px(11.5))
                        .font_weight(if cell_bold {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(p.ink)
                        .cursor_pointer()
                        .child(div().min_w_0().truncate().child(cell_text.clone()))
                        .when(selected, |d| {
                            d.child(
                                div()
                                    .absolute()
                                    .right_0()
                                    .bottom_0()
                                    .size(px(5.))
                                    .bg(p.muted),
                            )
                        }),
                    cx,
                    move |this, _, cx| {
                        this.selected_cell = (ri, ci);
                        cx.notify();
                    },
                ));
            }
            grid_rows = grid_rows.child(row_el);
        }

        let row_count = self
            .sheet_rows
            .iter()
            .skip(1)
            .filter(|r| r.iter().any(|c| !c.is_empty()))
            .count();

        div()
            .flex()
            .flex_col()
            .child(head)
            .child(toolbar)
            .child(formula_bar)
            .child(
                div()
                    .id(pro_eid(&self.id, "sh-grid-scroll"))
                    .overflow_x_scroll()
                    .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                        let (mut r, mut c) = this.selected_cell;
                        let max_r = this.sheet_rows.len().saturating_sub(1);
                        match ev.keystroke.key.as_str() {
                            "up" => r = r.saturating_sub(1),
                            "down" => r = (r + 1).min(max_r),
                            "left" => c = c.saturating_sub(1),
                            "right" => c = (c + 1).min(3),
                            _ => return,
                        }
                        this.selected_cell = (r, c);
                        cx.notify();
                    }))
                    .child(div().flex().flex_col().child(col_head).child(grid_rows)),
            )
            .child(
                div()
                    .h(px(28.))
                    .px(px(14.))
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(12.))
                    .text_size(px(11.5))
                    .text_color(p.muted)
                    .child(format!(
                        "{row_count} {}",
                        tr(cx, "fabrics compared", "种面料对比")
                    )),
            )
    }

    fn render_sources_doc(&self, p: &ProPalette, cx: &mut Context<Self>) -> Div {
        let checks = [
            "Prefer 55/45 linen-cotton at 160–180 gsm for all-day drape",
            "Step up to 180–200 gsm when worn open over a jersey tee",
            "Specify enzyme wash to soften first-wear handfeel",
        ];
        let mut checklist = div().flex().flex_col().gap(px(6.));
        for (ci, line) in checks.into_iter().enumerate() {
            let done = self.source_checks[ci];
            checklist = checklist.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("src-chk-{ci}")))
                    .role(Role::CheckBox)
                    .tab_index(0)
                    .py(px(3.))
                    .rounded(px(5.))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .cursor_pointer()
                    .child(
                        div()
                            .size(px(14.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(if done { p.ink } else { p.line_strong })
                            .bg(if done { p.ink } else { p.card })
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(done, |d| d.child(pro_icon(ProIcon::Check, 10., p.card))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(12.5))
                            .text_color(if done { p.muted } else { p.ink })
                            .when(done, |d| d.line_through())
                            .child(line),
                    ),
                cx,
                move |this, _, cx| {
                    this.source_checks[ci] = !this.source_checks[ci];
                    cx.notify();
                },
            ));
        }

        div()
            .p(px(14.))
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .size(px(26.))
                            .rounded(px(7.))
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Self::render_artifact_glyph(RailArtifactKind::Doc, 12., p)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child("sources.md"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(p.muted)
                                    .child("Just now · 4 web sources"),
                            ),
                    ),
            )
            .child(
                div()
                    .pt(px(10.))
                    .border_t_1()
                    .border_color(p.line_soft)
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .text_size(px(14.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child("Summer Overshirt Fabric Notes"),
                    )
                    .children(
                        [
                            ("Masters of Linen", "Flax hollow fibres wick 20% moisture before feeling damp; 150–190 gsm is standard shirting-to-overshirt territory."),
                            ("Albini 1876 Mill Spec", "55% linen / 45% long-staple cotton cuts crease recovery angles in half while retaining 84% of pure linen airflow."),
                            ("Portuguese Atelier Guide", "Unlined open-front overshirts hang cleanly at 180–200 gsm without clinging to 160 gsm combed cotton T-shirts."),
                        ]
                        .into_iter()
                        .map(|(src, note)| {
                            div()
                                .pl(px(10.))
                                .border_l_1()
                                .border_color(p.line_strong)
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(src),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(18.5))
                                        .text_color(p.text)
                                        .child(note),
                                )
                        }),
                    )
                    .child(checklist),
            )
    }

    fn render_artifacts_panel(&self, phase: f32, p: &ProPalette, cx: &mut Context<Self>) -> Div {
        let active_slot = self
            .artifact_tabs
            .get(self.active_artifact_tab)
            .copied()
            .unwrap_or(ArtifactSlot::FabricCsv);

        let tab_sensor = pro_selection_sensor();
        let mut tab_list = div()
            .relative()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(3.))
            .overflow_hidden();

        if !self.is_empty() {
            tab_list = tab_list.child(pro_pill_indicator(
                pro_eid(&self.id, "art-tab-pill"),
                format!("atab-{}:{}", self.active_artifact_tab, active_slot.id()),
                tab_sensor.clone(),
                6.,
                p.chip,
                None,
                false,
            ));
            for (ti, &slot) in self.artifact_tabs.iter().enumerate() {
                let selected = ti == self.active_artifact_tab;
                let t_title = slot.title(cx);
                let t_icon = slot.icon();
                let tab_count = self.artifact_tabs.len();
                let hover_c = p.hover;
                tab_list = tab_list.child(
                    div()
                        .relative()
                        .h(px(27.))
                        .max_w(px(148.))
                        .pl(px(7.))
                        .pr(px(4.))
                        .rounded(px(6.))
                        .when(selected, |d| d.child(pro_mark_selected(&tab_sensor)))
                        .when(!selected, |d| d.hover(move |s| s.bg(hover_c)))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("art-tab-{ti}")))
                                .role(Role::Tab)
                                .aria_selected(selected)
                                .tab_index(0)
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .cursor_pointer()
                                .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _, cx| {
                                    match ev.keystroke.key.as_str() {
                                        "left" if tab_count > 1 => {
                                            this.active_artifact_tab =
                                                if ti == 0 { tab_count - 1 } else { ti - 1 };
                                            cx.notify();
                                        }
                                        "right" if tab_count > 1 => {
                                            this.active_artifact_tab = (ti + 1) % tab_count;
                                            cx.notify();
                                        }
                                        "delete" | "backspace" => {
                                            this.close_artifact_tab(ti, cx);
                                        }
                                        _ => {}
                                    }
                                }))
                                .child(pro_icon(
                                    t_icon,
                                    13.,
                                    if selected { p.ink } else { p.muted },
                                ))
                                .child(
                                    div()
                                        .max_w(px(94.))
                                        .truncate()
                                        .text_size(px(12.))
                                        .font_weight(if selected {
                                            FontWeight::MEDIUM
                                        } else {
                                            FontWeight::NORMAL
                                        })
                                        .text_color(if selected { p.ink } else { p.muted })
                                        .child(t_title.clone()),
                                ),
                            cx,
                            move |this, _, cx| {
                                this.active_artifact_tab = ti;
                                cx.notify();
                            },
                        ))
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("art-tab-x-{ti}")))
                                .role(Role::Button)
                                .aria_label(format!("Close {t_title}"))
                                .tab_index(0)
                                .size(px(16.))
                                .flex_none()
                                .rounded(px(4.))
                                .hover(move |s| s.bg(p.hover))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .child(pro_icon(ProIcon::Close, 10., p.faint)),
                            cx,
                            move |this, _, cx| {
                                this.close_artifact_tab(ti, cx);
                            },
                        )),
                );
            }
            tab_list = tab_list.child(pro_icon_btn(
                pro_eid(&self.id, "art-tab-new"),
                tr(cx, "New tab", "新标签页"),
                ProIcon::Plus,
                24.,
                false,
                cx,
                |this, _, cx| {
                    this.open_artifact("new-tab", cx);
                },
            ));
        } else {
            tab_list = tab_list.child(
                div()
                    .px(px(6.))
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(tr(cx, "Artifacts", "产物")),
            );
        }

        let panel_head = div()
            .h(px(38.))
            .px(px(8.))
            .flex_none()
            .border_b_1()
            .border_color(p.line_soft)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(6.))
            .child(tab_list)
            .child(pro_icon_btn(
                pro_eid(&self.id, "close-artifacts-btn"),
                tr(cx, "Close artifacts", "收起产物面板"),
                ProIcon::SidebarRight,
                26.,
                false,
                cx,
                |this, _, cx| {
                    this.panel_open = false;
                    cx.notify();
                },
            ));

        let panel_body: AnyElement = if self.loading {
            div()
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(14.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(26., 26., 7., phase, cx))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(5.))
                                .child(pro_skeleton_bar(130., 12., 4., phase, cx))
                                .child(pro_skeleton_bar(62., 10., 4., phase, cx)),
                        ),
                )
                .child(pro_skeleton_bar(250., 24., 6., phase, cx))
                .child(pro_skeleton_bar(268., 28., 6., phase, cx))
                .child(pro_skeleton_bar(268., 154., 8., phase, cx))
                .into_any_element()
        } else if self.empty {
            pro_empty_state(
                ProIcon::ClipboardDoc,
                tr(cx, "No files yet", "暂无文件"),
                tr(
                    cx,
                    "Comparison tables and cited sources created in a chat appear here.",
                    "对话中生成的对比表格与引用来源会在此处打开。",
                ),
                None,
                cx,
            )
            .into_any_element()
        } else {
            match active_slot {
                ArtifactSlot::FabricCsv => self.render_sheet_view(p, cx).into_any_element(),
                ArtifactSlot::SourcesMd => self.render_sources_doc(p, cx).into_any_element(),
                ArtifactSlot::NewTab => div()
                    .p(px(14.))
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.faint)
                            .child(tr(cx, "Open an artifact", "打开产物")),
                    )
                    .children(
                        [
                            (
                                "fabric-comparison.csv",
                                "fabric-comparison.csv",
                                "4 fabrics · gsm & crease table",
                            ),
                            ("sources.md", "sources.md", "4 cited textile mill sources"),
                        ]
                        .into_iter()
                        .enumerate()
                        .map(|(si, (key, title, sub))| {
                            activate(
                                div()
                                    .id(pro_eid(&self.id, format!("newtab-opt-{si}")))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .py(px(8.))
                                    .px(px(10.))
                                    .rounded(px(8.))
                                    .bg(p.card)
                                    .border_1()
                                    .border_color(p.line)
                                    .hover(move |s| s.bg(p.hover))
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.))
                                    .cursor_pointer()
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child(title),
                                    )
                                    .child(
                                        div().text_size(px(11.5)).text_color(p.muted).child(sub),
                                    ),
                                cx,
                                move |this, _, cx| {
                                    this.open_artifact(key, cx);
                                },
                            )
                        }),
                    )
                    .into_any_element(),
            }
        };

        div()
            .w(px(self.panel_width))
            .h_full()
            .flex_none()
            .rounded(px(12.))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(panel_head)
            .child(
                div()
                    .id(pro_eid(&self.id, "art-panel-scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(pro_view_transition(
                        pro_eid(&self.id, "art-panel-vtx"),
                        format!("{}:{}:{}", active_slot.id(), self.loading, self.empty),
                        panel_body,
                    )),
            )
    }
}

impl Render for AgentChatRailScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let phase = self.skeleton_phase(w, cx);

        if self.standalone {
            return self
                .render_main_column(false, false, phase, &p, cx)
                .into_any_element();
        }

        let total_w = self.measured_width.get();
        let compact = total_w < 820.;
        let show_inline_panel = self.panel_open && total_w >= 960.;
        let measure_cell = self.measured_width.clone();
        let ed_for_slash = self.composer_editor.clone();

        let chat_col_w = self.chat_col_width;
        let panel_w = self.panel_width;
        let dragging_chat = matches!(self.dragging, Some((DragColumn::ChatSidebar, _, _)));
        let dragging_panel = matches!(self.dragging, Some((DragColumn::ArtifactPanel, _, _)));
        let target_chat_w = if compact || self.chat_col_collapsed {
            0.
        } else {
            chat_col_w
        };
        let anim_chat_w = pro_spring_f32(
            pro_eid(&self.id, "chat-col-w-spring"),
            target_chat_w,
            if dragging_chat { 0.02 } else { 0.25 },
            w,
            cx,
        )
        .max(0.);
        let target_panel_w = if show_inline_panel { panel_w } else { 0. };
        let anim_panel_w = pro_spring_f32(
            pro_eid(&self.id, "art-panel-w-spring"),
            target_panel_w,
            if dragging_panel { 0.02 } else { 0.26 },
            w,
            cx,
        )
        .max(0.);

        let chat_resizer = div()
            .id(pro_eid(&self.id, "resizer-chat-col"))
            .w(px(6.))
            .h_full()
            .ml(px(-3.))
            .mr(px(-3.))
            .cursor_col_resize()
            .hover(move |s| s.bg(p.line_strong))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                    if ev.click_count >= 2 {
                        this.chat_col_width = 226.;
                        this.dragging = None;
                    } else {
                        this.dragging = Some((
                            DragColumn::ChatSidebar,
                            f32::from(ev.position.x),
                            chat_col_w,
                        ));
                    }
                    cx.notify();
                }),
            );

        let middle_card = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .rounded(px(12.))
            .bg(p.panel)
            .border_1()
            .border_color(p.line)
            .overflow_hidden()
            .flex()
            .when(!compact && anim_chat_w > 0.5, |d| {
                d.child(
                    div()
                        .w(px(anim_chat_w))
                        .h_full()
                        .flex_none()
                        .overflow_hidden()
                        .child(self.render_chat_column(chat_col_w, &p, cx)),
                )
                .when(!self.chat_col_collapsed, |d| d.child(chat_resizer))
            })
            .child(self.render_main_column(compact, show_inline_panel, phase, &p, cx));

        let panel_resizer = div()
            .id(pro_eid(&self.id, "resizer-artifacts"))
            .w(px(8.))
            .h_full()
            .flex_none()
            .cursor_col_resize()
            .hover(move |s| s.bg(p.line_strong))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                    if ev.click_count >= 2 {
                        this.panel_width = 318.;
                        this.dragging = None;
                    } else {
                        this.dragging =
                            Some((DragColumn::ArtifactPanel, f32::from(ev.position.x), panel_w));
                    }
                    cx.notify();
                }),
            );

        let mut root = div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .min_h(px(560.))
            .bg(p.page)
            .text_size(px(13.))
            .text_color(p.text)
            .overflow_hidden()
            .flex()
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| {
                if let Some((col, start_x, start_w)) = this.dragging {
                    let dx = f32::from(ev.position.x) - start_x;
                    match col {
                        DragColumn::ChatSidebar => {
                            this.chat_col_width = (start_w + dx).clamp(184., 330.);
                        }
                        DragColumn::ArtifactPanel => {
                            this.panel_width = (start_w - dx).clamp(252., 460.);
                        }
                    }
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.dragging.take().is_some() {
                        cx.notify();
                    }
                }),
            )
            .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, w, cx| {
                let k = ev.keystroke.key.as_str();
                if (ev.keystroke.modifiers.platform || ev.keystroke.modifiers.control)
                    && k.eq_ignore_ascii_case("k")
                {
                    this.search_open = !this.search_open;
                    this.project_menu_open = false;
                    this.ws_menu_open = false;
                    cx.notify();
                    return;
                }
                if k == "escape"
                    && (this.search_open
                        || this.project_menu_open
                        || this.ws_menu_open
                        || this.notif_open
                        || this.account_menu_open
                        || this.composer_popover.is_some()
                        || this.mobile_drawer_open)
                {
                    this.search_open = false;
                    this.project_menu_open = false;
                    this.ws_menu_open = false;
                    this.notif_open = false;
                    this.account_menu_open = false;
                    this.composer_popover = None;
                    this.mobile_drawer_open = false;
                    cx.notify();
                    return;
                }
                if this.project_menu_open {
                    let all_p = ProjectFilter::all();
                    if k == "down" {
                        this.project_cursor = (this.project_cursor + 1) % all_p.len();
                        cx.notify();
                        return;
                    } else if k == "up" {
                        this.project_cursor = if this.project_cursor == 0 {
                            all_p.len() - 1
                        } else {
                            this.project_cursor - 1
                        };
                        cx.notify();
                        return;
                    } else if k == "enter" {
                        let chosen = all_p[this.project_cursor.min(all_p.len() - 1)];
                        this.select_project(chosen, cx);
                        return;
                    }
                }
                if this.ws_menu_open {
                    let ws_opts = ["Fieldwork", "Acme", "Northwind"];
                    if k == "down" {
                        this.ws_cursor = (this.ws_cursor + 1) % ws_opts.len();
                        cx.notify();
                        return;
                    } else if k == "up" {
                        this.ws_cursor = if this.ws_cursor == 0 {
                            ws_opts.len() - 1
                        } else {
                            this.ws_cursor - 1
                        };
                        cx.notify();
                        return;
                    } else if k == "enter" {
                        let chosen = ws_opts[this.ws_cursor.min(ws_opts.len() - 1)];
                        this.workspace = chosen.into();
                        this.ws_menu_open = false;
                        cx.emit(AgentChatRailEvent::Navigate(chosen.into()));
                        cx.notify();
                        return;
                    }
                }
                if k == "/"
                    && !ev.keystroke.modifiers.platform
                    && !ev.keystroke.modifiers.control
                    && !this.search_open
                    && let Some(ref ed) = ed_for_slash
                    && !ed.read(cx).focus.is_focused(w)
                {
                    {
                        let fh = ed.read(cx).focus.clone();
                        fh.focus(w, cx);
                    }
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }))
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
            .when(!compact, |d| d.child(self.render_icon_rail(&p, cx)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .py(px(8.))
                    .pr(px(8.))
                    .when(compact, |d| d.pl(px(8.)))
                    .flex()
                    .child(middle_card)
                    .when(total_w >= 960. && anim_panel_w > 0.5, |d| {
                        d.when(show_inline_panel, |d| d.child(panel_resizer)).child(
                            div()
                                .w(px(anim_panel_w))
                                .h_full()
                                .flex_none()
                                .overflow_hidden()
                                .child(self.render_artifacts_panel(phase, &p, cx)),
                        )
                    }),
            );

        // Narrow screen combined icon-rail + chat-sidebar drawer with spring presence
        if compact {
            let rail = self.render_icon_rail(&p, cx);
            let chat_col = self.render_chat_column(236., &p, cx);
            let scrim_id = pro_eid(&self.id, "mobile-drawer-scrim");
            let page_bg = p.page;
            let line_s = p.line_strong;
            let close_drawer = cx.listener(|this, _, _, cx| {
                this.mobile_drawer_open = false;
                cx.notify();
            });
            root = root.child(pro_presence(
                pro_eid(&self.id, "mobile-drawer-pres"),
                self.mobile_drawer_open,
                0.24,
                0.16,
                move |a| {
                    div()
                        .id(scrim_id)
                        .absolute()
                        .inset_0()
                        .bg(hsla(0., 0., 0., 0.45 * a))
                        .flex()
                        .on_mouse_down(MouseButton::Left, close_drawer)
                        .child(
                            div()
                                .relative()
                                .left(px(-288. * (1. - a)))
                                .h_full()
                                .bg(page_bg)
                                .border_r_1()
                                .border_color(line_s)
                                .flex()
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .child(rail)
                                .child(chat_col),
                        )
                        .into_any_element()
                },
            ));
        }

        // Workspace switcher popover from top-left rail button
        {
            let ws_opts = ["Fieldwork", "Acme", "Northwind"];
            let ws_items = ws_opts
                .into_iter()
                .enumerate()
                .map(|(wi, ws_name)| {
                    let current = self.workspace.as_ref() == ws_name;
                    let highlighted = wi == self.ws_cursor;
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("ws-opt-{wi}")))
                            .role(Role::MenuItem)
                            .tab_index(0)
                            .h(px(30.))
                            .px(px(8.))
                            .rounded(px(7.))
                            .when(highlighted, |d| d.bg(p.hover))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(12.5))
                            .text_color(p.ink)
                            .cursor_pointer()
                            .child(ws_name)
                            .when(current, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink))),
                        cx,
                        move |this, _, cx| {
                            this.workspace = ws_name.into();
                            this.ws_menu_open = false;
                            cx.emit(AgentChatRailEvent::Navigate(ws_name.into()));
                            cx.notify();
                        },
                    )
                })
                .collect::<Vec<_>>();
            let ws_pop_id = pro_eid(&self.id, "ws-menu-pop");
            let ws_head_lbl: SharedString = tr(cx, "Workspaces", "工作区").into();
            root = root.child(pro_presence(
                pro_eid(&self.id, "ws-menu-pres"),
                self.ws_menu_open,
                0.18,
                0.12,
                move |a| {
                    div()
                        .id(ws_pop_id)
                        .absolute()
                        .top(px(44. - 5. * (1. - a)))
                        .left(px(12.))
                        .opacity(a)
                        .w(px(208.))
                        .p(px(5.))
                        .rounded(px(11.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line_strong)
                        .shadow_md()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .px(px(8.))
                                .py(px(4.))
                                .text_size(px(11.5))
                                .text_color(p.faint)
                                .child(ws_head_lbl),
                        )
                        .children(ws_items)
                        .into_any_element()
                },
            ));
        }

        // Project switcher dropdown menu
        {
            let left_x = if compact { 20. } else { 60. };
            let pf_sensor = pro_selection_sensor();
            let pf_key: SharedString = format!("{:?}", self.project_filter).into();
            let pf_pill = pro_pill_indicator(
                pro_eid(&self.id, "pf-menu-pill"),
                pf_key,
                pf_sensor.clone(),
                7.,
                p.chip,
                None,
                false,
            );
            let pf_items = ProjectFilter::all()
                .into_iter()
                .enumerate()
                .map(|(pi, filter)| {
                    let selected = self.project_filter == filter;
                    let highlighted = pi == self.project_cursor;
                    let lbl: SharedString = filter.label(cx).into();
                    let sensor_for_opt = pf_sensor.clone();
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("pf-opt-{pi}")))
                            .role(Role::MenuItem)
                            .tab_index(0)
                            .relative()
                            .h(px(30.))
                            .px(px(8.))
                            .rounded(px(7.))
                            .when(selected, |d| d.child(pro_mark_selected(&sensor_for_opt)))
                            .when(!selected && highlighted, |d| d.bg(p.hover))
                            .when(!selected, |d| d.hover(move |s| s.bg(p.hover)))
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(12.5))
                            .font_weight(if selected {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(p.ink)
                            .cursor_pointer()
                            .child(lbl)
                            .when(selected, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink))),
                        cx,
                        move |this, _, cx| {
                            this.select_project(filter, cx);
                        },
                    )
                })
                .collect::<Vec<_>>();
            let proj_pop_id = pro_eid(&self.id, "project-menu-pop");
            root = root.child(pro_presence(
                pro_eid(&self.id, "project-menu-pres"),
                self.project_menu_open,
                0.18,
                0.12,
                move |a| {
                    div()
                        .id(proj_pop_id)
                        .absolute()
                        .top(px(48. - 5. * (1. - a)))
                        .left(px(left_x))
                        .opacity(a)
                        .w(px(214.))
                        .p(px(5.))
                        .rounded(px(11.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line_strong)
                        .shadow_md()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(pf_pill)
                        .children(pf_items)
                        .into_any_element()
                },
            ));
        }

        // Notifications popover from bottom-left bell
        {
            let notif_pop_id = pro_eid(&self.id, "notif-pop");
            let notif_title: SharedString = tr(cx, "Notifications", "通知").into();
            root = root.child(pro_presence(
                pro_eid(&self.id, "notif-pop-pres"),
                self.notif_open,
                0.18,
                0.12,
                move |a| {
                    div()
                        .id(notif_pop_id)
                        .absolute()
                        .bottom(px(84. - 5. * (1. - a)))
                        .left(px(56.))
                        .opacity(a)
                        .w(px(264.))
                        .p(px(10.))
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line_strong)
                        .shadow_md()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(notif_title),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(18.))
                                .text_color(p.text)
                                .child(
                                    "Portuguese mill lead times finished indexing 4 spec sheets.",
                                ),
                        )
                        .into_any_element()
                },
            ));
        }

        // Account menu popover from bottom-left user avatar
        {
            let acct_pop_id = pro_eid(&self.id, "acct-pop");
            let acct_user = self.user_name.clone();
            let acct_items = [
                tr(cx, "Profile", "个人资料"),
                tr(cx, "Keyboard shortcuts", "快捷键"),
                tr(cx, "Sign out", "退出登录"),
            ]
            .into_iter()
            .enumerate()
            .map(|(ai, item_lbl)| {
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("acct-opt-{ai}")))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(29.))
                        .px(px(8.))
                        .rounded(px(7.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .text_size(px(12.5))
                        .text_color(p.text)
                        .cursor_pointer()
                        .child(item_lbl),
                    cx,
                    move |this, _, cx| {
                        this.account_menu_open = false;
                        cx.emit(AgentChatRailEvent::Navigate(item_lbl.into()));
                        cx.notify();
                    },
                )
            })
            .collect::<Vec<_>>();
            root = root.child(pro_presence(
                pro_eid(&self.id, "acct-pop-pres"),
                self.account_menu_open,
                0.18,
                0.12,
                move |a| {
                    div()
                        .id(acct_pop_id)
                        .absolute()
                        .bottom(px(12. - 5. * (1. - a)))
                        .left(px(56.))
                        .opacity(a)
                        .w(px(208.))
                        .p(px(5.))
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line_strong)
                        .shadow_md()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .px(px(8.))
                                .py(px(5.))
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child(acct_user),
                        )
                        .children(acct_items)
                        .into_any_element()
                },
            ));
        }

        // Command search modal (⌘K)
        {
            let q = self
                .search_editor
                .as_ref()
                .map(|e| e.read(cx).value().to_ascii_lowercase())
                .unwrap_or_default();
            let matches: Vec<RailThreadEntry> = self
                .threads
                .iter()
                .filter(|t| q.is_empty() || t.title.to_ascii_lowercase().contains(&q))
                .take(7)
                .cloned()
                .collect();
            let cur = self.search_cursor.min(matches.len().saturating_sub(1));
            let matches_for_key = matches.clone();
            let search_scrim_id = pro_eid(&self.id, "cmd-search-scrim");
            let search_dlg_id = pro_eid(&self.id, "cmd-search-dialog");
            let search_ed_opt = self.search_editor.clone();
            let scrim_max = if p.dark { 0.45 } else { 0.16 };
            let close_search = cx.listener(|this, _, _, cx| {
                this.search_open = false;
                cx.notify();
            });
            let key_search = cx.listener(move |this, ev: &KeyDownEvent, _, cx| {
                if matches_for_key.is_empty() {
                    return;
                }
                match ev.keystroke.key.as_str() {
                    "down" => {
                        this.search_cursor = (this.search_cursor + 1) % matches_for_key.len();
                        cx.notify();
                    }
                    "up" => {
                        this.search_cursor = if this.search_cursor == 0 {
                            matches_for_key.len() - 1
                        } else {
                            this.search_cursor - 1
                        };
                        cx.notify();
                    }
                    "enter" => {
                        let chosen =
                            &matches_for_key[this.search_cursor.min(matches_for_key.len() - 1)];
                        this.search_open = false;
                        this.select_thread(&chosen.id, cx);
                    }
                    _ => {}
                }
            });
            let match_rows = matches
                .into_iter()
                .enumerate()
                .map(|(mi, item)| {
                    let tid = item.id.clone();
                    let proj_tag: SharedString = item
                        .project
                        .map(|pf| pf.label(cx))
                        .unwrap_or(tr(cx, "Chats", "对话"))
                        .into();
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("sres-{mi}")))
                            .role(Role::ListBoxOption)
                            .tab_index(0)
                            .h(px(32.))
                            .px(px(8.))
                            .rounded(px(7.))
                            .when(mi == cur, |d| d.bg(p.hover))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(10.))
                            .cursor_pointer()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(13.))
                                    .text_color(p.ink)
                                    .child(item.title),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.faint)
                                    .child(proj_tag),
                            ),
                        cx,
                        move |this, _, cx| {
                            this.search_open = false;
                            this.select_thread(&tid, cx);
                        },
                    )
                })
                .collect::<Vec<_>>();

            root = root.child(pro_presence(
                pro_eid(&self.id, "cmd-search-pres"),
                self.search_open,
                0.22,
                0.14,
                move |a| {
                    div()
                        .id(search_scrim_id)
                        .absolute()
                        .inset_0()
                        .bg(hsla(0., 0., 0., scrim_max * a))
                        .flex()
                        .items_start()
                        .justify_center()
                        .pt(px(76. + 8. * (1. - a)))
                        .on_mouse_down(MouseButton::Left, close_search)
                        .child(
                            div()
                                .id(search_dlg_id)
                                .role(Role::Dialog)
                                .opacity(a)
                                .w(px(440.))
                                .max_w(relative(0.92))
                                .rounded(px(12.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line_strong)
                                .shadow_lg()
                                .overflow_hidden()
                                .flex()
                                .flex_col()
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .on_key_down(key_search)
                                .child(
                                    div()
                                        .h(px(42.))
                                        .px(px(12.))
                                        .border_b_1()
                                        .border_color(p.line)
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(pro_icon(ProIcon::Search, 15., p.muted))
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .when_some(search_ed_opt, |d, ed| d.child(ed)),
                                        )
                                        .child(
                                            div()
                                                .px(px(5.))
                                                .py(px(2.))
                                                .rounded(px(4.))
                                                .bg(p.chip)
                                                .text_size(px(11.))
                                                .text_color(p.faint)
                                                .child("ESC"),
                                        ),
                                )
                                .child(
                                    div()
                                        .p(px(6.))
                                        .flex()
                                        .flex_col()
                                        .gap(px(2.))
                                        .children(match_rows),
                                ),
                        )
                        .into_any_element()
                },
            ));
        }

        root.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentChatRailFeedback, AgentChatRailRole, AgentChatRailScreen, ProjectFilter};
    use gpui::{AppContext, TestAppContext};

    #[test]
    fn project_switcher_scopes_visible_chats_and_switches_threads() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let screen = app.new(|cx| AgentChatRailScreen::new("test-rail-projects", cx));

        screen.update(&mut app, |s, cx| {
            // Default: All projects lists all 9 chats, with "best-fabric" active.
            assert_eq!(s.project_filter(), ProjectFilter::All);
            assert_eq!(s.visible_threads().len(), 9);
            assert_eq!(s.active_thread_id(), "best-fabric");
            assert_eq!(s.messages().len(), 4);

            // Filtering to Summer collection shows only its 3 project threads.
            s.select_project(ProjectFilter::SummerCollection, cx);
            assert_eq!(s.visible_threads().len(), 3);

            // Selecting another thread loads its messages and updates active_thread_id.
            s.select_thread("colour-names", cx);
            assert_eq!(s.active_thread_id(), "colour-names");
            assert_eq!(s.messages().len(), 2);
            assert_eq!(s.messages()[1].role, AgentChatRailRole::Assistant);
        });
    }

    #[test]
    fn sheet_default_cell_sorting_and_undo_preserve_header_row() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let screen = app.new(|cx| AgentChatRailScreen::new("test-rail-sheet", cx));

        screen.update(&mut app, |s, cx| {
            // Default selected cell is A2 -> "Linen".
            let (cell_ref, val) = s.selected_sheet_cell();
            assert_eq!(cell_ref, "A2");
            assert_eq!(val.as_ref(), "Linen");

            // Sorting descending by column A moves Linen-cotton 55/45 above Linen while keeping row 1 ("Fabric") pinned.
            s.sort_sheet(false, cx);
            assert_eq!(s.sheet_rows[0][0].as_ref(), "Fabric");
            let (_, after_sort) = s.selected_sheet_cell();
            assert_eq!(after_sort.as_ref(), "Linen-cotton 55/45");

            // Undo restores "Linen" at A2, and Redo re-applies the sort.
            assert!(s.sheet_undo(cx));
            assert_eq!(s.selected_sheet_cell().1.as_ref(), "Linen");
            assert!(s.sheet_redo(cx));
            assert_eq!(s.selected_sheet_cell().1.as_ref(), "Linen-cotton 55/45");
        });
    }

    #[test]
    fn loading_wins_over_empty_and_submission_waits_appropriately() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let screen = app.new(|cx| {
            AgentChatRailScreen::new("test-rail-states", cx)
                .with_loading(true)
                .with_empty(true)
        });

        screen.update(&mut app, |s, cx| {
            // Loading takes precedence over empty; sending waits under both states.
            assert!(s.is_loading());
            assert!(!s.is_empty());
            assert!(!s.submit_prompt("Compare linen weave weights", cx));

            s.set_loading(false, cx);
            assert!(s.is_empty());
            assert!(s.visible_threads().is_empty());
            assert!(!s.submit_prompt("Compare linen weave weights", cx));

            // Exiting empty state enables prompt submission, feedback toggles, and artifact tab closing.
            s.set_empty(false, cx);
            let before = s.messages().len();
            assert!(s.submit_prompt("Compare linen weave weights", cx));
            assert_eq!(s.messages().len(), before + 2);

            let last_id = s
                .messages()
                .last()
                .map(|m| m.id.clone())
                .unwrap_or_default();
            s.toggle_feedback(&last_id, AgentChatRailFeedback::Down, cx);
            assert_eq!(
                s.messages().last().and_then(|m| m.feedback),
                Some(AgentChatRailFeedback::Down)
            );

            s.close_artifact_tab(1, cx);
            s.close_artifact_tab(0, cx);
            assert!(!s.panel_open);
        });
    }
}
