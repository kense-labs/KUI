//! Orchestrating agent chat screen with sub-agent status pills and a live campaign brief.
//!
//! Combines [`AppShell`] (`Chat` preset) navigation, a multi-agent conversation thread
//! with collapsible reasoning steps, artifact chips, and assistant feedback controls,
//! a multi-line composer with model/approval pickers, and a tabbed campaign brief panel.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_btn, pro_eid, pro_empty_state, pro_fold, pro_icon_btn,
            pro_mark_selected, pro_pill_indicator, pro_presence, pro_selection_sensor,
            pro_skeleton_bar, pro_switch, pro_view_transition, tr,
        },
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

const SVG_BRAIN_ORCH: &str = r#"<path d="M7 4.5C5.34315 4.5 4 5.84315 4 7.5C4 8.06866 4.15822 8.60037 4.43304 9.0535C3.04727 9.31855 2 10.537 2 12C2 13.463 3.04727 14.6814 4.43304 14.9465M7 4.5C7 3.11929 8.11929 2 9.5 2C10.8807 2 12 3.11929 12 4.5V19.5C12 20.8807 10.8807 22 9.5 22C8.11929 22 7 20.8807 7 19.5C5.34315 19.5 4 18.1569 4 16.5C4 15.9313 4.15822 15.3996 4.43304 14.9465M7 4.5C7 5.31791 7.39278 6.04408 8 6.50018M4.43304 14.9465C4.78948 14.3588 5.34207 13.9032 6 13.6707" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M17 19.4999C18.6569 19.4999 20 18.1567 20 16.4999C20 15.9312 19.8418 15.3995 19.567 14.9464C20.9527 14.6813 22 13.4629 22 11.9999C22 10.5369 20.9527 9.31843 19.567 9.05338M17 19.4999C17 20.8806 15.8807 21.9999 14.5 21.9999C13.1193 21.9999 12 20.8806 12 19.4999L12 4.49988C12 3.11917 13.1193 1.99988 14.5 1.99988C15.8807 1.99988 17 3.11917 17 4.49988C18.6569 4.49988 20 5.84302 20 7.49988C20 8.06854 19.8418 8.60024 19.567 9.05338M17 19.4999C17 18.682 16.6072 17.9558 16 17.4997M19.567 9.05338C19.2105 9.64109 18.6579 10.0966 18 10.3292" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_RADAR_BRAND: &str = r#"<path d="M17 12C17 14.7614 14.7614 17 12 17C9.23858 17 7 14.7614 7 12C7 9.23858 9.23858 7 12 7" stroke="currentColor" stroke-linecap="round" stroke-width="1.9"></path><path d="M14 2.20004C13.3538 2.06886 12.6849 2 12 2C6.47715 2 2 6.47715 2 12C2 17.5228 6.47715 22 12 22C17.5228 22 22 17.5228 22 12C22 11.3151 21.9311 10.6462 21.8 10" stroke="currentColor" stroke-linecap="round" stroke-width="1.9"></path><path d="M12.0303 11.9625L16.5832 7.4096M19.7404 4.34462L19.1872 2.35748C19.0853 2.03011 18.6914 1.89965 18.4259 2.11662C16.9898 3.29018 15.4254 4.87091 16.703 7.36419C19.2771 8.56455 20.7466 6.94584 21.8733 5.5853C22.0975 5.3146 21.9623 4.90767 21.6247 4.81005L19.7404 4.34462Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_PALETTE_PIPE: &str = r#"<path d="M22 12C22 6.47715 17.5228 2 12 2C6.47715 2 2 6.47715 2 12C2 17.5228 6.47715 22 12 22C12.8417 22 14 22.1163 14 21C14 20.391 13.6832 19.9212 13.3686 19.4544C12.9082 18.7715 12.4523 18.0953 13 17C13.6667 15.6667 14.7778 15.6667 16.4815 15.6667C17.3334 15.6667 18.3334 15.6667 19.5 15.5C21.601 15.1999 22 13.9084 22 12Z" stroke="currentColor" stroke-width="1.9"></path><circle cx="9.5" cy="8.5" r="1.5" stroke="currentColor" stroke-width="1.9"></circle><circle cx="16.5" cy="9.5" r="1.5" stroke="currentColor" stroke-width="1.9"></circle><path d="M7.125 15H7M7.25 15C7.25 15.1381 7.13807 15.25 7 15.25C6.86193 15.25 6.75 15.1381 6.75 15C6.75 14.8619 6.86193 14.75 7 14.75C7.13807 14.75 7.25 14.8619 7.25 15Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_HAND_APPROVAL: &str = r#"<path d="M52.5305 32.6557V24.8463C52.5305 22.1016 54.7555 19.8766 57.5001 19.8766C60.2448 19.8766 62.4698 22.1016 62.4698 24.8463V42.027C62.4698 53.3194 53.3154 62.4735 42.0233 62.4735H34.2388M22.7127 39.7552L15.94 31.8537C14.8314 30.5602 13.2129 29.8159 11.5094 29.8159H11.1235C8.11379 29.8159 5.67395 32.2558 5.67395 35.2655C5.67395 36.3412 5.99241 37.393 6.58919 38.2881L16.4118 53.0221C20.3482 58.9269 26.9753 62.4735 34.0719 62.4735" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M52.534 24.1378V16.3284C52.534 13.5837 50.3091 11.3588 47.5644 11.3588C44.8197 11.3588 42.5948 13.5837 42.5948 16.3284V29.8174" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M42.5958 29.8198V10.6513C42.5958 7.90661 40.3708 5.68163 37.6262 5.68163C34.8815 5.68163 32.6565 7.90661 32.6565 10.6513V28.4" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path><path d="M32.6576 28.3975V16.3284C32.6576 13.5837 30.4326 11.3588 27.6879 11.3588C24.9433 11.3588 22.7183 13.5837 22.7183 16.3284V39.7567" stroke="currentColor" stroke-width="4.7" stroke-linecap="round" stroke-linejoin="round"></path>"#;

const SVG_SHIRT_THUMB: &str = r#"<path d="M5.99756 12V17.6841C5.99756 19.4952 5.99756 20.4008 6.58285 20.9635C7.24789 21.6028 9.6154 21.9785 11.9925 21.9991C14.3085 22.0192 16.6337 21.7022 17.4021 20.9635C17.9874 20.4008 17.9874 19.4952 17.9874 17.6841V12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M5.72228 14.022C4.8566 13.7083 3.22971 13.4394 2.49588 12.7032C1.48592 11.69 2.13864 10.3201 3.37707 7.73891C3.93449 6.5771 5.00094 5.72243 6.24362 5.38452C6.41238 5.33863 6.55884 5.23314 6.65592 5.08755L7.93933 3.08867C7.97639 3.03309 8.02343 2.98533 8.08061 2.95084C8.65909 2.60195 10.0921 1.99998 11.9925 1.99998C13.8929 1.99998 15.2321 2.60195 15.8105 2.95084C15.8677 2.98533 15.9148 3.03309 15.9518 3.08867L17.2721 5.0687C17.3692 5.21429 17.5156 5.31978 17.6844 5.36567C18.9271 5.70358 19.9451 6.45012 20.5026 7.61193C21.8937 10.1413 22.5105 11.6707 21.5005 12.6839C20.7667 13.4201 19.1174 13.7116 18.2517 14.0253" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M6.50098 5.49998L10.7345 8.79261C11.3409 9.26419 11.644 9.49998 12.001 9.49998C12.358 9.49998 12.6611 9.26419 13.2675 8.79261L17.501 5.49998" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M9.50098 2.99998L11.001 8.99998M14.501 2.99998L13.001 8.99998" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_CH_INSTAGRAM: &str = r#"<path d="M3 12C3 7.75736 3 5.63604 4.31802 4.31802C5.63604 3 7.75736 3 12 3C16.2426 3 18.364 3 19.682 4.31802C21 5.63604 21 7.75736 21 12C21 16.2426 21 18.364 19.682 19.682C18.364 21 16.2426 21 12 21C7.75736 21 5.63604 21 4.31802 19.682C3 18.364 3 16.2426 3 12Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M16 12C16 14.2091 14.2091 16 12 16C9.79086 16 8 14.2091 8 12C8 9.79086 9.79086 8 12 8C14.2091 8 16 9.79086 16 12Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><circle cx="17.25" cy="6.75" r="0.9" fill="currentColor"></circle>"#;

const SVG_CH_TIKTOK: &str = r#"<path d="M2.5 12C2.5 7.52166 2.5 5.28249 3.89124 3.89124C5.28249 2.5 7.52166 2.5 12 2.5C16.4783 2.5 18.7175 2.5 20.1088 3.89124C21.5 5.28249 21.5 7.52166 21.5 12C21.5 16.4783 21.5 18.7175 20.1088 20.1088C18.7175 21.5 16.4783 21.5 12 21.5C7.52166 21.5 5.28249 21.5 3.89124 20.1088C2.5 18.7175 2.5 16.4783 2.5 12Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"></path><path d="M10.5359 11.0075C9.71585 10.8916 7.84666 11.0834 6.93011 12.7782C6.01355 14.4729 6.9373 16.2368 7.51374 16.9069C8.08298 17.5338 9.89226 18.721 11.8114 17.5619C12.2871 17.2746 12.8797 17.0603 13.552 14.8153L13.4738 5.98145C13.3441 6.95419 14.4186 9.23575 17.478 9.5057" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_CH_FACEBOOK: &str = r#"<path d="M2.5 12C2.5 7.52166 2.5 5.28249 3.89124 3.89124C5.28249 2.5 7.52166 2.5 12 2.5C16.4783 2.5 18.7175 2.5 20.1088 3.89124C21.5 5.28249 21.5 7.52166 21.5 12C21.5 16.4783 21.5 18.7175 20.1088 20.1088C18.7175 21.5 16.4783 21.5 12 21.5C7.52166 21.5 5.28249 21.5 3.89124 20.1088C2.5 18.7175 2.5 16.4783 2.5 12Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"></path><path d="M16.9265 8.02637H13.9816C12.9378 8.02637 12.0894 8.86847 12.0817 9.91229L11.9964 21.4268M10.082 14.0017H14.8847" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_MAP_PIN: &str = r#"<path d="M13.6177 21.367C13.1841 21.773 12.6044 22 12.0011 22C11.3978 22 10.8182 21.773 10.3845 21.367C6.41302 17.626 1.09076 13.4469 3.68627 7.37966C5.08963 4.09916 8.45834 2 12.0011 2C15.5439 2 18.9126 4.09916 20.316 7.37966C22.9082 13.4393 17.599 17.6389 13.6177 21.367Z" stroke="currentColor" stroke-width="1.9"></path><path d="M15.5 11C15.5 12.933 13.933 14.5 12 14.5C10.067 14.5 8.5 12.933 8.5 11C8.5 9.067 10.067 7.5 12 7.5C13.933 7.5 15.5 9.067 15.5 11Z" stroke="currentColor" stroke-width="1.9"></path>"#;

const SVG_FILE_DOC: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><path d="M4 6.6h4M4 8.6h2.6" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"></path>"#;

const SVG_FILE_SHEET: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><rect x="3.4" y="5.6" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="6.3" y="5.6" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="3.4" y="7.8" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect><rect x="6.3" y="7.8" width="2.3" height="1.6" rx="0.4" fill="currentColor"></rect>"#;

const SVG_FILE_FOLDER: &str = r#"<path d="M1.4 3.1c0-.72.58-1.3 1.3-1.3h1.72c.38 0 .73.19.94.5l.6.9h3.34c.72 0 1.3.58 1.3 1.3V6H1.4Z" fill="currentColor"></path><path d="M1 6.95C.86 6.3 1.36 5.7 2.02 5.7h7.96c.66 0 1.16.6 1.02 1.25l-.5 2.4a1.3 1.3 0 0 1-1.27 1.05H2.77A1.3 1.3 0 0 1 1.5 9.35Z" fill="currentColor" opacity="0.45"></path>"#;

const SVG_THUMB_UP: &str = r#"<path d="M7 11V19C7 19.5523 6.55228 20 6 20H4C3.44772 20 3 19.5523 3 19V12C3 11.4477 3.44772 11 4 11H7ZM7 11L10.6 4.7C11.1 3.8 12.3 3.6 13.1 4.2C13.7 4.7 13.9 5.5 13.7 6.3L12.8 10H18.4C19.8 10 20.8 11.3 20.4 12.7L18.9 18.2C18.6 19.3 17.6 20 16.5 20H9C7.9 20 7 19.1 7 18" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"></path>"#;

const SVG_THUMB_DOWN: &str = r#"<path d="M7 13V5C7 4.44772 6.55228 4 6 4H4C3.44772 4 3 4.44772 3 5V12C3 12.5523 3.44772 13 4 13H7ZM7 13L10.6 19.3C11.1 20.2 12.3 20.4 13.1 19.8C13.7 19.3 13.9 18.5 13.7 17.7L12.8 14H18.4C19.8 14 20.8 12.7 20.4 11.3L18.9 5.8C18.6 4.7 17.6 4 16.5 4H9C7.9 4 7 4.9 7 6" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"></path>"#;

