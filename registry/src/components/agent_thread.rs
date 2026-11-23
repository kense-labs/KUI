//! Orchestrator conversation thread with collapsible tool runs, task cards, and decision dock.
//!
//! Renders [`AgentThreadScreen`] inside [`AppShell`] (`AppShellPreset::Agent`, active `"Thread"`),
//! pairing a single-column live conversation stream and multi-mode composer with a right-hand
//! work area for opened tasks and generated workspace artifacts.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_eid,
            pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected, pro_pill_indicator,
            pro_presence, pro_progress_ring, pro_selection_sensor, pro_skeleton_bar,
            pro_spring_bar, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        pro_icons::{AGENT_WAVE_MS, ProIcon, agent_pixel_cells, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const HAND_APPROVAL_SVG: &str = r#"<path d="M52.5305 32.6557V24.8463C52.5305 22.1016 54.7555 19.8766 57.5001 19.8766C60.2448 19.8766 62.4698 22.1016 62.4698 24.8463V42.027C62.4698 53.3194 53.3154 62.4735 42.0233 62.4735H34.2388M22.7127 39.7552L15.94 31.8537C14.8314 30.5602 13.2129 29.8159 11.5094 29.8159H11.1235C8.11379 29.8159 5.67395 32.2558 5.67395 35.2655C5.67395 36.3412 5.99241 37.393 6.58919 38.2881L16.4118 53.0221C20.3482 58.9269 26.9753 62.4735 34.0719 62.4735" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M52.534 24.1378V16.3284C52.534 13.5837 50.3091 11.3588 47.5644 11.3588C44.8197 11.3588 42.5948 13.5837 42.5948 16.3284V29.8174" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M42.5958 29.8198V10.6513C42.5958 7.90661 40.3708 5.68163 37.6262 5.68163C34.8815 5.68163 32.6565 7.90661 32.6565 10.6513V28.4" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M32.6576 28.3975V16.3284C32.6576 13.5837 30.4326 11.3588 27.6879 11.3588C24.9433 11.3588 22.7183 13.5837 22.7183 16.3284V39.7567" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path>"#;
const THREAD_DOC_SVG: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><path d="M4 6.6h4M4 8.6h2.6" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"></path>"#;
const THREAD_SHEET_SVG: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><rect x="3.4" y="5.6" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="6.3" y="5.6" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="3.4" y="7.8" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="6.3" y="7.8" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect>"#;
const THREAD_CODE_SVG: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><path d="M4 6.6h4M4 8.6h2.4" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"></path>"#;

/// Network connectivity state surfaced in the thread banner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThreadConnection {
    #[default]
    Online,
    Offline,
    Reconnecting,
}

/// Delivery state of a user message bubble.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UserMessageStatus {
    #[default]
    Sent,
    Queued,
    Sending,
    Failed,
}

/// Action approval policy chosen in the bottom composer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ApprovalPolicy {
    #[default]
    RequestApproval,
    AutoSafe,
    FullAutonomy,
}

impl ApprovalPolicy {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::RequestApproval => tr(cx, "Request approval", "关键操作需审批"),
            Self::AutoSafe => tr(cx, "Auto-approve reads", "只读操作自动批准"),
            Self::FullAutonomy => tr(cx, "Full autonomy", "全自动执行"),
        }
    }
}

/// Execution status of a single tool invocation inside a collapsible tool group.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThreadToolStatus {
    Running,
    #[default]
    Done,
    Failed,
    Stopped,
}

/// Accent ring color for inline task cards in the thread.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThreadRingTone {
    #[default]
    Amber,
    Orange,
    Teal,
    Green,
    Blue,
}

impl ThreadRingTone {
    fn resolve(self, p: &ProPalette) -> Hsla {
        match self {
            Self::Amber => p.amber,
            Self::Orange => p.orange,
            Self::Teal => p.teal,
            Self::Green => p.green,
            Self::Blue => p.blue,
        }
    }
}

/// Resolution outcome for an inline or docked approval request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApprovalOutcome {
    Approved,
    Edited(SharedString),
    Skipped,
}

/// File or voice attachment staged in the composer or sent with a message.
#[derive(Clone, Debug, PartialEq)]
pub struct ThreadAttachment {
    pub name: SharedString,
    pub size_label: SharedString,
    pub progress: f32,
    pub failed: bool,
}

impl ThreadAttachment {
    /// Construct a completed file attachment chip.
    pub fn ready(name: impl Into<SharedString>, size_label: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            size_label: size_label.into(),
            progress: 1.0,
            failed: false,
        }
    }
}

/// One tool execution row inside a [`ThreadPart::Tools`] group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadToolCall {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: ProIcon,
    pub status: ThreadToolStatus,
    pub duration_label: SharedString,
    pub error: Option<SharedString>,
}

impl ThreadToolCall {
    /// Construct a completed tool call row.
    pub fn done(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: ProIcon,
        duration_label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            status: ThreadToolStatus::Done,
            duration_label: duration_label.into(),
            error: None,
        }
    }
}

/// Inline task summary card embedded inside an orchestrator reply.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadTaskCard {
    pub id: SharedString,
    pub title: SharedString,
    pub status_text: SharedString,
    pub done_steps: usize,
    pub total_steps: usize,
    pub ring_tone: ThreadRingTone,
    pub done: bool,
}

impl ThreadTaskCard {
    /// Normalized step progress in `0.0..=1.0`.
    pub fn progress(&self) -> f32 {
        if self.done {
            return 1.0;
        }
        if self.total_steps == 0 {
            0.0
        } else {
            (self.done_steps as f32 / self.total_steps as f32).clamp(0.0, 1.0)
        }
    }
}

/// Inline action approval card with `Approve` / `Edit` / `Skip` controls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadApprovalCard {
    pub id: SharedString,
    pub title: SharedString,
    pub summary: SharedString,
    pub app: Option<SharedString>,
    pub facts: Vec<(SharedString, SharedString)>,
    pub outcome: Option<ApprovalOutcome>,
}

/// Structured content block inside an orchestrator message.
#[derive(Clone, Debug, PartialEq)]
pub enum ThreadPart {
    Text(SharedString),
    Tools {
        id: SharedString,
        summary_label: SharedString,
        calls: Vec<ThreadToolCall>,
    },
    Task(ThreadTaskCard),
    Artifacts(Vec<SharedString>),
    Asked {
        id: SharedString,
        question: SharedString,
        answered: bool,
    },
    Approval(ThreadApprovalCard),
    Error {
        text: SharedString,
        retryable: bool,
    },
}

/// One user or orchestrator turn in [`AgentThreadScreen`].
#[derive(Clone, Debug, PartialEq)]
pub enum ThreadMessage {
    User {
        id: SharedString,
        context_note: Option<SharedString>,
        text: SharedString,
        attachments: Vec<ThreadAttachment>,
        status: UserMessageStatus,
    },
    Agent {
        id: SharedString,
        at: SharedString,
        parts: Vec<ThreadPart>,
        streaming: bool,
        stopped: bool,
    },
}

/// Selectable option inside a docked [`ThreadDecision`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionOption {
    pub id: SharedString,
    pub title: SharedString,
    pub summary: SharedString,
    pub recommended: bool,
}

impl DecisionOption {
    /// Construct a decision option row.
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        summary: impl Into<SharedString>,
        recommended: bool,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            summary: summary.into(),
            recommended,
        }
    }
}

/// Pending decision or approval waiting in the dock above the composer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadDecision {
    pub id: SharedString,
    pub task_title: SharedString,
    pub task_id: SharedString,
    pub question: SharedString,
    pub options: Vec<DecisionOption>,
    pub facts: Vec<(SharedString, SharedString)>,
    pub is_approval: bool,
}

/// Optional model choice when explicit model switching is enabled on the composer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadModelOption {
    pub id: SharedString,
    pub name: SharedString,
    pub provider: SharedString,
}

/// Events emitted by [`AgentThreadScreen`].
#[derive(Clone, Debug, PartialEq)]
pub enum AgentThreadEvent {
    Navigate(SharedString),
    Send {
        text: SharedString,
        attachments: Vec<ThreadAttachment>,
    },
    Stop,
    Retry(SharedString),
    Reconnect,
    Decide {
        decision_id: SharedString,
        answer: SharedString,
    },
    Later(SharedString),
    ApprovalAction {
        approval_id: SharedString,
        outcome: ApprovalOutcome,
    },
    OpenTask(SharedString),
    ShowInTasks(SharedString),
    OpenArtifact(SharedString),
    OpenInbox(Option<SharedString>),
}

impl EventEmitter<AgentThreadEvent> for AgentThreadScreen {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DockState {
    Hidden,
    Expanded,
    Minimized,
}

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ComposerPopover {
    AddMenu,
    ApprovalMenu,
    ModelMenu,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum WorkPaneTab {
    Task(SharedString),
    Artifact(SharedString),
}

struct PartRenderCtx<'a> {
    msg_id: &'a SharedString,
    part_idx: usize,
    streaming: bool,
    tight_top: bool,
    elapsed_ms: f32,
}

struct TaskDetailRecord {
    id: SharedString,
    title: SharedString,
    project: SharedString,
    started: SharedString,
    status_text: SharedString,
    ring_tone: ThreadRingTone,
    goal: SharedString,
    ask_question: Option<SharedString>,
    steps: Vec<(SharedString, bool)>,
    history: Vec<SharedString>,
    artifacts: Vec<SharedString>,
}

/// Interactive Agent / Thread workspace screen (`agent-thread`).
pub struct AgentThreadScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    standalone: bool,
    messages: Vec<ThreadMessage>,
    decisions: Vec<ThreadDecision>,
    dock_state: DockState,
    dock_index: usize,
    dock_focus_row: usize,
    expanded_tools: Vec<SharedString>,
    work_tabs: Vec<WorkPaneTab>,
    active_work_tab: usize,
    running_count: usize,
    needs_count: usize,
    connection: ThreadConnection,
    last_connection: ThreadConnection,
    show_away_card: bool,
    show_jump_pill: bool,
    attachments: Vec<ThreadAttachment>,
    approval_policy: ApprovalPolicy,
    models: Vec<ThreadModelOption>,
    selected_model: usize,
    active_popover: Option<ComposerPopover>,
    last_popover: ComposerPopover,
    last_dock_expanded: bool,
    recording: bool,
    recording_started: Option<Instant>,
    editing_approval_id: Option<SharedString>,
    composer_editor: Option<Entity<Editor>>,
    custom_answer_editor: Option<Entity<Editor>>,
    inline_edit_editor: Option<Entity<Editor>>,
    editor_focused: bool,
    stream_started: Option<(SharedString, SharedString, Instant)>,
    loading: bool,
    empty: bool,
    focus: FocusHandle,
    scroll: ScrollHandle,
    boot_at: Instant,
    measured_width: Rc<Cell<f32>>,
    mobile_panel_open: bool,
}

