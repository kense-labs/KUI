//! AI chat screen with a live step-by-step progress, output, and session context panel.
//!
//! Combines a conversational thread, expandable tool call cards, and a rich message
//! composer on the left with a collapsible session progress trail, document output
//! preview, and used-context inventory beside the conversation.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProEmptyCta, ProPalette, pro_btn, pro_eid, pro_empty_state, pro_fold,
            pro_icon_btn, pro_mark_selected, pro_pill_indicator, pro_presence, pro_progress_ring,
            pro_selection_sensor, pro_skeleton_bar, pro_spring_bar, pro_spring_f32,
            pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        clipboard,
        control::activate,
        editor::{EditEvent, Editor, EditorShortcutEvent, InputMode},
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const HAND_APPROVAL_SVG: &str = r#"<path d="M52.53 32.66V24.85c0-2.75 2.23-4.97 4.97-4.97s4.97 2.22 4.97 4.97v17.18c0 11.29-9.15 20.44-20.45 20.44h-7.78M22.71 39.76l-6.77-7.9c-1.11-1.3-2.73-2.04-4.43-2.04h-.39c-3.01 0-5.45 2.44-5.45 5.45 0 1.08.32 2.13.92 3.02l9.82 14.73c3.94 5.91 10.57 9.45 17.66 9.45" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"/><path d="M52.53 24.14v-7.81c0-2.75-2.22-4.97-4.97-4.97s-4.97 2.22-4.97 4.97v13.49" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"/><path d="M42.6 29.82V10.65c0-2.74-2.23-4.97-4.97-4.97s-4.97 2.23-4.97 4.97V28.4" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"/><path d="M32.66 28.4V16.33c0-2.75-2.23-4.97-4.97-4.97s-4.97 2.22-4.97 4.97v23.43" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"/>"#;

const PLUG_CONNECTOR_SVG: &str = r#"<path d="M15.5 2V6M8.5 6V2" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M6.004 7.613C5.937 6.743 6.64 6 7.53 6h8.94c.89 0 1.593.743 1.526 1.613l-.184 2.379a10.02 10.02 0 0 1-1.68 4.785l-.6.885A3.07 3.07 0 0 1 12.984 17h-1.968a3.07 3.07 0 0 1-2.547-1.338l-.601-.885a10.02 10.02 0 0 1-1.68-4.785L6.004 7.613Z" stroke="currentColor" stroke-width="1.9"/><path d="M12 17V22M11 9H13" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;

const GDOC_SVG: &str = r##"<path d="M14.5 2H6.2A2.2 2.2 0 0 0 4 4.2v15.6A2.2 2.2 0 0 0 6.2 22h11.6a2.2 2.2 0 0 0 2.2-2.2V7.5z" fill="currentColor"/><path d="M14.5 2v3.8a1.7 1.7 0 0 0 1.7 1.7H20z" fill="#A1C2FA"/><path d="M7.5 11h9v1.3h-9zM7.5 13.8h9v1.3h-9zM7.5 16.6h6v1.3h-6z" fill="#ffffff"/>"##;

const GSHEET_SVG: &str = r##"<path d="M14.5 2H6.2A2.2 2.2 0 0 0 4 4.2v15.6A2.2 2.2 0 0 0 6.2 22h11.6a2.2 2.2 0 0 0 2.2-2.2V7.5z" fill="currentColor"/><path d="M14.5 2v3.8a1.7 1.7 0 0 0 1.7 1.7H20z" fill="#87CEAC"/><path d="M7.5 10.5h9v8h-9z" fill="#ffffff"/><path d="M11.4 10.5h1.2v8h-1.2zM7.5 13.1h9v1.1h-9zM7.5 15.8h9v1.1h-9z" fill="currentColor"/>"##;

const WORD_SVG: &str = r##"<rect x="3" y="3" width="18" height="18" rx="3.5" fill="currentColor"/><path d="M6.6 8h1.7l1.2 5.5L10.9 8h1.6l1.4 5.5L15.1 8h1.7l-2.1 8.2h-1.6L11.7 11l-1.4 5.2H8.7z" fill="#ffffff"/>"##;

const MARKDOWN_SVG: &str = r#"<rect x="2.6" y="6.1" width="18.8" height="11.8" rx="2.2" fill="none" stroke="currentColor" stroke-width="1.4"/><path d="M5.6 15V9h1.7l1.6 2.1L10.5 9h1.7v6h-1.7v-3.5l-1.6 2.1-1.6-2.1V15zm10.6 0-2.7-3.1h1.7V9h2v2.9h1.7z" fill="currentColor"/>"#;

/// Execution state of a step in the session progress trail.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProgressStepState {
    #[default]
    Pending,
    Running,
    Done,
}

/// One step in the session progress checklist beside the conversation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProgressStep {
    pub id: SharedString,
    pub label: SharedString,
    pub state: ProgressStepState,
    pub earlier: bool,
}

impl ProgressStep {
    /// Create a progress step with `id`, display `label`, and initial `state`.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        state: ProgressStepState,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            state,
            earlier: false,
        }
    }

    /// Mark the step as belonging to the collapsed earlier-steps group.
    pub fn with_earlier(mut self, earlier: bool) -> Self {
        self.earlier = earlier;
        self
    }
}

/// Status of an inline tool invocation card inside the chat thread.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolCallStatus {
    Running,
    #[default]
    Success,
    Error,
}

/// Leading visual mark for a [`ChatToolCall`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolGlyph {
    App(SharedString),
    Icon(ProIcon),
}

/// Expandable tool call trace card rendered inside an assistant turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatToolCall {
    pub id: SharedString,
    pub glyph: ToolGlyph,
    pub name: SharedString,
    pub summary: SharedString,
    pub duration: SharedString,
    pub status: ToolCallStatus,
    pub args: SharedString,
    pub result: SharedString,
}

/// One message or tool block in the conversation column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatTurnPart {
    UserBubble(SharedString),
    ToolCall(ChatToolCall),
    Paragraph(SharedString),
    BulletSummary {
        heading: SharedString,
        items: Vec<SharedString>,
    },
}

/// Document format icon for session outputs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputKind {
    #[default]
    GoogleDoc,
    GoogleSheet,
    WordDoc,
    Markdown,
}

/// File or document artifact produced during the chat session.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionOutput {
    pub id: SharedString,
    pub name: SharedString,
    pub preview_title: SharedString,
    pub preview_excerpt: SharedString,
    pub source: SharedString,
    pub kind: OutputKind,
}

/// Action approval mode selected in the chat composer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ApprovalPolicy {
    #[default]
    RequestApproval,
    AutoSafe,
    AlwaysAsk,
}

/// Events emitted by [`AgentChatProgressScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentChatProgressEvent {
    /// Sidebar item or folder output navigation requested.
    Navigate(SharedString),
    /// User submitted a new prompt in the chat composer.
    MessageSent(SharedString),
    /// User clicked an output preview or file row.
    OutputOpened(SharedString),
    /// Live step simulation started, paused, resumed, or finished.
    RunStateChanged { running: bool, paused: bool },
}