/// Sender role for a conversation turn in [`AgentChatScreen`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentChatRole {
    User,
    Assistant,
}

/// Helpful/unhelpful rating recorded on an assistant turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentChatFeedback {
    Up,
    Down,
}

/// Campaign copy angle selected in the right-hand brief.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SellingAngle {
    #[default]
    DesireDriven,
    ProblemSolution,
    SocialProof,
}

impl SellingAngle {
    /// Human-readable title displayed in the brief and thread.
    pub fn label(self) -> &'static str {
        match self {
            Self::DesireDriven => "Desire-driven",
            Self::ProblemSolution => "Problem–solution",
            Self::SocialProof => "Social proof",
        }
    }

    fn subtitle(self) -> &'static str {
        match self {
            Self::DesireDriven => "The first warm evening",
            Self::ProblemSolution => "Heat, itchy fabrics",
            Self::SocialProof => "412 sold, top reviews",
        }
    }

    fn signal_bars(self) -> u8 {
        match self {
            Self::DesireDriven => 3,
            Self::ProblemSolution => 2,
            Self::SocialProof => 1,
        }
    }
}

/// Segmented section inside the `Summer launch` brief tab.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BriefSection {
    #[default]
    Brief,
    Assets,
    Activity,
}

/// Action approval policy chosen in the bottom composer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ApprovalMode {
    #[default]
    RequestApproval,
    ApproveEdits,
    AutoRun,
}

impl ApprovalMode {
    /// Display label rendered in the composer button and menu.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::RequestApproval => tr(cx, "Request approval", "请求审批"),
            Self::ApproveEdits => tr(cx, "Approve safe edits", "自动批准安全编辑"),
            Self::AutoRun => tr(cx, "Auto-run in sandbox", "沙盒内自动执行"),
        }
    }
}

/// Visual type of an artifact linked from the thread or brief.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentArtifactKind {
    Doc,
    Sheet,
    Folder,
}

/// Sub-agent status pill displayed under `Agents working`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentWorkerChip {
    pub id: SharedString,
    pub label: SharedString,
    pub active: bool,
}

impl AgentWorkerChip {
    /// Construct a sub-agent status pill.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>, active: bool) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            active,
        }
    }
}

/// Collapsible reasoning or tool execution step inside an assistant response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentReasoningStep {
    pub id: SharedString,
    pub label: SharedString,
    pub duration: SharedString,
    pub done: bool,
}

impl AgentReasoningStep {
    /// Construct a completed or running reasoning step.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        duration: impl Into<SharedString>,
        done: bool,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            duration: duration.into(),
            done,
        }
    }
}

/// Clickable artifact chip that opens in the right-hand work area.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentArtifactRef {
    pub id: SharedString,
    pub name: SharedString,
    pub kind: AgentArtifactKind,
}

impl AgentArtifactRef {
    /// Construct an artifact chip reference.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        kind: AgentArtifactKind,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
        }
    }
}

/// One user or assistant turn in [`AgentChatScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentChatMessage {
    pub id: SharedString,
    pub role: AgentChatRole,
    pub paragraphs: Vec<SharedString>,
    pub worked_label: Option<SharedString>,
    pub reasoning_open: bool,
    pub steps: Vec<AgentReasoningStep>,
    pub agents: Vec<AgentWorkerChip>,
    pub artifacts: Vec<AgentArtifactRef>,
    pub feedback: Option<AgentChatFeedback>,
}

impl AgentChatMessage {
    /// Construct a user bubble message.
    pub fn user(id: impl Into<SharedString>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            role: AgentChatRole::User,
            paragraphs: vec![text.into()],
            worked_label: None,
            reasoning_open: false,
            steps: Vec::new(),
            agents: Vec::new(),
            artifacts: Vec::new(),
            feedback: None,
        }
    }

    /// Construct an assistant response turn.
    pub fn assistant(id: impl Into<SharedString>, paragraphs: Vec<SharedString>) -> Self {
        Self {
            id: id.into(),
            role: AgentChatRole::Assistant,
            paragraphs,
            worked_label: None,
            reasoning_open: false,
            steps: Vec::new(),
            agents: Vec::new(),
            artifacts: Vec::new(),
            feedback: None,
        }
    }
}

/// Events emitted by [`AgentChatScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentChatEvent {
    Navigate(SharedString),
    SendMessage {
        text: SharedString,
        model: SharedString,
        attachments: Vec<SharedString>,
    },
    StopGeneration,
    SelectModel(SharedString),
    SelectApproval(ApprovalMode),
    SelectAngle(SellingAngle),
    ToggleChannel {
        id: SharedString,
        enabled: bool,
    },
    OpenArtifact(SharedString),
    GenerateAds {
        angle: SellingAngle,
        ad_count: usize,
    },
    CopyMessage(SharedString),
    Regenerate(SharedString),
    Feedback {
        message_id: SharedString,
        feedback: Option<AgentChatFeedback>,
    },
}

impl EventEmitter<AgentChatEvent> for AgentChatScreen {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ComposerMenu {
    Add,
    Approval,
    Model,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkTabSlot {
    Brief,
    BrandGuide,
    LaunchBudget,
    ProductPhotos,
    NewTab,
}

impl WorkTabSlot {
    fn id(self) -> &'static str {
        match self {
            Self::Brief => "summer-launch",
            Self::BrandGuide => "brand-guide.md",
            Self::LaunchBudget => "launch-budget.csv",
            Self::ProductPhotos => "product-photos",
            Self::NewTab => "new-tab",
        }
    }

    fn title(self, cx: &App) -> SharedString {
        match self {
            Self::Brief => "Summer launch".into(),
            Self::BrandGuide => "brand-guide.md".into(),
            Self::LaunchBudget => "launch-budget.csv".into(),
            Self::ProductPhotos => "product-photos".into(),
            Self::NewTab => tr(cx, "New tab", "新标签页").into(),
        }
    }

    fn icon(self) -> ProIcon {
        match self {
            Self::Brief => ProIcon::Tasks,
            Self::BrandGuide => ProIcon::PageDoc,
            Self::LaunchBudget => ProIcon::ClipboardDoc,
            Self::ProductPhotos => ProIcon::Folder,
            Self::NewTab => ProIcon::Globe,
        }
    }
}

#[derive(Clone, Debug)]
struct ChannelToggle {
    id: SharedString,
    name: SharedString,
    subtitle: SharedString,
    enabled: bool,
}

/// Orchestrating AI agent chat screen with sub-agent pills and a tabbed campaign brief.
pub struct AgentChatScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    focus: FocusHandle,
    standalone: bool,
    workspace: SharedString,
    user_name: SharedString,
    user_email: SharedString,
    loading: bool,
    empty: bool,
    chat_title: SharedString,
    chat_menu_open: bool,
    messages: Vec<AgentChatMessage>,
    copied_msg_id: Option<SharedString>,
    composer_editor: Option<Entity<Editor>>,
    composer_menu: Option<ComposerMenu>,
    last_composer_menu: Cell<ComposerMenu>,
    active_model: SharedString,
    approval_mode: ApprovalMode,
    attachments: Vec<SharedString>,
    streaming: bool,
    dictating: bool,
    brief_section: BriefSection,
    selling_angle: SellingAngle,
    audience_selected: [bool; 4],
    channels: Vec<ChannelToggle>,
    ads_generated: bool,
    work_tabs: Vec<WorkTabSlot>,
    active_work_tab: usize,
    doc_checks: [bool; 3],
    budget_cell: (usize, usize),
    folder_selected: usize,
    quick_look_open: bool,
    started_at: Instant,
    measured_width: Rc<Cell<f32>>,
}