impl Focusable for AgentThreadScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl AgentThreadScreen {
    /// Construct an [`AgentThreadScreen`] with the canonical orchestrator conversation and work area.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Agent, shell_cx)
                .with_active("Thread")
                .with_work_area(true, false)
                .with_panel_width(340.);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_work_panel(w, cx));
            s
        });
        cx.subscribe(&shell, |this, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => cx.emit(AgentThreadEvent::Navigate(label.clone())),
            AppShellEvent::PanelOpen(false) => {
                this.mobile_panel_open = false;
                cx.notify();
            }
            AppShellEvent::PanelOpen(true) => {
                if this.work_tabs.is_empty() {
                    this.work_tabs
                        .push(WorkPaneTab::Artifact("load-test-results.csv".into()));
                }
                cx.notify();
            }
            _ => {}
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            shell,
            standalone: false,
            messages: sample_thread_messages(),
            decisions: sample_thread_decisions(),
            dock_state: DockState::Hidden,
            dock_index: 0,
            dock_focus_row: 0,
            expanded_tools: Vec::new(),
            work_tabs: vec![WorkPaneTab::Artifact("load-test-results.csv".into())],
            active_work_tab: 0,
            running_count: 9,
            needs_count: 5,
            connection: ThreadConnection::Online,
            last_connection: ThreadConnection::Offline,
            show_away_card: false,
            show_jump_pill: false,
            attachments: Vec::new(),
            approval_policy: ApprovalPolicy::RequestApproval,
            models: Vec::new(),
            selected_model: 0,
            active_popover: None,
            last_popover: ComposerPopover::AddMenu,
            last_dock_expanded: false,
            recording: false,
            recording_started: None,
            editing_approval_id: None,
            composer_editor: None,
            custom_answer_editor: None,
            inline_edit_editor: None,
            editor_focused: false,
            stream_started: None,
            loading: false,
            empty: false,
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            boot_at: Instant::now(),
            measured_width: Rc::new(Cell::new(980.)),
            mobile_panel_open: false,
        }
    }

    /// Render only the inner Thread column without the surrounding [`AppShell`].
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Replace the initial conversation messages.
    pub fn with_messages(mut self, messages: Vec<ThreadMessage>) -> Self {
        self.messages = messages;
        self
    }

    /// Replace the pending decisions queued for the dock above the composer.
    pub fn with_decisions(mut self, decisions: Vec<ThreadDecision>) -> Self {
        self.decisions = decisions;
        self.dock_index = 0;
        self
    }

    /// Open the decision/approval dock immediately on mount.
    pub fn with_dock_open(mut self, open: bool) -> Self {
        self.dock_state = if open && !self.decisions.is_empty() {
            DockState::Expanded
        } else {
            DockState::Hidden
        };
        self
    }

    /// Configure the header summary counts (`running`, `needs`).
    pub fn with_running_counts(mut self, running: usize, needs: usize) -> Self {
        self.running_count = running;
        self.needs_count = needs;
        self
    }

    /// Set the simulated network connection state (`Online`, `Offline`, `Reconnecting`).
    pub fn with_connection(mut self, connection: ThreadConnection) -> Self {
        self.connection = connection;
        self
    }

    /// Show or hide the `"While you were away"` catch-up card at the top of the thread.
    pub fn with_away_card(mut self, show: bool) -> Self {
        self.show_away_card = show;
        self
    }

    /// Provide optional selectable models in the composer toolbar.
    pub fn with_models(mut self, models: Vec<ThreadModelOption>) -> Self {
        self.models = models;
        self.selected_model = 0;
        self
    }

    /// Configure skeleton loading mode.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure empty thread mode (`loading` takes precedence over `empty`).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Toggle skeleton loading mode at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Toggle empty thread mode at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        cx.notify();
    }

    /// Update the connection banner state; flushes queued messages when returning `Online`.
    pub fn set_connection(&mut self, connection: ThreadConnection, cx: &mut Context<Self>) {
        self.connection = connection;
        if connection == ThreadConnection::Online {
            for msg in &mut self.messages {
                if let ThreadMessage::User { status, text, .. } = msg
                    && *status == UserMessageStatus::Queued
                {
                    *status = UserMessageStatus::Sent;
                    cx.emit(AgentThreadEvent::Send {
                        text: text.clone(),
                        attachments: Vec::new(),
                    });
                }
            }
        }
        cx.notify();
    }

    /// Return whether an orchestrator message is currently streaming.
    pub fn is_streaming(&self) -> bool {
        self.stream_started.is_some()
            || self.messages.iter().any(|m| {
                matches!(
                    m,
                    ThreadMessage::Agent {
                        streaming: true,
                        ..
                    }
                )
            })
    }

    /// Toggle expansion of a collapsible tool-call group by `group_id`.
    pub fn toggle_tool_group(&mut self, group_id: impl Into<SharedString>, cx: &mut Context<Self>) {
        let gid: SharedString = group_id.into();
        if let Some(pos) = self.expanded_tools.iter().position(|x| x == &gid) {
            self.expanded_tools.remove(pos);
        } else {
            self.expanded_tools.push(gid);
        }
        cx.notify();
    }

    /// Send a user prompt and trigger a simulated orchestrator reply when online.
    pub fn send_prompt(&mut self, raw_text: impl Into<String>, cx: &mut Context<Self>) -> bool {
        if self.loading {
            return false;
        }
        let trimmed = raw_text.into().trim().to_string();
        if trimmed.is_empty() && self.attachments.is_empty() {
            return false;
        }
        let sent_attachments: Vec<ThreadAttachment> =
            self.attachments.drain(..).filter(|a| !a.failed).collect();
        let msg_text: SharedString = if trimmed.is_empty() {
            "Attached workspace files".into()
        } else {
            trimmed.clone().into()
        };
        let msg_id: SharedString = format!("u-{}", self.messages.len() + 1).into();
        let status = if self.connection == ThreadConnection::Offline {
            UserMessageStatus::Queued
        } else {
            UserMessageStatus::Sent
        };
        self.empty = false;
        self.messages.push(ThreadMessage::User {
            id: msg_id,
            context_note: None,
            text: msg_text.clone(),
            attachments: sent_attachments.clone(),
            status,
        });

        if status == UserMessageStatus::Sent {
            cx.emit(AgentThreadEvent::Send {
                text: msg_text,
                attachments: sent_attachments,
            });
            let reply_id: SharedString = format!("a-{}", self.messages.len() + 1).into();
            let target_reply: SharedString = "Staged the ingest consumer deployment (`v0.4.2`) against staging-us-east. Before flipping traffic, confirm which queue backend and rollout approval to apply.".into();
            self.messages.push(ThreadMessage::Agent {
                id: reply_id.clone(),
                at: "Just now".into(),
                parts: vec![
                    ThreadPart::Tools {
                        id: format!("tg-live-{}", self.messages.len()).into(),
                        summary_label: "Ran 2 tools · 0.8s".into(),
                        calls: vec![
                            ThreadToolCall::done(
                                "tc-l1",
                                "Validated staging-us-east manifest and replay cursor",
                                ProIcon::Terminal,
                                "0.4s",
                            ),
                            ThreadToolCall::done(
                                "tc-l2",
                                "Prepared deployment approval gate",
                                ProIcon::Shield,
                                "0.8s",
                            ),
                        ],
                    },
                    ThreadPart::Text(SharedString::default()),
                    ThreadPart::Approval(ThreadApprovalCard {
                        id: format!("appr-{}", self.messages.len()).into(),
                        title: "Deploy ingest-service v0.4.2 to staging".into(),
                        summary:
                            "Applies 24h replay retention and routes 10% canary order traffic."
                                .into(),
                        app: Some("GitHub".into()),
                        facts: vec![
                            ("Target".into(), "staging-us-east".into()),
                            ("Replay".into(), "24h window".into()),
                        ],
                        outcome: None,
                    }),
                ],
                streaming: true,
                stopped: false,
            });
            self.stream_started = Some((reply_id, target_reply, Instant::now()));
        }
        self.scroll.scroll_to_bottom();
        cx.notify();
        true
    }

    /// Stop an in-flight streaming orchestrator answer.
    pub fn stop_streaming(&mut self, cx: &mut Context<Self>) {
        self.stream_started = None;
        for msg in &mut self.messages {
            if let ThreadMessage::Agent {
                streaming, stopped, ..
            } = msg
                && *streaming
            {
                *streaming = false;
                *stopped = true;
            }
        }
        cx.emit(AgentThreadEvent::Stop);
        cx.notify();
    }

    /// Answer the currently docked decision and advance or fold the dock.
    pub fn answer_docked_decision(
        &mut self,
        answer_label: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        if self.dock_index >= self.decisions.len() {
            return;
        }
        let answer: SharedString = answer_label.into();
        if answer.trim().is_empty() {
            return;
        }
        let removed = self.decisions.remove(self.dock_index);
        self.needs_count = self.needs_count.saturating_sub(1);

        // Mark matching inline Asked part as answered
        for msg in &mut self.messages {
            if let ThreadMessage::Agent { parts, .. } = msg {
                for part in parts {
                    if let ThreadPart::Asked { id, answered, .. } = part
                        && id == &removed.id
                    {
                        *answered = true;
                    }
                }
            }
        }

        self.messages.push(ThreadMessage::User {
            id: format!("u-ans-{}", self.messages.len() + 1).into(),
            context_note: Some(format!("Answered: {}", removed.question).into()),
            text: answer.clone(),
            attachments: Vec::new(),
            status: UserMessageStatus::Sent,
        });

        if self.decisions.is_empty() {
            self.dock_state = DockState::Hidden;
            self.dock_index = 0;
        } else if self.dock_index >= self.decisions.len() {
            self.dock_index = self.decisions.len() - 1;
        }
        self.dock_focus_row = 0;
        self.scroll.scroll_to_bottom();
        cx.emit(AgentThreadEvent::Decide {
            decision_id: removed.id,
            answer,
        });
        cx.notify();
    }

    /// Resolve an inline [`ThreadPart::Approval`] card (`Approve`, `Edit`, or `Skip`).
    pub fn resolve_inline_approval(
        &mut self,
        approval_id: &str,
        outcome: ApprovalOutcome,
        cx: &mut Context<Self>,
    ) {
        let mut found_id: Option<SharedString> = None;
        for msg in &mut self.messages {
            if let ThreadMessage::Agent { parts, .. } = msg {
                for part in parts {
                    if let ThreadPart::Approval(card) = part
                        && card.id.as_ref() == approval_id
                    {
                        card.outcome = Some(outcome.clone());
                        found_id = Some(card.id.clone());
                    }
                }
            }
        }
        if let Some(aid) = found_id {
            self.editing_approval_id = None;
            cx.emit(AgentThreadEvent::ApprovalAction {
                approval_id: aid,
                outcome,
            });
            cx.notify();
        }
    }

    /// Open a task inside the right-hand work area without leaving the thread.
    pub fn open_task_in_panel(&mut self, task_id: impl Into<SharedString>, cx: &mut Context<Self>) {
        let tid: SharedString = task_id.into();
        if let Some(pos) = self
            .work_tabs
            .iter()
            .position(|t| matches!(t, WorkPaneTab::Task(_)))
        {
            self.work_tabs[pos] = WorkPaneTab::Task(tid.clone());
            self.active_work_tab = pos;
        } else {
            self.work_tabs.insert(0, WorkPaneTab::Task(tid.clone()));
            self.active_work_tab = 0;
        }
        self.mobile_panel_open = true;
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        cx.emit(AgentThreadEvent::OpenTask(tid));
        cx.notify();
    }

    /// Open a generated artifact file in its own tab inside the right-hand work area.
    pub fn open_artifact_in_panel(
        &mut self,
        name: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let name: SharedString = name.into();
        if let Some(pos) = self
            .work_tabs
            .iter()
            .position(|t| matches!(t, WorkPaneTab::Artifact(n) if n == &name))
        {
            self.active_work_tab = pos;
        } else {
            self.work_tabs.push(WorkPaneTab::Artifact(name.clone()));
            self.active_work_tab = self.work_tabs.len() - 1;
        }
        self.mobile_panel_open = true;
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        cx.emit(AgentThreadEvent::OpenArtifact(name));
        cx.notify();
    }

    fn close_work_tab(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx >= self.work_tabs.len() {
            return;
        }
        self.work_tabs.remove(idx);
        if self.work_tabs.is_empty() {
            self.active_work_tab = 0;
            self.mobile_panel_open = false;
            self.shell.update(cx, |s, cx| s.set_panel_open(false, cx));
        } else if self.active_work_tab >= self.work_tabs.len() {
            self.active_work_tab = self.work_tabs.len() - 1;
        }
        cx.notify();
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.composer_editor.is_none() {
            let ph: SharedString = tr(cx, "Ask for anything…", "输入任何指令或问题…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::InlineMulti, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.5;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_entity, ev: &EditEvent, w, cx| match ev {
                EditEvent::Focused => {
                    this.editor_focused = true;
                    this.active_popover = None;
                    cx.notify();
                }
                EditEvent::Blurred => {
                    this.editor_focused = false;
                    cx.notify();
                }
                EditEvent::Changed => cx.notify(),
                EditEvent::Submitted => {
                    let val = ed_entity.read(cx).value().to_string();
                    if this.send_prompt(val, cx) {
                        ed_entity.update(cx, |e, cx| e.set_value("", w, cx));
                    }
                }
                EditEvent::Escape => {
                    if this.recording {
                        this.recording = false;
                        this.recording_started = None;
                        cx.notify();
                    } else if this.is_streaming() {
                        this.stop_streaming(cx);
                    }
                }
                _ => {}
            })
            .detach();
            self.composer_editor = Some(ed);
        }

        if self.custom_answer_editor.is_none() {
            let ph: SharedString = tr(cx, "Something else…", "输入其他回答…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
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
                EditEvent::Changed => cx.notify(),
                EditEvent::Submitted => {
                    let val = ed_entity.read(cx).value().to_string();
                    if !val.trim().is_empty() {
                        this.answer_docked_decision(val, cx);
                        ed_entity.update(cx, |e, cx| e.set_value("", w, cx));
                    }
                }
                EditEvent::Escape => {
                    if let Some(dec) = this.decisions.get(this.dock_index) {
                        let did = dec.id.clone();
                        this.dock_state = DockState::Minimized;
                        cx.emit(AgentThreadEvent::Later(did));
                        this.focus.focus(w, cx);
                        cx.notify();
                    }
                }
                _ => {}
            })
            .detach();
            self.custom_answer_editor = Some(ed);
        }

        if self.inline_edit_editor.is_none() {
            let ph: SharedString =
                tr(cx, "Adjust parameters before approving…", "审批前调整参数…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 12.5;
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
                EditEvent::Submitted => {
                    if let Some(aid) = this.editing_approval_id.clone() {
                        let note: SharedString =
                            ed_entity.read(cx).value().trim().to_owned().into();
                        this.resolve_inline_approval(&aid, ApprovalOutcome::Edited(note), cx);
                        ed_entity.update(cx, |e, cx| e.set_value("", w, cx));
                    }
                }
                EditEvent::Escape => {
                    this.editing_approval_id = None;
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.inline_edit_editor = Some(ed);
        }
    }

    fn handle_key(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if self.editor_focused || ev.keystroke.modifiers.platform || ev.keystroke.modifiers.control
        {
            return;
        }
        let k = ev.keystroke.key.as_str();
        if k == "/" {
            if let Some(ed) = self.composer_editor.clone() {
                ed.update(cx, |e, cx| e.focus(w, cx));
            }
            return;
        }
        if k == "end" {
            self.show_jump_pill = false;
            self.scroll.scroll_to_bottom();
            cx.notify();
            return;
        }
        if k == "escape" {
            if self.active_popover.take().is_some() {
                cx.notify();
                return;
            }
            if self.dock_state == DockState::Expanded
                && let Some(dec) = self.decisions.get(self.dock_index)
            {
                let did = dec.id.clone();
                self.dock_state = DockState::Minimized;
                cx.emit(AgentThreadEvent::Later(did));
                cx.notify();
                return;
            }
            if self.is_streaming() {
                self.stop_streaming(cx);
                return;
            }
        }

        if self.dock_state == DockState::Expanded
            && let Some(dec) = self.decisions.get(self.dock_index).cloned()
        {
            if dec.is_approval {
                match k {
                    "1" => self.answer_docked_decision("Approved", cx),
                    "2" => self.answer_docked_decision("Approved with edits", cx),
                    "3" => {
                        let did = dec.id.clone();
                        self.dock_state = DockState::Minimized;
                        cx.emit(AgentThreadEvent::Later(did));
                        cx.notify();
                    }
                    _ => {}
                }
            } else {
                let opt_len = dec.options.len();
                match k {
                    "1" | "2" | "3" | "4" | "5" => {
                        let num = k.parse::<usize>().unwrap_or(1).saturating_sub(1);
                        if let Some(opt) = dec.options.get(num) {
                            self.answer_docked_decision(opt.title.clone(), cx);
                        } else if num == opt_len
                            && let Some(ed) = self.custom_answer_editor.clone()
                        {
                            ed.update(cx, |e, cx| e.focus(w, cx));
                        }
                    }
                    "down" => {
                        if opt_len > 0 {
                            self.dock_focus_row = (self.dock_focus_row + 1) % opt_len;
                            cx.notify();
                        }
                    }
                    "up" => {
                        if opt_len > 0 {
                            self.dock_focus_row = (self.dock_focus_row + opt_len - 1) % opt_len;
                            cx.notify();
                        }
                    }
                    "enter" => {
                        if let Some(opt) = dec.options.get(self.dock_focus_row) {
                            self.answer_docked_decision(opt.title.clone(), cx);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn advance_stream(&mut self, w: &mut Window, reduced: bool) {
        let Some((msg_id, full_text, started_at)) = self.stream_started.clone() else {
            return;
        };
        let elapsed_ms = started_at.elapsed().as_secs_f32() * 1000.;
        let ratio = if reduced {
            1.0
        } else {
            (elapsed_ms / 880.).clamp(0.05, 1.0)
        };
        let chars: Vec<char> = full_text.chars().collect();
        let take_n = ((chars.len() as f32 * ratio).ceil() as usize).min(chars.len());
        let prefix: String = chars.into_iter().take(take_n).collect();
        let done_now = ratio >= 0.999;

        for msg in &mut self.messages {
            if let ThreadMessage::Agent {
                id,
                parts,
                streaming,
                ..
            } = msg
                && id == &msg_id
            {
                for part in parts {
                    if let ThreadPart::Text(txt) = part {
                        *txt = prefix.clone().into();
                    }
                }
                if done_now {
                    *streaming = false;
                }
            }
        }

        if done_now {
            self.stream_started = None;
            if !self.decisions.is_empty() && self.dock_state == DockState::Hidden {
                self.dock_state = DockState::Expanded;
            }
        } else {
            w.request_animation_frame();
        }
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        self.advance_stream(w, reduced);

        let elapsed_ms = self.boot_at.elapsed().as_secs_f32() * 1000.;
        let skel_phase = if reduced {
            0.
        } else {
            (elapsed_ms / 1000.) * 0.75
        };
        if (self.loading || self.recording || self.is_streaming()) && !reduced {
            w.request_animation_frame();
        }

        let measure_cell = self.measured_width.clone();
        let page_w = measure_cell.get();
        let narrow = page_w < 560.;
        let side_pad = if narrow { 12. } else { 24. };
        let panel_open = !self.standalone && self.shell.read(cx).is_panel_open();

        // Top floating summary header ("9 running · 5 need you")
        let head_el = (!panel_open && !self.loading).then(|| {
            div()
                .h(px(48.))
                .pl(px(20.))
                .pr(px(if self.standalone { 14. } else { 48. }))
                .flex_none()
                .flex()
                .items_center()
                .child(
                    div()
                        .ml_auto()
                        .flex()
                        .items_center()
                        .gap(px(2.))
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, "head-running"))
                                .role(Role::Button)
                                .aria_label(format!("{} tasks running", self.running_count))
                                .tab_index(0)
                                .h(px(28.))
                                .px(px(8.))
                                .rounded(px(6.))
                                .hover(move |s| s.bg(p.chip).text_color(p.ink))
                                .focus_visible(move |s| s.border_1().border_color(p.focus))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .cursor_pointer()
                                .child(
                                    div()
                                        .size(px(8.))
                                        .rounded_full()
                                        .border_1()
                                        .border_color(p.teal),
                                )
                                .child(
                                    div()
                                        .text_color(p.text)
                                        .child(self.running_count.to_string()),
                                )
                                .child(tr(cx, "running", "运行中")),
                            cx,
                            |_, _, cx| cx.emit(AgentThreadEvent::Navigate("Tasks".into())),
                        ))
                        .child(div().text_size(px(12.)).text_color(p.faint).child("·"))
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, "head-needs"))
                                .role(Role::Button)
                                .aria_label(format!("{} questions waiting", self.needs_count))
                                .tab_index(0)
                                .h(px(28.))
                                .px(px(8.))
                                .rounded(px(6.))
                                .hover(move |s| s.bg(p.chip).text_color(p.ink))
                                .focus_visible(move |s| s.border_1().border_color(p.focus))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .cursor_pointer()
                                .child(div().size(px(6.)).rounded_full().bg(p.amber))
                                .child(div().text_color(p.text).child(self.needs_count.to_string()))
                                .child(tr(cx, "need you", "待你确认")),
                            cx,
                            |this, _, cx| {
                                if !this.decisions.is_empty() {
                                    this.dock_state = DockState::Expanded;
                                    cx.notify();
                                } else {
                                    cx.emit(AgentThreadEvent::OpenInbox(None));
                                    cx.emit(AgentThreadEvent::Navigate("Inbox".into()));
                                }
                            },
                        )),
                )
        });

        // Connection status banner when offline / reconnecting
        if self.connection != ThreadConnection::Online {
            self.last_connection = self.connection;
        }
        if let Some(pop) = self.active_popover {
            self.last_popover = pop;
        }
        if self.dock_state != DockState::Hidden {
            self.last_dock_expanded = self.dock_state == DockState::Expanded;
        }
        let banner_conn = if self.connection != ThreadConnection::Online {
            self.connection
        } else {
            self.last_connection
        };
        let banner_txt = match banner_conn {
            ThreadConnection::Offline => tr(
                cx,
                "Offline – messages wait to send",
                "离线中——消息将在恢复连接后发送",
            ),
            ThreadConnection::Reconnecting => tr(cx, "Reconnecting… 5s", "正在重连… 5s"),
            ThreadConnection::Online => "",
        };
        let banner_inner = div().w_full().pt(px(6.)).flex().justify_center().child(
            div()
                .h(px(32.))
                .pl(px(12.))
                .pr(px(5.))
                .rounded_full()
                .bg(p.chip)
                .border_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .gap(px(8.))
                .text_size(px(12.5))
                .text_color(p.text)
                .child(pro_icon(ProIcon::Refresh, 13., p.muted))
                .child(banner_txt)
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "banner-retry"))
                        .role(Role::Button)
                        .tab_index(0)
                        .h(px(24.))
                        .px(px(9.))
                        .rounded_full()
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .cursor_pointer()
                        .child(tr(cx, "Retry now", "立即重试")),
                    cx,
                    |this, _, cx| {
                        this.set_connection(ThreadConnection::Online, cx);
                        cx.emit(AgentThreadEvent::Reconnect);
                    },
                )),
        );
        let conn_banner = pro_fold(
            pro_eid(&self.id, "conn-banner-fold"),
            self.connection != ThreadConnection::Online,
            38.,
            0.,
            move || banner_inner.into_any_element(),
        );

        // Scrollable conversation body
        let scroll_area = if self.loading {
            div()
                .id(pro_eid(&self.id, "thread-scroll"))
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(Self::render_loading_stream(side_pad, skel_phase, cx))
        } else if self.empty {
            div()
                .id(pro_eid(&self.id, "thread-scroll"))
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .flex()
                .items_center()
                .justify_center()
                .child(pro_empty_state(
                    ProIcon::Thread,
                    tr(cx, "No chats yet", "暂无对话"),
                    tr(
                        cx,
                        "Ask the orchestrator to kick off tasks, check staging metrics, or summarize progress.",
                        "向协调器布置任务、查询预发指标或汇总近期进展。",
                    ),
                    Some((
                        pro_eid(&self.id, "empty-start-chat"),
                        tr(cx, "Start a chat", "开始对话").into(),
                        Box::new(|this: &mut Self, w, cx| {
                            this.empty = false;
                            if let Some(ed) = this.composer_editor.clone() {
                                ed.update(cx, |e, cx| e.focus(w, cx));
                            }
                            cx.emit(AgentThreadEvent::Navigate("New chat".into()));
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
        } else {
            let away_wrap = div()
                .pb(px(24.))
                .flex()
                .flex_col()
                .gap(px(20.))
                .child(self.render_away_card(cx))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(div().flex_1().h(px(1.)).bg(p.line))
                        .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                            cx,
                            "While away",
                            "离开期间更新",
                        )))
                        .child(div().flex_1().h(px(1.)).bg(p.line)),
                );

            let mut msgs_col = div().flex().flex_col().gap(px(24.));
            let msgs_snapshot = self.messages.clone();
            for msg in &msgs_snapshot {
                msgs_col = msgs_col.child(self.render_message(msg, narrow, elapsed_ms, cx));
            }

            let col = div()
                .w_full()
                .max_w(px(680. + side_pad * 2.))
                .mx_auto()
                .px(px(side_pad))
                .pt(px(if head_el.is_some() { 10. } else { 24. }))
                .pb(px(28.))
                .flex()
                .flex_col()
                .child(pro_fold(
                    pro_eid(&self.id, "away-card-fold"),
                    self.show_away_card,
                    172.,
                    0.,
                    move || away_wrap.into_any_element(),
                ))
                .child(msgs_col);

            let jump_btn = activate(
                div()
                    .id(pro_eid(&self.id, "jump-latest"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(32.))
                    .pl(px(12.))
                    .pr(px(10.))
                    .rounded_full()
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .shadow_sm()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .cursor_pointer()
                    .child(div().size(px(6.)).rounded_full().bg(p.blue))
                    .child(tr(cx, "Jump to latest", "跳至最新"))
                    .child(pro_icon(ProIcon::ChevronDown, 13., p.muted)),
                cx,
                |this, _, cx| {
                    this.show_jump_pill = false;
                    this.scroll.scroll_to_bottom();
                    cx.notify();
                },
            );

            div()
                .id(pro_eid(&self.id, "thread-scroll"))
                .track_scroll(&self.scroll)
                .relative()
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(col)
                .child(
                    div()
                        .absolute()
                        .bottom(px(12.))
                        .left(px(0.))
                        .right(px(0.))
                        .flex()
                        .justify_center()
                        .child(pro_presence(
                            pro_eid(&self.id, "jump-pill-pres"),
                            self.show_jump_pill,
                            0.18,
                            0.14,
                            move |amount| {
                                div()
                                    .mb(px(-6. * (1. - amount)))
                                    .opacity(amount)
                                    .child(jump_btn)
                                    .into_any_element()
                            },
                        )),
                )
        };

        let footer = self.render_footer(side_pad, elapsed_ms, cx);

        // Mobile / standalone right-hand work sheet overlay
        let inline_sheet = (self.standalone || narrow).then(|| {
            let panel_data = self.render_work_panel(w, cx);
            let sheet_open = self.mobile_panel_open && panel_data.is_some();
            let scrim_eid = pro_eid(&self.id, "mobile-work-scrim");
            let scroll_eid = pro_eid(&self.id, "mobile-work-scroll");
            let close_btn = pro_icon_btn(
                pro_eid(&self.id, "close-mobile-work"),
                tr(cx, "Close panel", "关闭面板"),
                ProIcon::Close,
                28.,
                false,
                cx,
                |this, _, cx| {
                    this.mobile_panel_open = false;
                    cx.notify();
                },
            );
            let dismiss = cx.listener(|this, _, _, cx| {
                this.mobile_panel_open = false;
                cx.notify();
            });
            pro_presence(
                pro_eid(&self.id, "mobile-work-pres"),
                sheet_open,
                0.22,
                0.16,
                move |amount| {
                    let Some((title, ic, body)) = panel_data else {
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
                                .w(px(340.))
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
            .when_some(head_el, |d, h| d.child(h))
            .child(conn_banner)
            .child(scroll_area)
            .child(footer)
            .when_some(inline_sheet, |d, s| d.child(s))
    }

    fn render_away_card(&self, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let wait_bg = if p.dark {
            hsla(25. / 360., 1.0, 0.49, 0.145)
        } else {
            hsla(37. / 360., 1.0, 0.50, 0.16)
        };
        div()
            .p(px(10.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .px(px(6.))
                    .flex()
                    .items_baseline()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tr(cx, "While you were away", "你离开期间")),
                    )
                    .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                        cx,
                        "2 done · 3 need you",
                        "2 项完成 · 3 项待确认",
                    ))),
            )
            .children(
                [
                    (
                        "ingest",
                        "Ingest pipeline",
                        "Which queue should ingest use?",
                    ),
                    ("weekly", "Weekly update", "Ready to send to 14 recipients"),
                ]
                .into_iter()
                .map(|(tid, name, note)| {
                    let t_open: SharedString = tid.into();
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("away-{tid}")))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(32.))
                            .px(px(6.))
                            .rounded(px(6.))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .cursor_pointer()
                            .child(div().size(px(6.)).rounded_full().bg(p.amber))
                            .child(div().text_size(px(13.)).text_color(p.ink).child(name))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(note),
                            )
                            .child(
                                div()
                                    .px(px(6.))
                                    .h(px(18.))
                                    .rounded(px(4.))
                                    .bg(wait_bg)
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.amber_ink)
                                    .flex()
                                    .items_center()
                                    .child(tr(cx, "Needs you", "待确认")),
                            ),
                        cx,
                        move |this, _, cx| this.open_task_in_panel(t_open.clone(), cx),
                    )
                }),
            )
            .child(
                div()
                    .px(px(4.))
                    .pt(px(2.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_btn(
                        pro_eid(&self.id, "away-open-inbox"),
                        tr(cx, "Open Inbox", "打开收件箱"),
                        None,
                        ProBtnStyle::Ghost,
                        true,
                        cx,
                        |_, _, cx| {
                            cx.emit(AgentThreadEvent::OpenInbox(None));
                            cx.emit(AgentThreadEvent::Navigate("Inbox".into()));
                        },
                    ))
                    .child(pro_btn(
                        pro_eid(&self.id, "away-mark-read"),
                        tr(cx, "Mark as read", "标为已读"),
                        None,
                        ProBtnStyle::Ghost,
                        true,
                        cx,
                        |this, _, cx| {
                            this.show_away_card = false;
                            cx.notify();
                        },
                    )),
            )
    }

    fn render_message(
        &self,
        msg: &ThreadMessage,
        narrow: bool,
        elapsed_ms: f32,
        cx: &Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        match msg {
            ThreadMessage::User {
                id,
                context_note,
                text,
                attachments,
                status,
            } => {
                let max_pct = if narrow { 0.88 } else { 0.80 };
                let dimmed = matches!(
                    status,
                    UserMessageStatus::Queued | UserMessageStatus::Sending
                );
                let retry_id = id.clone();
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .items_end()
                    .gap(px(6.))
                    .when_some(context_note.clone(), |d, note| {
                        d.child(
                            div()
                                .max_w(relative(max_pct))
                                .mr(px(4.))
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .truncate()
                                .child(note),
                        )
                    })
                    .when(!attachments.is_empty(), |d| {
                        d.child(
                            div()
                                .max_w(relative(max_pct))
                                .flex()
                                .flex_wrap()
                                .justify_end()
                                .gap(px(6.))
                                .children(attachments.iter().map(|att| {
                                    div()
                                        .h(px(30.))
                                        .px(px(9.))
                                        .rounded(px(8.))
                                        .bg(p.card)
                                        .border_1()
                                        .border_color(p.line)
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .text_size(px(12.))
                                        .text_color(p.ink)
                                        .child(pro_icon(ProIcon::Paperclip, 12., p.muted))
                                        .child(att.name.clone())
                                })),
                        )
                    })
                    .child(
                        div()
                            .max_w(relative(max_pct))
                            .px(px(14.))
                            .py(px(8.))
                            .rounded(px(16.))
                            .bg(p.chip)
                            .when(dimmed, |d| d.opacity(0.55))
                            .text_size(px(14.))
                            .line_height(px(22.))
                            .text_color(p.ink)
                            .child(text.clone()),
                    )
                    .when(*status == UserMessageStatus::Queued, |d| {
                        d.child(
                            div()
                                .mr(px(4.))
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child(pro_icon(ProIcon::Clock, 12., p.faint))
                                .child(tr(cx, "Waiting to send", "等待发送")),
                        )
                    })
                    .when(*status == UserMessageStatus::Failed, |d| {
                        d.child(
                            div()
                                .mr(px(4.))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .text_size(px(12.))
                                .text_color(p.red_ink)
                                .child(tr(cx, "Not sent", "发送失败"))
                                .child(pro_btn(
                                    pro_eid(&self.id, format!("retry-{id}")),
                                    tr(cx, "Retry", "重试"),
                                    None,
                                    ProBtnStyle::Ghost,
                                    true,
                                    cx,
                                    move |this, _, cx| {
                                        for m in &mut this.messages {
                                            if let ThreadMessage::User {
                                                id: mid,
                                                status: st,
                                                ..
                                            } = m
                                                && mid == &retry_id
                                            {
                                                *st = UserMessageStatus::Sent;
                                            }
                                        }
                                        cx.emit(AgentThreadEvent::Retry(retry_id.clone()));
                                        cx.notify();
                                    },
                                )),
                        )
                    })
            }
            ThreadMessage::Agent {
                id,
                at,
                parts,
                streaming,
                stopped,
            } => {
                let mut agent_col = div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        div()
                            .h(px(22.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(orchestrator_pixel_mark(
                                p.purple, *streaming, elapsed_ms, reduced,
                            ))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(tr(cx, "Orchestrator", "协调器")),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.faint)
                                    .child(at.clone()),
                            ),
                    );

                let mut prev_was_task = false;
                for (pi, part) in parts.iter().enumerate() {
                    let is_task = matches!(part, ThreadPart::Task(_));
                    let part_el = self.render_part(
                        part,
                        PartRenderCtx {
                            msg_id: id,
                            part_idx: pi,
                            streaming: *streaming,
                            tight_top: prev_was_task && is_task,
                            elapsed_ms,
                        },
                        cx,
                    );
                    prev_was_task = is_task;
                    agent_col = agent_col.child(part_el);
                }

                if *stopped {
                    agent_col = agent_col.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(div().size(px(7.)).rounded(px(1.5)).bg(p.faint))
                            .child(tr(cx, "Stopped", "已停止")),
                    );
                }
                agent_col
            }
        }
    }

    fn render_part(
        &self,
        part: &ThreadPart,
        pctx: PartRenderCtx<'_>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let PartRenderCtx {
            msg_id,
            part_idx,
            streaming,
            tight_top,
            elapsed_ms,
        } = pctx;
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        match part {
            ThreadPart::Text(txt) => {
                let caret_alpha = if reduced {
                    0.45
                } else {
                    0.20 + 0.45 * ((elapsed_ms / 1100.) * std::f32::consts::TAU).cos().abs()
                };
                div()
                    .flex()
                    .flex_wrap()
                    .items_baseline()
                    .text_size(px(14.))
                    .line_height(px(22.4))
                    .text_color(p.ink)
                    .child(txt.clone())
                    .when(streaming, |d| {
                        d.child(
                            div()
                                .ml(px(3.))
                                .w(px(2.))
                                .h(px(14.))
                                .rounded(px(1.))
                                .bg(p.ink.opacity(caret_alpha)),
                        )
                    })
                    .into_any_element()
            }
            ThreadPart::Tools {
                id: gid,
                summary_label,
                calls,
            } => {
                let is_open = self.expanded_tools.contains(gid);
                let any_failed = calls.iter().any(|c| c.status == ThreadToolStatus::Failed);
                let any_running = calls.iter().any(|c| c.status == ThreadToolStatus::Running);
                let lead_color = if any_failed {
                    p.red
                } else if any_running {
                    p.muted
                } else {
                    p.green
                };
                let lead_icon = if any_failed {
                    ProIcon::AlertTriangle
                } else if any_running {
                    ProIcon::Refresh
                } else {
                    ProIcon::Check
                };
                let gid_toggle = gid.clone();

                let mut grp = div().flex().flex_col().items_start().child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("tools-{gid}")))
                        .role(Role::Button)
                        .tab_index(0)
                        .ml(px(-8.))
                        .h(px(28.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .hover(move |s| s.bg(p.hover).text_color(p.text))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(13.))
                        .text_color(p.muted)
                        .cursor_pointer()
                        .child(pro_icon(lead_icon, 14., lead_color))
                        .child(summary_label.clone())
                        .child(pro_icon(
                            if is_open {
                                ProIcon::ChevronDown
                            } else {
                                ProIcon::ChevronRight
                            },
                            12.,
                            p.faint,
                        )),
                    cx,
                    move |this, _, cx| this.toggle_tool_group(gid_toggle.clone(), cx),
                ));

                let calls_list = div()
                    .w_full()
                    .pt(px(2.))
                    .ml(px(6.))
                    .pl(px(12.))
                    .border_l_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .children(calls.iter().map(|call| {
                        let (st_ic, st_c) = match call.status {
                            ThreadToolStatus::Done => (ProIcon::Check, p.green),
                            ThreadToolStatus::Failed => (ProIcon::AlertTriangle, p.red),
                            ThreadToolStatus::Running => (ProIcon::Refresh, p.blue),
                            ThreadToolStatus::Stopped => (ProIcon::Minus, p.faint),
                        };
                        div()
                            .h(px(28.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(13.))
                            .text_color(if call.status == ThreadToolStatus::Running {
                                p.ink
                            } else {
                                p.text
                            })
                            .child(pro_icon(st_ic, 12., st_c))
                            .child(pro_icon(call.icon, 13., p.faint))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .child(call.label.clone()),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .text_size(px(12.))
                                    .text_color(p.faint)
                                    .child(call.duration_label.clone()),
                            )
                    }));
                let est_h = (calls.len() as f32) * 28. + 4.;
                grp = grp.child(pro_fold(
                    pro_eid(&self.id, format!("tools-fold-{gid}")),
                    is_open,
                    est_h,
                    0.,
                    move || calls_list.into_any_element(),
                ));
                grp.into_any_element()
            }
            ThreadPart::Task(task_card) => {
                let ring_c = if task_card.done {
                    p.green
                } else {
                    task_card.ring_tone.resolve(&p)
                };
                let open_tid = task_card.id.clone();
                let open_tid_btn = task_card.id.clone();
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("tcard-{msg_id}-{part_idx}")))
                        .role(Role::Button)
                        .tab_index(0)
                        .when(tight_top, |d| d.mt(px(-4.)))
                        .min_h(px(52.))
                        .pl(px(12.))
                        .pr(px(8.))
                        .py(px(8.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .hover(move |s| s.border_color(p.line_strong))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .cursor_pointer()
                        .child(thread_pie_ring(
                            task_card.progress(),
                            14.,
                            ring_c,
                            task_card.done,
                        ))
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
                                        .line_height(px(20.))
                                        .text_color(p.ink)
                                        .truncate()
                                        .child(task_card.title.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(task_card.status_text.clone()),
                                ),
                        )
                        .when(task_card.total_steps > 0, |d| {
                            d.child(
                                div()
                                    .flex_none()
                                    .text_size(px(12.))
                                    .text_color(p.faint)
                                    .child(format!(
                                        "{} / {}",
                                        task_card.done_steps, task_card.total_steps
                                    )),
                            )
                        })
                        .child(pro_btn(
                            pro_eid(&self.id, format!("tcard-open-{msg_id}-{part_idx}")),
                            tr(cx, "Open", "打开"),
                            None,
                            ProBtnStyle::Ghost,
                            true,
                            cx,
                            move |this, _, cx| this.open_task_in_panel(open_tid_btn.clone(), cx),
                        )),
                    cx,
                    move |this, _, cx| this.open_task_in_panel(open_tid.clone(), cx),
                )
                .into_any_element()
            }
            ThreadPart::Artifacts(names) => div()
                .mt(px(-2.))
                .flex()
                .flex_wrap()
                .gap(px(6.))
                .children(names.iter().enumerate().map(|(ai, fname)| {
                    let (svg_str, tint) = thread_file_glyph(fname, &p);
                    let f_open = fname.clone();
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("tart-{msg_id}-{part_idx}-{ai}")))
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
                            .child(pro_svg("0 0 12 12", svg_str, 12., tint))
                            .child(fname.clone()),
                        cx,
                        move |this, _, cx| this.open_artifact_in_panel(f_open.clone(), cx),
                    )
                }))
                .into_any_element(),
            ThreadPart::Asked {
                id: ask_id,
                question,
                answered,
            } => {
                let dot_c = if *answered { p.green } else { p.amber };
                let state_lbl = if *answered {
                    tr(cx, "· answered below", "· 已在下方回答")
                } else {
                    tr(cx, "· waiting", "· 等待确认")
                };
                let aid_c = ask_id.clone();
                let is_ans = *answered;
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("asked-{msg_id}-{part_idx}")))
                        .role(Role::Button)
                        .tab_index(0)
                        .min_h(px(28.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(13.))
                        .text_color(p.muted)
                        .cursor_pointer()
                        .child(div().mx(px(4.)).size(px(6.)).rounded_full().bg(dot_c))
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_color(p.text)
                                .child(format!("{}: {question}", tr(cx, "Asked", "询问"))),
                        )
                        .child(div().flex_none().text_color(p.faint).child(state_lbl)),
                    cx,
                    move |this, _, cx| {
                        if !is_ans {
                            if let Some(pos) = this.decisions.iter().position(|d| d.id == aid_c) {
                                this.dock_index = pos;
                                this.dock_state = DockState::Expanded;
                                cx.notify();
                            } else {
                                cx.emit(AgentThreadEvent::OpenInbox(Some(aid_c.clone())));
                            }
                        }
                    },
                )
                .into_any_element()
            }
            ThreadPart::Approval(card) => {
                let aid = card.id.clone();
                let aid_edit = card.id.clone();
                let aid_skip = card.id.clone();
                let aid_apply = card.id.clone();
                let is_editing = self.editing_approval_id.as_ref() == Some(&card.id);
                let inline_ed = self.inline_edit_editor.clone();

                let mut box_el = div()
                    .p(px(12.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .when_some(card.app.as_ref(), |d, app| {
                                d.child(pro_app_tile(app, 26., cx))
                            })
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
                                            .child(card.title.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.muted)
                                            .child(card.summary.clone()),
                                    ),
                            ),
                    );

                if !card.facts.is_empty() {
                    box_el = box_el.child(
                        div()
                            .px(px(8.))
                            .py(px(6.))
                            .rounded(px(8.))
                            .bg(p.chip)
                            .flex()
                            .flex_wrap()
                            .gap(px(14.))
                            .children(card.facts.iter().map(|(k, v)| {
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .text_size(px(12.))
                                    .child(div().text_color(p.muted).child(format!("{k}:")))
                                    .child(div().text_color(p.ink).child(v.clone()))
                            })),
                    );
                }

                if let Some(ref out) = card.outcome {
                    let (ic, c, lbl): (ProIcon, Hsla, SharedString) = match out {
                        ApprovalOutcome::Approved => (
                            ProIcon::Check,
                            p.green_ink,
                            tr(cx, "Approved", "已批准").into(),
                        ),
                        ApprovalOutcome::Edited(note) => (
                            ProIcon::Check,
                            p.blue,
                            if note.is_empty() {
                                tr(cx, "Approved with edits", "修改后已批准").into()
                            } else {
                                format!("{}: {note}", tr(cx, "Edited", "已修改")).into()
                            },
                        ),
                        ApprovalOutcome::Skipped => {
                            (ProIcon::Minus, p.muted, tr(cx, "Skipped", "已跳过").into())
                        }
                    };
                    box_el = box_el.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(c)
                            .child(pro_icon(ic, 13., c))
                            .child(lbl),
                    );
                } else {
                    let active_ed = if is_editing { inline_ed } else { None };
                    let edit_row = div().pb(px(8.)).child(
                        div()
                            .h(px(32.))
                            .px(px(8.))
                            .rounded(px(7.))
                            .bg(p.panel)
                            .border_1()
                            .border_color(p.line_strong)
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .when_some(active_ed, |d, ed| {
                                d.child(div().flex_1().min_w_0().h(px(26.)).child(ed))
                            })
                            .child(pro_btn(
                                pro_eid(&self.id, format!("appr-save-{msg_id}-{part_idx}")),
                                tr(cx, "Save & approve", "保存并批准"),
                                None,
                                ProBtnStyle::Primary,
                                true,
                                cx,
                                move |this, w, cx| {
                                    let note: SharedString = this
                                        .inline_edit_editor
                                        .as_ref()
                                        .map(|e| e.read(cx).value().trim().to_owned())
                                        .unwrap_or_default()
                                        .into();
                                    this.resolve_inline_approval(
                                        &aid_apply,
                                        ApprovalOutcome::Edited(note),
                                        cx,
                                    );
                                    if let Some(ed) = this.inline_edit_editor.clone() {
                                        ed.update(cx, |e, cx| e.set_value("", w, cx));
                                    }
                                },
                            )),
                    );
                    box_el = box_el.child(
                        div()
                            .flex()
                            .flex_col()
                            .child(pro_fold(
                                pro_eid(&self.id, format!("appr-edit-fold-{msg_id}-{part_idx}")),
                                is_editing,
                                40.,
                                0.,
                                move || edit_row.into_any_element(),
                            ))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_end()
                                    .gap(px(6.))
                                    .child(pro_btn(
                                        pro_eid(&self.id, format!("appr-skip-{msg_id}-{part_idx}")),
                                        tr(cx, "Skip", "跳过"),
                                        None,
                                        ProBtnStyle::Ghost,
                                        true,
                                        cx,
                                        move |this, _, cx| {
                                            this.resolve_inline_approval(
                                                &aid_skip,
                                                ApprovalOutcome::Skipped,
                                                cx,
                                            );
                                        },
                                    ))
                                    .child(pro_btn(
                                        pro_eid(&self.id, format!("appr-edit-{msg_id}-{part_idx}")),
                                        tr(cx, "Edit", "编辑"),
                                        None,
                                        ProBtnStyle::Secondary,
                                        true,
                                        cx,
                                        move |this, w, cx| {
                                            this.editing_approval_id = Some(aid_edit.clone());
                                            if let Some(ed) = this.inline_edit_editor.clone() {
                                                ed.update(cx, |e, cx| e.focus(w, cx));
                                            }
                                            cx.notify();
                                        },
                                    ))
                                    .child(pro_btn(
                                        pro_eid(&self.id, format!("appr-ok-{msg_id}-{part_idx}")),
                                        tr(cx, "Approve", "批准"),
                                        Some(ProIcon::Check),
                                        ProBtnStyle::Primary,
                                        true,
                                        cx,
                                        move |this, _, cx| {
                                            this.resolve_inline_approval(
                                                &aid,
                                                ApprovalOutcome::Approved,
                                                cx,
                                            );
                                        },
                                    )),
                            ),
                    );
                }
                box_el.into_any_element()
            }
            ThreadPart::Error { text, retryable } => {
                let rid = msg_id.clone();
                div()
                    .min_h(px(28.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .text_color(p.text)
                    .child(pro_icon(ProIcon::AlertTriangle, 14., p.red))
                    .child(div().flex_1().min_w_0().child(text.clone()))
                    .when(*retryable, |d| {
                        d.child(pro_btn(
                            pro_eid(&self.id, format!("err-retry-{msg_id}-{part_idx}")),
                            tr(cx, "Retry", "重试"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            move |_, _, cx| cx.emit(AgentThreadEvent::Retry(rid.clone())),
                        ))
                    })
                    .into_any_element()
            }
        }
    }

    fn render_dock(&self, cx: &Context<Self>) -> Option<(AnyElement, f32)> {
        let p = ProPalette::get(cx);
        if self.decisions.is_empty() || self.loading {
            return None;
        }
        let render_expanded = match self.dock_state {
            DockState::Expanded => true,
            DockState::Minimized => false,
            DockState::Hidden => self.last_dock_expanded,
        };
        if !render_expanded {
            let count = self.decisions.len();
            let text: SharedString = if count == 1 {
                tr(cx, "1 decision waiting", "1 项决定待确认").into()
            } else {
                format!("{count} {}", tr(cx, "decisions waiting", "项决定待确认")).into()
            };
            let min_pill = div()
                .h(px(36.))
                .pl(px(14.))
                .pr(px(8.))
                .rounded(px(14.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .gap(px(8.))
                .text_size(px(12.))
                .text_color(p.muted)
                .child(div().size(px(6.)).rounded_full().bg(p.amber))
                .child(div().flex_1().child(text))
                .child(pro_btn(
                    pro_eid(&self.id, "dock-show-btn"),
                    tr(cx, "Show", "展开"),
                    None,
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    |this, _, cx| {
                        this.dock_state = DockState::Expanded;
                        cx.notify();
                    },
                ));
            return Some((
                pro_view_transition(pro_eid(&self.id, "dock-mode-vtx"), "minimized", min_pill),
                44.,
            ));
        }

        let dec = self
            .decisions
            .get(self.dock_index)
            .or_else(|| self.decisions.first())?;
        let total = self.decisions.len();
        let cur_num = self.dock_index.min(total.saturating_sub(1)) + 1;
        let did_later = dec.id.clone();
        let custom_ed = if self.dock_state == DockState::Expanded {
            self.custom_answer_editor.clone()
        } else {
            None
        };
        let custom_val = self
            .custom_answer_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_string())
            .unwrap_or_default();

        let head_row = div()
            .h(px(24.))
            .px(px(4.))
            .flex()
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(dec.task_title.clone()),
            )
            .when(total > 1, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(2.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(pro_icon_btn(
                            pro_eid(&self.id, "dock-prev"),
                            tr(cx, "Previous decision", "上一项"),
                            ProIcon::ChevronLeft,
                            22.,
                            false,
                            cx,
                            |this, _, cx| {
                                let n = this.decisions.len().max(1);
                                this.dock_index = (this.dock_index + n - 1) % n;
                                cx.notify();
                            },
                        ))
                        .child(format!("{cur_num} of {total}"))
                        .child(pro_icon_btn(
                            pro_eid(&self.id, "dock-next"),
                            tr(cx, "Next decision", "下一项"),
                            ProIcon::ChevronRight,
                            22.,
                            false,
                            cx,
                            |this, _, cx| {
                                let n = this.decisions.len().max(1);
                                this.dock_index = (this.dock_index + 1) % n;
                                cx.notify();
                            },
                        )),
                )
            })
            .child(pro_btn(
                pro_eid(&self.id, "dock-later"),
                tr(cx, "Later", "稍后"),
                None,
                ProBtnStyle::Ghost,
                true,
                cx,
                move |this, _, cx| {
                    this.dock_state = DockState::Minimized;
                    cx.emit(AgentThreadEvent::Later(did_later.clone()));
                    cx.notify();
                },
            ));

        let mut dec_body = div().flex().flex_col().gap(px(6.)).child(
            div()
                .px(px(4.))
                .text_size(px(14.))
                .font_weight(FontWeight::MEDIUM)
                .line_height(px(20.))
                .text_color(p.ink)
                .child(dec.question.clone()),
        );

        if !dec.facts.is_empty() {
            dec_body = dec_body.child(div().px(px(4.)).flex().flex_col().gap(px(2.)).children(
                dec.facts.iter().map(|(k, v)| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(13.))
                        .child(div().w(px(72.)).text_color(p.muted).child(k.clone()))
                        .child(div().flex_1().truncate().text_color(p.ink).child(v.clone()))
                }),
            ));
        }

        if dec.is_approval {
            let skip_id = dec.id.clone();
            dec_body = dec_body.child(
                div()
                    .px(px(4.))
                    .pt(px(4.))
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(6.))
                    .child(pro_btn(
                        pro_eid(&self.id, "dock-appr-skip"),
                        tr(cx, "Skip", "跳过"),
                        None,
                        ProBtnStyle::Ghost,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.dock_state = DockState::Minimized;
                            cx.emit(AgentThreadEvent::Later(skip_id.clone()));
                            cx.notify();
                        },
                    ))
                    .child(pro_btn(
                        pro_eid(&self.id, "dock-appr-edit"),
                        tr(cx, "Edit", "编辑"),
                        None,
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, _, cx| {
                            this.answer_docked_decision("Approved with rollout cap", cx);
                        },
                    ))
                    .child(pro_btn(
                        pro_eid(&self.id, "dock-appr-ok"),
                        tr(cx, "Approve", "批准"),
                        Some(ProIcon::Check),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        |this, _, cx| this.answer_docked_decision("Approved", cx),
                    )),
            );
        } else {
            let mut list = div().flex().flex_col().gap(px(2.));
            for (oi, opt) in dec.options.iter().enumerate() {
                let focused_row = oi == self.dock_focus_row;
                let ans_title = opt.title.clone();
                list = list.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("dock-opt-{oi}")))
                        .role(Role::ListBoxOption)
                        .tab_index(0)
                        .min_h(px(38.))
                        .pl(px(4.))
                        .pr(px(8.))
                        .py(px(4.))
                        .rounded(px(8.))
                        .when(focused_row, |d| d.bg(p.hover))
                        .hover(move |s| s.bg(p.chip))
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .cursor_pointer()
                        .child(
                            div()
                                .size(px(22.))
                                .rounded(px(6.))
                                .bg(p.chip)
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .child((oi + 1).to_string()),
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
                                        .child(opt.title.clone()),
                                )
                                .when(!opt.summary.is_empty(), |d| {
                                    d.child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(opt.summary.clone()),
                                    )
                                }),
                        )
                        .when(opt.recommended, |d| {
                            d.child(
                                div()
                                    .px(px(6.))
                                    .h(px(18.))
                                    .rounded(px(4.))
                                    .bg(p.chip)
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .flex()
                                    .items_center()
                                    .child(tr(cx, "Recommended", "推荐")),
                            )
                        }),
                    cx,
                    move |this, _, cx| {
                        this.answer_docked_decision(ans_title.clone(), cx);
                    },
                ));
            }

            // Custom "Something else…" row
            let next_num = dec.options.len() + 1;
            list = list.child(
                div()
                    .min_h(px(36.))
                    .px(px(4.))
                    .rounded(px(8.))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .size(px(22.))
                            .rounded(px(6.))
                            .bg(p.chip)
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(next_num.to_string()),
                    )
                    .when_some(custom_ed, |d, ed| {
                        d.child(div().flex_1().min_w_0().h(px(28.)).child(ed))
                    })
                    .when(!custom_val.is_empty(), |d| {
                        d.child(activate(
                            div()
                                .id(pro_eid(&self.id, "dock-custom-send"))
                                .role(Role::Button)
                                .tab_index(0)
                                .size(px(24.))
                                .rounded_full()
                                .bg(p.primary)
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .child(pro_icon(ProIcon::ArrowUp, 13., p.on_primary)),
                            cx,
                            |this, w, cx| {
                                if let Some(ed) = this.custom_answer_editor.clone() {
                                    let val = ed.read(cx).value().to_string();
                                    this.answer_docked_decision(val, cx);
                                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                                }
                            },
                        ))
                    }),
            );
            dec_body = dec_body.child(list);
        }

        let dec_vtx_key: SharedString = format!("{}:{}", self.dock_index, dec.id).into();
        let card = div()
            .p(px(10.))
            .pb(px(8.))
            .rounded(px(16.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(head_row)
            .child(pro_view_transition(
                pro_eid(&self.id, "dock-dec-vtx"),
                dec_vtx_key,
                dec_body,
            ));

        let est_h = if dec.is_approval {
            136. + (dec.facts.len() as f32) * 20.
        } else {
            116. + (dec.options.len() as f32) * 42.
        };
        Some((
            pro_view_transition(pro_eid(&self.id, "dock-mode-vtx"), "expanded", card),
            est_h,
        ))
    }

    fn render_footer(&self, side_pad: f32, elapsed_ms: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let streaming = self.is_streaming();
        let draft = self
            .composer_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_string())
            .unwrap_or_default();
        let can_send = !self.loading && (!draft.is_empty() || !self.attachments.is_empty());
        let ed_clone = self.composer_editor.clone();

        let mut foot_col = div()
            .w_full()
            .max_w(px(680.))
            .mx_auto()
            .relative()
            .flex()
            .flex_col();

        let dock_visible =
            !self.decisions.is_empty() && !self.loading && self.dock_state != DockState::Hidden;
        if let Some((dock_el, dock_est)) = self.render_dock(cx) {
            foot_col = foot_col.child(pro_fold(
                pro_eid(&self.id, "dock-fold"),
                dock_visible,
                dock_est + 8.,
                0.,
                move || div().pb(px(8.)).child(dock_el).into_any_element(),
            ));
        }

        if !self.attachments.is_empty() {
            foot_col = foot_col.child(div().pb(px(8.)).flex().flex_wrap().gap(px(6.)).children(
                self.attachments.iter().enumerate().map(|(ai, att)| {
                    div()
                        .h(px(36.))
                        .pl(px(8.))
                        .pr(px(6.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .size(px(24.))
                                .rounded(px(6.))
                                .bg(p.chip)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(pro_icon(ProIcon::PageDoc, 13., p.muted)),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(att.name.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(p.muted)
                                        .child(att.size_label.clone()),
                                ),
                        )
                        .child(pro_icon_btn(
                            pro_eid(&self.id, format!("rm-att-{ai}")),
                            tr(cx, "Remove attachment", "移除附件"),
                            ProIcon::Close,
                            22.,
                            false,
                            cx,
                            move |this, _, cx| {
                                if ai < this.attachments.len() {
                                    this.attachments.remove(ai);
                                    cx.notify();
                                }
                            },
                        ))
                }),
            ));
        }

        // Composer popovers (+ menu, Request approval menu, optional Model menu)
        let pop = self.active_popover.unwrap_or(self.last_popover);
        let pop_open = self.active_popover.is_some();
        let pop_right_align = pop == ComposerPopover::ModelMenu;
        let pop_vtx_key: SharedString = format!("{pop:?}").into();
        let pop_vtx_eid = pro_eid(&self.id, "composer-pop-vtx");
        let menu_el = match pop {
            ComposerPopover::AddMenu => div()
                .id(pro_eid(&self.id, "add-menu"))
                .role(Role::Menu)
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
                        .text_color(p.muted)
                        .child(tr(cx, "Add to the chat", "添加到对话")),
                )
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "add-file-item"))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(32.))
                        .px(px(8.))
                        .rounded(px(7.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .gap(px(9.))
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Paperclip, 15., p.muted))
                        .child(tr(cx, "Attach file", "上传文件")),
                    cx,
                    |this, _, cx| {
                        this.attachments
                            .push(ThreadAttachment::ready("staging-config.toml", "14 KB"));
                        this.active_popover = None;
                        cx.notify();
                    },
                ))
                .child(div().my(px(3.)).mx(px(6.)).h(px(1.)).bg(p.line))
                .children(
                    ["Gmail", "Google Drive", "Slack", "Linear"]
                        .into_iter()
                        .enumerate()
                        .map(|(pi, app_name)| {
                            activate(
                                div()
                                    .id(pro_eid(&self.id, format!("add-app-{pi}")))
                                    .role(Role::MenuItem)
                                    .tab_index(0)
                                    .h(px(32.))
                                    .px(px(8.))
                                    .rounded(px(7.))
                                    .hover(move |s| s.bg(p.hover))
                                    .flex()
                                    .items_center()
                                    .gap(px(9.))
                                    .text_size(px(13.))
                                    .text_color(p.ink)
                                    .cursor_pointer()
                                    .child(pro_app_tile(app_name, 18., cx))
                                    .child(app_name),
                                cx,
                                move |this, _, cx| {
                                    this.attachments.push(ThreadAttachment::ready(
                                        format!("{app_name} context"),
                                        "Connected",
                                    ));
                                    this.active_popover = None;
                                    cx.notify();
                                },
                            )
                        }),
                ),
            ComposerPopover::ApprovalMenu => div()
                .id(pro_eid(&self.id, "approval-menu"))
                .role(Role::ListBox)
                .w(px(256.))
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
                        .text_color(p.muted)
                        .child(tr(
                            cx,
                            "How should actions be approved?",
                            "如何审批外部操作？",
                        )),
                )
                .children(
                    [
                        (
                            ApprovalPolicy::RequestApproval,
                            tr(cx, "Request approval", "关键操作需审批"),
                            tr(cx, "Ask before writes or deploys", "写入或部署前请求确认"),
                            false,
                        ),
                        (
                            ApprovalPolicy::AutoSafe,
                            tr(cx, "Auto-approve reads", "只读操作自动批准"),
                            tr(cx, "Run queries without pausing", "安全查询无需暂停"),
                            false,
                        ),
                        (
                            ApprovalPolicy::FullAutonomy,
                            tr(cx, "Full autonomy", "全自动执行"),
                            tr(cx, "Execute without prompts", "无需确认直接执行"),
                            true,
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(ai, (pol, title, desc, alert))| {
                        let selected = self.approval_policy == pol;
                        activate(
                            div()
                                .id(pro_eid(&self.id, format!("pol-opt-{ai}")))
                                .role(Role::ListBoxOption)
                                .tab_index(0)
                                .px(px(8.))
                                .py(px(6.))
                                .rounded(px(7.))
                                .hover(move |s| s.bg(p.hover))
                                .flex()
                                .items_start()
                                .gap(px(8.))
                                .cursor_pointer()
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .font_weight(if selected {
                                                    FontWeight::MEDIUM
                                                } else {
                                                    FontWeight::NORMAL
                                                })
                                                .text_color(if alert { p.red_ink } else { p.ink })
                                                .child(title),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(p.muted)
                                                .child(desc),
                                        ),
                                )
                                .when(selected, |d| d.child(pro_icon(ProIcon::Check, 14., p.ink))),
                            cx,
                            move |this, _, cx| {
                                this.approval_policy = pol;
                                this.active_popover = None;
                                cx.notify();
                            },
                        )
                    }),
                ),
            ComposerPopover::ModelMenu => div()
                .id(pro_eid(&self.id, "model-menu"))
                .role(Role::ListBox)
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
                .children(self.models.iter().enumerate().map(|(mi, mdl)| {
                    let chosen = mi == self.selected_model;
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("mdl-opt-{mi}")))
                            .role(Role::ListBoxOption)
                            .tab_index(0)
                            .h(px(32.))
                            .px(px(8.))
                            .rounded(px(7.))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(13.))
                            .text_color(p.ink)
                            .cursor_pointer()
                            .child(pro_app_tile(&mdl.provider, 18., cx))
                            .child(div().flex_1().truncate().child(mdl.name.clone()))
                            .when(chosen, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink))),
                        cx,
                        move |this, _, cx| {
                            this.selected_model = mi;
                            this.active_popover = None;
                            cx.notify();
                        },
                    )
                })),
        };
        let pop_anchor = div().relative().w_full().h(px(0.)).child(
            div()
                .absolute()
                .bottom(px(8.))
                .when(pop_right_align, |d| d.right(px(0.)))
                .when(!pop_right_align, |d| d.left(px(0.)))
                .child(pro_presence(
                    pro_eid(&self.id, "composer-pop-pres"),
                    pop_open,
                    0.16,
                    0.12,
                    move |amount| {
                        div()
                            .mb(px(-6. * (1. - amount)))
                            .opacity(amount)
                            .child(pro_view_transition(pop_vtx_eid, pop_vtx_key, menu_el))
                            .into_any_element()
                    },
                )),
        );
        foot_col = foot_col.child(pop_anchor);

        let is_alert_pol = self.approval_policy == ApprovalPolicy::FullAutonomy;
        let pol_fg = if is_alert_pol { p.red_ink } else { p.muted };

        let right_controls = if self.recording {
            let rec_s = self
                .recording_started
                .map(|t| t.elapsed().as_secs())
                .unwrap_or(0);
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(div().size(px(6.)).rounded_full().bg(p.red))
                .child(div().h(px(20.)).flex().items_center().gap(px(2.)).children(
                    (0..12usize).map(|bi| {
                        let h = if reduced {
                            8.
                        } else {
                            4. + 12. * ((elapsed_ms / 180. + bi as f32 * 0.7).sin().abs())
                        };
                        div().w(px(2.)).h(px(h)).rounded(px(1.)).bg(p.blue)
                    }),
                ))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(format!("0:{rec_s:02}")),
                )
                .child(pro_icon_btn(
                    pro_eid(&self.id, "voice-cancel"),
                    tr(cx, "Discard recording", "放弃录音"),
                    ProIcon::Close,
                    28.,
                    false,
                    cx,
                    |this, _, cx| {
                        this.recording = false;
                        this.recording_started = None;
                        cx.notify();
                    },
                ))
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "voice-commit"))
                        .role(Role::Button)
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
                    |this, w, cx| {
                        this.recording = false;
                        this.recording_started = None;
                        if let Some(ed) = this.composer_editor.clone() {
                            ed.update(cx, |e, cx| {
                                e.set_value(
                                    "Use Kafka with 24-hour retention and ship staging first.",
                                    w,
                                    cx,
                                );
                            });
                        }
                        cx.notify();
                    },
                ))
        } else {
            div()
                .flex()
                .items_center()
                .gap(px(4.))
                .when(!self.models.is_empty(), |d| {
                    let cur_mdl = self.models.get(self.selected_model).cloned().unwrap_or(
                        ThreadModelOption {
                            id: "auto".into(),
                            name: "Auto".into(),
                            provider: "Claude".into(),
                        },
                    );
                    d.child(activate(
                        div()
                            .id(pro_eid(&self.id, "composer-model-btn"))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(28.))
                            .px(px(8.))
                            .rounded(px(8.))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.5))
                            .text_color(p.ink)
                            .cursor_pointer()
                            .child(pro_app_tile(&cur_mdl.provider, 16., cx))
                            .child(cur_mdl.name)
                            .child(pro_icon(ProIcon::ChevronDown, 12., p.muted)),
                        cx,
                        |this, _, cx| {
                            this.active_popover =
                                if this.active_popover == Some(ComposerPopover::ModelMenu) {
                                    None
                                } else {
                                    Some(ComposerPopover::ModelMenu)
                                };
                            cx.notify();
                        },
                    ))
                })
                .child(pro_icon_btn(
                    pro_eid(&self.id, "composer-mic"),
                    tr(cx, "Dictate voice note", "语音输入"),
                    ProIcon::Mic,
                    28.,
                    false,
                    cx,
                    |this, _, cx| {
                        this.recording = true;
                        this.recording_started = Some(Instant::now());
                        cx.notify();
                    },
                ))
                .child(if streaming {
                    activate(
                        div()
                            .id(pro_eid(&self.id, "composer-stop"))
                            .role(Role::Button)
                            .aria_label(tr(cx, "Stop answer", "停止生成"))
                            .tab_index(0)
                            .size(px(28.))
                            .rounded(px(8.))
                            .bg(p.chip_hover)
                            .hover(move |s| s.bg(p.line_strong))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .child(div().size(px(9.)).rounded(px(2.)).bg(p.ink)),
                        cx,
                        |this, _, cx| this.stop_streaming(cx),
                    )
                } else {
                    activate(
                        div()
                            .id(pro_eid(&self.id, "composer-send"))
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
                            .child(pro_icon(
                                ProIcon::ArrowUp,
                                15.,
                                if can_send { p.on_primary } else { p.faint },
                            )),
                        cx,
                        |this, w, cx| {
                            if let Some(ed) = this.composer_editor.clone() {
                                let val = ed.read(cx).value().to_string();
                                if this.send_prompt(val, cx) {
                                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                                }
                            }
                        },
                    )
                })
        };

        let composer_box = div()
            .w_full()
            .pt(px(12.))
            .pb(px(10.))
            .px(px(10.))
            .rounded(px(16.))
            .bg(p.card)
            .border_1()
            .border_color(if self.editor_focused {
                p.line_strong
            } else {
                p.line
            })
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .px(px(4.))
                    .h(px(40.))
                    .when_some(ed_clone, |d, ed| d.child(ed)),
            )
            .child(
                div()
                    .h(px(28.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(pro_icon_btn(
                                pro_eid(&self.id, "composer-plus"),
                                tr(cx, "Add attachment or app", "添加附件或连接应用"),
                                ProIcon::Plus,
                                28.,
                                self.active_popover == Some(ComposerPopover::AddMenu),
                                cx,
                                |this, _, cx| {
                                    this.active_popover =
                                        if this.active_popover == Some(ComposerPopover::AddMenu) {
                                            None
                                        } else {
                                            Some(ComposerPopover::AddMenu)
                                        };
                                    cx.notify();
                                },
                            ))
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, "composer-approval"))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .h(px(28.))
                                    .pl(px(8.))
                                    .pr(px(10.))
                                    .rounded(px(8.))
                                    .when(
                                        self.active_popover == Some(ComposerPopover::ApprovalMenu),
                                        |d| d.bg(p.hover),
                                    )
                                    .hover(move |s| s.bg(p.hover).text_color(p.ink))
                                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(13.))
                                    .text_color(pol_fg)
                                    .cursor_pointer()
                                    .child(pro_svg(
                                        "0 0 68.155 68.155",
                                        HAND_APPROVAL_SVG,
                                        15.,
                                        pol_fg,
                                    ))
                                    .child(self.approval_policy.label(cx)),
                                cx,
                                |this, _, cx| {
                                    this.active_popover = if this.active_popover
                                        == Some(ComposerPopover::ApprovalMenu)
                                    {
                                        None
                                    } else {
                                        Some(ComposerPopover::ApprovalMenu)
                                    };
                                    cx.notify();
                                },
                            )),
                    )
                    .child(right_controls),
            );

        foot_col = foot_col.child(composer_box);

        div()
            .flex_none()
            .px(px(side_pad))
            .pt(px(4.))
            .pb(px(if side_pad < 16. { 10. } else { 20. }))
            .child(foot_col)
    }

    fn render_loading_stream(side_pad: f32, phase: f32, cx: &App) -> Div {
        div()
            .w_full()
            .max_w(px(680. + side_pad * 2.))
            .mx_auto()
            .px(px(side_pad))
            .pt(px(36.))
            .pb(px(28.))
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(
                div()
                    .w_full()
                    .flex()
                    .justify_end()
                    .child(pro_skeleton_bar(290., 42., 16., phase, cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(pro_skeleton_bar(16., 16., 4., phase, cx))
                            .child(pro_skeleton_bar(86., 12., 4., phase + 0.05, cx)),
                    )
                    .child(pro_skeleton_bar(124., 14., 6., phase + 0.1, cx))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(pro_skeleton_bar(420., 13., 4., phase + 0.15, cx))
                            .child(pro_skeleton_bar(260., 13., 4., phase + 0.2, cx)),
                    )
                    .child(pro_skeleton_bar(420., 52., 10., phase + 0.25, cx))
                    .child(pro_skeleton_bar(420., 52., 10., phase + 0.3, cx)),
            )
            .child(div().w_full().flex().justify_end().child(pro_skeleton_bar(
                196.,
                34.,
                16.,
                phase + 0.35,
                cx,
            )))
    }

    fn render_work_panel(
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
                .gap(px(14.))
                .child(pro_skeleton_bar(210., 13., 4., phase, cx))
                .child(pro_skeleton_bar(165., 13., 4., phase + 0.1, cx))
                .child(pro_skeleton_bar(260., 96., 10., phase + 0.2, cx))
                .into_any_element();
            return Some((
                tr(cx, "Artifacts", "工作产物").into(),
                ProIcon::PageDoc,
                skel,
            ));
        }

        let current_tab = self
            .work_tabs
            .get(self.active_work_tab)
            .cloned()
            .unwrap_or_else(|| WorkPaneTab::Artifact("load-test-results.csv".into()));

        let tab_strip = (self.work_tabs.len() > 1).then(|| {
            let wtab_sensor = pro_selection_sensor();
            let wtab_key: SharedString = format!("wtab-{}", self.active_work_tab).into();
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
                    pro_eid(&self.id, "wtab-pill-ind"),
                    wtab_key,
                    wtab_sensor.clone(),
                    6.,
                    p.chip,
                    None,
                    false,
                ))
                .children(
                    self.work_tabs
                        .clone()
                        .into_iter()
                        .enumerate()
                        .map(|(ti, wt)| {
                            let is_cur = ti == self.active_work_tab;
                            let (lbl, ic): (SharedString, ProIcon) = match &wt {
                                WorkPaneTab::Task(tid) => {
                                    (task_detail_by_id(tid).title, ProIcon::Tasks)
                                }
                                WorkPaneTab::Artifact(name) => (name.clone(), ProIcon::PageDoc),
                            };
                            let wtab_sensor_item = wtab_sensor.clone();
                            div()
                                .relative()
                                .h(px(26.))
                                .pl(px(7.))
                                .pr(px(4.))
                                .rounded(px(6.))
                                .flex()
                                .items_center()
                                .gap(px(5.))
                                .when(is_cur, |d| d.child(pro_mark_selected(&wtab_sensor_item)))
                                .child(activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("wtab-{ti}")))
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
                                        this.active_work_tab = ti;
                                        cx.notify();
                                    },
                                ))
                                .child(activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("wtab-close-{ti}")))
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
                                    move |this, _, cx| this.close_work_tab(ti, cx),
                                ))
                        }),
                )
        });

        match current_tab {
            WorkPaneTab::Task(tid) => {
                let pane_key: SharedString = format!("task:{tid}").into();
                let rec = task_detail_by_id(&tid);
                let title = rec.title.clone();
                let pane = self.render_task_work_pane(rec, elapsed_ms, cx);
                let full = div()
                    .flex()
                    .flex_col()
                    .when_some(tab_strip, |d, ts| d.child(ts))
                    .child(pro_view_transition(
                        pro_eid(&self.id, "work-pane-vtx"),
                        pane_key,
                        pane,
                    ))
                    .into_any_element();
                Some((title, ProIcon::Tasks, full))
            }
            WorkPaneTab::Artifact(name) => {
                let pane_key: SharedString = format!("art:{name}").into();
                let pane = Self::render_artifact_work_pane(&name, cx);
                let full = div()
                    .flex()
                    .flex_col()
                    .when_some(tab_strip, |d, ts| d.child(ts))
                    .child(pro_view_transition(
                        pro_eid(&self.id, "work-pane-vtx"),
                        pane_key,
                        pane,
                    ))
                    .into_any_element();
                Some((name, ProIcon::PageDoc, full))
            }
        }
    }

    fn render_task_work_pane(
        &self,
        rec: TaskDetailRecord,
        elapsed_ms: f32,
        cx: &Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let done_n = rec.steps.iter().filter(|(_, d)| *d).count();
        let total_n = rec.steps.len();
        let pct = if total_n == 0 {
            0.0
        } else {
            done_n as f32 / total_n as f32
        };
        let ring_c = rec.ring_tone.resolve(&p);
        let show_tid = rec.id.clone();

        let mut col = div().p(px(14.)).flex().flex_col().gap(px(18.)).child(
            div()
                .text_size(px(13.))
                .line_height(px(20.))
                .text_color(p.text)
                .child(rec.goal),
        );

        if let Some(q) = rec.ask_question {
            let wait_bg = if p.dark {
                hsla(25. / 360., 1.0, 0.49, 0.145)
            } else {
                hsla(37. / 360., 1.0, 0.50, 0.16)
            };
            col = col.child(
                div()
                    .p(px(10.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(3.))
                            .child(
                                div()
                                    .px(px(6.))
                                    .h(px(18.))
                                    .w(px(68.))
                                    .rounded(px(4.))
                                    .bg(wait_bg)
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.amber_ink)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(tr(cx, "Needs you", "待确认")),
                            )
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(19.))
                                    .text_color(p.ink)
                                    .child(q),
                            ),
                    )
                    .child(pro_btn(
                        pro_eid(&self.id, "tp-decide-btn"),
                        tr(cx, "Decide", "去决定"),
                        None,
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        |this, _, cx| {
                            if !this.decisions.is_empty() {
                                this.dock_state = DockState::Expanded;
                                cx.notify();
                            } else {
                                cx.emit(AgentThreadEvent::OpenInbox(None));
                            }
                        },
                    )),
            );
        }

        col = col.child(
            div()
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
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child(tr(cx, "Status", "状态")),
                        )
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(thread_pie_ring(pct, 14., ring_c, false))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(p.ink)
                                        .truncate()
                                        .child(rec.status_text),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.5))
                                        .text_color(p.faint)
                                        .child(format!("{done_n} / {total_n}")),
                                ),
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
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child(tr(cx, "Project", "项目")),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(p.ink)
                                .child(rec.project),
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
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child(tr(cx, "Started", "开始于")),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(p.ink)
                                .child(rec.started),
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
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child(tr(cx, "Worker", "执行代理")),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(pro_agent_avatar(
                                    "Orchestrator",
                                    p.purple,
                                    18.,
                                    true,
                                    elapsed_ms,
                                    cx,
                                ))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(p.ink)
                                        .child("Orchestrator"),
                                ),
                        ),
                ),
        );

        let first_undone = rec.steps.iter().position(|(_, d)| !*d);
        let tray_bg = if p.dark {
            hsla(0., 0., 1., 0.025)
        } else {
            hsla(0., 0., 0., 0.022)
        };
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
                                .child(format!("{done_n}/{total_n}")),
                        ),
                )
                .child(pro_spring_bar(
                    pro_eid(&self.id, "tp-steps-prog"),
                    pct,
                    5.,
                    2.5,
                    p.line_soft,
                    ring_c,
                ))
                .child(
                    div()
                        .px(px(10.))
                        .py(px(6.))
                        .rounded(px(10.))
                        .bg(tray_bg)
                        .flex()
                        .flex_col()
                        .children(rec.steps.into_iter().enumerate().map(|(si, (lbl, done))| {
                            let active = Some(si) == first_undone;
                            div()
                                .min_h(px(28.))
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .child(if done {
                                    div()
                                        .size(px(15.))
                                        .rounded_full()
                                        .bg(p.blue)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(pro_icon(ProIcon::Check, 10., rgb(0xffffff).into()))
                                } else if active {
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
                                })
                                .child(
                                    div()
                                        .flex_1()
                                        .truncate()
                                        .text_size(px(13.))
                                        .text_color(if active { p.ink } else { p.muted })
                                        .child(lbl),
                                )
                        })),
                ),
        );

        if !rec.history.is_empty() {
            col = col.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "What happened so far", "进展记录")),
                    )
                    .children(rec.history.into_iter().enumerate().map(|(hi, line)| {
                        div()
                            .flex()
                            .items_start()
                            .gap(px(8.))
                            .child(
                                div()
                                    .mt(px(8.))
                                    .ml(px(4.))
                                    .size(px(4.))
                                    .rounded_full()
                                    .bg(p.faint),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(13.))
                                    .line_height(px(19.))
                                    .text_color(if hi == 0 { p.ink } else { p.text })
                                    .child(line),
                            )
                    })),
            );
        }

        if !rec.artifacts.is_empty() {
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
                            .child(tr(cx, "Artifacts", "产物")),
                    )
                    .child(div().flex().flex_wrap().gap(px(6.)).children(
                        rec.artifacts.into_iter().enumerate().map(|(ai, aname)| {
                            let (svg_str, tint) = thread_file_glyph(&aname, &p);
                            let open_n = aname.clone();
                            activate(
                                div()
                                    .id(pro_eid(&self.id, format!("tp-art-{ai}")))
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
                                    .child(pro_svg("0 0 12 12", svg_str, 12., tint))
                                    .child(aname),
                                cx,
                                move |this, _, cx| {
                                    this.open_artifact_in_panel(open_n.clone(), cx);
                                },
                            )
                        }),
                    )),
            );
        }

        col = col.child(
            div().flex().justify_end().child(activate(
                div()
                    .id(pro_eid(&self.id, "tp-show-in-tasks"))
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
                    .child(tr(cx, "Show in Tasks", "在任务列表中查看"))
                    .child(pro_icon(ProIcon::ChevronRight, 12., p.faint)),
                cx,
                move |_, _, cx| {
                    cx.emit(AgentThreadEvent::ShowInTasks(show_tid.clone()));
                    cx.emit(AgentThreadEvent::Navigate("Tasks".into()));
                },
            )),
        );
        col
    }

    fn render_artifact_work_pane(name: &str, cx: &App) -> Div {
        let p = ProPalette::get(cx);
        let (svg_str, tint) = thread_file_glyph(name, &p);
        let is_csv = name.ends_with(".csv");

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
                        .child(pro_svg("0 0 12 12", svg_str, 14., tint)),
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
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child("Updated at 8:01 AM · Northwind"),
                        ),
                ),
        );

        if is_csv {
            let rows = [
                ["Rate / min", "p50", "p95", "Errors"],
                ["10,000", "18 ms", "42 ms", "0.00%"],
                ["20,000", "31 ms", "84 ms", "0.02%"],
                ["30,000", "64 ms", "178 ms", "0.19%"],
                ["40,000", "112 ms", "340 ms", "1.14%"],
            ];
            pane = pane.child(
                div()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(p.line)
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .children(rows.into_iter().enumerate().map(|(ri, r)| {
                        div()
                            .h(px(28.))
                            .px(px(8.))
                            .when(ri == 0, |d| d.bg(p.chip))
                            .when(ri > 0, |d| d.border_t_1().border_color(p.line_soft))
                            .flex()
                            .items_center()
                            .children(r.into_iter().enumerate().map(|(ci, cell)| {
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
                                    .child(cell)
                            }))
                    })),
            );
        } else {
            pane = pane
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child("Northwind Ingest — Design & Replay Plan"),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.text)
                        .child("Order webhook receivers validate signatures at the edge and append versioned JSON envelopes into a 24-hour replay buffer. At 20,000 events/min, p95 stays at 84 ms with 0.02% transient retries."),
                );
        }

        pane
    }
}