impl EventEmitter<AgentChatProgressEvent> for AgentChatProgressScreen {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ComposerPopover {
    Add,
    Approval,
    Model,
}

/// Truncate long filenames in the middle while keeping the stem tail and extension intact.
pub fn middle_truncate(name: &str, max_chars: usize) -> String {
    let total = name.chars().count();
    if total <= max_chars || max_chars < 6 {
        return name.to_string();
    }
    let budget = max_chars - 1; // 1 char for '…'
    if let Some(dot_byte) = name.rfind('.') {
        let ext = &name[dot_byte..];
        let stem = &name[..dot_byte];
        let ext_chars = ext.chars().count();
        if !stem.is_empty() && ext_chars + 4 <= budget {
            let remain = budget - ext_chars;
            let tail_keep = (remain / 3).max(2);
            let head_keep = remain - tail_keep;
            let head: String = stem.chars().take(head_keep).collect();
            let stem_len = stem.chars().count();
            let tail: String = stem
                .chars()
                .skip(stem_len.saturating_sub(tail_keep))
                .collect();
            return format!("{head}…{tail}{ext}");
        }
    }
    let tail_keep = budget / 2;
    let head_keep = budget - tail_keep;
    let head: String = name.chars().take(head_keep).collect();
    let tail: String = name.chars().skip(total.saturating_sub(tail_keep)).collect();
    format!("{head}…{tail}")
}

fn default_turns() -> Vec<ChatTurnPart> {
    vec![
        ChatTurnPart::UserBubble(
            "Can you automate the follow-up after our sales calls? The notes are in Notion, the team list is in Drive.".into(),
        ),
        ChatTurnPart::ToolCall(ChatToolCall {
            id: "tc-notion".into(),
            glyph: ToolGlyph::App("notion".into()),
            name: "notion.search".into(),
            summary: "Searched your call notes in Notion".into(),
            duration: "4.2s".into(),
            status: ToolCallStatus::Success,
            args: "{\n  \"query\": \"Sales call notes\",\n  \"limit\": 12,\n  \"sort\": \"last_edited_time\"\n}".into(),
            result: "{\n  \"pages_matched\": 12,\n  \"structure\": [\"Thank-you\", \"Agreed next step\", \"Offer link\"]\n}".into(),
        }),
        ChatTurnPart::Paragraph(
            "I read your last twelve call notes. Every follow-up has the same three parts: a thank-you, the next step you agreed on and the offer as a link.".into(),
        ),
        ChatTurnPart::ToolCall(ChatToolCall {
            id: "tc-drive".into(),
            glyph: ToolGlyph::App("google drive".into()),
            name: "drive.read_file".into(),
            summary: "Read Team Directory (Meetings)".into(),
            duration: "1.8s".into(),
            status: ToolCallStatus::Success,
            args: "{\n  \"file\": \"Team Directory\",\n  \"tab\": \"Meetings\"\n}".into(),
            result: "{\n  \"rows\": 28,\n  \"internal_domain\": \"acme.com\"\n}".into(),
        }),
        ChatTurnPart::BulletSummary {
            heading: "What happens after each call now:".into(),
            items: vec![
                "The notes are read as soon as the call ends in Notion".into(),
                "Everyone in the Team Directory counts as your side, the rest as the customer".into(),
                "A follow-up email is drafted from the offer template and waits in Slack for your OK".into(),
            ],
        },
        ChatTurnPart::UserBubble(
            "Why did the first draft treat Jo from our team as the customer?".into(),
        ),
        ChatTurnPart::ToolCall(ChatToolCall {
            id: "tc-directory".into(),
            glyph: ToolGlyph::Icon(ProIcon::Users),
            name: "directory.match_people".into(),
            summary: "Checked the call's people against the team list".into(),
            duration: "2.6s".into(),
            status: ToolCallStatus::Success,
            args: "{\n  \"call\": \"Northwind demo\",\n  \"attendees\": [\"alex@acme.com\", \"jo.lee@gmail.com\", \"maya@northwind.co\"]\n}".into(),
            result: "{\n  \"unmatched_internal\": \"jo.lee@gmail.com\",\n  \"added_alias\": true\n}".into(),
        }),
        ChatTurnPart::Paragraph(
            "Jo joined the call with a personal Gmail address, and only work addresses were in the Team Directory. I added Jo's Gmail there – the next draft counts Jo as your side. If more of you join from personal addresses, add them to the list the same way.".into(),
        ),
    ]
}

fn default_steps() -> Vec<ProgressStep> {
    vec![
        ProgressStep::new(
            "step-1",
            "Read last twelve call notes in Notion",
            ProgressStepState::Done,
        )
        .with_earlier(true),
        ProgressStep::new(
            "step-2",
            "Match attendees with Team Directory",
            ProgressStepState::Done,
        )
        .with_earlier(true),
        ProgressStep::new(
            "step-3",
            "Connect Slack approval webhook",
            ProgressStepState::Done,
        )
        .with_earlier(true),
        ProgressStep::new(
            "step-4",
            "Draft a test email for Northwind",
            ProgressStepState::Pending,
        ),
        ProgressStep::new(
            "step-5",
            "Fix how team and customer are told apart",
            ProgressStepState::Done,
        ),
    ]
}

fn default_outputs() -> Vec<SessionOutput> {
    vec![
        SessionOutput {
            id: "out-northwind".into(),
            name: "Northwind – follow-up email".into(),
            preview_title: "Northwind: follow-up".into(),
            preview_excerpt:
                "Thanks for the demo today – here is what we agreed on and the offer we promised."
                    .into(),
            source: "Google Docs".into(),
            kind: OutputKind::GoogleDoc,
        },
        SessionOutput {
            id: "out-directory".into(),
            name: "Team Directory".into(),
            preview_title: "Team Directory · Meetings".into(),
            preview_excerpt:
                "29 team members · work and personal aliases matched before drafting customer recaps."
                    .into(),
            source: "Google Sheets".into(),
            kind: OutputKind::GoogleSheet,
        },
        SessionOutput {
            id: "out-offer".into(),
            name: "Offer template.docx".into(),
            preview_title: "Enterprise Offer Template".into(),
            preview_excerpt:
                "Standard Q3 commercial summary, rollout milestones, and mutual next-step checklist."
                    .into(),
            source: "Word".into(),
            kind: OutputKind::WordDoc,
        },
        SessionOutput {
            id: "out-notes".into(),
            name: "call-notes-summary.md".into(),
            preview_title: "Call Notes Summary".into(),
            preview_excerpt:
                "Twelve recent discovery and demo calls distilled into recurring follow-up patterns."
                    .into(),
            source: "Markdown".into(),
            kind: OutputKind::Markdown,
        },
    ]
}

fn render_output_glyph(kind: OutputKind, side: f32, p: &ProPalette) -> AnyElement {
    match kind {
        OutputKind::GoogleDoc => pro_svg("0 0 24 24", GDOC_SVG, side, rgb(0x4285f4).into()),
        OutputKind::GoogleSheet => pro_svg("0 0 24 24", GSHEET_SVG, side, rgb(0x0f9d58).into()),
        OutputKind::WordDoc => pro_svg("0 0 24 24", WORD_SVG, side, rgb(0x2b579a).into()),
        OutputKind::Markdown => pro_svg("0 0 24 24", MARKDOWN_SVG, side, p.ink),
    }
}

/// AI chat screen pairing a tool-assisted thread with a live progress and outputs panel.
pub struct AgentChatProgressScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    _shell_sub: Option<Subscription>,
    shell_dirty: bool,
    standalone: bool,
    workspace: SharedString,
    user_name: SharedString,
    user_email: SharedString,
    loading: bool,
    empty: bool,
    focus: FocusHandle,
    thread_scroll: ScrollHandle,
    composer_editor: Option<Entity<Editor>>,
    turns: Vec<ChatTurnPart>,
    expanded_tools: Vec<SharedString>,
    focused_tool_idx: usize,
    kb_nav_active: bool,
    copied_key: Option<(SharedString, Instant)>,
    steps: Vec<ProgressStep>,
    show_earlier_steps: bool,
    panel_open: bool,
    sec_progress_open: bool,
    sec_outputs_open: bool,
    sec_used_open: bool,
    outputs: Vec<SessionOutput>,
    featured_output: usize,
    approval_policy: ApprovalPolicy,
    selected_model: SharedString,
    open_popover: Option<ComposerPopover>,
    last_popover: Cell<ComposerPopover>,
    voice_recording: bool,
    running: bool,
    paused: bool,
    sim_started: Instant,
    sim_accum_ms: f32,
    measured_width: Rc<Cell<f32>>,
}