impl AgentChatScreen {
    /// Construct an [`AgentChatScreen`] with the default `Acme` workspace and `Summer launch` brief.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let focus = cx.focus_handle();
        let this = cx.entity();
        let groups = Self::build_nav_groups(false, cx);
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Chat, shell_cx)
                .with_groups(groups)
                .with_active("Generate on-brand creative")
                .with_work_area(true, true)
                .with_panel_width(344.);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_inspector(w, cx));
            s
        });

        cx.subscribe(&shell, |this, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                this.handle_sidebar_navigate(label.clone(), cx);
            }
            AppShellEvent::PanelOpen(open) => {
                if *open && this.work_tabs.is_empty() {
                    this.work_tabs.push(WorkTabSlot::Brief);
                    this.active_work_tab = 0;
                }
                cx.notify();
            }
            _ => {}
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            shell,
            focus,
            standalone: false,
            workspace: "Acme".into(),
            user_name: "Alex Morgan".into(),
            user_email: "alex@acme.ai".into(),
            loading: false,
            empty: false,
            chat_title: "Generate on-brand creative".into(),
            chat_menu_open: false,
            messages: Self::default_messages(),
            copied_msg_id: None,
            composer_editor: None,
            composer_menu: None,
            last_composer_menu: Cell::new(ComposerMenu::Model),
            active_model: "Auto".into(),
            approval_mode: ApprovalMode::RequestApproval,
            attachments: Vec::new(),
            streaming: false,
            dictating: false,
            brief_section: BriefSection::Brief,
            selling_angle: SellingAngle::DesireDriven,
            audience_selected: [true, true, true, true],
            channels: Self::default_channels(),
            ads_generated: false,
            work_tabs: vec![WorkTabSlot::Brief],
            active_work_tab: 0,
            doc_checks: [true, true, false],
            budget_cell: (0, 0),
            folder_selected: 0,
            quick_look_open: false,
            started_at: Instant::now(),
            measured_width: Rc::new(Cell::new(960.)),
        }
    }

    fn build_nav_groups(empty: bool, cx: &App) -> Vec<ShellNavGroup> {
        let mut groups = vec![ShellNavGroup::primary(vec![
            ShellNavItem::new("new-chat", tr(cx, "New chat", "新建对话"), ProIcon::NewChat),
            ShellNavItem::new("folders", tr(cx, "Folders", "文件夹"), ProIcon::Folder),
            ShellNavItem::new(
                "integrations",
                tr(cx, "Integrations", "集成"),
                ProIcon::Grid,
            ),
        ])];

        if !empty {
            groups.push(ShellNavGroup::named(
                tr(cx, "Pinned", "已置顶"),
                vec![ShellNavItem::new(
                    "q3-messaging",
                    "Q3 launch messaging",
                    ProIcon::Pin,
                )],
            ));
        }

        let mut projects = vec![ShellNavItem::new(
            "q3-campaign",
            "Q3 Campaign",
            ProIcon::ProjectFolder,
        )];
        if !empty {
            projects.push(
                ShellNavItem::new(
                    "ad-hooks",
                    "Ad hooks for the linen overshirt",
                    ProIcon::Thread,
                )
                .with_unread(true),
            );
            projects.push(ShellNavItem::new(
                "q3-budget",
                "Q3 budget split by channel",
                ProIcon::Thread,
            ));
        }
        projects.extend([
            ShellNavItem::new("brand-assets", "Brand assets", ProIcon::Folder),
            ShellNavItem::new("hiring", "Hiring", ProIcon::Folder),
            ShellNavItem::new("research", "Research", ProIcon::Folder),
        ]);
        groups.push(ShellNavGroup::named(tr(cx, "Projects", "项目"), projects));

        if !empty {
            groups.push(ShellNavGroup::named(
                tr(cx, "Chats", "对话"),
                vec![
                    ShellNavItem::new(
                        "generate-creative",
                        "Generate on-brand creative",
                        ProIcon::Thread,
                    ),
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
            ));
        }
        groups
    }

    fn default_messages() -> Vec<AgentChatMessage> {
        let mut answer = AgentChatMessage::assistant(
            "msg-assistant-1",
            vec![
                "I'll pull your bestseller first, then line up the creative generation.".into(),
                "Your bestseller is the Linen Overshirt (the only product synced so far – Shopify is still importing, so treat that ranking as provisional).".into(),
                "Before I brief and generate the ads, I need one thing I can't infer, because it drives the whole copy structure: how should we sell this shirt?".into(),
            ],
        );
        answer.worked_label = Some("Worked for 3m 38s".into());
        answer.steps = vec![
            AgentReasoningStep::new(
                "step-shopify",
                "Synced Shopify catalog · 412 orders this month",
                "48s",
                true,
            ),
            AgentReasoningStep::new(
                "step-sku",
                "Ranked Linen Overshirt ($89) as provisional #1 bestseller",
                "32s",
                true,
            ),
            AgentReasoningStep::new(
                "step-brand",
                "Extracted Sand / Olive palette & voice rules from brand-guide.md",
                "1m 14s",
                true,
            ),
            AgentReasoningStep::new(
                "step-brief",
                "Prepared 3 selling angles and channel aspect specs",
                "1m 04s",
                true,
            ),
        ];
        answer.agents = vec![
            AgentWorkerChip::new("campaign-manager", "Campaign manager", true),
            AgentWorkerChip::new("brand-research", "Brand research", false),
            AgentWorkerChip::new("creative-pipeline", "Creative pipeline", false),
            AgentWorkerChip::new("audience-research", "Audience research", false),
            AgentWorkerChip::new("creative-direction", "Creative direction", false),
        ];
        answer.artifacts = vec![
            AgentArtifactRef::new("brand-guide.md", "brand-guide.md", AgentArtifactKind::Doc),
            AgentArtifactRef::new(
                "launch-budget.csv",
                "launch-budget.csv",
                AgentArtifactKind::Sheet,
            ),
            AgentArtifactRef::new(
                "product-photos",
                "product-photos",
                AgentArtifactKind::Folder,
            ),
        ];
        vec![
            AgentChatMessage::user("msg-user-1", "Desire-driven"),
            answer,
        ]
    }

    fn default_channels() -> Vec<ChannelToggle> {
        vec![
            ChannelToggle {
                id: "instagram".into(),
                name: "Instagram".into(),
                subtitle: "Feed and Story".into(),
                enabled: true,
            },
            ChannelToggle {
                id: "tiktok".into(),
                name: "TikTok".into(),
                subtitle: "Story format only".into(),
                enabled: true,
            },
            ChannelToggle {
                id: "facebook".into(),
                name: "Facebook".into(),
                subtitle: "Feed and Banner".into(),
                enabled: false,
            },
        ]
    }

    /// Render without the outer [`AppShell`] frame when embedded in a host container.
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

    /// Override the signed-in account name and email in the sidebar footer.
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

    /// Enable or disable the skeleton loading placeholder state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Enable or disable the empty state (`loading` takes precedence when both are `true`).
    pub fn with_empty(mut self, empty: bool, cx: &mut Context<Self>) -> Self {
        self.empty = empty;
        self.sync_shell(cx);
        self
    }

    /// Replace the conversation messages shown in the thread.
    pub fn with_messages(mut self, messages: Vec<AgentChatMessage>) -> Self {
        self.messages = messages;
        self
    }

    /// Set the active model label in the composer (`"Auto"`, `"Claude Sonnet 4.5"`, `"GPT-5"`, `"Gemini 2.5 Pro"`).
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.active_model = model.into();
        self
    }

    /// Return whether the loading skeleton state is active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Return whether the empty state is configured.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// Return the currently selected composer model label.
    pub fn active_model(&self) -> &str {
        self.active_model.as_ref()
    }

    /// Return the current campaign selling angle.
    pub fn selling_angle(&self) -> SellingAngle {
        self.selling_angle
    }

    /// Return the conversation messages currently in the thread.
    pub fn messages(&self) -> &[AgentChatMessage] {
        &self.messages
    }

    /// Compute the planned ad creative count (`3` creatives per enabled social channel).
    pub fn planned_ad_count(&self) -> usize {
        let enabled = self.channels.iter().filter(|c| c.enabled).count();
        enabled * 3
    }

    /// Update the loading skeleton state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Update the empty state at runtime and synchronize sidebar groups.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            self.sync_shell(cx);
            cx.notify();
        }
    }

    /// Select a selling angle in the campaign brief.
    pub fn select_angle(&mut self, angle: SellingAngle, cx: &mut Context<Self>) {
        if self.selling_angle != angle {
            self.selling_angle = angle;
            if let Some(first_user) = self
                .messages
                .iter_mut()
                .find(|m| m.role == AgentChatRole::User)
            {
                first_user.paragraphs = vec![angle.label().into()];
            }
            cx.emit(AgentChatEvent::SelectAngle(angle));
            cx.notify();
        }
    }

    /// Toggle one of the social distribution channels by id (`"instagram"`, `"tiktok"`, `"facebook"`).
    pub fn toggle_channel(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(ch) = self.channels.iter_mut().find(|c| c.id.as_ref() == id) {
            ch.enabled = !ch.enabled;
            let enabled = ch.enabled;
            let cid = ch.id.clone();
            cx.emit(AgentChatEvent::ToggleChannel { id: cid, enabled });
            cx.notify();
        }
    }

    /// Open an artifact file (`"brand-guide.md"`, `"launch-budget.csv"`, `"product-photos"`) in the work area.
    pub fn open_artifact(&mut self, key: &str, cx: &mut Context<Self>) {
        let slot = match key {
            "brand-guide.md" => WorkTabSlot::BrandGuide,
            "launch-budget.csv" => WorkTabSlot::LaunchBudget,
            "product-photos" => WorkTabSlot::ProductPhotos,
            "summer-launch" => WorkTabSlot::Brief,
            _ => WorkTabSlot::NewTab,
        };
        if let Some(pos) = self.work_tabs.iter().position(|&t| t == slot) {
            self.active_work_tab = pos;
        } else if self
            .work_tabs
            .get(self.active_work_tab)
            .is_some_and(|&t| t == WorkTabSlot::NewTab)
            && slot != WorkTabSlot::NewTab
        {
            self.work_tabs[self.active_work_tab] = slot;
        } else {
            self.work_tabs.push(slot);
            self.active_work_tab = self.work_tabs.len() - 1;
        }
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        cx.emit(AgentChatEvent::OpenArtifact(slot.id().into()));
        cx.notify();
    }

    /// Close a work-area tab by index; closing the last folds the work area and restores the brief.
    pub fn close_work_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.work_tabs.len() {
            return;
        }
        self.work_tabs.remove(index);
        if self.work_tabs.is_empty() {
            self.work_tabs.push(WorkTabSlot::Brief);
            self.active_work_tab = 0;
            self.shell.update(cx, |s, cx| s.set_panel_open(false, cx));
        } else if self.active_work_tab >= self.work_tabs.len() {
            self.active_work_tab = self.work_tabs.len() - 1;
        }
        cx.notify();
    }

    /// Submit a prompt string into the thread when neither loading nor empty-locked.
    pub fn submit_prompt(&mut self, raw: &str, cx: &mut Context<Self>) -> bool {
        let trimmed = raw.trim();
        if self.loading || (trimmed.is_empty() && self.attachments.is_empty()) {
            return false;
        }
        if self.empty {
            self.empty = false;
            self.messages.clear();
            self.sync_shell(cx);
        }
        let prompt_text: SharedString = if trimmed.is_empty() {
            format!("Attached {} file(s)", self.attachments.len()).into()
        } else {
            trimmed.to_owned().into()
        };
        let next_idx = self.messages.len() + 1;
        self.messages.push(AgentChatMessage::user(
            format!("msg-u-{next_idx}"),
            prompt_text.clone(),
        ));

        let mut reply = AgentChatMessage::assistant(
            format!("msg-a-{next_idx}"),
            vec![
                format!(
                    "Updated the {} brief for Linen Overshirt using {} routing.",
                    self.selling_angle.label(),
                    self.active_model
                )
                .into(),
                format!(
                    "{} ad creatives are queued across the enabled channels — review Brief or click Generate ads when ready.",
                    self.planned_ad_count().max(3)
                )
                .into(),
            ],
        );
        reply.worked_label = Some("Worked for 4s".into());
        reply.steps = vec![
            AgentReasoningStep::new(
                format!("step-route-{next_idx}"),
                "Orchestrator routed prompt to Campaign manager & Creative direction",
                "1.4s",
                true,
            ),
            AgentReasoningStep::new(
                format!("step-sync-{next_idx}"),
                "Synchronized brief constraints with brand-guide.md",
                "2.6s",
                true,
            ),
        ];
        reply.artifacts = vec![AgentArtifactRef::new(
            "brand-guide.md",
            "brand-guide.md",
            AgentArtifactKind::Doc,
        )];
        self.messages.push(reply);

        let sent_attachments = std::mem::take(&mut self.attachments);
        self.streaming = false;
        cx.emit(AgentChatEvent::SendMessage {
            text: prompt_text,
            model: self.active_model.clone(),
            attachments: sent_attachments,
        });
        cx.notify();
        true
    }

    /// Record or toggle thumbs-up/down feedback on an assistant message.
    pub fn toggle_feedback(
        &mut self,
        message_id: &str,
        vote: AgentChatFeedback,
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
            let next_fb = msg.feedback;
            let mid = msg.id.clone();
            cx.emit(AgentChatEvent::Feedback {
                message_id: mid,
                feedback: next_fb,
            });
            cx.notify();
        }
    }

    fn sync_shell(&self, cx: &mut Context<Self>) {
        let groups = Self::build_nav_groups(self.empty && !self.loading, cx);
        let active = if self.empty && !self.loading {
            SharedString::from(tr(cx, "New chat", "新建对话"))
        } else {
            self.chat_title.clone()
        };
        let host = cx.entity();
        let ws = self.workspace.clone();
        let uname = self.user_name.clone();
        let uemail = self.user_email.clone();
        self.shell.update(cx, |s, s_cx| {
            let open = s.is_panel_open();
            let mut next = AppShell::new("agent-chat-shell", s_cx)
                .with_preset(AppShellPreset::Chat, s_cx)
                .with_workspace(ws)
                .with_user(uname, uemail)
                .with_groups(groups)
                .with_active(active)
                .with_work_area(true, open)
                .with_panel_width(344.);
            next.bind_page(&host, s_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            next.bind_panel(&host, |screen, w, cx| screen.render_inspector(w, cx));
            *s = next;
            s_cx.notify();
        });
    }

    fn handle_sidebar_navigate(&mut self, label: SharedString, cx: &mut Context<Self>) {
        let lower = label.to_ascii_lowercase();
        if lower == "new chat" || label.as_ref() == "新建对话" {
            self.empty = false;
            self.chat_title = tr(cx, "New chat", "新建对话").into();
            self.messages = vec![AgentChatMessage::assistant(
                "msg-welcome",
                vec![
                    "Ready to coordinate Campaign manager, Brand research, and Creative direction — describe the launch or drop @context to begin.".into(),
                ],
            )];
        } else if lower == "generate on-brand creative" {
            self.empty = false;
            self.chat_title = "Generate on-brand creative".into();
            self.messages = Self::default_messages();
        } else if matches!(
            lower.as_str(),
            "q3 launch messaging"
                | "ad hooks for the linen overshirt"
                | "q3 budget split by channel"
                | "onboarding email draft"
                | "august content calendar"
        ) {
            self.empty = false;
            self.chat_title = label.clone();
            let mut msg = AgentChatMessage::assistant(
                "msg-switched",
                vec![
                    format!(
                        "Loaded history for \"{label}\". Sub-agents synced the latest Summer launch context and files."
                    )
                    .into(),
                ],
            );
            msg.worked_label = Some("Worked for 1m 12s".into());
            msg.steps = vec![AgentReasoningStep::new(
                "step-hist",
                "Restored workspace thread & campaign artifacts",
                "1.2s",
                true,
            )];
            msg.artifacts = vec![
                AgentArtifactRef::new("brand-guide.md", "brand-guide.md", AgentArtifactKind::Doc),
                AgentArtifactRef::new(
                    "launch-budget.csv",
                    "launch-budget.csv",
                    AgentArtifactKind::Sheet,
                ),
            ];
            self.messages = vec![AgentChatMessage::user("msg-switched-u", label.clone()), msg];
        }
        cx.emit(AgentChatEvent::Navigate(label));
        cx.notify();
    }

    fn ensure_composer(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.composer_editor.is_none() {
            let ph: SharedString = tr(
                cx,
                "Ask anything or @ to add context",
                "输入问题，或输入 @ 添加上下文",
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
                EditEvent::Changed => {
                    cx.notify();
                }
                EditEvent::Submitted => {
                    let val = ed_ent.read(cx).value().to_string();
                    if this.submit_prompt(&val, cx) {
                        ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                    }
                }
                EditEvent::Escape => {
                    if this.streaming {
                        this.streaming = false;
                        cx.emit(AgentChatEvent::StopGeneration);
                        cx.notify();
                    } else if this.dictating || this.composer_menu.is_some() {
                        this.dictating = false;
                        this.composer_menu = None;
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

    fn render_blue_pixel_mark(p: &ProPalette) -> Div {
        let dot_c = if p.dark {
            p.blue.opacity(0.86)
        } else {
            hsla(214. / 360., 1.0, 0.44, 0.77)
        };
        let pattern = [true, true, true, true, false, true, true, true, false];
        let mut col = div()
            .size(px(15.))
            .flex_none()
            .flex()
            .flex_col()
            .justify_center()
            .gap(px(1.5));
        for row_cells in pattern.as_chunks::<3>().0 {
            let mut row = div().flex().items_center().justify_center().gap(px(1.5));
            for &on in row_cells {
                row = row.child(div().size(px(3.)).rounded(px(0.8)).bg(if on {
                    dot_c
                } else {
                    dot_c.opacity(0.28)
                }));
            }
            col = col.child(row);
        }
        col
    }

    fn render_worker_glyph(worker_id: &str, color: Hsla) -> AnyElement {
        match worker_id {
            "campaign-manager" => pro_icon(ProIcon::Megaphone, 14., color),
            "brand-research" => pro_svg("0 0 24 24", SVG_RADAR_BRAND, 14., color),
            "creative-pipeline" => pro_svg("0 0 24 24", SVG_PALETTE_PIPE, 14., color),
            "audience-research" => pro_icon(ProIcon::Users, 14., color),
            _ => pro_svg("0 0 24 24", SVG_BRAIN_ORCH, 14., color),
        }
    }

    fn render_artifact_glyph(kind: AgentArtifactKind, size_px: f32, p: &ProPalette) -> AnyElement {
        match kind {
            AgentArtifactKind::Doc => pro_svg("0 0 12 12", SVG_FILE_DOC, size_px, p.blue),
            AgentArtifactKind::Sheet => pro_svg("0 0 12 12", SVG_FILE_SHEET, size_px, p.green),
            AgentArtifactKind::Folder => pro_svg("0 0 12 12", SVG_FILE_FOLDER, size_px, p.amber),
        }
    }

    fn render_file_chip(
        &self,
        id_suffix: String,
        art: &AgentArtifactRef,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let key = art.id.clone();
        let name = art.name.clone();
        let glyph = Self::render_artifact_glyph(art.kind, 12., p);
        activate(
            div()
                .id(pro_eid(&self.id, id_suffix))
                .role(Role::Button)
                .aria_label(name.clone())
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
        )
    }

    fn render_thread_loading(&self, phase: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        div()
            .w_full()
            .max_w(px(680.))
            .mx_auto()
            .flex()
            .flex_col()
            .gap(px(22.))
            .child(
                div()
                    .self_end()
                    .px(px(14.))
                    .py(px(10.))
                    .rounded(px(12.))
                    .bg(p.chip)
                    .child(pro_skeleton_bar(96., 14., 5., phase, cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap(px(14.))
                    .child(pro_skeleton_bar(320., 14., 5., phase, cx))
                    .child(
                        div()
                            .h(px(28.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(pro_skeleton_bar(15., 15., 4., phase, cx))
                            .child(pro_skeleton_bar(112., 12., 4., phase, cx)),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(pro_skeleton_bar(420., 14., 5., phase, cx))
                            .child(pro_skeleton_bar(290., 14., 5., phase, cx)),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(pro_skeleton_bar(400., 14., 5., phase, cx))
                            .child(pro_skeleton_bar(240., 14., 5., phase, cx)),
                    )
                    .child(pro_skeleton_bar(86., 12., 4., phase, cx))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(6.))
                            .child(pro_skeleton_bar(134., 30., 999., phase, cx))
                            .child(pro_skeleton_bar(116., 30., 999., phase, cx))
                            .child(pro_skeleton_bar(124., 30., 999., phase, cx))
                            .child(pro_skeleton_bar(132., 30., 999., phase, cx)),
                    ),
            )
    }

    fn render_message(
        &self,
        mi: usize,
        msg: &AgentChatMessage,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if msg.role == AgentChatRole::User {
            let body = msg.paragraphs.join("\n");
            return div()
                .w_full()
                .flex()
                .justify_end()
                .child(
                    div()
                        .max_w(relative(0.80))
                        .px(px(14.))
                        .py(px(8.))
                        .rounded(px(12.))
                        .bg(p.chip)
                        .text_size(px(14.))
                        .line_height(px(21.))
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
            .gap(px(14.))
            .text_size(px(14.))
            .line_height(px(22.4))
            .text_color(p.ink);

        if let Some(first) = msg.paragraphs.first() {
            col = col.child(div().w_full().child(first.clone()));
        }

        if let Some(ref worked) = msg.worked_label {
            let is_open = msg.reasoning_open;
            let msg_id = msg.id.clone();
            col = col.child(
                div()
                    .w_full()
                    .my(px(-4.))
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap(px(4.))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("worked-{mi}")))
                            .role(Role::Button)
                            .aria_expanded(is_open)
                            .tab_index(0)
                            .h(px(30.))
                            .ml(px(-6.))
                            .px(px(6.))
                            .rounded(px(7.))
                            .hover(move |s| s.bg(p.hover))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .cursor_pointer()
                            .child(Self::render_blue_pixel_mark(p))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(worked.clone()),
                            )
                            .when(!msg.steps.is_empty(), |d| {
                                d.child(pro_icon(
                                    if is_open {
                                        ProIcon::ChevronDown
                                    } else {
                                        ProIcon::ChevronRight
                                    },
                                    12.,
                                    p.faint,
                                ))
                            }),
                        cx,
                        move |this, _, cx| {
                            if let Some(target) = this.messages.iter_mut().find(|m| m.id == msg_id)
                            {
                                target.reasoning_open = !target.reasoning_open;
                                cx.notify();
                            }
                        },
                    ))
                    .child({
                        let steps_snapshot = msg.steps.clone();
                        let est_h = (steps_snapshot.len() as f32) * 28.;
                        let line_c = p.line;
                        let green_c = p.green;
                        let amber_c = p.amber;
                        let text_c = p.text;
                        let faint_c = p.faint;
                        pro_fold(
                            pro_eid(&self.id, format!("worked-fold-{}", msg.id)),
                            is_open && !msg.steps.is_empty(),
                            est_h,
                            4.,
                            move || {
                                let mut list = div()
                                    .ml(px(7.))
                                    .pl(px(12.))
                                    .border_l_1()
                                    .border_color(line_c)
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.));
                                for step in steps_snapshot {
                                    list = list.child(
                                        div()
                                            .min_h(px(24.))
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

        for para in msg.paragraphs.iter().skip(1) {
            col = col.child(div().w_full().child(para.clone()));
        }

        if !msg.agents.is_empty() {
            let worker_sensor = pro_selection_sensor();
            let active_worker_key = msg
                .agents
                .iter()
                .find(|a| a.active)
                .map(|a| a.id.clone())
                .unwrap_or_else(|| "none".into());
            let mut pills = div()
                .relative()
                .mt(px(-6.))
                .flex()
                .flex_wrap()
                .gap(px(6.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, format!("worker-pill-{}", msg.id)),
                    active_worker_key,
                    worker_sensor.clone(),
                    15.,
                    p.chip,
                    None,
                    false,
                ));
            for (wi, worker) in msg.agents.iter().enumerate() {
                let wid = worker.id.clone();
                let m_id = msg.id.clone();
                let active = worker.active;
                let ic_color = if active { p.ink } else { p.muted };
                let fg_color = if active { p.ink } else { p.text };
                let hover_c = p.hover;
                pills = pills.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("worker-{mi}-{wi}")))
                        .role(Role::Button)
                        .tab_index(0)
                        .relative()
                        .h(px(30.))
                        .pl(px(10.))
                        .pr(px(12.))
                        .rounded_full()
                        .when(active, |d| d.child(pro_mark_selected(&worker_sensor)))
                        .when(!active, |d| {
                            d.border_1()
                                .border_color(p.line_strong)
                                .hover(move |s| s.bg(hover_c))
                        })
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .text_size(px(13.))
                        .text_color(fg_color)
                        .cursor_pointer()
                        .child(Self::render_worker_glyph(&wid, ic_color))
                        .child(worker.label.clone()),
                    cx,
                    move |this, _, cx| {
                        if let Some(target) = this.messages.iter_mut().find(|m| m.id == m_id) {
                            for a in &mut target.agents {
                                a.active = a.id == wid;
                            }
                            this.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
                            cx.notify();
                        }
                    },
                ));
            }
            col = col
                .child(
                    div()
                        .pt(px(4.))
                        .text_size(px(12.5))
                        .text_color(p.muted)
                        .child(tr(cx, "Agents working", "协作中的智能体")),
                )
                .child(pills);
        }

        if !msg.artifacts.is_empty() {
            let mut files_row = div().flex().flex_wrap().gap(px(6.));
            for (ai, art) in msg.artifacts.iter().enumerate() {
                files_row = files_row.child(self.render_file_chip(
                    format!("msg-art-{mi}-{ai}"),
                    art,
                    p,
                    cx,
                ));
            }
            col = col.child(files_row);
        }

        // Copy / Regenerate / Feedback action bar on assistant message
        let copy_id = msg.id.clone();
        let copy_text: String = msg
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
            .mt(px(-2.))
            .ml(px(-4.))
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_icon_btn(
                pro_eid(&self.id, format!("act-copy-{mi}")),
                tr(cx, "Copy response", "复制回复"),
                if is_copied {
                    ProIcon::Check
                } else {
                    ProIcon::Copy
                },
                26.,
                is_copied,
                cx,
                move |this, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(copy_text.clone()));
                    this.copied_msg_id = Some(copy_id.clone());
                    cx.emit(AgentChatEvent::CopyMessage(copy_id.clone()));
                    cx.notify();
                },
            ))
            .child(pro_icon_btn(
                pro_eid(&self.id, format!("act-regen-{mi}")),
                tr(cx, "Regenerate response", "重新生成"),
                ProIcon::Refresh,
                26.,
                false,
                cx,
                move |this, _, cx| {
                    if let Some(target) = this.messages.iter_mut().find(|m| m.id == regen_id) {
                        target.reasoning_open = false;
                    }
                    cx.emit(AgentChatEvent::Regenerate(regen_id.clone()));
                    cx.notify();
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, format!("act-up-{mi}")))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Helpful response", "有帮助"))
                    .tab_index(0)
                    .size(px(26.))
                    .rounded(px(7.))
                    .bg(if fb == Some(AgentChatFeedback::Up) {
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
                        14.,
                        if fb == Some(AgentChatFeedback::Up) {
                            p.green_ink
                        } else {
                            p.muted
                        },
                    )),
                cx,
                move |this, _, cx| {
                    this.toggle_feedback(&up_id, AgentChatFeedback::Up, cx);
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, format!("act-down-{mi}")))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Unhelpful response", "无帮助"))
                    .tab_index(0)
                    .size(px(26.))
                    .rounded(px(7.))
                    .bg(if fb == Some(AgentChatFeedback::Down) {
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
                        14.,
                        if fb == Some(AgentChatFeedback::Down) {
                            p.red_ink
                        } else {
                            p.muted
                        },
                    )),
                cx,
                move |this, _, cx| {
                    this.toggle_feedback(&down_id, AgentChatFeedback::Down, cx);
                },
            ));

        col.child(actions).into_any_element()
    }

    fn render_composer(
        &self,
        narrow: bool,
        phase: f32,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        let draft = self
            .composer_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default();
        let line_count = draft
            .split('\n')
            .count()
            .max((draft.chars().count() / 62) + 1)
            .clamp(2, 7);
        let field_h = (line_count as f32) * 20.;
        let can_send = !self.loading && (!draft.trim().is_empty() || !self.attachments.is_empty());

        let mut wrap = div()
            .w_full()
            .max_w(px(744.))
            .mx_auto()
            .pt(px(8.))
            .pb(px(if narrow { 10. } else { 24. }))
            .px(px(if narrow { 12. } else { 32. }))
            .relative()
            .flex()
            .flex_col()
            .gap(px(8.));

        // Route line: [avatar] Orchestrator  Routes each request
        wrap = wrap.child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .text_size(px(13.))
                .child(
                    div()
                        .size(px(22.))
                        .rounded_full()
                        .bg(p.chip)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_svg("0 0 24 24", SVG_BRAIN_ORCH, 12., p.ink)),
                )
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(tr(cx, "Orchestrator", "编排器")),
                )
                .child(div().text_color(p.muted).child(tr(
                    cx,
                    "Routes each request",
                    "按需路由每条请求",
                ))),
        );

        // Attachment pills with spring fold
        {
            let mut att_row = div().flex().flex_wrap().gap(px(6.));
            let hover_c = p.hover;
            for (ai, name) in self.attachments.iter().enumerate() {
                att_row = att_row.child(
                    div()
                        .h(px(26.))
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
                                .id(pro_eid(&self.id, format!("rm-att-{ai}")))
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
            wrap = wrap.child(pro_fold(
                pro_eid(&self.id, "cmp-att-fold"),
                !self.attachments.is_empty(),
                32.,
                8.,
                move || att_row.into_any_element(),
            ));
        }

        // Popover menu above composer with spring presence & pill indicators
        if let Some(m) = self.composer_menu {
            self.last_composer_menu.set(m);
        }
        {
            let menu = self
                .composer_menu
                .unwrap_or_else(|| self.last_composer_menu.get());
            let menu_open = self.composer_menu.is_some();
            let align_end = matches!(menu, ComposerMenu::Model);
            let hover_c = p.hover;
            let pop = match menu {
                ComposerMenu::Add => div()
                    .w(px(240.))
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
                                "brand-guide.md",
                                "Attach brand-guide.md",
                                ProIcon::Paperclip,
                            ),
                            (
                                "launch-budget.csv",
                                "Attach launch-budget.csv",
                                ProIcon::ClipboardDoc,
                            ),
                            (
                                "lookbook-swatch.png",
                                "Upload product photo",
                                ProIcon::Upload,
                            ),
                        ]
                        .into_iter()
                        .enumerate()
                        .map(|(idx, (fname, label, ic))| {
                            activate(
                                div()
                                    .id(pro_eid(&self.id, format!("add-item-{idx}")))
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
                                    .child(label),
                                cx,
                                move |this, _, cx| {
                                    let f: SharedString = fname.into();
                                    if !this.attachments.contains(&f) {
                                        this.attachments.push(f);
                                    }
                                    this.composer_menu = None;
                                    cx.notify();
                                },
                            )
                        }),
                    ),
                ComposerMenu::Approval => {
                    let appr_sensor = pro_selection_sensor();
                    div()
                        .w(px(250.))
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
                                    pro_eid(&self.id, "cmp-appr-pill"),
                                    format!("{:?}", self.approval_mode),
                                    appr_sensor.clone(),
                                    7.,
                                    p.chip,
                                    None,
                                    false,
                                ))
                                .children(
                                    [
                                        ApprovalMode::RequestApproval,
                                        ApprovalMode::ApproveEdits,
                                        ApprovalMode::AutoRun,
                                    ]
                                    .into_iter()
                                    .enumerate()
                                    .map(|(idx, mode)| {
                                        let selected = self.approval_mode == mode;
                                        activate(
                                            div()
                                                .id(pro_eid(&self.id, format!("appr-opt-{idx}")))
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
                                                this.composer_menu = None;
                                                cx.emit(AgentChatEvent::SelectApproval(mode));
                                                cx.notify();
                                            },
                                        )
                                    }),
                                ),
                        )
                }
                ComposerMenu::Model => {
                    let mdl_sensor = pro_selection_sensor();
                    div()
                        .w(px(256.))
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
                                    pro_eid(&self.id, "cmp-mdl-pill"),
                                    self.active_model.clone(),
                                    mdl_sensor.clone(),
                                    7.,
                                    p.chip,
                                    None,
                                    false,
                                ))
                                .children(
                                    [
                                        ("Auto", "sparkles", "Routes per sub-agent"),
                                        ("Claude Sonnet 4.5", "claude", "Best for copy & briefs"),
                                        ("GPT-5", "openai", "Deep reasoning"),
                                        ("Gemini 2.5 Pro", "gemini", "Long multimodal context"),
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
                                                    .id(pro_eid(&self.id, format!("mdl-opt-{idx}")))
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
                                                    this.composer_menu = None;
                                                    cx.emit(AgentChatEvent::SelectModel(chosen));
                                                    cx.notify();
                                                },
                                            )
                                        },
                                    ),
                                ),
                        )
                }
            };
            let pop_vtx_id = pro_eid(&self.id, "cmp-pop-vtx");
            let pop_key: SharedString = format!("{menu:?}").into();
            wrap = wrap.child(pro_presence(
                pro_eid(&self.id, "cmp-pop-pres"),
                menu_open,
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
                        .child(pro_view_transition(pop_vtx_id, pop_key, pop))
                        .into_any_element()
                },
            ));
        }

        let ed_opt = self.composer_editor.clone();
        let ed_for_send = self.composer_editor.clone();

        let left_tools = div()
            .flex()
            .items_center()
            .gap(px(4.))
            .child(pro_icon_btn(
                pro_eid(&self.id, "cmp-plus"),
                tr(cx, "Add to the chat", "添加到对话"),
                ProIcon::Plus,
                28.,
                self.composer_menu == Some(ComposerMenu::Add),
                cx,
                |this, _, cx| {
                    this.composer_menu = if this.composer_menu == Some(ComposerMenu::Add) {
                        None
                    } else {
                        Some(ComposerMenu::Add)
                    };
                    cx.notify();
                },
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "cmp-approval"))
                    .role(Role::Button)
                    .aria_expanded(self.composer_menu == Some(ComposerMenu::Approval))
                    .tab_index(0)
                    .h(px(28.))
                    .pl(px(8.))
                    .pr(px(10.))
                    .rounded(px(8.))
                    .when(self.composer_menu == Some(ComposerMenu::Approval), |d| {
                        d.bg(p.hover)
                    })
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
                    .when(!narrow, |d| d.child(self.approval_mode.label(cx))),
                cx,
                |this, _, cx| {
                    this.composer_menu = if this.composer_menu == Some(ComposerMenu::Approval) {
                        None
                    } else {
                        Some(ComposerMenu::Approval)
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
                let h_bar =
                    5. + 11. * ((phase * std::f32::consts::TAU + i as f32 * 0.7).sin().abs());
                wave = wave.child(div().w(px(2.)).h(px(h_bar)).rounded(px(1.)).bg(p.blue));
            }
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(wave)
                .child(pro_icon_btn(
                    pro_eid(&self.id, "voice-cancel"),
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
                        .id(pro_eid(&self.id, "voice-commit"))
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
                        if let Some(ref ed) = ed_opt {
                            ed.update(cx, |e, cx| {
                                e.set_value(
                                    "Focus the Story hook on breathable linen for warm summer evenings",
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
                        .id(pro_eid(&self.id, "cmp-stop"))
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
                        cx.emit(AgentChatEvent::StopGeneration);
                        cx.notify();
                    },
                )
                .into_any_element()
            } else {
                activate(
                    div()
                        .id(pro_eid(&self.id, "cmp-send"))
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
                        .id(pro_eid(&self.id, "cmp-model"))
                        .role(Role::Button)
                        .aria_expanded(self.composer_menu == Some(ComposerMenu::Model))
                        .tab_index(0)
                        .h(px(28.))
                        .pl(px(8.))
                        .pr(px(6.))
                        .rounded(px(8.))
                        .when(self.composer_menu == Some(ComposerMenu::Model), |d| {
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
                                .max_w(px(136.))
                                .truncate()
                                .child(self.active_model.clone()),
                        )
                        .child(pro_icon(ProIcon::ChevronDown, 13., p.muted)),
                    cx,
                    |this, _, cx| {
                        this.composer_menu = if this.composer_menu == Some(ComposerMenu::Model) {
                            None
                        } else {
                            Some(ComposerMenu::Model)
                        };
                        cx.notify();
                    },
                ))
                .child(pro_icon_btn(
                    pro_eid(&self.id, "cmp-mic"),
                    tr(cx, "Dictate message", "语音输入"),
                    ProIcon::Mic,
                    28.,
                    false,
                    cx,
                    |this, _, cx| {
                        this.dictating = true;
                        this.composer_menu = None;
                        cx.notify();
                    },
                ))
                .child(submit_btn)
        };

        let card_bg = if p.dark { rgb(0x1c1c1e).into() } else { p.card };
        let composer_card = div()
            .w_full()
            .rounded(px(16.))
            .bg(card_bg)
            .border_1()
            .border_color(p.line)
            .pt(px(14.))
            .px(px(10.))
            .pb(px(10.))
            .flex()
            .flex_col()
            .gap(px(14.))
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
            );

        wrap.child(composer_card)
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_composer(w, cx);
        let p = ProPalette::get(cx);
        let phase = self.skeleton_phase(w, cx);
        let width = self.measured_width.get();
        let narrow = width < 600.;
        let measure_cell = self.measured_width.clone();
        let panel_open = self.standalone || self.shell.read(cx).is_panel_open();

        // Top bar with Home / Chat title / options menu
        let crumbs = div()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(8.))
            .text_size(px(13.))
            .when(!narrow, |d| {
                d.child(activate(
                    div()
                        .id(pro_eid(&self.id, "crumb-home"))
                        .role(Role::Button)
                        .tab_index(0)
                        .h(px(26.))
                        .px(px(4.))
                        .rounded(px(6.))
                        .hover(move |s| s.text_color(p.ink))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .text_color(p.muted)
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::NewChat, 15., p.muted))
                        .child(tr(cx, "Home", "首页")),
                    cx,
                    |_, _, cx| {
                        cx.emit(AgentChatEvent::Navigate("Home".into()));
                    },
                ))
                .child(div().text_color(p.faint).child("/"))
            })
            .child(if self.loading {
                pro_skeleton_bar(154., 13., 5., phase, cx).into_any_element()
            } else {
                div()
                    .min_w_0()
                    .truncate()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(if self.empty {
                        SharedString::from(tr(cx, "New chat", "新建对话"))
                    } else {
                        self.chat_title.clone()
                    })
                    .into_any_element()
            })
            .when(!self.loading && !self.empty, |d| {
                d.child(pro_icon_btn(
                    pro_eid(&self.id, "chat-more"),
                    tr(cx, "Chat options", "对话选项"),
                    ProIcon::MoreHorizontal,
                    24.,
                    self.chat_menu_open,
                    cx,
                    |this, _, cx| {
                        this.chat_menu_open = !this.chat_menu_open;
                        cx.notify();
                    },
                ))
            });

        let top_bar = div()
            .h(px(48.))
            .flex_none()
            .pl(px(if narrow { 48. } else { 20. }))
            .pr(px(if !panel_open { 52. } else { 12. }))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .child(crumbs);

        let thread_body: AnyElement = if self.loading {
            self.render_thread_loading(phase, cx).into_any_element()
        } else if self.empty {
            let empty_cta_id = pro_eid(&self.id, "empty-start-chat");
            div()
                .w_full()
                .min_h(px(320.))
                .flex()
                .items_center()
                .justify_center()
                .child(pro_empty_state(
                    ProIcon::Thread,
                    tr(cx, "No chats yet", "暂无对话"),
                    tr(
                        cx,
                        "Start a conversation with the orchestrator — sub-agents and the campaign brief fill in beside it.",
                        "与编排助手开启对话——协作智能体与推广简报将在右侧实时联动生成。",
                    ),
                    Some((
                        empty_cta_id,
                        tr(cx, "Start a chat", "开始对话").into(),
                        Box::new(|this: &mut Self, w, cx| {
                            this.set_empty(false, cx);
                            if let Some(ref ed) = this.composer_editor {
                                { let fh = ed.read(cx).focus.clone(); fh.focus(w, cx); }
                            }
                            cx.emit(AgentChatEvent::Navigate("New chat".into()));
                        }),
                    )),
                    cx,
                ))
                .into_any_element()
        } else {
            let msgs_snapshot = self.messages.clone();
            let mut col = div()
                .w_full()
                .max_w(px(680.))
                .mx_auto()
                .flex()
                .flex_col()
                .gap(px(22.));
            for (mi, msg) in msgs_snapshot.iter().enumerate() {
                col = col.child(self.render_message(mi, msg, &p, cx));
            }
            col.into_any_element()
        };

        let composer_el = self.render_composer(narrow, phase, &p, cx);
        let focus_for_ed = self.composer_editor.clone();

        let mut page = div()
            .id(pro_eid(&self.id, "page"))
            .track_focus(&self.focus)
            .size_full()
            .min_h_0()
            .relative()
            .bg(p.panel)
            .flex()
            .flex_col()
            .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, w, cx| {
                let k = ev.keystroke.key.as_str();
                if k == "/"
                    && !ev.keystroke.modifiers.platform
                    && !ev.keystroke.modifiers.control
                    && let Some(ref ed) = focus_for_ed
                    && !ed.read(cx).focus.is_focused(w)
                {
                    {
                        let fh = ed.read(cx).focus.clone();
                        fh.focus(w, cx);
                    }
                    w.prevent_default();
                    cx.stop_propagation();
                } else if k == "escape"
                    && (this.composer_menu.is_some() || this.chat_menu_open || this.quick_look_open)
                {
                    this.composer_menu = None;
                    this.chat_menu_open = false;
                    this.quick_look_open = false;
                    cx.notify();
                } else if k == "left" && ev.keystroke.modifiers.alt && this.work_tabs.len() > 1 {
                    this.active_work_tab = if this.active_work_tab == 0 {
                        this.work_tabs.len() - 1
                    } else {
                        this.active_work_tab - 1
                    };
                    cx.notify();
                } else if k == "right" && ev.keystroke.modifiers.alt && this.work_tabs.len() > 1 {
                    this.active_work_tab = (this.active_work_tab + 1) % this.work_tabs.len();
                    cx.notify();
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
            .child(top_bar)
            .child(
                div()
                    .id(pro_eid(&self.id, "thread-scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .pt(px(if narrow { 20. } else { 32. }))
                    .px(px(if narrow { 16. } else { 32. }))
                    .pb(px(16.))
                    .child(pro_view_transition(
                        pro_eid(&self.id, "thread-body-vtx"),
                        format!("{}:{}:{}", self.chat_title, self.loading, self.empty),
                        thread_body,
                    )),
            )
            .child(composer_el);

        {
            let copt_id = pro_eid(&self.id, "chat-opt-pop");
            let copt_items = [
                (ProIcon::Pin, tr(cx, "Pin conversation", "置顶对话"), false),
                (ProIcon::Tasks, tr(cx, "Open brief", "打开简报"), false),
                (ProIcon::Copy, tr(cx, "Duplicate thread", "复制会话"), false),
                (ProIcon::Trash, tr(cx, "Clear messages", "清空消息"), true),
            ]
            .into_iter()
            .enumerate()
            .map(|(idx, (ic, lbl, danger))| {
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("copt-{idx}")))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(30.))
                        .px(px(8.))
                        .rounded(px(7.))
                        .hover(move |s| s.bg(if danger { p.red_soft } else { p.hover }))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(12.5))
                        .text_color(if danger { p.red_ink } else { p.ink })
                        .cursor_pointer()
                        .child(pro_icon(ic, 14., if danger { p.red_ink } else { p.muted }))
                        .child(lbl),
                    cx,
                    move |this, _, cx| {
                        this.chat_menu_open = false;
                        if idx == 1 {
                            this.open_artifact("summer-launch", cx);
                        } else if idx == 3 {
                            this.set_empty(true, cx);
                        }
                        cx.notify();
                    },
                )
            })
            .collect::<Vec<_>>();
            page = page.child(pro_presence(
                pro_eid(&self.id, "chat-opt-pres"),
                self.chat_menu_open,
                0.18,
                0.12,
                move |a| {
                    div()
                        .id(copt_id)
                        .absolute()
                        .top(px(42. - 5. * (1. - a)))
                        .left(px(180.))
                        .opacity(a)
                        .w(px(196.))
                        .p(px(5.))
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line_strong)
                        .shadow_md()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .children(copt_items)
                        .into_any_element()
                },
            ));
        }

        page
    }

    fn render_brief_loading(&self, phase: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        div()
            .p(px(12.))
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(pro_skeleton_bar(280., 32., 10., phase, cx))
            .child(
                div()
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .p(px(12.))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(pro_skeleton_bar(40., 40., 10., phase, cx))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(pro_skeleton_bar(110., 13., 4., phase, cx))
                            .child(pro_skeleton_bar(136., 11., 4., phase, cx)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(pro_skeleton_bar(84., 11., 4., phase, cx))
                    .child(
                        div()
                            .rounded(px(12.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line_strong)
                            .p(px(12.))
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .child(pro_skeleton_bar(240., 28., 8., phase, cx))
                            .child(pro_skeleton_bar(220., 28., 8., phase, cx))
                            .child(pro_skeleton_bar(210., 28., 8., phase, cx)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(pro_skeleton_bar(72., 11., 4., phase, cx))
                    .child(
                        div()
                            .rounded(px(12.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line_strong)
                            .p(px(12.))
                            .flex()
                            .flex_wrap()
                            .gap(px(8.))
                            .child(pro_skeleton_bar(88., 24., 999., phase, cx))
                            .child(pro_skeleton_bar(56., 24., 999., phase, cx))
                            .child(pro_skeleton_bar(96., 24., 999., phase, cx)),
                    ),
            )
    }

    fn render_brief_tab(&self, p: &ProPalette, cx: &mut Context<Self>) -> Div {
        let seg_sensor = pro_selection_sensor();
        let ink_c = p.ink;
        let seg = div()
            .relative()
            .p(px(3.))
            .rounded(px(10.))
            .bg(p.chip)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "brief-seg-pill"),
                format!("{:?}", self.brief_section),
                seg_sensor.clone(),
                7.,
                p.card,
                Some(p.line_soft),
                true,
            ))
            .children(
                [
                    (BriefSection::Brief, tr(cx, "Brief", "简报")),
                    (BriefSection::Assets, tr(cx, "Assets", "素材")),
                    (BriefSection::Activity, tr(cx, "Activity", "动态")),
                ]
                .into_iter()
                .enumerate()
                .map(|(si, (sec, lbl))| {
                    let selected = self.brief_section == sec;
                    activate(
                        div()
                            .id(pro_eid(&self.id, format!("bseg-{si}")))
                            .role(Role::Tab)
                            .aria_selected(selected)
                            .tab_index(0)
                            .relative()
                            .flex_1()
                            .h(px(26.))
                            .rounded(px(7.))
                            .when(selected, |d| d.child(pro_mark_selected(&seg_sensor)))
                            .hover(move |s| s.text_color(ink_c))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if selected { p.ink } else { p.muted })
                            .cursor_pointer()
                            .child(lbl),
                        cx,
                        move |this, _, cx| {
                            this.brief_section = sec;
                            cx.notify();
                        },
                    )
                }),
            );

        if self.empty {
            let (title, desc) = match self.brief_section {
                BriefSection::Brief => (
                    tr(cx, "Campaign brief", "推广简报"),
                    tr(
                        cx,
                        "Product ranking, selling angles, audience segments, and channel specs appear here once a chat starts.",
                        "开启对话后，畅销单品、卖点切入角、目标用户与渠道规格将显示在此处。",
                    ),
                ),
                BriefSection::Assets => (
                    tr(cx, "Generated assets", "生成素材"),
                    tr(
                        cx,
                        "Feed, Story, and Banner ad variations generated from your brief will show up here.",
                        "基于简报生成的动态流、快拍与横幅广告稿将展示在此处。",
                    ),
                ),
                BriefSection::Activity => (
                    tr(cx, "Agent activity", "智能体动态"),
                    tr(
                        cx,
                        "Every tool call, sync step, and sub-agent handoff is logged here in chronological order.",
                        "每次工具调用、数据同步与子智能体交接都会按时间顺序记录在此。",
                    ),
                ),
            };
            return div()
                .p(px(12.))
                .flex()
                .flex_col()
                .gap(px(18.))
                .child(seg)
                .child(pro_empty_state(ProIcon::Tasks, title, desc, None, cx));
        }

        let content: AnyElement = match self.brief_section {
            BriefSection::Brief => {
                let thumb_bg: Hsla = if p.dark {
                    rgb(0x3a3329).into()
                } else {
                    rgb(0xefeada).into()
                };
                let thumb_fg: Hsla = if p.dark {
                    rgb(0xd9cda5).into()
                } else {
                    rgb(0xb25e00).into()
                };

                let product_card = div()
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .py(px(10.))
                    .px(px(12.))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        div()
                            .size(px(40.))
                            .flex_none()
                            .rounded(px(10.))
                            .bg(thumb_bg)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_svg("0 0 24 24", SVG_SHIRT_THUMB, 18., thumb_fg)),
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
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .truncate()
                                    .child("Linen Overshirt"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(17.))
                                    .text_color(p.muted)
                                    .truncate()
                                    .child("$89 · 412 sold this month"),
                            ),
                    )
                    .child(
                        div()
                            .flex_none()
                            .px(px(8.))
                            .py(px(2.))
                            .rounded_full()
                            .bg(p.green_soft)
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.green_ink)
                            .child("Bestseller"),
                    );

                // Selling angle radio card with sliding pill
                let angle_sensor = pro_selection_sensor();
                let hover_c = p.hover;
                let mut angle_card = div()
                    .relative()
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .p(px(4.))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(pro_pill_indicator(
                        pro_eid(&self.id, "angle-radio-pill"),
                        format!("{:?}", self.selling_angle),
                        angle_sensor.clone(),
                        10.,
                        p.chip,
                        None,
                        false,
                    ));
                for (ai, angle) in [
                    SellingAngle::DesireDriven,
                    SellingAngle::ProblemSolution,
                    SellingAngle::SocialProof,
                ]
                .into_iter()
                .enumerate()
                {
                    let checked = self.selling_angle == angle;
                    let bars_on = angle.signal_bars();
                    let off_bar_bg: Hsla = if p.dark {
                        rgb(0x3a3a3f).into()
                    } else {
                        rgb(0xd9d9d9).into()
                    };
                    let signal = div()
                        .h(px(11.))
                        .flex_none()
                        .flex()
                        .items_end()
                        .gap(px(2.))
                        .children([5.0f32, 8.0, 11.0].into_iter().enumerate().map(|(bi, bh)| {
                            let active_bar = (bi as u8) < bars_on;
                            div().w(px(3.)).h(px(bh)).rounded(px(1.)).bg(if active_bar {
                                p.green_ink
                            } else {
                                off_bar_bg
                            })
                        }));

                    let radio = if checked {
                        div()
                            .size(px(14.))
                            .flex_none()
                            .rounded_full()
                            .bg(p.ink)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(div().size(px(6.)).rounded_full().bg(p.card))
                    } else {
                        div()
                            .size(px(14.))
                            .flex_none()
                            .rounded_full()
                            .border_1()
                            .border_color(p.line_strong)
                    };

                    angle_card = angle_card.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("angle-{ai}")))
                            .role(Role::RadioButton)
                            .tab_index(0)
                            .relative()
                            .min_h(px(46.))
                            .px(px(8.))
                            .rounded(px(10.))
                            .when(checked, |d| d.child(pro_mark_selected(&angle_sensor)))
                            .when(!checked, |d| d.hover(move |s| s.bg(hover_c)))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .cursor_pointer()
                            .child(radio)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .line_height(px(18.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(if checked { p.ink } else { p.text })
                                            .truncate()
                                            .child(angle.label()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .line_height(px(17.))
                                            .text_color(p.faint)
                                            .truncate()
                                            .child(angle.subtitle()),
                                    ),
                            )
                            .child(signal),
                        cx,
                        move |this, _, cx| {
                            this.select_angle(angle, cx);
                        },
                    ));
                }

                // Audience pills card
                let audiences = [
                    "Women 25–34",
                    "Urban",
                    "Warm climates",
                    "Linen & natural fabrics",
                ];
                let mut aud_wrap = div().flex().flex_wrap().gap(px(8.));
                for (i, aud_lbl) in audiences.into_iter().enumerate() {
                    let on = self.audience_selected[i];
                    aud_wrap = aud_wrap.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("aud-{i}")))
                            .role(Role::Button)
                            .tab_index(0)
                            .px(px(10.))
                            .py(px(4.))
                            .rounded_full()
                            .border_1()
                            .border_color(if on { p.line_strong } else { p.line })
                            .when(!on, |d| d.opacity(0.55))
                            .hover(move |s| s.bg(p.hover))
                            .text_size(px(12.5))
                            .text_color(if on { p.text } else { p.faint })
                            .cursor_pointer()
                            .child(aud_lbl),
                        cx,
                        move |this, _, cx| {
                            this.audience_selected[i] = !this.audience_selected[i];
                            cx.notify();
                        },
                    ));
                }

                // Channels card
                let mut ch_card = div()
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .py(px(4.))
                    .px(px(12.))
                    .flex()
                    .flex_col();
                for (ci, ch) in self.channels.iter().enumerate() {
                    let cid = ch.id.clone();
                    let ch_svg = match cid.as_ref() {
                        "instagram" => SVG_CH_INSTAGRAM,
                        "tiktok" => SVG_CH_TIKTOK,
                        _ => SVG_CH_FACEBOOK,
                    };
                    ch_card = ch_card.child(
                        div()
                            .min_h(px(46.))
                            .when(ci > 0, |d| d.border_t_1().border_color(p.line_soft))
                            .flex()
                            .items_center()
                            .gap(px(11.))
                            .child(pro_svg("0 0 24 24", ch_svg, 16., p.muted))
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
                                            .child(ch.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.muted)
                                            .child(ch.subtitle.clone()),
                                    ),
                            )
                            .child(pro_switch(
                                pro_eid(&self.id, format!("ch-sw-{ci}")),
                                ch.name.clone(),
                                ch.enabled,
                                cx,
                                move |this, _, cx| {
                                    this.toggle_channel(&cid, cx);
                                },
                            )),
                    );
                }

                // Palette & Formats split card
                let swatches = [
                    ("Sand", rgb(0xe7dec8)),
                    ("Olive", rgb(0xe5ad19)),
                    ("Off-white", rgb(0xf5f4ee)),
                    ("Ink", rgb(0x1f1f23)),
                ];
                let palette_formats = div()
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .p(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(10.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "Palette", "配色")),
                            )
                            .child(div().flex().flex_wrap().gap(px(14.)).children(
                                swatches.into_iter().map(|(name, col)| {
                                    div()
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .gap(px(6.))
                                        .child(
                                            div()
                                                .size(px(22.))
                                                .rounded_full()
                                                .bg(col)
                                                .border_1()
                                                .border_color(p.line_strong),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(p.muted)
                                                .child(name),
                                        )
                                }),
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(10.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "Formats", "版式规格")),
                            )
                            .child(
                                div().flex().items_end().gap(px(16.)).children(
                                    [
                                        ("Feed", 20.0f32, 20.0f32),
                                        ("Story", 14.0, 24.0),
                                        ("Banner", 28.0, 16.0),
                                    ]
                                    .into_iter()
                                    .map(|(fname, fw, fh)| {
                                        div()
                                            .min_h(px(44.))
                                            .flex()
                                            .flex_col()
                                            .items_center()
                                            .justify_end()
                                            .gap(px(6.))
                                            .child(
                                                div()
                                                    .w(px(fw))
                                                    .h(px(fh))
                                                    .rounded(px(4.))
                                                    .border_1()
                                                    .border_color(p.faint),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .text_color(p.muted)
                                                    .child(fname),
                                            )
                                    }),
                                ),
                            ),
                    );

                // Details card
                let details_card = div()
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .p(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .children(
                        [
                            (
                                pro_icon(ProIcon::Calendar, 14., p.muted),
                                tr(cx, "Launch window", "投放周期"),
                                "May 12 – Jun 9",
                            ),
                            (
                                pro_icon(ProIcon::Coins, 14., p.muted),
                                tr(cx, "Daily budget", "日预算"),
                                "$450 / day",
                            ),
                            (
                                pro_svg("0 0 24 24", SVG_MAP_PIN, 14., p.muted),
                                tr(cx, "Markets", "目标市场"),
                                "US, UK, AU",
                            ),
                        ]
                        .into_iter()
                        .map(|(ic, dt_lbl, dd_val)| {
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(12.))
                                .text_size(px(13.))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .text_color(p.muted)
                                        .child(ic)
                                        .child(dt_lbl),
                                )
                                .child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(dd_val),
                                )
                        }),
                    );

                // Files section
                let brief_files = [
                    AgentArtifactRef::new(
                        "brand-guide.md",
                        "brand-guide.md",
                        AgentArtifactKind::Doc,
                    ),
                    AgentArtifactRef::new(
                        "launch-budget.csv",
                        "launch-budget.csv",
                        AgentArtifactKind::Sheet,
                    ),
                    AgentArtifactRef::new(
                        "product-photos",
                        "product-photos",
                        AgentArtifactKind::Folder,
                    ),
                ];
                let mut files_wrap = div().flex().flex_wrap().gap(px(6.));
                for (fi, art) in brief_files.iter().enumerate() {
                    files_wrap = files_wrap.child(self.render_file_chip(
                        format!("brief-art-{fi}"),
                        art,
                        p,
                        cx,
                    ));
                }

                // Bottom Generate ads bar
                let ad_count = self.planned_ad_count();
                let go_bar = div()
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .py(px(8.))
                    .pl(px(12.))
                    .pr(px(10.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .child(
                                div()
                                    .px(px(9.))
                                    .py(px(3.))
                                    .rounded_full()
                                    .border_1()
                                    .border_color(p.line_strong)
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.text)
                                    .child(format!("{ad_count} {}", tr(cx, "ads", "组广告"))),
                            )
                            .child(
                                div()
                                    .px(px(9.))
                                    .py(px(3.))
                                    .rounded_full()
                                    .border_1()
                                    .border_color(p.line_strong)
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.text)
                                    .child("~2 min"),
                            ),
                    )
                    .child(pro_btn(
                        pro_eid(&self.id, "generate-ads-btn"),
                        if self.ads_generated {
                            tr(cx, "View assets", "查看素材")
                        } else {
                            tr(cx, "Generate ads", "生成广告")
                        },
                        None,
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        |this, _, cx| {
                            this.ads_generated = true;
                            this.brief_section = BriefSection::Assets;
                            let count = this.planned_ad_count();
                            let angle = this.selling_angle;
                            cx.emit(AgentChatEvent::GenerateAds {
                                angle,
                                ad_count: count,
                            });
                            cx.notify();
                        },
                    ));

                div()
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .child(product_card)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .px(px(4.))
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "Selling angle", "核心卖点切入角")),
                            )
                            .child(angle_card),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .px(px(4.))
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "Audience", "目标用户")),
                            )
                            .child(
                                div()
                                    .rounded(px(12.))
                                    .bg(p.card)
                                    .border_1()
                                    .border_color(p.line_strong)
                                    .p(px(12.))
                                    .child(aud_wrap),
                            ),
                    )
                    .child(go_bar)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .px(px(4.))
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "Channels", "投放渠道")),
                            )
                            .child(ch_card),
                    )
                    .child(palette_formats)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .px(px(4.))
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "Details", "投放详情")),
                            )
                            .child(details_card),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .px(px(4.))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.muted)
                                            .child(tr(cx, "Files", "关联文件")),
                                    )
                                    .child(pro_icon_btn(
                                        pro_eid(&self.id, "brief-new-tab"),
                                        tr(cx, "New tab", "新标签页"),
                                        ProIcon::Plus,
                                        22.,
                                        false,
                                        cx,
                                        |this, _, cx| {
                                            this.open_artifact("new-tab", cx);
                                        },
                                    )),
                            )
                            .child(files_wrap),
                    )
                    .into_any_element()
            }
            BriefSection::Assets => {
                let copy_cards = match self.selling_angle {
                    SellingAngle::DesireDriven => [
                        (
                            "The first warm evening, dressed.",
                            "IG Feed",
                            "1:1",
                            0xe7dec8u32,
                            0x1e1b16u32,
                        ),
                        (
                            "Airy 170 gsm linen that catches the breeze.",
                            "TikTok",
                            "9:16",
                            0x2c2820,
                            0xf5f0e2,
                        ),
                        (
                            "Unbuttoned at 7 PM — Linen Overshirt.",
                            "IG Story",
                            "9:16",
                            0xdccaa5,
                            0x1c1914,
                        ),
                        (
                            "Cut relaxed for golden-hour layering.",
                            "Feed",
                            "4:5",
                            0x3b3526,
                            0xf2ebd9,
                        ),
                    ],
                    SellingAngle::ProblemSolution => [
                        (
                            "Zero itch. Pure airflow above 80°F.",
                            "IG Feed",
                            "1:1",
                            0xe7dec8,
                            0x1e1b16,
                        ),
                        (
                            "Ditch heavy twill for washed linen.",
                            "TikTok",
                            "9:16",
                            0x2c2820,
                            0xf5f0e2,
                        ),
                        (
                            "Stays cool on the commute.",
                            "IG Story",
                            "9:16",
                            0xdccaa5,
                            0x1c1914,
                        ),
                        (
                            "Pre-washed weave, soft from day one.",
                            "Feed",
                            "4:5",
                            0x3b3526,
                            0xf2ebd9,
                        ),
                    ],
                    SellingAngle::SocialProof => [
                        (
                            "412 sold this month · 4.9★ rated.",
                            "IG Feed",
                            "1:1",
                            0xe7dec8,
                            0x1e1b16,
                        ),
                        (
                            "\"The only summer layer I pack.\"",
                            "TikTok",
                            "9:16",
                            0x2c2820,
                            0xf5f0e2,
                        ),
                        (
                            "Back in Sand & Olive — #1 bestseller.",
                            "IG Story",
                            "9:16",
                            0xdccaa5,
                            0x1c1914,
                        ),
                        (
                            "Verified summer staple across 400+ orders.",
                            "Feed",
                            "4:5",
                            0x3b3526,
                            0xf2ebd9,
                        ),
                    ],
                };
                let mut rows = div().flex().flex_col().gap(px(12.));
                for pair in copy_cards.chunks(2) {
                    let mut row = div().flex().gap(px(10.));
                    for &(headline, ch_lbl, ratio, bg_hex, fg_hex) in pair {
                        row = row.child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .h(px(116.))
                                        .p(px(11.))
                                        .rounded(px(12.))
                                        .bg(rgb(bg_hex))
                                        .border_1()
                                        .border_color(p.line)
                                        .flex()
                                        .flex_col()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_size(px(14.))
                                                .line_height(px(17.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(rgb(fg_hex))
                                                .child(headline),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .text_color(Hsla::from(rgb(fg_hex)).opacity(0.7))
                                                .child("Acme · $89"),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .text_size(px(12.))
                                        .child(
                                            div()
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .child(ch_lbl),
                                        )
                                        .child(div().text_color(p.muted).child(ratio)),
                                ),
                        );
                    }
                    rows = rows.child(row);
                }
                rows.into_any_element()
            }
            BriefSection::Activity => {
                let events = [
                    (
                        ProIcon::Megaphone,
                        "Campaign manager drafted brief from Linen Overshirt",
                        "Just now",
                    ),
                    (
                        ProIcon::Sparkles,
                        "Creative direction aligned Sand & Olive swatches",
                        "1m ago",
                    ),
                    (
                        ProIcon::Users,
                        "Audience research matched warm-climate segment",
                        "2m ago",
                    ),
                    (
                        ProIcon::Database,
                        "Shopify sync imported 412 orders for Q3 launch",
                        "3m ago",
                    ),
                ];
                let len = events.len();
                div()
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .pt(px(14.))
                    .px(px(12.))
                    .pb(px(4.))
                    .flex()
                    .flex_col()
                    .children(
                        events
                            .into_iter()
                            .enumerate()
                            .map(|(ei, (ic, text, when))| {
                                div()
                                    .relative()
                                    .pb(px(16.))
                                    .flex()
                                    .items_start()
                                    .gap(px(12.))
                                    .when(ei + 1 < len, |d| {
                                        d.child(
                                            div()
                                                .absolute()
                                                .left(px(13.))
                                                .top(px(28.))
                                                .bottom(px(2.))
                                                .w(px(1.))
                                                .bg(p.line),
                                        )
                                    })
                                    .child(
                                        div()
                                            .size(px(27.))
                                            .flex_none()
                                            .rounded_full()
                                            .border_1()
                                            .border_color(p.line_strong)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(pro_icon(ic, 13., p.muted)),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .pt(px(2.))
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.))
                                            .child(
                                                div()
                                                    .text_size(px(12.5))
                                                    .line_height(px(18.))
                                                    .text_color(p.ink)
                                                    .child(text),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(11.5))
                                                    .text_color(p.faint)
                                                    .child(when),
                                            ),
                                    )
                            }),
                    )
                    .into_any_element()
            }
        };

        let section_key = if self.brief_section == BriefSection::Assets {
            format!("assets-{:?}", self.selling_angle)
        } else {
            format!("{:?}", self.brief_section)
        };
        div()
            .p(px(12.))
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(seg)
            .child(pro_view_transition(
                pro_eid(&self.id, "brief-sec-vtx"),
                section_key,
                content,
            ))
    }

    fn render_artifact_tab(
        &self,
        slot: WorkTabSlot,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Div {
        match slot {
            WorkTabSlot::Brief => self.render_brief_tab(p, cx),
            WorkTabSlot::BrandGuide => {
                let checks = [
                    "Lead with tactile coolness & natural drape",
                    "Keep headlines <= 7 words in sentence case",
                    "Pair Sand (#E7DEC8) backgrounds with Ink typography",
                ];
                let mut check_list = div().flex().flex_col().gap(px(6.));
                for (ci, line) in checks.into_iter().enumerate() {
                    let done = self.doc_checks[ci];
                    check_list = check_list.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("doc-chk-{ci}")))
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
                            this.doc_checks[ci] = !this.doc_checks[ci];
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
                                    .size(px(28.))
                                    .rounded(px(7.))
                                    .bg(p.chip)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(Self::render_artifact_glyph(
                                        AgentArtifactKind::Doc,
                                        13.,
                                        p,
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child("brand-guide.md"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.5))
                                            .text_color(p.muted)
                                            .child("Brand assets · Updated 12m ago"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .pt(px(8.))
                            .border_t_1()
                            .border_color(p.line_soft)
                            .flex()
                            .flex_col()
                            .gap(px(10.))
                            .child(
                                div()
                                    .text_size(px(15.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child("Summer Linen — Voice & Palette"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .line_height(px(19.5))
                                    .text_color(p.text)
                                    .child(
                                        "Write like a friend recommending one well-made piece before a weekend trip. Avoid urgency countdowns or exclamation marks.",
                                    ),
                            )
                            .child(check_list),
                    )
            }
            WorkTabSlot::LaunchBudget => {
                let rows = [
                    ["Instagram", "$210", "52%", "3.4x"],
                    ["TikTok", "$150", "33%", "3.1x"],
                    ["Facebook", "$90", "15%", "2.6x"],
                    ["Total", "$450", "100%", "3.2x"],
                ];
                let (sel_r, sel_c) = self.budget_cell;
                let col_letter = (b'A' + sel_c as u8) as char;
                let ref_str = format!("{col_letter}{}", sel_r + 2);
                let val_str = rows
                    .get(sel_r)
                    .and_then(|r| r.get(sel_c))
                    .copied()
                    .unwrap_or("");

                div()
                    .p(px(14.))
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
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
                                    .child(Self::render_artifact_glyph(
                                        AgentArtifactKind::Sheet,
                                        13.,
                                        p,
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .child("launch-budget.csv"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.5))
                                            .text_color(p.muted)
                                            .child("Q3 Campaign · Just now"),
                                    ),
                            ),
                    )
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
                                    .child(ref_str),
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
                            .child(div().px(px(6.)).text_color(p.ink).child(val_str)),
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
                                    .h(px(26.))
                                    .bg(p.chip)
                                    .px(px(8.))
                                    .flex()
                                    .items_center()
                                    .children(
                                        ["Channel", "Daily", "Share", "ROAS"].into_iter().map(
                                            |h| {
                                                div()
                                                    .flex_1()
                                                    .text_size(px(11.5))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.muted)
                                                    .child(h)
                                            },
                                        ),
                                    ),
                            )
                            .children(rows.into_iter().enumerate().map(|(ri, row)| {
                                div()
                                    .h(px(28.))
                                    .border_t_1()
                                    .border_color(p.line_soft)
                                    .flex()
                                    .items_center()
                                    .children(row.into_iter().enumerate().map(|(ci, cell)| {
                                        let is_sel = (ri, ci) == (sel_r, sel_c);
                                        activate(
                                            div()
                                                .id(pro_eid(&self.id, format!("bcell-{ri}-{ci}")))
                                                .role(Role::Cell)
                                                .tab_index(0)
                                                .flex_1()
                                                .h_full()
                                                .px(px(8.))
                                                .when(is_sel, |d| {
                                                    d.bg(p.accent_soft)
                                                        .border_1()
                                                        .border_color(p.blue)
                                                })
                                                .flex()
                                                .items_center()
                                                .text_size(px(12.))
                                                .font_weight(if ri == 3 {
                                                    FontWeight::MEDIUM
                                                } else {
                                                    FontWeight::NORMAL
                                                })
                                                .text_color(p.ink)
                                                .cursor_pointer()
                                                .child(cell),
                                            cx,
                                            move |this, _, cx| {
                                                this.budget_cell = (ri, ci);
                                                cx.notify();
                                            },
                                        )
                                    }))
                            })),
                    )
            }
            WorkTabSlot::ProductPhotos => {
                let items = [
                    ("overshirt-sand-front.jpg", "4.2 MB", "Today"),
                    ("overshirt-olive-detail.jpg", "3.8 MB", "Today"),
                    ("collar-weave-macro.png", "5.1 MB", "Yesterday"),
                    ("lookbook-warm-evening.jpg", "3.4 MB", "May 4"),
                ];
                let sel = self.folder_selected.min(items.len() - 1);
                div()
                    .p(px(14.))
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
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
                                            .child(Self::render_artifact_glyph(
                                                AgentArtifactKind::Folder,
                                                13.,
                                                p,
                                            )),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .text_size(px(13.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.ink)
                                                    .child("product-photos"),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(11.5))
                                                    .text_color(p.muted)
                                                    .child("4 items · 16.5 MB"),
                                            ),
                                    ),
                            )
                            .child(pro_btn(
                                pro_eid(&self.id, "ql-btn"),
                                tr(cx, "Quick Look", "快速预览"),
                                Some(ProIcon::Eye),
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                |this, _, cx| {
                                    this.quick_look_open = !this.quick_look_open;
                                    cx.notify();
                                },
                            )),
                    )
                    .child({
                        let (fname, fsize, _) = items[sel];
                        let card_c = p.card;
                        let line_s = p.line_strong;
                        let ink_c = p.ink;
                        let muted_c = p.muted;
                        let ql_vtx_id = pro_eid(&self.id, "ql-photo-vtx");
                        let ql_key: SharedString = format!("ql-{sel}").into();
                        pro_fold(
                            pro_eid(&self.id, "ql-photo-fold"),
                            self.quick_look_open,
                            146.,
                            12.,
                            move || {
                                pro_view_transition(
                                    ql_vtx_id,
                                    ql_key,
                                    div()
                                        .rounded(px(10.))
                                        .bg(card_c)
                                        .border_1()
                                        .border_color(line_s)
                                        .p(px(12.))
                                        .flex()
                                        .flex_col()
                                        .gap(px(8.))
                                        .child(
                                            div()
                                                .h(px(96.))
                                                .rounded(px(8.))
                                                .bg(rgb(0xe7dec8))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .child(pro_svg(
                                                    "0 0 24 24",
                                                    SVG_SHIRT_THUMB,
                                                    34.,
                                                    rgb(0x4a3d28).into(),
                                                )),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .justify_between()
                                                .text_size(px(12.))
                                                .child(div().text_color(ink_c).child(fname))
                                                .child(div().text_color(muted_c).child(fsize)),
                                        ),
                                )
                            },
                        )
                    })
                    .child({
                        let fd_sensor = pro_selection_sensor();
                        let hover_c = p.hover;
                        div()
                            .relative()
                            .rounded(px(8.))
                            .border_1()
                            .border_color(p.line)
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .child(pro_pill_indicator(
                                pro_eid(&self.id, "fd-row-pill"),
                                format!("fd-{sel}"),
                                fd_sensor.clone(),
                                0.,
                                p.accent_soft,
                                None,
                                false,
                            ))
                            .children(items.into_iter().enumerate().map(
                                |(fi, (fname, fsize, fmod))| {
                                    let selected = fi == sel;
                                    let sensor_for_row = fd_sensor.clone();
                                    activate(
                                        div()
                                            .id(pro_eid(&self.id, format!("fd-row-{fi}")))
                                            .role(Role::ListBoxOption)
                                            .tab_index(0)
                                            .relative()
                                            .h(px(32.))
                                            .px(px(10.))
                                            .when(fi > 0, |d| {
                                                d.border_t_1().border_color(p.line_soft)
                                            })
                                            .when(selected, |d| {
                                                d.child(pro_mark_selected(&sensor_for_row))
                                            })
                                            .when(!selected, |d| d.hover(move |s| s.bg(hover_c)))
                                            .flex()
                                            .items_center()
                                            .gap(px(8.))
                                            .cursor_pointer()
                                            .child(pro_icon(ProIcon::PageDoc, 13., p.pink))
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .truncate()
                                                    .text_size(px(12.5))
                                                    .text_color(p.ink)
                                                    .child(fname),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(11.5))
                                                    .text_color(p.muted)
                                                    .child(format!("{fsize} · {fmod}")),
                                            ),
                                        cx,
                                        move |this, _, cx| {
                                            this.folder_selected = fi;
                                            cx.notify();
                                        },
                                    )
                                },
                            ))
                    })
            }
            WorkTabSlot::NewTab => div()
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.faint)
                        .child(tr(cx, "Suggested", "推荐打开")),
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
                        .children(
                            [
                                ("summer-launch", "Summer launch", "Campaign brief"),
                                ("brand-guide.md", "brand-guide.md", "Brand assets"),
                                ("launch-budget.csv", "launch-budget.csv", "Channel sheet"),
                                ("product-photos", "product-photos", "4 studio photos"),
                            ]
                            .into_iter()
                            .enumerate()
                            .map(|(si, (key, name, meta))| {
                                activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("sug-{si}")))
                                        .role(Role::Button)
                                        .tab_index(0)
                                        .h(px(38.))
                                        .px(px(10.))
                                        .when(si > 0, |d| d.border_t_1().border_color(p.line_soft))
                                        .hover(move |s| s.bg(p.hover))
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .cursor_pointer()
                                        .child(
                                            div()
                                                .text_size(px(12.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .child(name),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.5))
                                                .text_color(p.muted)
                                                .child(meta),
                                        ),
                                    cx,
                                    move |this, _, cx| {
                                        this.open_artifact(key, cx);
                                    },
                                )
                            }),
                        ),
                ),
        }
    }

    fn render_inspector(
        &mut self,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SharedString, ProIcon, AnyElement)> {
        let p = ProPalette::get(cx);
        let phase = self.skeleton_phase(w, cx);
        let active_slot = self
            .work_tabs
            .get(self.active_work_tab)
            .copied()
            .unwrap_or(WorkTabSlot::Brief);
        let title = active_slot.title(cx);
        let icon = active_slot.icon();

        if self.loading {
            return Some((
                title,
                icon,
                self.render_brief_loading(phase, cx).into_any_element(),
            ));
        }

        let mut root = div().flex().flex_col().min_h_full();

        if self.work_tabs.len() > 1 {
            let ptab_sensor = pro_selection_sensor();
            let hover_c = p.hover;
            let mut strip = div()
                .relative()
                .px(px(10.))
                .py(px(6.))
                .border_b_1()
                .border_color(p.line_soft)
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(4.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "ptab-pill"),
                    format!("ptab-{}:{}", self.active_work_tab, active_slot.id()),
                    ptab_sensor.clone(),
                    6.,
                    p.chip,
                    None,
                    false,
                ));
            for (ti, &slot) in self.work_tabs.iter().enumerate() {
                let selected = ti == self.active_work_tab;
                let t_lbl = slot.title(cx);
                strip = strip.child(
                    div()
                        .relative()
                        .h(px(26.))
                        .pl(px(8.))
                        .pr(px(4.))
                        .rounded(px(6.))
                        .when(selected, |d| d.child(pro_mark_selected(&ptab_sensor)))
                        .when(!selected, |d| d.hover(move |s| s.bg(hover_c)))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("ptab-{ti}")))
                                .role(Role::Tab)
                                .aria_selected(selected)
                                .tab_index(0)
                                .text_size(px(12.))
                                .font_weight(if selected {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .text_color(if selected { p.ink } else { p.muted })
                                .cursor_pointer()
                                .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _, cx| {
                                    if ev.keystroke.key == "delete"
                                        || ev.keystroke.key == "backspace"
                                    {
                                        this.close_work_tab(ti, cx);
                                    }
                                }))
                                .child(t_lbl.clone()),
                            cx,
                            move |this, _, cx| {
                                this.active_work_tab = ti;
                                cx.notify();
                            },
                        ))
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("ptab-x-{ti}")))
                                .role(Role::Button)
                                .aria_label(format!("Close {t_lbl}"))
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
                            move |this, _, cx| {
                                this.close_work_tab(ti, cx);
                            },
                        )),
                );
            }
            strip = strip.child(pro_icon_btn(
                pro_eid(&self.id, "ptab-add"),
                tr(cx, "New tab", "新标签页"),
                ProIcon::Plus,
                24.,
                false,
                cx,
                |this, _, cx| {
                    this.open_artifact("new-tab", cx);
                },
            ));
            root = root.child(strip);
        }

        root = root.child(pro_view_transition(
            pro_eid(&self.id, "ptab-body-vtx"),
            format!("{}:{}", active_slot.id(), self.empty),
            self.render_artifact_tab(active_slot, &p, cx),
        ));
        Some((title, icon, root.into_any_element()))
    }
}