impl Render for AgentThreadScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

/// Bare 16x16 3x3 mirrored purple pixel mark for the Orchestrator author header.
fn orchestrator_pixel_mark(color: Hsla, busy: bool, elapsed_ms: f32, reduced: bool) -> Div {
    let cells = agent_pixel_cells("Orchestrator");
    let mut grid = div()
        .size(px(16.))
        .flex_none()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(1.5));
    for r in 0..3usize {
        let mut row = div().flex().items_center().gap(px(1.5));
        for c in 0..3usize {
            let i = r * 3 + c;
            let on = cells[i];
            let alpha = if busy && !reduced {
                let phase = ((elapsed_ms + AGENT_WAVE_MS[i]) / 1200.) * std::f32::consts::TAU;
                let wave = 0.55 + 0.45 * phase.cos();
                if on { wave } else { 0.16 * wave }
            } else if on {
                1.0
            } else {
                0.0
            };
            row = row.child(
                div()
                    .size(px(3.))
                    .rounded(px(0.75))
                    .bg(color.opacity(alpha)),
            );
        }
        grid = grid.child(row);
    }
    grid
}

fn thread_pie_ring(progress: f32, side: f32, color: Hsla, done: bool) -> AnyElement {
    let pct = progress.clamp(0., 1.);
    if done {
        return div()
            .relative()
            .size(px(side))
            .flex_none()
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

fn thread_file_glyph(name: &str, p: &ProPalette) -> (&'static str, Hsla) {
    let lower = name.trim().to_ascii_lowercase();
    if lower.ends_with(".csv") || lower.ends_with(".xlsx") {
        (THREAD_SHEET_SVG, p.green)
    } else if lower.ends_with(".json") || lower.ends_with(".ts") || lower.ends_with(".toml") {
        (THREAD_CODE_SVG, p.orange)
    } else {
        (THREAD_DOC_SVG, p.blue)
    }
}

fn task_detail_by_id(id: &str) -> TaskDetailRecord {
    match id {
        "migration" => TaskDetailRecord {
            id: "migration".into(),
            title: "API migration".into(),
            project: "Platform".into(),
            started: "Fri, Oct 31".into(),
            status_text: "Moving our own apps".into(),
            ring_tone: ThreadRingTone::Orange,
            goal: "Migrate internal services and partner webhooks off legacy v1 routes onto typed v2 endpoints.".into(),
            ask_question: Some("When should we sunset the legacy v1 endpoints?".into()),
            steps: vec![
                ("Inventory remaining v1 callers".into(), true),
                ("Move our own apps onto v2".into(), false),
                ("Pick the v1 sunset date".into(), false),
                ("Send partner deprecation notice".into(), false),
            ],
            history: vec![
                "Moving 6 internal workers onto v2 endpoints".into(),
                "Exported v1-caller-audit.csv across last 14 days".into(),
            ],
            artifacts: vec!["v1-caller-audit.csv".into(), "deprecation-notice.md".into()],
        },
        "ingest" => TaskDetailRecord {
            id: "ingest".into(),
            title: "Ingest pipeline".into(),
            project: "Northwind".into(),
            started: "Mon, Nov 3".into(),
            status_text: "Picking a queue".into(),
            ring_tone: ThreadRingTone::Amber,
            goal: "Take in Northwind's order events and keep a day of them to replay after a bad deploy.".into(),
            ask_question: Some("Which queue should the ingest service use?".into()),
            steps: vec![
                ("Draft the event schema".into(), true),
                ("Load test at 20k a minute".into(), true),
                ("Pick a queue".into(), false),
                ("Deploy to staging".into(), false),
            ],
            history: vec![
                "Passed load test at 20k events/min (p95 84 ms)".into(),
                "Wrote ingest-design.md and event-schema.json".into(),
            ],
            artifacts: vec!["load-test-results.csv".into(), "ingest-design.md".into()],
        },
        _ => TaskDetailRecord {
            id: "dashboard".into(),
            title: "Client dashboard".into(),
            project: "Northwind".into(),
            started: "Mon, Nov 3".into(),
            status_text: "Drawing the empty states".into(),
            ring_tone: ThreadRingTone::Amber,
            goal: "Give Northwind account leads a live dashboard for event throughput, retry rates, and SLA health.".into(),
            ask_question: Some("Which empty state layout fits first-time workspaces?".into()),
            steps: vec![
                ("Map metrics and query latency".into(), true),
                ("Draw the empty states".into(), false),
                ("Wire live customer filters".into(), false),
            ],
            history: vec![
                "Drafting three candidate empty states for first-run workspaces".into(),
                "Verified dashboard query p95 under 45 ms".into(),
            ],
            artifacts: vec!["ingest-design.md".into()],
        },
    }
}

fn sample_thread_messages() -> Vec<ThreadMessage> {
    vec![
        ThreadMessage::User {
            id: "m1".into(),
            context_note: None,
            text: "Morning. Northwind first this week – the ingest pipeline, the client dashboard and the weekly update. Keep the API migration moving on the side.".into(),
            attachments: Vec::new(),
            status: UserMessageStatus::Sent,
        },
        ThreadMessage::Agent {
            id: "m2".into(),
            at: "5:21 AM".into(),
            parts: vec![
                ThreadPart::Tools {
                    id: "tg-1".into(),
                    summary_label: "Ran 3 tools · 2.1s".into(),
                    calls: vec![
                        ThreadToolCall::done(
                            "t1-1",
                            "Read Northwind Q4 priorities and backlog",
                            ProIcon::PageDoc,
                            "0.6s",
                        ),
                        ThreadToolCall::done(
                            "t1-2",
                            "Created 4 tasks across Northwind and Platform",
                            ProIcon::Tasks,
                            "0.9s",
                        ),
                        ThreadToolCall::done(
                            "t1-3",
                            "Dispatched background workers in parallel",
                            ProIcon::Thread,
                            "2.1s",
                        ),
                    ],
                },
                ThreadPart::Text(
                    "On it. I split it into four tasks and started them side by side – you'll only hear from me when something needs a decision.".into(),
                ),
                ThreadPart::Task(ThreadTaskCard {
                    id: "dashboard".into(),
                    title: "Client dashboard".into(),
                    status_text: "Drawing the empty states".into(),
                    done_steps: 1,
                    total_steps: 3,
                    ring_tone: ThreadRingTone::Amber,
                    done: false,
                }),
                ThreadPart::Task(ThreadTaskCard {
                    id: "migration".into(),
                    title: "API migration".into(),
                    status_text: "Moving our own apps".into(),
                    done_steps: 1,
                    total_steps: 4,
                    ring_tone: ThreadRingTone::Orange,
                    done: false,
                }),
            ],
            streaming: false,
            stopped: false,
        },
        ThreadMessage::User {
            id: "m3".into(),
            context_note: None,
            text: "How did the ingest load test go?".into(),
            attachments: Vec::new(),
            status: UserMessageStatus::Sent,
        },
        ThreadMessage::Agent {
            id: "m4".into(),
            at: "8:01 AM".into(),
            parts: vec![
                ThreadPart::Tools {
                    id: "tg-2".into(),
                    summary_label: "Ran 2 tools · 1.1s".into(),
                    calls: vec![
                        ThreadToolCall::done(
                            "t2-1",
                            "Queried Northwind ingest load-test telemetry",
                            ProIcon::Search,
                            "0.5s",
                        ),
                        ThreadToolCall::done(
                            "t2-2",
                            "Exported load-test-results.csv and ingest-design.md",
                            ProIcon::PageDoc,
                            "1.1s",
                        ),
                    ],
                },
                ThreadPart::Text(
                    "It passed at 20k events a minute: p95 stayed under 100 ms and errors at 0.02%. Past 30k it starts to fall behind – well above Northwind's bursts.".into(),
                ),
                ThreadPart::Artifacts(vec![
                    "load-test-results.csv".into(),
                    "ingest-design.md".into(),
                ]),
            ],
            streaming: false,
            stopped: false,
        },
        ThreadMessage::User {
            id: "m5".into(),
            context_note: None,
            text: "Good. Get ingest onto staging today – and tell me what you need from me.".into(),
            attachments: Vec::new(),
            status: UserMessageStatus::Sent,
        },
    ]
}

fn sample_thread_decisions() -> Vec<ThreadDecision> {
    vec![
        ThreadDecision {
            id: "ingest-queue".into(),
            task_title: "Ingest pipeline · Northwind".into(),
            task_id: "ingest".into(),
            question: "Which queue should the ingest service use?".into(),
            options: vec![
                DecisionOption::new(
                    "kafka",
                    "Kafka",
                    "24h replay log, partitioned by workspace id",
                    true,
                ),
                DecisionOption::new(
                    "sqs",
                    "AWS SQS + S3 archive",
                    "Zero broker ops, replay via S3 batch re-drive",
                    false,
                ),
                DecisionOption::new(
                    "redis",
                    "Redis Streams",
                    "Sub-10ms p95, memory-bounded retention",
                    false,
                ),
            ],
            facts: Vec::new(),
            is_approval: false,
        },
        ThreadDecision {
            id: "staging-deploy".into(),
            task_title: "Ingest pipeline · Staging rollout".into(),
            task_id: "ingest".into(),
            question: "Deploy ingest-service v0.4.2 to staging-us-east?".into(),
            options: Vec::new(),
            facts: vec![
                ("Service".into(), "ingest-service v0.4.2".into()),
                ("Target".into(), "staging-us-east (10% canary)".into()),
                ("Replay".into(), "24 hours retained".into()),
            ],
            is_approval: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        ApprovalOutcome, ApprovalPolicy, ThreadConnection, ThreadMessage, ThreadPart,
        ThreadRingTone, ThreadTaskCard, UserMessageStatus, sample_thread_decisions,
        sample_thread_messages,
    };

    #[test]
    fn sample_conversation_matches_turns_tools_and_artifacts() {
        let msgs = sample_thread_messages();
        assert_eq!(msgs.len(), 5);
        assert!(matches!(
            &msgs[0],
            ThreadMessage::User {
                status: UserMessageStatus::Sent,
                ..
            }
        ));

        if let ThreadMessage::Agent { parts, at, .. } = &msgs[1] {
            assert_eq!(at.as_ref(), "5:21 AM");
            assert_eq!(parts.len(), 4);
            assert!(matches!(&parts[0], ThreadPart::Tools { calls, .. } if calls.len() == 3));
        } else {
            panic!("expected second turn to be agent");
        }

        if let ThreadMessage::Agent { parts, at, .. } = &msgs[3] {
            assert_eq!(at.as_ref(), "8:01 AM");
            assert!(matches!(&parts[2], ThreadPart::Artifacts(files) if files.len() == 2));
        } else {
            panic!("expected fourth turn to be agent");
        }
    }

    #[test]
    fn decision_queue_and_task_card_progress_behave_deterministically() {
        let decisions = sample_thread_decisions();
        assert_eq!(decisions.len(), 2);
        assert!(!decisions[0].is_approval);
        assert_eq!(decisions[0].options.len(), 3);
        assert!(decisions[1].is_approval);

        let card = ThreadTaskCard {
            id: "migration".into(),
            title: "API migration".into(),
            status_text: "Moving our own apps".into(),
            done_steps: 1,
            total_steps: 4,
            ring_tone: ThreadRingTone::Orange,
            done: false,
        };
        assert!((card.progress() - 0.25).abs() < 1e-5);
        assert_eq!(ThreadConnection::default(), ThreadConnection::Online);
        assert_eq!(ApprovalPolicy::default(), ApprovalPolicy::RequestApproval);
        assert_eq!(
            ApprovalOutcome::Edited("cap 5%".into()),
            ApprovalOutcome::Edited("cap 5%".into())
        );
    }
}