impl Focusable for AgentChatProgressScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl AgentChatProgressScreen {
    /// Construct a new [`AgentChatProgressScreen`] with default `Acme` conversation and progress data.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let eid = ElementId::Name(id_str.clone());
        let workspace: SharedString = "Acme".into();
        let user_name: SharedString = "Alex Morgan".into();
        let user_email: SharedString = "alex@acme.ai".into();
        let this = cx.entity();
        let shell = Self::build_shell(
            &id_str,
            &this,
            &workspace,
            &user_name,
            &user_email,
            false,
            cx,
        );
        let sub = cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| {
            if let AppShellEvent::Navigate(label) = ev {
                cx.emit(AgentChatProgressEvent::Navigate(label.clone()));
            }
        });

        Self {
            id: eid,
            shell,
            _shell_sub: Some(sub),
            shell_dirty: false,
            standalone: false,
            workspace,
            user_name,
            user_email,
            loading: false,
            empty: false,
            focus: cx.focus_handle(),
            thread_scroll: ScrollHandle::new(),
            composer_editor: None,
            turns: default_turns(),
            expanded_tools: Vec::new(),
            focused_tool_idx: 0,
            kb_nav_active: false,
            copied_key: None,
            steps: default_steps(),
            show_earlier_steps: false,
            panel_open: true,
            sec_progress_open: true,
            sec_outputs_open: true,
            sec_used_open: true,
            outputs: default_outputs(),
            featured_output: 0,
            approval_policy: ApprovalPolicy::RequestApproval,
            selected_model: "Auto".into(),
            open_popover: None,
            last_popover: Cell::new(ComposerPopover::Model),
            voice_recording: false,
            running: false,
            paused: false,
            sim_started: Instant::now(),
            sim_accum_ms: 0.,
            measured_width: Rc::new(Cell::new(960.)),
        }
    }

    fn sidebar_groups(empty: bool, cx: &App) -> Vec<ShellNavGroup> {
        let primary = ShellNavGroup::primary(vec![
            ShellNavItem::new("new-chat", tr(cx, "New chat", "新建对话"), ProIcon::NewChat),
            ShellNavItem::new("folders", tr(cx, "Folders", "文件夹"), ProIcon::Folder),
            ShellNavItem::new(
                "integrations",
                tr(cx, "Integrations", "集成"),
                ProIcon::Grid,
            ),
        ]);
        if empty {
            return vec![primary];
        }
        vec![
            primary,
            ShellNavGroup::named(
                tr(cx, "Pinned", "已置顶"),
                vec![ShellNavItem::new(
                    "q3-launch-msg",
                    "Q3 launch messaging",
                    ProIcon::Pin,
                )],
            ),
            ShellNavGroup::named(
                tr(cx, "Projects", "项目"),
                vec![
                    ShellNavItem::new("q3-campaign", "Q3 Campaign", ProIcon::ProjectFolder),
                    ShellNavItem::new(
                        "follow-up-sales",
                        "Follow-up after sales calls",
                        ProIcon::Thread,
                    ),
                    ShellNavItem::new(
                        "ad-hooks-linen",
                        "Ad hooks for the linen overshirt",
                        ProIcon::Thread,
                    )
                    .with_unread(true),
                    ShellNavItem::new(
                        "q3-budget-split",
                        "Q3 budget split by channel",
                        ProIcon::Thread,
                    ),
                    ShellNavItem::new("brand-assets", "Brand assets", ProIcon::Folder),
                    ShellNavItem::new("hiring", "Hiring", ProIcon::Folder),
                    ShellNavItem::new("research", "Research", ProIcon::Folder),
                ],
            ),
            ShellNavGroup::named(
                tr(cx, "Chats", "对话"),
                vec![
                    ShellNavItem::new(
                        "onboarding-email",
                        "Onboarding email draft",
                        ProIcon::Thread,
                    ),
                    ShellNavItem::new(
                        "august-calendar",
                        "August content calendar",
                        ProIcon::Thread,
                    ),
                ],
            ),
        ]
    }

    fn build_shell(
        id_str: &str,
        host: &Entity<Self>,
        workspace: &SharedString,
        user_name: &SharedString,
        user_email: &SharedString,
        empty: bool,
        cx: &mut Context<Self>,
    ) -> Entity<AppShell> {
        let ws = workspace.clone();
        let un = user_name.clone();
        let ue = user_email.clone();
        cx.new(|shell_cx| {
            let groups = Self::sidebar_groups(empty, shell_cx);
            let active = if empty {
                "New chat"
            } else {
                "Follow-up after sales calls"
            };
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_workspace(ws)
                .with_user(un, ue)
                .with_groups(groups)
                .with_active(active)
                .with_side_cards(false)
                .with_work_area(false, false);
            s.bind_page(host, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s
        })
    }

    fn sync_shell_if_dirty(&mut self, cx: &mut Context<Self>) {
        if !self.shell_dirty {
            return;
        }
        self.shell_dirty = false;
        let this = cx.entity();
        let id_str = self.id.to_string();
        let shell = Self::build_shell(
            &id_str,
            &this,
            &self.workspace,
            &self.user_name,
            &self.user_email,
            self.empty && !self.loading,
            cx,
        );
        self._shell_sub = Some(cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| {
            if let AppShellEvent::Navigate(label) = ev {
                cx.emit(AgentChatProgressEvent::Navigate(label.clone()));
            }
        }));
        self.shell = shell;
    }

    /// Render without the surrounding [`AppShell`] sidebar frame.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Override the workspace title shown in the [`AppShell`] sidebar.
    pub fn with_workspace(mut self, workspace: impl Into<SharedString>) -> Self {
        self.workspace = workspace.into();
        self.shell_dirty = true;
        self
    }

    /// Override the user account name and email in the sidebar footer.
    pub fn with_user(
        mut self,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
    ) -> Self {
        self.user_name = name.into();
        self.user_email = email.into();
        self.shell_dirty = true;
        self
    }

    /// Show skeleton placeholders while the conversation and session progress load.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self.shell_dirty = true;
        self
    }

    /// Show the empty-thread state with no prior chats, outputs, or used context.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self.shell_dirty = true;
        self
    }

    /// Replace the initial step trail.
    pub fn with_steps(mut self, steps: Vec<ProgressStep>) -> Self {
        self.steps = steps;
        self
    }

    /// Update the loading skeleton state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            self.shell_dirty = true;
            cx.notify();
        }
    }

    /// Update the empty conversation state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            self.shell_dirty = true;
            cx.notify();
        }
    }

    /// Return `(completed_steps, total_steps)` across the current progress trail.
    pub fn step_counts(&self) -> (usize, usize) {
        let done = self
            .steps
            .iter()
            .filter(|s| s.state == ProgressStepState::Done)
            .count();
        (done, self.steps.len())
    }

    /// Fraction (`0.0..=1.0`) of steps currently marked [`ProgressStepState::Done`].
    pub fn progress_ratio(&self) -> f32 {
        let (done, total) = self.step_counts();
        if total == 0 {
            0.
        } else {
            done as f32 / total as f32
        }
    }

    /// Toggle whether collapsed earlier steps are shown in the progress trail.
    pub fn toggle_earlier_steps(&mut self) {
        self.show_earlier_steps = !self.show_earlier_steps;
    }

    /// Toggle the expanded arguments/result trace of a tool call card by `id`.
    pub fn toggle_tool_trace(&mut self, id: &str) {
        if let Some(pos) = self.expanded_tools.iter().position(|x| x.as_ref() == id) {
            self.expanded_tools.remove(pos);
        } else {
            self.expanded_tools.push(id.to_owned().into());
        }
    }

    /// Return whether a tool call trace card is currently expanded.
    pub fn is_tool_expanded(&self, id: &str) -> bool {
        self.expanded_tools.iter().any(|x| x.as_ref() == id)
    }

    /// Start or replay live step-by-step progression from the first non-earlier step.
    pub fn replay_progress(&mut self, cx: &mut Context<Self>) {
        if self.loading || self.empty {
            return;
        }
        for step in &mut self.steps {
            if step.earlier {
                step.state = ProgressStepState::Done;
            } else {
                step.state = ProgressStepState::Pending;
            }
        }
        if let Some(first_vis) = self.steps.iter_mut().find(|s| !s.earlier) {
            first_vis.state = ProgressStepState::Running;
        }
        self.running = true;
        self.paused = false;
        self.sim_accum_ms = 0.;
        self.sim_started = Instant::now();
        cx.emit(AgentChatProgressEvent::RunStateChanged {
            running: true,
            paused: false,
        });
        cx.notify();
    }

    /// Pause or stop an active live progress turn.
    pub fn stop_progress(&mut self, cx: &mut Context<Self>) {
        if !self.running {
            return;
        }
        self.sim_accum_ms += self.sim_started.elapsed().as_secs_f32() * 1000.;
        self.running = false;
        self.paused = true;
        cx.emit(AgentChatProgressEvent::RunStateChanged {
            running: false,
            paused: true,
        });
        cx.notify();
    }

    /// Resume a paused progress turn.
    pub fn resume_progress(&mut self, cx: &mut Context<Self>) {
        if self.loading || self.empty || !self.paused {
            return;
        }
        self.running = true;
        self.paused = false;
        self.sim_started = Instant::now();
        cx.emit(AgentChatProgressEvent::RunStateChanged {
            running: true,
            paused: false,
        });
        cx.notify();
    }

    fn tick_simulation(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if !self.running || self.loading || self.empty {
            return;
        }
        let elapsed_ms = self.sim_accum_ms + self.sim_started.elapsed().as_secs_f32() * 1000.;
        let non_earlier: Vec<usize> = self
            .steps
            .iter()
            .enumerate()
            .filter(|(_, s)| !s.earlier)
            .map(|(i, _)| i)
            .collect();
        if non_earlier.is_empty() {
            self.running = false;
            return;
        }
        let stage = (elapsed_ms / 1300.).floor() as usize;
        for (pos, &idx) in non_earlier.iter().enumerate() {
            self.steps[idx].state = if pos < stage {
                ProgressStepState::Done
            } else if pos == stage {
                ProgressStepState::Running
            } else {
                ProgressStepState::Pending
            };
        }
        if stage >= non_earlier.len() {
            self.running = false;
            self.paused = false;
            for part in &mut self.turns {
                if let ChatTurnPart::ToolCall(tc) = part
                    && tc.status == ToolCallStatus::Running
                {
                    tc.status = ToolCallStatus::Success;
                    tc.duration = "1.4s".into();
                }
            }
            cx.emit(AgentChatProgressEvent::RunStateChanged {
                running: false,
                paused: false,
            });
        } else {
            w.request_animation_frame();
        }
    }

    fn send_composer_message(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.loading || self.empty {
            return;
        }
        let Some(ed) = self.composer_editor.clone() else {
            return;
        };
        let raw = ed.read(cx).value().trim().to_string();
        if raw.is_empty() {
            return;
        }
        ed.update(cx, |e, cx| e.set_value("", w, cx));
        let msg: SharedString = raw.clone().into();
        let next_idx = self.turns.len() + 1;
        let tc_id: SharedString = format!("tc-live-{next_idx}").into();
        self.turns.push(ChatTurnPart::UserBubble(msg.clone()));
        self.turns.push(ChatTurnPart::ToolCall(ChatToolCall {
            id: tc_id,
            glyph: ToolGlyph::App("slack".into()),
            name: "slack.post_draft".into(),
            summary: "Prepared updated Northwind follow-up preview".into(),
            duration: "0.9s".into(),
            status: ToolCallStatus::Running,
            args: format!(
                "{{\n  \"channel\": \"#sales-follow-ups\",\n  \"note\": {:?}\n}}",
                raw
            )
            .into(),
            result: "{\n  \"ok\": true,\n  \"preview_ts\": \"1732350912.0042\"\n}".into(),
        }));
        self.turns.push(ChatTurnPart::Paragraph(
            "Updated the Northwind follow-up draft and queued it in Slack for approval.".into(),
        ));
        for s in &mut self.steps {
            if s.state == ProgressStepState::Running {
                s.state = ProgressStepState::Done;
            }
        }
        let step_id = format!("step-live-{}", self.steps.len() + 1);
        self.steps.push(ProgressStep::new(
            step_id,
            "Queue updated Northwind draft in Slack",
            ProgressStepState::Running,
        ));
        self.running = true;
        self.paused = false;
        self.sim_accum_ms = 0.;
        self.sim_started = Instant::now();
        self.open_popover = None;
        self.thread_scroll.scroll_to_bottom();
        cx.emit(AgentChatProgressEvent::MessageSent(msg));
        cx.emit(AgentChatProgressEvent::RunStateChanged {
            running: true,
            paused: false,
        });
        cx.notify();
    }

    fn ensure_composer(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.composer_editor.is_none() {
            let ph: SharedString = tr(cx, "Reply…", "回复…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Multi, w, cx);
                e.set_placeholder(ph);
                e.set_command_submit(true);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |_, _, ev: &EditEvent, cx| {
                if matches!(ev, EditEvent::Changed | EditEvent::Submitted) {
                    cx.notify();
                }
            })
            .detach();
            cx.subscribe_in(
                &ed,
                w,
                |this: &mut Self, _, _: &EditorShortcutEvent, w, cx| {
                    this.send_composer_message(w, cx);
                },
            )
            .detach();
            self.composer_editor = Some(ed);
        }
    }

    fn render_tool_card(
        &self,
        tc: &ChatToolCall,
        kb_focused: bool,
        spin_phase: f32,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let is_open = self.is_tool_expanded(&tc.id);
        let tc_id = tc.id.clone();

        let lead_icon = match &tc.glyph {
            ToolGlyph::App(app) => pro_app_artwork(app, 15., cx)
                .unwrap_or_else(|| pro_icon(ProIcon::Bolt, 15., p.muted)),
            ToolGlyph::Icon(ic) => pro_icon(*ic, 16., p.muted),
        };

        let status_mark = match tc.status {
            ToolCallStatus::Success => div()
                .size(px(14.))
                .rounded_full()
                .bg(p.blue)
                .flex()
                .items_center()
                .justify_center()
                .child(pro_icon(ProIcon::Check, 9., rgb(0xffffff).into()))
                .into_any_element(),
            ToolCallStatus::Running => {
                pro_progress_ring(0.28 + 0.55 * spin_phase, 14., 1.6, p.blue, p.line_strong)
            }
            ToolCallStatus::Error => div()
                .size(px(14.))
                .rounded_full()
                .bg(p.red)
                .flex()
                .items_center()
                .justify_center()
                .child(pro_icon(ProIcon::Close, 9., rgb(0xffffff).into()))
                .into_any_element(),
        };

        let head = activate(
            div()
                .id(pro_eid(&self.id, format!("tclc-head-{}", tc.id)))
                .role(Role::Button)
                .aria_label(format!("{} — {}", tc.name, tc.summary))
                .tab_index(0)
                .min_h(px(44.))
                .px(px(14.))
                .flex()
                .items_center()
                .gap(px(10.))
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .child(
                    div()
                        .size(px(16.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(lead_icon),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_baseline()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex_none()
                                .font_family("monospace")
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(tc.name.clone()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(13.))
                                .text_color(p.text)
                                .child(tc.summary.clone()),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .text_color(if tc.status == ToolCallStatus::Error {
                            p.red
                        } else {
                            p.muted
                        })
                        .child(status_mark)
                        .child(tc.duration.clone()),
                )
                .child(pro_icon(
                    if is_open {
                        ProIcon::ChevronUp
                    } else {
                        ProIcon::ChevronDown
                    },
                    12.,
                    p.muted,
                )),
            cx,
            move |this, _, cx| {
                this.kb_nav_active = false;
                this.toggle_tool_trace(&tc_id);
                cx.notify();
            },
        );

        let card = div()
            .id(pro_eid(&self.id, format!("tclc-{}", tc.id)))
            .my(px(2.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(if kb_focused { p.focus } else { p.line })
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(head);

        let code_bg: Hsla = if p.dark {
            rgb(0x212123).into()
        } else {
            rgb(0xfafafa).into()
        };
        let args_key: SharedString = format!("{}-args", tc.id).into();
        let res_key: SharedString = format!("{}-res", tc.id).into();
        let args_copied = self
            .copied_key
            .as_ref()
            .is_some_and(|(k, t)| k == &args_key && t.elapsed().as_millis() < 1600);
        let res_copied = self
            .copied_key
            .as_ref()
            .is_some_and(|(k, t)| k == &res_key && t.elapsed().as_millis() < 1600);

        let args_str = tc.args.clone();
        let args_view = tc.args.clone();
        let res_str = tc.result.clone();
        let res_view = tc.result.clone();
        let ak = args_key.clone();
        let rk = res_key.clone();
        let copy_args_btn = pro_icon_btn(
            pro_eid(&self.id, format!("copy-{ak}")),
            tr(cx, "Copy arguments", "复制参数"),
            if args_copied {
                ProIcon::Check
            } else {
                ProIcon::Copy
            },
            26.,
            args_copied,
            cx,
            move |this, _, cx| {
                clipboard::write(&args_str, cx).detach();
                this.copied_key = Some((ak.clone(), Instant::now()));
                cx.notify();
            },
        );
        let copy_res_btn = pro_icon_btn(
            pro_eid(&self.id, format!("copy-{rk}")),
            tr(cx, "Copy result", "复制结果"),
            if res_copied {
                ProIcon::Check
            } else {
                ProIcon::Copy
            },
            26.,
            res_copied,
            cx,
            move |this, _, cx| {
                clipboard::write(&res_str, cx).detach();
                this.copied_key = Some((rk.clone(), Instant::now()));
                cx.notify();
            },
        );
        let args_lbl: SharedString = tr(cx, "ARGUMENTS", "调用参数").into();
        let res_lbl: SharedString = tr(cx, "RESULT", "返回结果").into();
        let line_c = p.line;
        let muted_c = p.muted;
        let text_c = p.text;

        card.child(pro_fold(
            pro_eid(&self.id, format!("tclc-fold-{}", tc.id)),
            is_open,
            250.,
            0.,
            move || {
                div()
                    .px(px(14.))
                    .pt(px(12.))
                    .pb(px(14.))
                    .border_t_1()
                    .border_color(line_c)
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .h(px(26.))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(muted_c)
                                            .child(args_lbl),
                                    )
                                    .child(copy_args_btn),
                            )
                            .child(
                                div()
                                    .p(px(10.))
                                    .rounded(px(8.))
                                    .bg(code_bg)
                                    .border_1()
                                    .border_color(line_c)
                                    .font_family("monospace")
                                    .text_size(px(12.))
                                    .line_height(px(19.))
                                    .text_color(text_c)
                                    .child(args_view),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .h(px(26.))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(muted_c)
                                            .child(res_lbl),
                                    )
                                    .child(copy_res_btn),
                            )
                            .child(
                                div()
                                    .p(px(10.))
                                    .rounded(px(8.))
                                    .bg(code_bg)
                                    .border_1()
                                    .border_color(line_c)
                                    .font_family("monospace")
                                    .text_size(px(12.))
                                    .line_height(px(19.))
                                    .text_color(text_c)
                                    .child(res_view),
                            ),
                    )
                    .into_any_element()
            },
        ))
    }

    fn render_composer(&mut self, narrow: bool, spin_phase: f32, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let has_text = self
            .composer_editor
            .as_ref()
            .is_some_and(|e| !e.read(cx).value().trim().is_empty());
        let can_send = has_text && !self.loading && !self.empty;

        let approval_label = match self.approval_policy {
            ApprovalPolicy::RequestApproval => tr(cx, "Request approval", "请求审批"),
            ApprovalPolicy::AutoSafe => tr(cx, "Auto-approve safe", "安全自动通过"),
            ApprovalPolicy::AlwaysAsk => tr(cx, "Always ask", "每次询问"),
        };

        if let Some(which) = self.open_popover {
            self.last_popover.set(which);
        }
        let which = self.open_popover.unwrap_or_else(|| self.last_popover.get());
        let pop_open = self.open_popover.is_some();
        let pop_is_model = which == ComposerPopover::Model;
        let pop_key: SharedString = format!("{which:?}").into();
        let hover_c = p.hover;
        let popover_menu: AnyElement = match which {
            ComposerPopover::Add => div()
                .w(px(236.))
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
                        .py(px(4.))
                        .text_size(px(11.5))
                        .text_color(p.faint)
                        .child(tr(cx, "Add to the chat", "添加到对话")),
                )
                .children(
                    [
                        (
                            "upload",
                            tr(cx, "Upload file or screenshot", "上传文件或截图"),
                            Some(ProIcon::Paperclip),
                            None,
                        ),
                        (
                            "notion",
                            tr(cx, "Reference Notion page", "引用 Notion 页面"),
                            None,
                            Some("notion"),
                        ),
                        (
                            "drive",
                            tr(cx, "Attach from Google Drive", "从 Google Drive 添加"),
                            None,
                            Some("google drive"),
                        ),
                        (
                            "slack",
                            tr(cx, "Share to Slack channel", "分享至 Slack 频道"),
                            None,
                            Some("slack"),
                        ),
                    ]
                    .into_iter()
                    .map(|(key, lbl, ic, app)| {
                        let insert_tag = format!("@{key} ");
                        activate(
                            div()
                                .id(pro_eid(&self.id, format!("add-opt-{key}")))
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
                                .child(if let Some(icon_kind) = ic {
                                    pro_icon(icon_kind, 14., p.muted)
                                } else if let Some(app_name) = app {
                                    pro_app_artwork(app_name, 14., cx)
                                        .unwrap_or_else(|| pro_icon(ProIcon::Link, 14., p.muted))
                                } else {
                                    pro_icon(ProIcon::Plus, 14., p.muted)
                                })
                                .child(lbl),
                            cx,
                            move |this, w, cx| {
                                if let Some(ed) = this.composer_editor.clone() {
                                    let cur = ed.read(cx).value().to_string();
                                    ed.update(cx, |e, cx| {
                                        e.set_value(format!("{cur}{insert_tag}"), w, cx)
                                    });
                                }
                                this.open_popover = None;
                                cx.notify();
                            },
                        )
                    }),
                )
                .into_any_element(),
            ComposerPopover::Approval => {
                let pol_sensor = pro_selection_sensor();
                div()
                    .w(px(248.))
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
                                pro_eid(&self.id, "ccc-pol-pill"),
                                format!("{:?}", self.approval_policy),
                                pol_sensor.clone(),
                                7.,
                                p.chip,
                                None,
                                false,
                            ))
                            .children(
                                [
                                    (
                                        ApprovalPolicy::RequestApproval,
                                        tr(cx, "Request approval", "请求审批"),
                                        tr(
                                            cx,
                                            "Ask before sending emails or editing sheets",
                                            "发送邮件或改表前询问",
                                        ),
                                    ),
                                    (
                                        ApprovalPolicy::AutoSafe,
                                        tr(cx, "Auto-approve safe", "安全自动通过"),
                                        tr(
                                            cx,
                                            "Run read-only and draft tools automatically",
                                            "只读与草稿工具自动执行",
                                        ),
                                    ),
                                    (
                                        ApprovalPolicy::AlwaysAsk,
                                        tr(cx, "Always ask", "每次询问"),
                                        tr(
                                            cx,
                                            "Confirm every external connector call",
                                            "每次外部连接器调用均需确认",
                                        ),
                                    ),
                                ]
                                .into_iter()
                                .enumerate()
                                .map(
                                    |(idx, (pol, title, desc))| {
                                        let selected = self.approval_policy == pol;
                                        activate(
                                            div()
                                                .id(pro_eid(&self.id, format!("pol-opt-{idx}")))
                                                .role(Role::ListBoxOption)
                                                .tab_index(0)
                                                .relative()
                                                .px(px(8.))
                                                .py(px(6.))
                                                .rounded(px(7.))
                                                .when(selected, |d| {
                                                    d.child(pro_mark_selected(&pol_sensor))
                                                })
                                                .when(!selected, |d| {
                                                    d.hover(move |s| s.bg(hover_c))
                                                })
                                                .flex()
                                                .items_start()
                                                .justify_between()
                                                .gap(px(8.))
                                                .cursor_pointer()
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .flex()
                                                        .flex_col()
                                                        .gap(px(1.))
                                                        .child(
                                                            div()
                                                                .text_size(px(12.5))
                                                                .font_weight(if selected {
                                                                    FontWeight::MEDIUM
                                                                } else {
                                                                    FontWeight::NORMAL
                                                                })
                                                                .text_color(p.ink)
                                                                .child(title),
                                                        )
                                                        .child(
                                                            div()
                                                                .text_size(px(11.5))
                                                                .text_color(p.muted)
                                                                .child(desc),
                                                        ),
                                                )
                                                .when(selected, |d| {
                                                    d.child(div().pt(px(2.)).child(pro_icon(
                                                        ProIcon::Check,
                                                        13.,
                                                        p.ink,
                                                    )))
                                                }),
                                            cx,
                                            move |this, _, cx| {
                                                this.approval_policy = pol;
                                                this.open_popover = None;
                                                cx.notify();
                                            },
                                        )
                                    },
                                ),
                            ),
                    )
                    .into_any_element()
            }
            ComposerPopover::Model => {
                let mdl_sensor = pro_selection_sensor();
                div()
                    .w(px(220.))
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
                                pro_eid(&self.id, "ccc-mdl-pill"),
                                self.selected_model.clone(),
                                mdl_sensor.clone(),
                                7.,
                                p.chip,
                                None,
                                false,
                            ))
                            .children(
                                [
                                    ("Auto", None),
                                    ("Claude Sonnet 4.5", Some("claude")),
                                    ("GPT-5", Some("openai")),
                                    ("Gemini 2.5 Pro", Some("gemini")),
                                ]
                                .into_iter()
                                .enumerate()
                                .map(|(mi, (m_name, brand))| {
                                    let selected = self.selected_model.as_ref() == m_name;
                                    let target: SharedString = m_name.into();
                                    activate(
                                        div()
                                            .id(pro_eid(&self.id, format!("mdl-opt-{mi}")))
                                            .role(Role::ListBoxOption)
                                            .tab_index(0)
                                            .relative()
                                            .h(px(30.))
                                            .px(px(8.))
                                            .rounded(px(7.))
                                            .when(selected, |d| {
                                                d.child(pro_mark_selected(&mdl_sensor))
                                            })
                                            .when(!selected, |d| d.hover(move |s| s.bg(hover_c)))
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap(px(8.))
                                            .text_size(px(12.5))
                                            .text_color(p.ink)
                                            .cursor_pointer()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(7.))
                                                    .child(if let Some(b) = brand {
                                                        pro_app_artwork(b, 14., cx).unwrap_or_else(
                                                            || {
                                                                pro_icon(
                                                                    ProIcon::Sparkles,
                                                                    14.,
                                                                    p.muted,
                                                                )
                                                            },
                                                        )
                                                    } else {
                                                        pro_icon(ProIcon::Sparkles, 14., p.muted)
                                                    })
                                                    .child(m_name),
                                            )
                                            .when(selected, |d| {
                                                d.child(pro_icon(ProIcon::Check, 13., p.ink))
                                            }),
                                        cx,
                                        move |this, _, cx| {
                                            this.selected_model = target.clone();
                                            this.open_popover = None;
                                            cx.notify();
                                        },
                                    )
                                }),
                            ),
                    )
                    .into_any_element()
            }
        };

        let action_btn = if self.running {
            activate(
                div()
                    .id(pro_eid(&self.id, "ccc-stop"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Stop response", "停止生成"))
                    .tab_index(0)
                    .size(px(28.))
                    .rounded(px(8.))
                    .bg(p.primary)
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(move |s| s.opacity(0.88))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .child(div().size(px(9.)).rounded(px(2.)).bg(p.on_primary)),
                cx,
                |this, _, cx| this.stop_progress(cx),
            )
            .into_any_element()
        } else {
            activate(
                div()
                    .id(pro_eid(&self.id, "ccc-send"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Send message", "发送消息"))
                    .tab_index(0)
                    .size(px(28.))
                    .rounded(px(8.))
                    .bg(if can_send { p.primary } else { p.chip })
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(can_send, |d| {
                        d.cursor_pointer().hover(move |s| s.opacity(0.9))
                    })
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .child(pro_icon(
                        ProIcon::ArrowUp,
                        15.,
                        if can_send { p.on_primary } else { p.faint },
                    )),
                cx,
                |this, w, cx| this.send_composer_message(w, cx),
            )
            .into_any_element()
        };

        div()
            .w_full()
            .px(px(if narrow { 12. } else { 24. }))
            .pb(px(if narrow { 10. } else { 20. }))
            .flex_none()
            .child(
                div()
                    .relative()
                    .w_full()
                    .max_w(px(680.))
                    .mx_auto()
                    .child({
                        let pop_vtx_id = pro_eid(&self.id, "ccc-pop-vtx");
                        pro_presence(
                            pro_eid(&self.id, "ccc-pop-pres"),
                            pop_open,
                            0.18,
                            0.12,
                            move |a| {
                                div()
                                    .absolute()
                                    .bottom(px(100. - 5. * (1. - a)))
                                    .opacity(a)
                                    .when(pop_is_model, |pos| pos.right(px(0.)))
                                    .when(!pop_is_model, |pos| pos.left(px(0.)))
                                    .child(pro_view_transition(pop_vtx_id, pop_key, popover_menu))
                                    .into_any_element()
                            },
                        )
                    })
                    .child(
                        div()
                            .rounded(px(16.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .pt(px(14.))
                            .px(px(10.))
                            .pb(px(10.))
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .px(px(4.))
                                    .min_h(px(40.))
                                    .when_some(self.composer_editor.clone(), |d, ed| d.child(ed)),
                            )
                            .child(
                                div()
                                    .mt(px(10.))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(8.))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .min_w_0()
                                            .child(pro_icon_btn(
                                                pro_eid(&self.id, "ccc-plus"),
                                                tr(cx, "Add to the chat", "添加到对话"),
                                                ProIcon::Plus,
                                                28.,
                                                self.open_popover == Some(ComposerPopover::Add),
                                                cx,
                                                |this, _, cx| {
                                                    this.open_popover = if this.open_popover
                                                        == Some(ComposerPopover::Add)
                                                    {
                                                        None
                                                    } else {
                                                        Some(ComposerPopover::Add)
                                                    };
                                                    cx.notify();
                                                },
                                            ))
                                            .child(activate(
                                                div()
                                                    .id(pro_eid(&self.id, "ccc-approval"))
                                                    .role(Role::Button)
                                                    .aria_label(approval_label)
                                                    .tab_index(0)
                                                    .h(px(28.))
                                                    .px(px(8.))
                                                    .rounded(px(8.))
                                                    .when(
                                                        self.open_popover
                                                            == Some(ComposerPopover::Approval),
                                                        |d| d.bg(p.hover),
                                                    )
                                                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                                                    .focus_visible(move |s| {
                                                        s.border_1().border_color(p.focus)
                                                    })
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(6.))
                                                    .text_size(px(13.))
                                                    .text_color(p.muted)
                                                    .cursor_pointer()
                                                    .child(pro_svg(
                                                        "0 0 68.155 68.155",
                                                        HAND_APPROVAL_SVG,
                                                        15.,
                                                        p.muted,
                                                    ))
                                                    .when(!narrow, |d| d.child(approval_label)),
                                                cx,
                                                |this, _, cx| {
                                                    this.open_popover = if this.open_popover
                                                        == Some(ComposerPopover::Approval)
                                                    {
                                                        None
                                                    } else {
                                                        Some(ComposerPopover::Approval)
                                                    };
                                                    cx.notify();
                                                },
                                            )),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .flex_none()
                                            .child(activate(
                                                div()
                                                    .id(pro_eid(&self.id, "ccc-model"))
                                                    .role(Role::Button)
                                                    .aria_label(tr(cx, "Select a model", "选择模型"))
                                                    .tab_index(0)
                                                    .h(px(28.))
                                                    .pl(px(8.))
                                                    .pr(px(6.))
                                                    .rounded(px(8.))
                                                    .when(
                                                        self.open_popover
                                                            == Some(ComposerPopover::Model),
                                                        |d| d.bg(p.hover),
                                                    )
                                                    .hover(move |s| s.bg(p.hover))
                                                    .focus_visible(move |s| {
                                                        s.border_1().border_color(p.focus)
                                                    })
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(4.))
                                                    .text_size(px(13.))
                                                    .text_color(p.ink)
                                                    .cursor_pointer()
                                                    .child(self.selected_model.clone())
                                                    .child(pro_icon(
                                                        ProIcon::ChevronDown,
                                                        13.,
                                                        p.muted,
                                                    )),
                                                cx,
                                                |this, _, cx| {
                                                    this.open_popover = if this.open_popover
                                                        == Some(ComposerPopover::Model)
                                                    {
                                                        None
                                                    } else {
                                                        Some(ComposerPopover::Model)
                                                    };
                                                    cx.notify();
                                                },
                                            ))
                                            .child({
                                                let voice_ind = div()
                                                    .mr(px(4.))
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(3.))
                                                    .children((0..5usize).map(|bi| {
                                                        let wave = ((spin_phase
                                                            * std::f32::consts::TAU
                                                            + bi as f32 * 0.9)
                                                            .sin()
                                                            .abs()
                                                            * 9.
                                                            + 4.)
                                                            .round();
                                                        div()
                                                            .w(px(2.))
                                                            .h(px(wave))
                                                            .rounded(px(1.))
                                                            .bg(p.blue)
                                                    }))
                                                    .when(!narrow, |r| {
                                                        r.child(
                                                            div()
                                                                .ml(px(3.))
                                                                .text_size(px(11.5))
                                                                .text_color(p.blue)
                                                                .child(tr(
                                                                    cx,
                                                                    "Listening…",
                                                                    "正在聆听…",
                                                                )),
                                                        )
                                                    });
                                                pro_presence(
                                                    pro_eid(&self.id, "ccc-voice-pres"),
                                                    self.voice_recording,
                                                    0.16,
                                                    0.12,
                                                    move |a| {
                                                        voice_ind.opacity(a).into_any_element()
                                                    },
                                                )
                                            })
                                            .child(pro_icon_btn(
                                                pro_eid(&self.id, "ccc-mic"),
                                                tr(cx, "Voice dictation", "语音输入"),
                                                ProIcon::Mic,
                                                28.,
                                                self.voice_recording,
                                                cx,
                                                |this, w, cx| {
                                                    this.voice_recording = !this.voice_recording;
                                                    if !this.voice_recording
                                                        && let Some(ed) =
                                                            this.composer_editor.clone()
                                                        && ed.read(cx).value().is_empty()
                                                    {
                                                        ed.update(cx, |e, cx| {
                                                            e.set_value(
                                                                "Send the updated Northwind follow-up test to Slack.",
                                                                w,
                                                                cx,
                                                            )
                                                        });
                                                    }
                                                    cx.notify();
                                                },
                                            ))
                                            .child(action_btn),
                                    ),
                            ),
                    ),
            )
    }

    fn render_side_panel(
        &self,
        skel_phase: f32,
        spin_phase: f32,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let is_loading = self.loading;
        let is_empty = !is_loading && self.empty;

        let earlier_steps: Vec<&ProgressStep> = self.steps.iter().filter(|s| s.earlier).collect();
        let recent_steps: Vec<&ProgressStep> = self.steps.iter().filter(|s| !s.earlier).collect();
        let earlier_count = earlier_steps.len();

        let render_step_row = |step: &ProgressStep, is_last: bool| {
            let mark = match step.state {
                ProgressStepState::Done => div()
                    .size(px(16.))
                    .rounded_full()
                    .bg(p.muted)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_icon(ProIcon::Check, 10., p.panel))
                    .into_any_element(),
                ProgressStepState::Running => {
                    pro_progress_ring(0.25 + 0.60 * spin_phase, 16., 1.8, p.blue, p.line_strong)
                }
                ProgressStepState::Pending => div()
                    .size(px(16.))
                    .rounded_full()
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line_strong)
                    .into_any_element(),
            };

            div()
                .relative()
                .pl(px(26.))
                .pb(px(if is_last { 0. } else { 9. }))
                .when(!is_last, |d| {
                    d.child(
                        div()
                            .absolute()
                            .left(px(7.))
                            .top(px(17.))
                            .bottom(px(0.))
                            .w(px(2.))
                            .rounded(px(1.))
                            .bg(p.line_strong),
                    )
                })
                .child(div().absolute().top(px(1.)).left(px(0.)).child(mark))
                .child(
                    div()
                        .text_size(px(12.5))
                        .line_height(px(18.))
                        .text_color(if step.state == ProgressStepState::Running {
                            p.ink
                        } else {
                            p.muted
                        })
                        .child(step.label.clone()),
                )
        };

        // Section 1: Progress
        let progress_head = div().pr(px(56.)).flex().items_center().child(activate(
            div()
                .id(pro_eid(&self.id, "sec-progress-btn"))
                .role(Role::Button)
                .aria_label(tr(cx, "Progress", "进度"))
                .tab_index(0)
                .h(px(26.))
                .px(px(6.))
                .ml(px(-6.))
                .rounded(px(7.))
                .hover(move |s| s.bg(p.hover))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .flex()
                .items_center()
                .gap(px(5.))
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink)
                .cursor_pointer()
                .child(tr(cx, "Progress", "进度"))
                .child(pro_icon(
                    if self.sec_progress_open {
                        ProIcon::ChevronDown
                    } else {
                        ProIcon::ChevronRight
                    },
                    12.,
                    p.muted,
                )),
            cx,
            |this, _, cx| {
                this.sec_progress_open = !this.sec_progress_open;
                cx.notify();
            },
        ));

        let progress_body = if is_loading {
            let mut skel_steps = div().flex().flex_col().gap(px(10.)).pt(px(10.));
            for w_bar in [104., 162., 148.] {
                skel_steps = skel_steps.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(16., 16., 99., skel_phase, cx))
                        .child(pro_skeleton_bar(w_bar, 12., 4., skel_phase, cx)),
                );
            }
            skel_steps.into_any_element()
        } else if is_empty || self.steps.is_empty() {
            div()
                .pt(px(8.))
                .text_size(px(12.5))
                .line_height(px(18.))
                .text_color(p.muted)
                .child(tr(cx, "No steps yet", "暂无步骤"))
                .into_any_element()
        } else {
            let mut trail = div()
                .pt(px(6.))
                .flex()
                .flex_col()
                .child(div().pb(px(10.)).child(pro_spring_bar(
                    pro_eid(&self.id, "sec-progress-bar"),
                    self.progress_ratio(),
                    3.,
                    1.5,
                    p.chip,
                    if self.paused { p.amber } else { p.blue },
                )));

            if earlier_count > 0 {
                let earlier_lbl = if self.show_earlier_steps {
                    tr(cx, "Hide earlier steps", "收起早期步骤").to_string()
                } else {
                    format!("{earlier_count} {}", tr(cx, "earlier steps", "个早期步骤"))
                };
                trail = trail.child(
                    div()
                        .relative()
                        .pl(px(26.))
                        .pb(px(9.))
                        .child(
                            div()
                                .absolute()
                                .left(px(7.))
                                .top(px(3.))
                                .bottom(px(0.))
                                .w(px(2.))
                                .flex()
                                .flex_col()
                                .justify_between()
                                .children((0..3).map(|_| {
                                    div().w(px(2.)).h(px(3.)).rounded(px(1.)).bg(p.line_strong)
                                })),
                        )
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, "earlier-steps-btn"))
                                .role(Role::Button)
                                .aria_label(earlier_lbl.clone())
                                .tab_index(0)
                                .ml(px(-6.))
                                .px(px(6.))
                                .py(px(2.))
                                .rounded(px(6.))
                                .hover(move |s| s.bg(p.hover).text_color(p.ink))
                                .focus_visible(move |s| s.border_1().border_color(p.focus))
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .text_size(px(12.5))
                                .line_height(px(18.))
                                .text_color(p.muted)
                                .cursor_pointer()
                                .child(earlier_lbl),
                            cx,
                            |this, _, cx| {
                                this.toggle_earlier_steps();
                                cx.notify();
                            },
                        )),
                );

                let has_recent = !recent_steps.is_empty();
                let mut earlier_col = div().flex().flex_col();
                for (si, step) in earlier_steps.iter().enumerate() {
                    let is_last = !has_recent && si + 1 == earlier_count;
                    earlier_col = earlier_col.child(render_step_row(step, is_last));
                }
                trail = trail.child(pro_fold(
                    pro_eid(&self.id, "earlier-steps-fold"),
                    self.show_earlier_steps,
                    earlier_count as f32 * 27.,
                    0.,
                    move || earlier_col.into_any_element(),
                ));
            }

            let recent_len = recent_steps.len();
            for (si, step) in recent_steps.into_iter().enumerate() {
                let is_last = si + 1 == recent_len;
                trail = trail.child(render_step_row(step, is_last));
            }
            trail.into_any_element()
        };

        let progress_sec = div()
            .py(px(12.))
            .flex()
            .flex_col()
            .child(progress_head)
            .child(pro_fold(
                pro_eid(&self.id, "sec-progress-fold"),
                self.sec_progress_open,
                148.,
                0.,
                move || progress_body,
            ));

        // Section 2: Outputs (omitted when empty)
        let outputs_sec = if !is_empty && (is_loading || !self.outputs.is_empty()) {
            let out_head = div().flex().items_center().child(activate(
                div()
                    .id(pro_eid(&self.id, "sec-outputs-btn"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Outputs", "产出文件"))
                    .tab_index(0)
                    .h(px(26.))
                    .px(px(6.))
                    .ml(px(-6.))
                    .rounded(px(7.))
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .cursor_pointer()
                    .child(tr(cx, "Outputs", "产出文件"))
                    .child(pro_icon(
                        if self.sec_outputs_open {
                            ProIcon::ChevronDown
                        } else {
                            ProIcon::ChevronRight
                        },
                        12.,
                        p.muted,
                    )),
                cx,
                |this, _, cx| {
                    this.sec_outputs_open = !this.sec_outputs_open;
                    cx.notify();
                },
            ));

            let out_body = if is_loading {
                div()
                    .pt(px(6.))
                    .flex()
                    .flex_col()
                    .child(pro_skeleton_bar(266., 128., 10., skel_phase, cx).mb(px(8.)))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(9.))
                            .mb(px(10.))
                            .child(pro_skeleton_bar(18., 18., 4., skel_phase, cx))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.))
                                    .child(pro_skeleton_bar(148., 12., 4., skel_phase, cx))
                                    .child(pro_skeleton_bar(72., 10., 4., skel_phase, cx)),
                            ),
                    )
                    .children([120., 134., 142.].into_iter().map(|w_row| {
                        div()
                            .h(px(30.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(9.))
                                    .child(pro_skeleton_bar(16., 16., 4., skel_phase, cx))
                                    .child(pro_skeleton_bar(w_row, 11., 4., skel_phase, cx)),
                            )
                            .child(pro_skeleton_bar(52., 10., 4., skel_phase, cx))
                    }))
                    .into_any_element()
            } else {
                let feat_idx = self
                    .featured_output
                    .min(self.outputs.len().saturating_sub(1));
                let feat = &self.outputs[feat_idx];
                let feat_id = feat.id.clone();
                let feat_vtx_key = feat.id.clone();

                let preview_btn = activate(
                    div()
                        .id(pro_eid(&self.id, "featured-output"))
                        .role(Role::Button)
                        .aria_label(feat.name.clone())
                        .tab_index(0)
                        .pt(px(2.))
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .cursor_pointer()
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .child(
                            div()
                                .h(px(132.))
                                .px(px(18.))
                                .py(px(16.))
                                .rounded(px(10.))
                                .bg(p.panel)
                                .border_1()
                                .border_color(p.line)
                                .hover(move |s| s.border_color(p.line_strong))
                                .overflow_hidden()
                                .flex()
                                .flex_col()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .text_size(px(15.))
                                        .line_height(px(20.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(feat.preview_title.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.5))
                                        .line_height(px(15.))
                                        .text_color(p.text)
                                        .child(feat.preview_excerpt.clone()),
                                )
                                .child(
                                    div()
                                        .pt(px(4.))
                                        .flex()
                                        .flex_col()
                                        .gap(px(6.))
                                        .child(div().w_full().h(px(5.)).rounded(px(3.)).bg(p.chip))
                                        .child(
                                            div()
                                                .w(relative(0.82))
                                                .h(px(5.))
                                                .rounded(px(3.))
                                                .bg(p.chip),
                                        )
                                        .child(
                                            div()
                                                .w(relative(0.64))
                                                .h(px(5.))
                                                .rounded(px(3.))
                                                .bg(p.chip),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(9.))
                                .min_w_0()
                                .child(render_output_glyph(feat.kind, 18., &p))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(px(12.5))
                                                .line_height(px(18.))
                                                .text_color(p.ink)
                                                .truncate()
                                                .child(middle_truncate(&feat.name, 28)),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.5))
                                                .line_height(px(16.))
                                                .text_color(p.muted)
                                                .child(feat.source.clone()),
                                        ),
                                ),
                        ),
                    cx,
                    move |_, _, cx| {
                        cx.emit(AgentChatProgressEvent::OutputOpened(feat_id.clone()));
                        cx.emit(AgentChatProgressEvent::Navigate("Folders".into()));
                    },
                );

                let mut list = div().mt(px(8.)).mx(px(-4.)).flex().flex_col().gap(px(1.));
                for (oi, out) in self.outputs.iter().enumerate() {
                    if oi == feat_idx {
                        continue;
                    }
                    let out_id = out.id.clone();
                    list = list.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("out-row-{oi}")))
                            .role(Role::Button)
                            .aria_label(format!("{} ({})", out.name, out.source))
                            .tab_index(0)
                            .h(px(32.))
                            .px(px(4.))
                            .rounded(px(7.))
                            .hover(move |s| s.bg(p.hover))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .gap(px(9.))
                            .cursor_pointer()
                            .child(
                                div()
                                    .size(px(18.))
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(render_output_glyph(out.kind, 17., &p)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(12.5))
                                    .line_height(px(18.))
                                    .text_color(p.text)
                                    .truncate()
                                    .child(middle_truncate(&out.name, 24)),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .text_size(px(11.5))
                                    .line_height(px(16.))
                                    .text_color(p.muted)
                                    .child(out.source.clone()),
                            ),
                        cx,
                        move |this, _, cx| {
                            this.featured_output = oi;
                            cx.emit(AgentChatProgressEvent::OutputOpened(out_id.clone()));
                            cx.notify();
                        },
                    ));
                }

                div()
                    .pt(px(6.))
                    .flex()
                    .flex_col()
                    .child(pro_view_transition(
                        pro_eid(&self.id, "feat-out-vtx"),
                        feat_vtx_key,
                        preview_btn,
                    ))
                    .child(list)
                    .into_any_element()
            };

            Some(
                div()
                    .py(px(12.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .child(out_head)
                    .child(pro_fold(
                        pro_eid(&self.id, "sec-outputs-fold"),
                        self.sec_outputs_open,
                        250.,
                        0.,
                        move || out_body,
                    )),
            )
        } else {
            None
        };

        // Section 3: Used in this session (omitted when empty)
        let used_sec = if !is_empty {
            let used_head = div().flex().items_center().child(activate(
                div()
                    .id(pro_eid(&self.id, "sec-used-btn"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Used in this session", "本次会话使用"))
                    .tab_index(0)
                    .h(px(26.))
                    .px(px(6.))
                    .ml(px(-6.))
                    .rounded(px(7.))
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .cursor_pointer()
                    .child(tr(cx, "Used in this session", "本次会话使用"))
                    .child(pro_icon(
                        if self.sec_used_open {
                            ProIcon::ChevronDown
                        } else {
                            ProIcon::ChevronRight
                        },
                        12.,
                        p.muted,
                    )),
                cx,
                |this, _, cx| {
                    this.sec_used_open = !this.sec_used_open;
                    cx.notify();
                },
            ));

            let used_body = if is_loading {
                div()
                    .pt(px(6.))
                    .flex()
                    .flex_col()
                    .children([110., 84., 72., 94.].into_iter().map(|w_meta| {
                        div()
                            .h(px(30.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(9.))
                                    .child(pro_skeleton_bar(16., 16., 4., skel_phase, cx))
                                    .child(pro_skeleton_bar(76., 11., 4., skel_phase, cx)),
                            )
                            .child(pro_skeleton_bar(w_meta, 11., 4., skel_phase, cx))
                    }))
                    .into_any_element()
            } else {
                let connectors_logos = div().flex().items_center().gap(px(6.)).children(
                    ["notion", "google drive", "slack"]
                        .into_iter()
                        .filter_map(|app| pro_app_artwork(app, 14., cx)),
                );

                div()
                    .pt(px(6.))
                    .mt(px(4.))
                    .mx(px(-4.))
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(
                        div()
                            .h(px(32.))
                            .px(px(4.))
                            .flex()
                            .items_center()
                            .gap(px(9.))
                            .child(pro_icon(ProIcon::Paperclip, 15., p.muted))
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(12.5))
                                    .text_color(p.text)
                                    .child(tr(cx, "Uploads", "上传附件")),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .child(middle_truncate("call-screenshot.png +1", 24)),
                            ),
                    )
                    .child(
                        div()
                            .h(px(32.))
                            .px(px(4.))
                            .flex()
                            .items_center()
                            .gap(px(9.))
                            .child(pro_svg("0 0 24 24", PLUG_CONNECTOR_SVG, 15., p.muted))
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(12.5))
                                    .text_color(p.text)
                                    .child(tr(cx, "Connectors", "连接器")),
                            )
                            .child(connectors_logos),
                    )
                    .child(
                        div()
                            .h(px(32.))
                            .px(px(4.))
                            .flex()
                            .items_center()
                            .gap(px(9.))
                            .child(pro_icon(ProIcon::Globe, 15., p.muted))
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(12.5))
                                    .text_color(p.text)
                                    .child(tr(cx, "Web search", "网页搜索")),
                            )
                            .child(div().text_size(px(11.5)).text_color(p.muted).child(tr(
                                cx,
                                "2 pages",
                                "2 个网页",
                            ))),
                    )
                    .child(
                        div()
                            .h(px(32.))
                            .px(px(4.))
                            .flex()
                            .items_center()
                            .gap(px(9.))
                            .child(pro_icon(ProIcon::Sparkles, 15., p.muted))
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(12.5))
                                    .text_color(p.text)
                                    .child(tr(cx, "Skills", "技能")),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .child("sales-follow-up"),
                            ),
                    )
                    .into_any_element()
            };

            Some(
                div()
                    .py(px(12.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .child(used_head)
                    .child(pro_fold(
                        pro_eid(&self.id, "sec-used-fold"),
                        self.sec_used_open,
                        140.,
                        0.,
                        move || used_body,
                    )),
            )
        } else {
            None
        };

        let run_ctrl_icon = if self.running {
            ProIcon::Pause
        } else if self.paused {
            ProIcon::Play
        } else {
            ProIcon::Refresh
        };
        let run_ctrl_aria = if self.running {
            tr(cx, "Pause progress", "暂停进度")
        } else if self.paused {
            tr(cx, "Resume progress", "继续进度")
        } else {
            tr(cx, "Replay progress", "重播进度")
        };

        div()
            .id(pro_eid(&self.id, "prg-side-scroll"))
            .role(Role::Region)
            .aria_label(tr(cx, "Progress", "进度面板"))
            .relative()
            .w(px(300.))
            .h_full()
            .flex_none()
            .border_l_1()
            .border_color(p.line)
            .bg(p.panel)
            .overflow_y_scroll()
            .px(px(16.))
            .pb(px(20.))
            .child(
                div()
                    .absolute()
                    .top(px(12.))
                    .right(px(12.))
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .when(!is_loading && !is_empty, |d| {
                        d.child(pro_icon_btn(
                            pro_eid(&self.id, "prg-ctrl-btn"),
                            run_ctrl_aria,
                            run_ctrl_icon,
                            26.,
                            self.running || self.paused,
                            cx,
                            |this, _, cx| {
                                if this.running {
                                    this.stop_progress(cx);
                                } else if this.paused {
                                    this.resume_progress(cx);
                                } else {
                                    this.replay_progress(cx);
                                }
                            },
                        ))
                    })
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "prg-close-btn"),
                        tr(cx, "Hide progress", "隐藏进度"),
                        ProIcon::Close,
                        26.,
                        false,
                        cx,
                        |this, _, cx| {
                            this.panel_open = false;
                            cx.notify();
                        },
                    )),
            )
            .child(progress_sec)
            .when_some(outputs_sec, |d, s| d.child(s))
            .when_some(used_sec, |d, s| d.child(s))
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_composer(w, cx);
        self.tick_simulation(w, cx);

        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        if (self.loading || self.voice_recording) && !reduced {
            w.request_animation_frame();
        }
        let now_s = self.sim_started.elapsed().as_secs_f32();
        let skel_phase = if reduced { 0.3 } else { (now_s * 0.85).fract() };
        let spin_phase = if reduced { 0.5 } else { (now_s * 1.25).fract() };

        let measure_cell = self.measured_width.clone();
        let container_w = measure_cell.get();
        let narrow = container_w < 720.;

        let is_loading = self.loading;
        let is_empty = !is_loading && self.empty;

        let out_count = if is_empty { 0 } else { self.outputs.len() };
        let ratio = self.progress_ratio();

        // Top header bar above chat
        let bar_run_el = {
            let status_el = div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(pro_progress_ring(
                    if self.running {
                        0.3 + 0.5 * spin_phase
                    } else {
                        ratio
                    },
                    14.,
                    1.8,
                    if self.paused { p.amber } else { p.blue },
                    p.line_strong,
                ))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(if self.paused {
                            tr(cx, "Paused", "已暂停")
                        } else {
                            tr(cx, "Working…", "执行中…")
                        }),
                )
                .child(pro_btn(
                    pro_eid(&self.id, "bar-run-toggle"),
                    if self.paused {
                        tr(cx, "Resume", "继续")
                    } else {
                        tr(cx, "Stop", "停止")
                    },
                    Some(if self.paused {
                        ProIcon::Play
                    } else {
                        ProIcon::Pause
                    }),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    |this, _, cx| {
                        if this.paused {
                            this.resume_progress(cx);
                        } else {
                            this.stop_progress(cx);
                        }
                    },
                ));
            pro_presence(
                pro_eid(&self.id, "bar-run-pres"),
                self.running || self.paused,
                0.18,
                0.14,
                move |a| {
                    status_el
                        .opacity(a)
                        .mt(px(-3. * (1. - a)))
                        .into_any_element()
                },
            )
        };

        let top_bar =
            div()
                .h(px(48.))
                .pl(px(if narrow { 48. } else { 20. }))
                .pr(px(12.))
                .flex_none()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .child(div().flex().items_center().child(bar_run_el))
                .child(div().flex().items_center().gap(px(8.)).when(
                    !self.panel_open || narrow,
                    |d| {
                        let aria = if out_count > 0 {
                            format!("Progress ({out_count} outputs)")
                        } else {
                            "Progress".to_string()
                        };
                        d.child(activate(
                            div()
                                .id(pro_eid(&self.id, "prg-open-btn"))
                                .role(Role::Button)
                                .aria_label(aria)
                                .tab_index(0)
                                .h(px(28.))
                                .px(px(8.))
                                .rounded(px(8.))
                                .hover(move |s| s.bg(p.hover).text_color(p.ink))
                                .focus_visible(move |s| s.border_1().border_color(p.focus))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.text)
                                .cursor_pointer()
                                .child(pro_progress_ring(ratio, 13., 1.6, p.blue, p.line_strong))
                                .child(tr(cx, "Progress", "进度"))
                                .when(out_count > 0, |b| {
                                    b.child(
                                        div()
                                            .px(px(5.))
                                            .h(px(18.))
                                            .rounded_full()
                                            .bg(p.chip)
                                            .flex()
                                            .items_center()
                                            .text_size(px(11.))
                                            .text_color(p.muted)
                                            .child(out_count.to_string()),
                                    )
                                }),
                            cx,
                            |this, _, cx| {
                                this.panel_open = !this.panel_open;
                                cx.notify();
                            },
                        ))
                    },
                ));

        // Main thread scroll area
        let thread_body = if is_loading {
            div()
                .max_w(px(680.))
                .w_full()
                .mx_auto()
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(
                    div()
                        .ml_auto()
                        .px(px(14.))
                        .py(px(11.))
                        .rounded(px(16.))
                        .bg(p.chip)
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(pro_skeleton_bar(280., 12., 4., skel_phase, cx))
                        .child(pro_skeleton_bar(196., 12., 4., skel_phase, cx)),
                )
                .child(
                    div()
                        .h(px(44.))
                        .px(px(14.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(16., 16., 4., skel_phase, cx))
                        .child(pro_skeleton_bar(96., 12., 4., skel_phase, cx))
                        .child(pro_skeleton_bar(180., 12., 4., skel_phase, cx))
                        .child(
                            div()
                                .ml_auto()
                                .child(pro_skeleton_bar(42., 12., 4., skel_phase, cx)),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(7.))
                        .child(pro_skeleton_bar(420., 12., 4., skel_phase, cx))
                        .child(pro_skeleton_bar(360., 12., 4., skel_phase, cx)),
                )
                .child(
                    div()
                        .h(px(44.))
                        .px(px(14.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(16., 16., 4., skel_phase, cx))
                        .child(pro_skeleton_bar(88., 12., 4., skel_phase, cx))
                        .child(pro_skeleton_bar(164., 12., 4., skel_phase, cx))
                        .child(
                            div()
                                .ml_auto()
                                .child(pro_skeleton_bar(38., 12., 4., skel_phase, cx)),
                        ),
                )
                .into_any_element()
        } else if is_empty {
            let cta: ProEmptyCta<Self> = (
                pro_eid(&self.id, "empty-start-chat"),
                tr(cx, "Start a chat", "开始对话").into(),
                Box::new(|_, _, cx| {
                    cx.emit(AgentChatProgressEvent::Navigate("New chat".into()));
                }),
            );
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(pro_empty_state(
                    ProIcon::Thread,
                    tr(cx, "No chats yet", "暂无对话"),
                    tr(
                        cx,
                        "Ask a question or hand off a task — progress and outputs stay beside the conversation.",
                        "提问或交办任务后，步骤进度与产出文件会同步显示在对话旁侧。",
                    ),
                    Some(cta),
                    cx,
                ))
                .into_any_element()
        } else {
            let parts = self.turns.clone();
            let mut col = div()
                .max_w(px(680.))
                .w_full()
                .mx_auto()
                .flex()
                .flex_col()
                .gap(px(14.));
            let mut tool_ord = 0usize;
            for part in &parts {
                match part {
                    ChatTurnPart::UserBubble(txt) => {
                        col = col.child(
                            div()
                                .ml_auto()
                                .max_w(px(520.))
                                .mt(px(4.))
                                .px(px(14.))
                                .py(px(9.))
                                .rounded(px(16.))
                                .bg(p.chip)
                                .text_size(px(14.))
                                .line_height(px(21.))
                                .text_color(p.ink)
                                .child(txt.clone()),
                        );
                    }
                    ChatTurnPart::ToolCall(tc) => {
                        let kb_focused = self.kb_nav_active && tool_ord == self.focused_tool_idx;
                        tool_ord += 1;
                        col = col.child(self.render_tool_card(tc, kb_focused, spin_phase, cx));
                    }
                    ChatTurnPart::Paragraph(txt) => {
                        col = col.child(
                            div()
                                .text_size(px(14.))
                                .line_height(px(22.))
                                .text_color(p.ink)
                                .child(txt.clone()),
                        );
                    }
                    ChatTurnPart::BulletSummary { heading, items } => {
                        col = col.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .text_size(px(14.))
                                        .line_height(px(22.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(p.ink)
                                        .child(heading.clone()),
                                )
                                .child(div().pl(px(6.)).flex().flex_col().gap(px(3.)).children(
                                    items.iter().map(|li| {
                                        div()
                                            .flex()
                                            .items_start()
                                            .gap(px(8.))
                                            .text_size(px(14.))
                                            .line_height(px(22.))
                                            .text_color(p.ink)
                                            .child(div().text_color(p.muted).child("•"))
                                            .child(div().flex_1().min_w_0().child(li.clone()))
                                    }),
                                )),
                        );
                    }
                }
            }
            col.into_any_element()
        };

        let thread_state_key = if is_loading {
            "loading"
        } else if is_empty {
            "empty"
        } else {
            "ready"
        };
        let thread_body = pro_view_transition(
            pro_eid(&self.id, "thread-state-vtx"),
            thread_state_key,
            thread_body,
        );

        let composer_el = self.render_composer(narrow, spin_phase, cx);
        let side_w = if narrow {
            0.
        } else {
            pro_spring_f32(
                pro_eid(&self.id, "side-panel-w"),
                if self.panel_open { 300. } else { 0. },
                0.24,
                w,
                cx,
            )
        };
        let desktop_side_el = if !narrow && side_w > 0.5 {
            let side = self.render_side_panel(skel_phase, spin_phase, cx);
            Some(
                div()
                    .w(px(side_w.round()))
                    .h_full()
                    .flex_none()
                    .overflow_hidden()
                    .child(side),
            )
        } else {
            None
        };
        let narrow_side_el = if narrow {
            let side = self.render_side_panel(skel_phase, spin_phase, cx);
            let close_scrim = cx.listener(|this, _, _, cx| {
                this.panel_open = false;
                cx.notify();
            });
            let scrim_id = pro_eid(&self.id, "narrow-side-scrim");
            let dark = p.dark;
            Some(pro_presence(
                pro_eid(&self.id, "narrow-side-pres"),
                self.panel_open,
                0.22,
                0.16,
                move |a| {
                    div()
                        .id(scrim_id)
                        .absolute()
                        .inset_0()
                        .bg(hsla(0., 0., 0., (if dark { 0.36 } else { 0.14 }) * a))
                        .flex()
                        .justify_end()
                        .on_mouse_down(MouseButton::Left, close_scrim)
                        .child(
                            div()
                                .h_full()
                                .mr(px(-24. * (1. - a)))
                                .opacity(a)
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .child(side),
                        )
                        .into_any_element()
                },
            ))
        } else {
            None
        };

        div()
            .id(pro_eid(&self.id, "page"))
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .flex()
            .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, w, cx| {
                let editor_focused = this
                    .composer_editor
                    .as_ref()
                    .is_some_and(|e| e.read(cx).focus.is_focused(w));
                let key = ev.keystroke.key.as_str();
                if key == "escape" {
                    if this.open_popover.take().is_some() {
                        cx.notify();
                    } else if narrow && this.panel_open {
                        this.panel_open = false;
                        cx.notify();
                    } else if this.voice_recording {
                        this.voice_recording = false;
                        cx.notify();
                    } else if this.running {
                        this.stop_progress(cx);
                    }
                    return;
                }
                if editor_focused {
                    if key == "enter" && ev.keystroke.modifiers.platform {
                        this.send_composer_message(w, cx);
                    }
                    return;
                }
                let tool_ids: Vec<SharedString> = this
                    .turns
                    .iter()
                    .filter_map(|t| match t {
                        ChatTurnPart::ToolCall(tc) => Some(tc.id.clone()),
                        _ => None,
                    })
                    .collect();
                match key {
                    "/" => {
                        if let Some(ed) = &this.composer_editor {
                            {
                                let fh = ed.read(cx).focus.clone();
                                fh.focus(w, cx);
                            }
                        }
                    }
                    "j" | "down" if !tool_ids.is_empty() => {
                        this.kb_nav_active = true;
                        this.focused_tool_idx = (this.focused_tool_idx + 1) % tool_ids.len();
                        cx.notify();
                    }
                    "k" | "up" if !tool_ids.is_empty() => {
                        this.kb_nav_active = true;
                        this.focused_tool_idx = this
                            .focused_tool_idx
                            .checked_sub(1)
                            .unwrap_or(tool_ids.len() - 1);
                        cx.notify();
                    }
                    "enter" | "space" if !tool_ids.is_empty() => {
                        this.kb_nav_active = true;
                        let idx = this.focused_tool_idx.min(tool_ids.len() - 1);
                        let id = tool_ids[idx].to_string();
                        this.toggle_tool_trace(&id);
                        cx.notify();
                    }
                    "e" => {
                        this.toggle_earlier_steps();
                        cx.notify();
                    }
                    "p" | "[" => {
                        this.panel_open = !this.panel_open;
                        cx.notify();
                    }
                    "r" => {
                        this.replay_progress(cx);
                    }
                    _ => {}
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
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(top_bar)
                    .child(
                        div()
                            .id(pro_eid(&self.id, "thread-scroll"))
                            .track_scroll(&self.thread_scroll)
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .pt(px(8.))
                            .px(px(if narrow { 16. } else { 24. }))
                            .pb(px(if narrow { 16. } else { 24. }))
                            .child(thread_body),
                    )
                    .child(composer_el),
            )
            .when_some(desktop_side_el, |d, side| d.child(side))
            .when_some(narrow_side_el, |d, side| d.child(side))
    }
}

impl Render for AgentChatProgressScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_shell_if_dirty(cx);
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{ProgressStep, ProgressStepState, default_steps, middle_truncate};

    #[test]
    fn middle_truncate_preserves_stem_end_and_extension() {
        assert_eq!(middle_truncate("Team Directory", 24), "Team Directory");
        let v1 = middle_truncate("northwind-q3-enterprise-follow-up-draft-v1.docx", 24);
        let v2 = middle_truncate("northwind-q3-enterprise-follow-up-draft-v2.docx", 24);
        assert!(v1.ends_with("-v1.docx"), "got {v1}");
        assert!(v2.ends_with("-v2.docx"), "got {v2}");
        assert_ne!(v1, v2);
        assert!(v1.chars().count() <= 24);
    }

    #[test]
    fn default_progress_trail_separates_earlier_and_recent_steps() {
        let mut steps: Vec<ProgressStep> = default_steps();
        let earlier_done = steps
            .iter()
            .filter(|s| s.earlier && s.state == ProgressStepState::Done)
            .count();
        assert_eq!(earlier_done, 3);
        let mut recent: Vec<&mut ProgressStep> = steps.iter_mut().filter(|s| !s.earlier).collect();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].state, ProgressStepState::Pending);
        assert_eq!(recent[1].state, ProgressStepState::Done);
        recent[0].state = ProgressStepState::Running;
        let done_total = steps
            .iter()
            .filter(|s| s.state == ProgressStepState::Done)
            .count();
        assert_eq!(done_total, 4);
    }
}