impl Render for AgentChatScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentChatFeedback, AgentChatRole, AgentChatScreen, SellingAngle};
    use gpui::{AppContext, TestAppContext};

    #[test]
    fn brief_channels_and_angle_update_ad_count_and_thread() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let screen = app.new(|cx| AgentChatScreen::new("test-agent-chat", cx));

        screen.update(&mut app, |s, cx| {
            // Instagram + TikTok enabled by default -> 6 ads.
            assert_eq!(s.planned_ad_count(), 6);
            assert_eq!(s.selling_angle(), SellingAngle::DesireDriven);

            s.toggle_channel("facebook", cx);
            assert_eq!(s.planned_ad_count(), 9);

            s.select_angle(SellingAngle::SocialProof, cx);
            assert_eq!(s.selling_angle(), SellingAngle::SocialProof);
            assert_eq!(
                s.messages()[0].paragraphs[0].as_ref(),
                SellingAngle::SocialProof.label()
            );
        });
    }

    #[test]
    fn loading_blocks_submission_and_closing_last_tab_restores_brief() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let screen = app.new(|cx| {
            AgentChatScreen::new("test-agent-chat-tabs", cx)
                .with_loading(true)
                .with_empty(true, cx)
        });

        screen.update(&mut app, |s, cx| {
            // While loading, sending waits even if text is non-empty.
            assert!(s.is_loading());
            assert!(!s.submit_prompt("Draft 3 linen Story hooks", cx));

            // Clearing loading unlocks submission and exits empty state.
            s.set_loading(false, cx);
            assert!(s.is_empty());
            assert!(s.submit_prompt("Draft 3 linen Story hooks", cx));
            assert!(!s.is_empty());
            assert_eq!(s.messages().len(), 2);
            assert_eq!(s.messages()[1].role, AgentChatRole::Assistant);

            // Feedback toggles cleanly on and off.
            let aid = s.messages()[1].id.clone();
            s.toggle_feedback(&aid, AgentChatFeedback::Up, cx);
            assert_eq!(s.messages()[1].feedback, Some(AgentChatFeedback::Up));
            s.toggle_feedback(&aid, AgentChatFeedback::Up, cx);
            assert_eq!(s.messages()[1].feedback, None);

            // Opening an artifact adds a tab; closing all tabs folds panel and restores Brief.
            s.open_artifact("brand-guide.md", cx);
            assert_eq!(s.work_tabs.len(), 2);
            s.close_work_tab(1, cx);
            s.close_work_tab(0, cx);
            assert_eq!(s.work_tabs.len(), 1);
            assert!(!s.shell.read(cx).is_panel_open());
        });
    }
}
