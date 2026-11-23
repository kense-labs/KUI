//! Interactive node-graph workflow builder with palette, live test execution, and step inspector.
//!
//! Renders [`WorkflowBuilderScreen`] inside [`AppShell`] (`"Northwind"` workspace, `"Workflows"` active)
//! with a zoomable/pannable dot-grid canvas, cubic Bezier edges, a floating node palette,
//! `+` step insertion picker, comment pins, a right-hand step settings inspector, and a
//! multi-tab test-run drawer that animates execution across edges and nodes.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_agent_avatar, pro_app_tile, pro_btn, pro_eid,
            pro_empty_state, pro_fold, pro_icon_btn, pro_mark_selected, pro_person_avatar,
            pro_pill_indicator, pro_presence, pro_selection_sensor, pro_skeleton_bar,
            pro_spring_bar, pro_spring_f32, pro_switch, pro_toast_banner, pro_view_transition, tr,
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

const NODE_W: f32 = 272.;
const NODE_H: f32 = 110.;

const SVG_ROCKET: &str = r#"<path d="M6.21875 11.618L3.31189 11.2378C2.71644 11.1606 2.31883 10.5841 2.58348 10.0458C3.42523 8.33365 5.6195 6.35437 9.73988 6.68079M6.21875 11.618C7.27445 9.85426 8.57175 8.00435 9.73988 6.68079M6.21875 11.618L11.882 17.2812M9.73988 6.68079C13.4105 2.58986 17.1745 1.728 19.5937 2.06685C20.5508 2.2009 21.2991 2.94917 21.4332 3.90626C21.772 6.32551 20.9101 10.0895 16.8192 13.7601M11.882 17.2812L12.2622 20.1881C12.3394 20.7836 12.9159 21.1812 13.4542 20.9165C15.1664 20.0748 17.1456 17.8805 16.8192 13.7601M11.882 17.2812C13.6457 16.2255 15.4956 14.9282 16.8192 13.7601" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M17.5 8C17.5 6.89543 16.6046 6 15.5 6C14.3954 6 13.5 6.89543 13.5 8C13.5 9.10457 14.3954 10 15.5 10C16.6046 10 17.5 9.10457 17.5 8Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M4 22L8 18M4 17L5.5 15.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_USER_PLUS: &str = r#"<path d="M3 20.5002C3.28417 16.8058 6.3 13.7193 10.0008 13.5379C10.3134 13.5226 10.6446 13.5097 11 13.5L11.995 13.5663C12.6939 13.6129 13.3665 13.7543 14 13.9777" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M18 15.5V21.5M21 18.5L15 18.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><circle cx="11" cy="6.5" r="4" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></circle>"#;

const SVG_CLOUD_UPLOAD: &str = r#"<path d="M17.4776 9.01106C17.485 9.01102 17.4925 9.01101 17.5 9.01101C19.9853 9.01101 22 11.0294 22 13.5193C22 15.8398 20.25 17.7508 18 18M17.4776 9.01106C17.4924 8.84606 17.5 8.67896 17.5 8.51009C17.5 5.46695 15.0376 3 12 3C9.12324 3 6.76233 5.21267 6.52042 8.03192M17.4776 9.01106C17.3753 10.1476 16.9286 11.1846 16.2428 12.0165M6.52042 8.03192C3.98398 8.27373 2 10.4139 2 13.0183C2 15.4417 3.71776 17.4632 6 17.9273M6.52042 8.03192C6.67826 8.01687 6.83823 8.00917 7 8.00917C8.12582 8.00917 9.16474 8.38194 10.0005 9.01101" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M12 13L12 21M12 13C11.2998 13 9.99153 14.9943 9.5 15.5M12 13C12.7002 13 14.0085 14.9943 14.5 15.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_WEBHOOK: &str = r#"<path d="M5.062 13C3.83229 13.6824 3 14.994 3 16.5C3 18.7091 4.79086 20.5 7 20.5C9.20914 20.5 11 18.7091 11 16.5H17" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M12 7.5L15.0571 13.0027C15.6323 12.6825 16.2949 12.5 17 12.5C19.2091 12.5 21 14.2909 21 16.5C21 18.7091 19.2091 20.5 17 20.5C16.0541 20.5 15.1848 20.1716 14.5 19.6227" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M16 7.5C16 5.29086 14.2091 3.5 12 3.5C9.79086 3.5 8 5.29086 8 7.5C8 9.004 8.83007 10.3141 10.0571 10.9973L7 16.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_BRAIN: &str = r#"<path d="M7 4.5C5.34315 4.5 4 5.84315 4 7.5C4 8.06866 4.15822 8.60037 4.43304 9.0535C3.04727 9.31855 2 10.537 2 12C2 13.463 3.04727 14.6814 4.43304 14.9465M7 4.5C7 3.11929 8.11929 2 9.5 2C10.8807 2 12 3.11929 12 4.5V19.5C12 20.8807 10.8807 22 9.5 22C8.11929 22 7 20.8807 7 19.5C5.34315 19.5 4 18.1569 4 16.5C4 15.9313 4.15822 15.3996 4.43304 14.9465" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M17 19.4999C18.6569 19.4999 20 18.1567 20 16.4999C20 15.9312 19.8418 15.3995 19.567 14.9464C20.9527 14.6813 22 13.4629 22 11.9999C22 10.5369 20.9527 9.31843 19.567 9.05338M17 19.4999C17 20.8806 15.8807 21.9999 14.5 21.9999C13.1193 21.9999 12 20.8806 12 19.4999L12 4.49988C12 3.11917 13.1193 1.99988 14.5 1.99988C15.8807 1.99988 17 3.11917 17 4.49988C18.6569 4.49988 20 5.84302 20 7.49988C20 8.06854 19.8418 8.60024 19.567 9.05338" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_CALC: &str = r#"<path d="M5.5 3V8M8 5.5L3 5.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M8 16L4 20M8 20L4 16" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M20 6L16 6M20 18.5L16 18.5M20 15.5L16 15.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M22 12L2 12M12 22L12 2" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_SPLIT_BRANCH: &str = r#"<path d="M7 19H13C15.8284 19 17.2426 19 18.1213 18.1213C19 17.2426 19 15.8284 19 13V10M19 10C19.7002 10 21.0085 11.9943 21.5 12.5M19 10C18.2998 10 16.9915 11.9943 16.5 12.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M5 7L5 17" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><circle cx="5" cy="5" r="2" stroke="currentColor" stroke-width="1.9"></circle><circle cx="19" cy="5" r="2" stroke="currentColor" stroke-width="1.9"></circle><circle cx="5" cy="19" r="2" stroke="currentColor" stroke-width="1.9"></circle>"#;

const SVG_HOURGLASS: &str = r#"<path d="M4 3H20M4 21H20" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M5.5 3V5.03C5.5 6.27 6.08 7.44 7.06 8.2L12 12L16.94 8.2C17.92 7.44 18.5 6.27 18.5 5.03V3" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M5.5 21V18.97C5.5 17.73 6.08 16.56 7.06 15.8L12 12L16.94 15.8C17.92 16.56 18.5 17.73 18.5 18.97V21" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_LOOP: &str = r#"<path d="M16.3884 3L17.3913 3.97574C17.8393 4.41165 18.0633 4.62961 17.9844 4.81481C17.9056 5 17.5888 5 16.9552 5H9.19422C5.22096 5 2 8.13401 2 12C2 13.4872 2.47668 14.8662 3.2895 16" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M7.61156 21L6.60875 20.0243C6.16074 19.5883 5.93673 19.3704 6.01557 19.1852C6.09441 19 6.4112 19 7.04478 19H14.8058C18.779 19 22 15.866 22 12C22 10.5128 21.5233 9.13383 20.7105 8" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_STOP_END: &str = r#"<circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="1.9"></circle><rect x="8.5" y="8.5" width="7" height="7" rx="1.8" stroke="currentColor" stroke-width="1.9"></rect>"#;

const SVG_CURSOR_SELECT: &str = r#"<path d="M5.10772 14.3857L5.58594 7.91256C5.61875 7.46854 5.64642 7.05187 5.67232 6.66186C5.85017 3.98379 5.94481 2.55876 7.04807 2.10979C8.15132 1.66082 9.2022 2.61969 11.1771 4.42168C11.4647 4.68413 11.772 4.96446 12.1018 5.26093L16.9102 9.58273C18.2626 10.7983 18.9389 11.4062 18.9934 11.9885C19.0309 12.3882 18.9067 12.7862 18.6489 13.0924C18.2733 13.5385 17.3734 13.6473 15.5737 13.8647C14.8156 13.9563 14.4365 14.0021 14.2073 14.2038C14.0479 14.344 13.9376 14.5321 13.8925 14.7404C13.8277 15.0399 13.9707 15.3964 14.2567 16.1095L15.7394 19.8058C15.9107 20.2328 15.9963 20.4464 15.995 20.6429C15.9932 20.9078 15.8865 21.1609 15.6986 21.3462C15.5591 21.4837 15.3471 21.57 14.9232 21.7425C14.4993 21.915 14.2873 22.0013 14.0921 22C13.8292 21.9982 13.5778 21.8907 13.3939 21.7015C13.2574 21.561 13.1717 21.3475 13.0004 20.9204L11.5177 17.2241C11.2317 16.5111 11.0887 16.1545 10.8355 15.9844C10.6595 15.8662 10.4503 15.8081 10.239 15.8187C9.935 15.834 9.63074 16.0663 9.02224 16.5308C7.57763 17.6337 6.85532 18.1851 6.27746 18.1269C5.88085 18.0871 5.51701 17.8877 5.26831 17.574C4.90595 17.1169 4.9732 16.2065 5.10772 14.3857Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_COMMENT_TOOL: &str = r#"<path d="M8 13.5H16M8 8.5H12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M6.09881 19C4.7987 18.8721 3.82475 18.4816 3.17157 17.8284C2 16.6569 2 14.7712 2 11V10.5C2 6.72876 2 4.84315 3.17157 3.67157C4.34315 2.5 6.22876 2.5 10 2.5H14C17.7712 2.5 19.6569 2.5 20.8284 3.67157C22 4.84315 22 6.72876 22 10.5V11C22 14.7712 22 16.6569 20.8284 17.8284C19.6569 19 17.7712 19 14 19C13.4395 19.0125 12.9931 19.0551 12.5546 19.155C11.3562 19.4309 10.2465 20.0441 9.14987 20.5789C7.58729 21.3408 6.806 21.7218 6.31569 21.3651C5.37769 20.6665 6.29454 18.5019 6.5 17.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_FIT_VIEW: &str = r#"<path d="M3 4V20M21 4V20" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M15.6216 9L16.5002 9.87868C17.5002 10.8787 18.0002 11.3787 18.0002 12C18.0002 12.6213 17.5002 13.1213 16.5002 14.1213L15.6216 15M8.37892 9L7.50024 9.87868C6.50024 10.8787 6.00024 11.3787 6.00024 12C6.00024 12.6213 6.50024 13.1213 7.50024 14.1213L8.37892 15M6.37892 12H17.6216" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_DOC_MINI: &str = r#"<path d="M3.3 1h3.9L10 3.8v6.4c0 .72-.58 1.3-1.3 1.3H3.3c-.72 0-1.3-.58-1.3-1.3V2.3C2 1.58 2.58 1 3.3 1Z" fill="currentColor" opacity="0.45"></path><path d="M7.2 1v1.8c0 .55.45 1 1 1H10Z" fill="currentColor"></path><path d="M4 6.6h4M4 8.6h2.6" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"></path>"#;

const SVG_CHECK_SM: &str = r#"<path d="M4.5 12.5l4 4 11-11" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round"></path>"#;

const APP_ACCOUNTS: [(&str, &str); 6] = [
    ("slack", "Slack · northwind-prod"),
    ("google drive", "Google Drive · ops-shared"),
    ("stripe", "Stripe · live-billing"),
    ("gmail", "Gmail · support@northwind.co"),
    ("hubspot", "HubSpot · crm-sync"),
    ("database", "Postgres · primary-db"),
];

const AGENT_MODELS: [&str; 3] = ["Auto (Claude 3.7 Sonnet)", "GPT-4o", "Gemini 2.5 Pro"];

const AGENT_TOOLS: [&str; 4] = ["CRM lookup", "Web search", "Billing API", "Slack directory"];

const WORKFLOW_PROJECTS: [&str; 4] = [
    "Customer onboarding",
    "Invoice reconciliation",
    "Lead enrichment",
    "Weekly usage digest",
];

const WORKFLOW_BRANCHES: [(&str, &str); 2] = [("Main", "PRODUCTION"), ("Draft", "STAGING")];

/// Semantic category of a [`WorkflowNode`], controlling its pill badge and accent tint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WorkflowNodeKind {
    /// Entry-point event trigger (`TRIGGER`, blue).
    #[default]
    Trigger,
    /// Autonomous LLM / tool-calling step (`AGENT`, violet).
    Agent,
    /// Deterministic HTTP, parser, or math step (`TOOL`, neutral).
    Tool,
    /// Conditional branch, merge, wait, or loop (`LOGIC`, amber).
    Logic,
    /// Side-effecting database, Slack, email, or webhook step (`ACTION`, green).
    Action,
}

impl WorkflowNodeKind {
    /// Localized uppercase kind badge text.
    pub fn badge_label(self, cx: &App) -> &'static str {
        match self {
            Self::Trigger => tr(cx, "TRIGGER", "触发器"),
            Self::Agent => tr(cx, "AGENT", "智能体"),
            Self::Tool => tr(cx, "TOOL", "工具"),
            Self::Logic => tr(cx, "LOGIC", "逻辑"),
            Self::Action => tr(cx, "ACTION", "动作"),
        }
    }

    fn colors(self, p: &ProPalette) -> (Hsla, Hsla) {
        match self {
            Self::Trigger => (p.accent_soft, p.blue),
            Self::Agent => (p.purple.opacity(0.16), p.purple),
            Self::Tool => (p.chip, p.muted),
            Self::Logic => (p.orange.opacity(0.15), p.orange),
            Self::Action => (p.green_soft, p.green_ink),
        }
    }
}

/// Output port identifier from which a [`WorkflowEdge`] leaves a node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WorkflowPort {
    /// Single centered bottom output port.
    #[default]
    Out,
    /// Left branch output (`Yes`).
    Yes,
    /// Right branch output (`No`).
    No,
}

impl WorkflowPort {
    fn offset_x(self) -> f32 {
        match self {
            Self::Out => NODE_W * 0.5,
            Self::Yes => 80.,
            Self::No => 192.,
        }
    }

    fn tag_label(self, cx: &App) -> Option<&'static str> {
        match self {
            Self::Out => None,
            Self::Yes => Some(tr(cx, "YES", "是")),
            Self::No => Some(tr(cx, "NO", "否")),
        }
    }
}

/// Active pointer tool on the [`WorkflowBuilderScreen`] canvas toolbar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WorkflowCanvasTool {
    /// Select, drag nodes, or pan the background.
    #[default]
    Select,
    /// Click anywhere on the canvas to drop a comment pin.
    Comment,
}

/// Tab selection in the bottom test-run drawer (`.ui-wf-run`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WorkflowRunTab {
    /// Sample JSON trigger payload.
    Payload,
    /// Configured vs unconfigured workflow triggers.
    Triggers,
    /// Validation warnings and step execution errors.
    Errors,
    /// Horizontal step latency waterfall chart.
    Timeline,
    /// Step-by-step test run history and credits.
    #[default]
    Runs,
}

/// Node template entry inside the left `"Nodes"` palette and `+` step picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowPaletteItem {
    /// Unique template slug (`"new-signup"`, `"ai-agent"`, `"branch"`, etc.).
    pub node_type: SharedString,
    /// Display name in the palette list.
    pub label: SharedString,
    /// Category tint and behavior kind.
    pub kind: WorkflowNodeKind,
    /// Default one-line subtitle placed on newly added nodes.
    pub sub: SharedString,
    /// Default monospace expression / endpoint / prompt.
    pub default_expr: SharedString,
}

/// Grouped section inside the `"Nodes"` palette (`Triggers`, `Tools`, `Logic`, `Actions`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowPaletteGroup {
    /// Group identifier.
    pub id: SharedString,
    /// Uppercase group heading.
    pub label: SharedString,
    /// Items belonging to this group.
    pub items: Vec<WorkflowPaletteItem>,
}

/// Interactive step card positioned on the [`WorkflowBuilderScreen`] canvas.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkflowNode {
    /// Stable node identifier.
    pub id: SharedString,
    /// Palette template slug (`"new-signup"`, `"document-upload"`, `"ai-agent"`, etc.).
    pub node_type: SharedString,
    /// Semantic kind (`Trigger`, `Agent`, `Tool`, `Logic`, `Action`).
    pub kind: WorkflowNodeKind,
    /// Node card heading.
    pub title: SharedString,
    /// One-line description under the heading.
    pub sub: SharedString,
    /// Resting right-hand status/meta chip (`"Ready"`, `"2 tools"`, `"GET"`, `"2 paths"`).
    pub chip: SharedString,
    /// Whether required trigger/action settings are complete (`false` shows `"Not configured"`).
    pub configured: bool,
    /// Whether this step is enabled during workflow execution.
    pub enabled: bool,
    /// World X coordinate in logical canvas pixels.
    pub x: f32,
    /// World Y coordinate in logical canvas pixels.
    pub y: f32,
    /// Optional connected provider/app slug rendered via `pro_app_tile`.
    pub app: Option<SharedString>,
    /// Connected account label edited in the right inspector.
    pub account: SharedString,
    /// Selected model name for `Agent` steps.
    pub model: SharedString,
    /// Active tool names for `Agent` steps.
    pub tools: Vec<SharedString>,
    /// Accepted file extensions for upload triggers (`.pdf`, `.docx`).
    pub files: Vec<SharedString>,
    /// Editable prompt, endpoint URL, branch condition, or payload mapping.
    pub expression: SharedString,
    /// Sample JSON output returned when testing this step.
    pub sample_output: SharedString,
}

impl WorkflowNode {
    /// Construct a configured node at canvas coordinates `(x, y)`.
    pub fn new(
        id: impl Into<SharedString>,
        node_type: impl Into<SharedString>,
        kind: WorkflowNodeKind,
        title: impl Into<SharedString>,
        x: f32,
        y: f32,
    ) -> Self {
        Self {
            id: id.into(),
            node_type: node_type.into(),
            kind,
            title: title.into(),
            sub: SharedString::default(),
            chip: "Ready".into(),
            configured: true,
            enabled: true,
            x,
            y,
            app: None,
            account: "Slack · northwind-prod".into(),
            model: "Auto (Claude 3.7 Sonnet)".into(),
            tools: Vec::new(),
            files: Vec::new(),
            expression: SharedString::default(),
            sample_output: "{\n  \"ok\": true\n}".into(),
        }
    }
}

/// Directed Bezier connection between two [`WorkflowNode`]s.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowEdge {
    /// Unique edge identifier.
    pub id: SharedString,
    /// Source node id.
    pub from: SharedString,
    /// Output port on the source node (`Out`, `Yes`, `No`).
    pub port: WorkflowPort,
    /// Destination node id.
    pub to: SharedString,
}

impl WorkflowEdge {
    /// Construct an edge from `(from, port)` to `to`.
    pub fn new(
        id: impl Into<SharedString>,
        from: impl Into<SharedString>,
        port: WorkflowPort,
        to: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            from: from.into(),
            port,
            to: to.into(),
        }
    }
}

/// Pinned canvas annotation placed with the `Comment` tool.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkflowComment {
    /// Unique comment pin identifier.
    pub id: SharedString,
    /// World X coordinate on the canvas.
    pub x: f32,
    /// World Y coordinate on the canvas.
    pub y: f32,
    /// Author full name.
    pub author: SharedString,
    /// Comment text body.
    pub text: SharedString,
}

/// Per-step timing and cost recorded during a workflow test run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowStepRun {
    /// Executed node id.
    pub node_id: SharedString,
    /// Node title at execution time.
    pub title: SharedString,
    /// Node kind badge.
    pub kind: WorkflowNodeKind,
    /// Step latency in milliseconds.
    pub duration_ms: u64,
    /// Credits / tokens consumed.
    pub credits: u32,
    /// Optional error message if the step failed.
    pub error: Option<SharedString>,
}

/// Completed or in-flight test run entry displayed in the bottom `Runs` drawer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowRunRecord {
    /// Run identifier (`"#104"`).
    pub id: SharedString,
    /// Relative start label (`"Just now"`).
    pub started_at: SharedString,
    /// Ordered step outcomes.
    pub steps: Vec<WorkflowStepRun>,
    /// Total duration in milliseconds.
    pub total_ms: u64,
    /// Total credits consumed.
    pub total_credits: u32,
}

/// Events emitted by [`WorkflowBuilderScreen`].
#[derive(Clone, Debug)]
pub enum WorkflowBuilderEvent {
    /// User clicked a sidebar item in [`AppShell`].
    Navigate(SharedString),
    /// User switched the active workflow project in the path bar.
    ProjectChanged(SharedString),
    /// User switched the active branch (`Main` / `Draft`).
    BranchChanged(SharedString),
    /// User toggled or triggered workflow publishing.
    Published(bool),
    /// User selected a node to inspect.
    SelectNode(SharedString),
    /// User added a node from the palette or `+` insertion picker.
    NodeAdded(WorkflowNode),
    /// User modified a node's configuration in the right inspector.
    NodeUpdated(WorkflowNode),
    /// User deleted a node from the canvas.
    NodeDeleted(SharedString),
    /// User started a live workflow test run.
    RunStarted,
    /// Live workflow test run completed.
    RunFinished { total_ms: u64, steps: usize },
}

impl EventEmitter<WorkflowBuilderEvent> for WorkflowBuilderScreen {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TopMenu {
    Org,
    Project,
    Branch,
    InspectorAccount,
    InspectorModel,
}

#[derive(Clone)]
enum CanvasDrag {
    Pan {
        start_mouse: Point<Pixels>,
        orig_pan_x: f32,
        orig_pan_y: f32,
    },
    Node {
        node_id: SharedString,
        start_mouse: Point<Pixels>,
        orig_x: f32,
        orig_y: f32,
        moved: bool,
    },
}

#[derive(Clone)]
struct InsertPickerState {
    after_node_id: SharedString,
    port: WorkflowPort,
}

#[derive(Clone)]
struct EdgePaintSpec {
    p0: (f32, f32),
    p3: (f32, f32),
    selected: bool,
    lit_done: bool,
    flowing: bool,
    flow_t: f32,
    sender_color: Hsla,
}

const RUN_STAGE_MS: f32 = 560.;

/// Visual node-graph automation builder with palette, step inspector, and animated test runner.
pub struct WorkflowBuilderScreen {
    id: ElementId,
    id_name: SharedString,
    shell: Entity<AppShell>,
    standalone: bool,
    workspace: SharedString,
    project: SharedString,
    branch: SharedString,
    user_name: SharedString,
    user_email: SharedString,
    published: bool,
    palette_open: bool,
    palette: Vec<WorkflowPaletteGroup>,
    nodes: Vec<WorkflowNode>,
    edges: Vec<WorkflowEdge>,
    comments: Vec<WorkflowComment>,
    open_comment_id: Option<SharedString>,
    tool: WorkflowCanvasTool,
    zoom: f32,
    pan_x: f32,
    pan_y: f32,
    centered_once: bool,
    drag: Option<CanvasDrag>,
    selected_node_id: Option<SharedString>,
    last_inspected_id: Option<SharedString>,
    synced_node_id: Option<SharedString>,
    selected_edge_id: Option<SharedString>,
    insert_picker: Option<InsertPickerState>,
    last_picker: Option<InsertPickerState>,
    picker_editor: Option<Entity<Editor>>,
    node_title_editor: Option<Entity<Editor>>,
    node_expr_editor: Option<Entity<Editor>>,
    comment_editor: Option<Entity<Editor>>,
    top_menu: Option<TopMenu>,
    run_panel_open: bool,
    run_tab: WorkflowRunTab,
    run_started_at: Option<Instant>,
    run_order: Vec<(SharedString, Option<SharedString>, u64, u32)>,
    runs: Vec<WorkflowRunRecord>,
    step_test_banner: Option<(SharedString, SharedString)>,
    toast: Option<(SharedString, Instant)>,
    next_node_seq: usize,
    loading: bool,
    empty: bool,
    editor_focused: bool,
    focus: FocusHandle,
    boot_at: Instant,
    measured_width: Rc<Cell<f32>>,
    canvas_size: Rc<Cell<(f32, f32)>>,
    canvas_origin: Rc<Cell<(f32, f32)>>,
}

impl Focusable for WorkflowBuilderScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl WorkflowBuilderScreen {
    fn build_shell(
        id_str: &str,
        workspace: SharedString,
        user_name: SharedString,
        user_email: SharedString,
        run_count: u32,
        cx: &mut Context<Self>,
    ) -> Entity<AppShell> {
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let groups = vec![
                ShellNavGroup::primary(vec![
                    ShellNavItem::new(
                        "workflows",
                        tr(shell_cx, "Workflows", "工作流"),
                        ProIcon::GitBranch,
                    ),
                    ShellNavItem::new("runs", tr(shell_cx, "Runs", "运行记录"), ProIcon::Play)
                        .with_count(run_count),
                    ShellNavItem::new(
                        "templates",
                        tr(shell_cx, "Templates", "模板库"),
                        ProIcon::Grid,
                    ),
                    ShellNavItem::new(
                        "integrations",
                        tr(shell_cx, "Integrations", "应用集成"),
                        ProIcon::Bolt,
                    ),
                    ShellNavItem::new(
                        "variables",
                        tr(shell_cx, "Variables", "环境变量"),
                        ProIcon::Code,
                    ),
                    ShellNavItem::new(
                        "data-tables",
                        tr(shell_cx, "Data tables", "数据表"),
                        ProIcon::Database,
                    ),
                ]),
                ShellNavGroup::named(
                    tr(shell_cx, "Workspace", "工作区"),
                    vec![
                        ShellNavItem::new(
                            "knowledge",
                            tr(shell_cx, "Knowledge", "知识库"),
                            ProIcon::PageDoc,
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
                .with_active("workflows")
                .with_side_cards(false)
                .with_work_area(false, false);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s
        });
        cx.subscribe(&shell, |this, _, ev: &AppShellEvent, cx| {
            if let AppShellEvent::Navigate(label) = ev {
                if label.as_ref() == "Runs" || label.as_ref() == "运行记录" {
                    this.run_panel_open = true;
                    this.run_tab = WorkflowRunTab::Runs;
                }
                cx.emit(WorkflowBuilderEvent::Navigate(label.clone()));
                cx.notify();
            }
        })
        .detach();
        shell
    }

    fn sync_shell(&mut self, cx: &mut Context<Self>) {
        let count = (self.runs.len() as u32).max(3);
        self.shell = Self::build_shell(
            &self.id_name,
            self.workspace.clone(),
            self.user_name.clone(),
            self.user_email.clone(),
            count,
            cx,
        );
    }

    /// Construct a [`WorkflowBuilderScreen`] with the `"Customer onboarding"` graph and node palette.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let workspace: SharedString = "Northwind".into();
        let user_name: SharedString = "Alex Morgan".into();
        let user_email: SharedString = "alex@northwind.co".into();
        let shell = Self::build_shell(
            &id_str,
            workspace.clone(),
            user_name.clone(),
            user_email.clone(),
            3,
            cx,
        );
        let nodes = sample_nodes();
        let edges = sample_edges();
        let next_node_seq = nodes.len() + 1;

        Self {
            id: ElementId::Name(id_str.clone()),
            id_name: id_str,
            shell,
            standalone: false,
            workspace,
            project: "Customer onboarding".into(),
            branch: "Main".into(),
            user_name,
            user_email,
            published: false,
            palette_open: true,
            palette: sample_palette(),
            nodes,
            edges,
            comments: Vec::new(),
            open_comment_id: None,
            tool: WorkflowCanvasTool::Select,
            zoom: 0.94,
            pan_x: 360.,
            pan_y: 78.,
            centered_once: false,
            drag: None,
            selected_node_id: None,
            last_inspected_id: None,
            synced_node_id: None,
            selected_edge_id: None,
            insert_picker: None,
            last_picker: None,
            picker_editor: None,
            node_title_editor: None,
            node_expr_editor: None,
            comment_editor: None,
            top_menu: None,
            run_panel_open: false,
            run_tab: WorkflowRunTab::Runs,
            run_started_at: None,
            run_order: Vec::new(),
            runs: Vec::new(),
            step_test_banner: None,
            toast: None,
            next_node_seq,
            loading: false,
            empty: false,
            editor_focused: false,
            focus: cx.focus_handle(),
            boot_at: Instant::now(),
            measured_width: Rc::new(Cell::new(980.)),
            canvas_size: Rc::new(Cell::new((880., 560.))),
            canvas_origin: Rc::new(Cell::new((0., 0.))),
        }
    }

    /// Render without the outer [`AppShell`] sidebar frame.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Override the organisation/workspace shown in the path bar and sidebar.
    pub fn with_workspace(
        mut self,
        workspace: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        self.workspace = workspace.into();
        self.sync_shell(cx);
        self
    }

    /// Override the workflow project name in the path bar.
    pub fn with_project(mut self, project: impl Into<SharedString>) -> Self {
        self.project = project.into();
        self
    }

    /// Override the branch name (`"Main"` displays `"PRODUCTION"`).
    pub fn with_branch(mut self, branch: impl Into<SharedString>) -> Self {
        self.branch = branch.into();
        self
    }

    /// Override the signed-in user in the sidebar footer.
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

    /// Replace the initial workflow nodes.
    pub fn with_nodes(mut self, nodes: Vec<WorkflowNode>) -> Self {
        self.next_node_seq = nodes.len() + 1;
        self.nodes = nodes;
        self
    }

    /// Replace the initial workflow edges.
    pub fn with_edges(mut self, edges: Vec<WorkflowEdge>) -> Self {
        self.edges = edges;
        self
    }

    /// Configure the skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the empty `"Add a trigger"` canvas state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Toggle skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        if loading {
            self.run_started_at = None;
        }
        cx.notify();
    }

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        if empty {
            self.selected_node_id = None;
            self.run_started_at = None;
        }
        cx.notify();
    }

    /// Select a node by id and open its right-hand settings inspector.
    pub fn select_node(&mut self, node_id: Option<SharedString>, cx: &mut Context<Self>) {
        self.selected_node_id = node_id.clone();
        self.synced_node_id = None;
        self.selected_edge_id = None;
        self.insert_picker = None;
        self.step_test_banner = None;
        if let Some(id) = node_id {
            cx.emit(WorkflowBuilderEvent::SelectNode(id));
        }
        cx.notify();
    }

    /// Compute the execution trace order starting from the primary configured trigger.
    pub fn build_execution_plan(&self) -> Vec<(SharedString, Option<SharedString>, u64, u32)> {
        if self.empty {
            return Vec::new();
        }
        compute_execution_plan(&self.nodes, &self.edges)
    }

    /// Trigger a live animated test run across the workflow graph.
    pub fn start_test_run(&mut self, cx: &mut Context<Self>) {
        if self.loading || self.empty {
            return;
        }
        let plan = self.build_execution_plan();
        if plan.is_empty() {
            return;
        }
        self.run_order = plan;
        self.run_panel_open = true;
        self.run_tab = WorkflowRunTab::Runs;
        self.insert_picker = None;
        self.top_menu = None;

        if Theme::global(cx).reduced_motion {
            self.finalize_run(cx);
        } else {
            self.run_started_at = Some(Instant::now());
        }
        cx.emit(WorkflowBuilderEvent::RunStarted);
        cx.notify();
    }

    fn finalize_run(&mut self, cx: &mut Context<Self>) {
        self.run_started_at = None;
        if self.run_order.is_empty() {
            return;
        }
        let mut steps = Vec::with_capacity(self.run_order.len());
        let mut total_ms = 0u64;
        let mut total_credits = 0u32;
        for (nid, _, ms, cred) in &self.run_order {
            if let Some(node) = self.nodes.iter().find(|n| &n.id == nid) {
                total_ms += *ms;
                total_credits += *cred;
                steps.push(WorkflowStepRun {
                    node_id: node.id.clone(),
                    title: node.title.clone(),
                    kind: node.kind,
                    duration_ms: *ms,
                    credits: *cred,
                    error: None,
                });
            }
        }
        let step_count = steps.len();
        let run_num = 104 + self.runs.len();
        self.runs.insert(
            0,
            WorkflowRunRecord {
                id: format!("#{run_num}").into(),
                started_at: "Just now".into(),
                steps,
                total_ms,
                total_credits,
            },
        );
        cx.emit(WorkflowBuilderEvent::RunFinished {
            total_ms,
            steps: step_count,
        });
    }

    /// Add a new node from a palette template, optionally wiring it below `(after_node, port)`.
    pub fn add_node_from_template(
        &mut self,
        template: &WorkflowPaletteItem,
        after: Option<(SharedString, WorkflowPort)>,
        cx: &mut Context<Self>,
    ) -> WorkflowNode {
        let new_id: SharedString = format!("n{}", self.next_node_seq).into();
        self.next_node_seq += 1;

        let (x, y) = if let Some((ref parent_id, port)) = after
            && let Some(parent) = self.nodes.iter().find(|n| &n.id == parent_id)
        {
            let dx = match port {
                WorkflowPort::Out => 0.,
                WorkflowPort::Yes => -160.,
                WorkflowPort::No => 160.,
            };
            (parent.x + dx, parent.y + 150.)
        } else if let Some(ref sel_id) = self.selected_node_id
            && let Some(sel) = self.nodes.iter().find(|n| &n.id == sel_id)
        {
            (sel.x, sel.y + 150.)
        } else {
            let max_y = self.nodes.iter().map(|n| n.y).fold(-150.0_f32, f32::max);
            (0., (max_y + 150.).max(0.))
        };

        let chip: SharedString = match template.kind {
            WorkflowNodeKind::Agent => "1 tool".into(),
            WorkflowNodeKind::Tool => "GET".into(),
            WorkflowNodeKind::Logic if template.node_type.as_ref() == "branch" => "2 paths".into(),
            _ => "Ready".into(),
        };
        let app = match template.node_type.as_ref() {
            "notification" => Some("slack".into()),
            "send-email" => Some("gmail".into()),
            "document-upload" => Some("google drive".into()),
            _ => None,
        };
        let mut node = WorkflowNode::new(
            new_id.clone(),
            template.node_type.clone(),
            template.kind,
            template.label.clone(),
            x,
            y,
        );
        node.sub = template.sub.clone();
        node.chip = chip;
        node.app = app;
        node.expression = template.default_expr.clone();
        if template.kind == WorkflowNodeKind::Agent {
            node.tools = vec!["CRM lookup".into()];
        }

        self.empty = false;
        self.nodes.push(node.clone());

        if let Some((parent_id, port)) = after {
            let eid = format!("e{}", self.edges.len() + 1);
            self.edges
                .push(WorkflowEdge::new(eid, parent_id, port, new_id.clone()));
        } else if template.kind != WorkflowNodeKind::Trigger
            && let Some(sel_id) = self.selected_node_id.clone()
            && !self
                .edges
                .iter()
                .any(|e| e.from == sel_id && e.port == WorkflowPort::Out)
        {
            let eid = format!("e{}", self.edges.len() + 1);
            self.edges.push(WorkflowEdge::new(
                eid,
                sel_id,
                WorkflowPort::Out,
                new_id.clone(),
            ));
        }

        self.insert_picker = None;
        self.select_node(Some(new_id), cx);
        cx.emit(WorkflowBuilderEvent::NodeAdded(node.clone()));
        node
    }

    /// Delete `node_id` and any incident edges from the workflow graph.
    pub fn delete_node(&mut self, node_id: &str, cx: &mut Context<Self>) -> bool {
        let before = self.nodes.len();
        self.nodes.retain(|n| n.id.as_ref() != node_id);
        if self.nodes.len() == before {
            return false;
        }
        self.edges
            .retain(|e| e.from.as_ref() != node_id && e.to.as_ref() != node_id);
        if self.selected_node_id.as_deref() == Some(node_id) {
            self.selected_node_id = None;
            self.synced_node_id = None;
        }
        if self.last_inspected_id.as_deref() == Some(node_id) {
            self.last_inspected_id = None;
        }
        cx.emit(WorkflowBuilderEvent::NodeDeleted(node_id.to_owned().into()));
        cx.notify();
        true
    }

    /// Fit all current nodes inside the visible canvas area (never zooming past `100%`).
    pub fn fit_to_view(&mut self, cx: &mut Context<Self>) {
        if self.nodes.is_empty() {
            self.zoom = 1.0;
            self.pan_x = 320.;
            self.pan_y = 80.;
            cx.notify();
            return;
        }
        let (cw, ch) = self.canvas_size.get();
        let left_pad = if self.palette_open && cw > 640. {
            236.
        } else {
            24.
        };
        let right_pad = if self.selected_node_id.is_some() && cw > 760. {
            340.
        } else {
            24.
        };
        let top_pad = 64.;
        let bot_pad = 64.;
        let avail_w = (cw - left_pad - right_pad).max(240.);
        let avail_h = (ch - top_pad - bot_pad).max(220.);

        let min_x = self.nodes.iter().map(|n| n.x).fold(f32::INFINITY, f32::min);
        let max_x = self
            .nodes
            .iter()
            .map(|n| n.x + NODE_W)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = self
            .nodes
            .iter()
            .map(|n| n.y - 20.)
            .fold(f32::INFINITY, f32::min);
        let max_y = self
            .nodes
            .iter()
            .map(|n| n.y + NODE_H + 24.)
            .fold(f32::NEG_INFINITY, f32::max);

        let graph_w = (max_x - min_x).max(NODE_W);
        let graph_h = (max_y - min_y).max(NODE_H);
        let zx = avail_w / graph_w;
        let zy = avail_h / graph_h;
        let next_z = zx.min(zy).clamp(0.55, 1.0);
        self.zoom = (next_z * 100.).round() / 100.;
        self.pan_x = left_pad + (avail_w - graph_w * self.zoom) * 0.5 - min_x * self.zoom;
        self.pan_y = top_pad + (avail_h - graph_h * self.zoom) * 0.5 - min_y * self.zoom;
        cx.notify();
    }

    fn unconfigured_count(&self) -> usize {
        if self.empty {
            0
        } else {
            self.nodes.iter().filter(|n| !n.configured).count()
        }
    }

    fn has_outgoing_port(&self, node_id: &str, port: WorkflowPort) -> bool {
        self.edges
            .iter()
            .any(|e| e.from.as_ref() == node_id && e.port == port)
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.picker_editor.is_none() {
            let ph: SharedString = tr(cx, "Search nodes…", "搜索节点类型…").into();
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
                    this.insert_picker = None;
                    cx.notify();
                }
                EditEvent::Changed => cx.notify(),
                _ => {}
            })
            .detach();
            self.picker_editor = Some(ed);
        }

        if self.node_title_editor.is_none() {
            let ph: SharedString = tr(cx, "Step name", "步骤名称").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |this, ed_ent, ev: &EditEvent, cx| match ev {
                EditEvent::Focused => this.editor_focused = true,
                EditEvent::Blurred => this.editor_focused = false,
                EditEvent::Changed | EditEvent::Submitted => {
                    let val = ed_ent.read(cx).value().trim().to_owned();
                    if !val.is_empty()
                        && let Some(ref sid) = this.selected_node_id
                        && let Some(node) = this.nodes.iter_mut().find(|n| &n.id == sid)
                        && node.title.as_ref() != val
                    {
                        node.title = val.into();
                        let updated = node.clone();
                        cx.emit(WorkflowBuilderEvent::NodeUpdated(updated));
                    }
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.node_title_editor = Some(ed);
        }

        if self.node_expr_editor.is_none() {
            let ph: SharedString = tr(
                cx,
                "Prompt, URL, condition or field mapping…",
                "输入提示词、接口地址、条件或字段映射…",
            )
            .into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Multi, w, cx);
                e.set_placeholder(ph);
                e.font_size = 12.5;
                e
            });
            cx.subscribe(&ed, |this, ed_ent, ev: &EditEvent, cx| match ev {
                EditEvent::Focused => this.editor_focused = true,
                EditEvent::Blurred => this.editor_focused = false,
                EditEvent::Changed => {
                    let val = ed_ent.read(cx).value().to_owned();
                    if let Some(ref sid) = this.selected_node_id
                        && let Some(node) = this.nodes.iter_mut().find(|n| &n.id == sid)
                        && node.expression.as_ref() != val
                    {
                        node.expression = val.into();
                        let updated = node.clone();
                        cx.emit(WorkflowBuilderEvent::NodeUpdated(updated));
                    }
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.node_expr_editor = Some(ed);
        }

        if self.comment_editor.is_none() {
            let ph: SharedString = tr(cx, "Add a canvas note…", "添加画布批注…").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Multi, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |this, ed_ent, ev: &EditEvent, cx| match ev {
                EditEvent::Focused => this.editor_focused = true,
                EditEvent::Blurred => this.editor_focused = false,
                EditEvent::Changed => {
                    let val = ed_ent.read(cx).value().to_owned();
                    if let Some(ref cid) = this.open_comment_id
                        && let Some(cmt) = this.comments.iter_mut().find(|c| &c.id == cid)
                    {
                        cmt.text = val.into();
                    }
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.comment_editor = Some(ed);
        }

        if self.selected_node_id != self.synced_node_id {
            self.synced_node_id = self.selected_node_id.clone();
            if let Some(ref sid) = self.selected_node_id
                && let Some(node) = self.nodes.iter().find(|n| &n.id == sid).cloned()
            {
                if let Some(ed) = self.node_title_editor.clone() {
                    ed.update(cx, |e, cx| e.set_value(node.title.to_string(), w, cx));
                }
                if let Some(ed) = self.node_expr_editor.clone() {
                    ed.update(cx, |e, cx| e.set_value(node.expression.to_string(), w, cx));
                }
            }
        }
    }

    fn handle_key(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let m = ev.keystroke.modifiers;
        let cmd = m.platform || m.control;
        let k = ev.keystroke.key.as_str();

        if cmd && k == "enter" {
            self.start_test_run(cx);
            return;
        }

        if k == "escape" {
            if self.top_menu.take().is_some() {
                cx.notify();
                return;
            }
            if self.insert_picker.take().is_some() {
                cx.notify();
                return;
            }
            if self.open_comment_id.take().is_some() {
                cx.notify();
                return;
            }
            if self.selected_node_id.take().is_some() {
                self.synced_node_id = None;
                cx.notify();
                return;
            }
            if self.run_panel_open {
                self.run_panel_open = false;
                cx.notify();
                return;
            }
            if self.tool == WorkflowCanvasTool::Comment {
                self.tool = WorkflowCanvasTool::Select;
                cx.notify();
                return;
            }
            return;
        }

        if self.editor_focused || cmd {
            return;
        }

        match k {
            "v" => {
                self.tool = WorkflowCanvasTool::Select;
                cx.notify();
            }
            "c" => {
                self.tool = WorkflowCanvasTool::Comment;
                cx.notify();
            }
            "r" => self.start_test_run(cx),
            "+" | "=" => {
                self.zoom = ((self.zoom + 0.1).clamp(0.45, 1.4) * 100.).round() / 100.;
                cx.notify();
            }
            "-" => {
                self.zoom = ((self.zoom - 0.1).clamp(0.45, 1.4) * 100.).round() / 100.;
                cx.notify();
            }
            "1" if m.shift => self.fit_to_view(cx),
            "a" => {
                if let Some(ref sid) = self.selected_node_id {
                    let port = if self
                        .nodes
                        .iter()
                        .any(|n| &n.id == sid && n.node_type.as_ref() == "branch")
                    {
                        if !self.has_outgoing_port(sid, WorkflowPort::Yes) {
                            WorkflowPort::Yes
                        } else {
                            WorkflowPort::No
                        }
                    } else {
                        WorkflowPort::Out
                    };
                    self.insert_picker = Some(InsertPickerState {
                        after_node_id: sid.clone(),
                        port,
                    });
                    if let Some(ed) = self.picker_editor.clone() {
                        ed.update(cx, |e, cx| {
                            e.set_value("", w, cx);
                            e.focus(w, cx);
                        });
                    }
                    cx.notify();
                }
            }
            "delete" | "backspace" => {
                if let Some(sid) = self.selected_node_id.clone() {
                    self.delete_node(&sid, cx);
                } else if let Some(eid) = self.selected_edge_id.take() {
                    self.edges.retain(|e| e.id != eid);
                    cx.notify();
                }
            }
            "left" | "right" | "up" | "down" if m.alt => {
                if let Some(ref sid) = self.selected_node_id
                    && let Some(node) = self.nodes.iter_mut().find(|n| &n.id == sid)
                {
                    let step = if m.shift { 50. } else { 10. };
                    match k {
                        "left" => node.x -= step,
                        "right" => node.x += step,
                        "up" => node.y -= step,
                        "down" => node.y += step,
                        _ => {}
                    }
                    cx.notify();
                }
            }
            "down" => self.navigate_graph(true, cx),
            "up" => self.navigate_graph(false, cx),
            _ => {}
        }
    }

    fn navigate_graph(&mut self, forward: bool, cx: &mut Context<Self>) {
        if self.nodes.is_empty() {
            return;
        }
        let Some(ref cur) = self.selected_node_id else {
            let first = self.nodes.first().map(|n| n.id.clone());
            self.select_node(first, cx);
            return;
        };
        let next = if forward {
            self.edges
                .iter()
                .find(|e| &e.from == cur)
                .map(|e| e.to.clone())
        } else {
            self.edges
                .iter()
                .find(|e| &e.to == cur)
                .map(|e| e.from.clone())
        };
        if let Some(nid) = next {
            self.select_node(Some(nid), cx);
        }
    }

    fn render_palette_glyph(node_type: &str, side: f32, color: Hsla) -> AnyElement {
        match node_type {
            "new-signup" => pro_svg("0 0 24 24", SVG_USER_PLUS, side, color),
            "document-upload" => pro_svg("0 0 24 24", SVG_CLOUD_UPLOAD, side, color),
            "schedule" => pro_icon(ProIcon::Calendar, side, color),
            "webhook" => pro_svg("0 0 24 24", SVG_WEBHOOK, side, color),
            "ai-agent" => pro_svg("0 0 24 24", SVG_BRAIN, side, color),
            "http-request" => pro_icon(ProIcon::Globe, side, color),
            "calculation" => pro_svg("0 0 24 24", SVG_CALC, side, color),
            "file-parser" => pro_icon(ProIcon::PageDoc, side, color),
            "validation" => pro_icon(ProIcon::Shield, side, color),
            "branch" => pro_svg("0 0 24 24", SVG_SPLIT_BRANCH, side, color),
            "merge" => pro_icon(ProIcon::GitBranch, side, color),
            "wait" => pro_svg("0 0 24 24", SVG_HOURGLASS, side, color),
            "loop" => pro_svg("0 0 24 24", SVG_LOOP, side, color),
            "database" => pro_icon(ProIcon::Database, side, color),
            "notification" => pro_icon(ProIcon::Bell, side, color),
            "send-email" => pro_icon(ProIcon::Mail, side, color),
            "webhook-call" => pro_icon(ProIcon::Link, side, color),
            "end" => pro_svg("0 0 24 24", SVG_STOP_END, side, color),
            _ => pro_icon(ProIcon::Bolt, side, color),
        }
    }

    fn render_node_tile(
        node: &WorkflowNode,
        side: f32,
        is_running: bool,
        elapsed_ms: f32,
        p: &ProPalette,
        cx: &App,
    ) -> AnyElement {
        if !node.configured {
            return div()
                .size(px(side))
                .flex_none()
                .rounded(px(7.))
                .bg(p.red_soft)
                .flex()
                .items_center()
                .justify_center()
                .child(pro_icon(
                    ProIcon::AlertTriangle,
                    (side * 0.58).round(),
                    p.red,
                ))
                .into_any_element();
        }
        if node.kind == WorkflowNodeKind::Agent {
            return pro_agent_avatar(&node.title, p.purple, side, is_running, elapsed_ms, cx);
        }
        if let Some(ref app) = node.app {
            return pro_app_tile(app, side, cx);
        }
        div()
            .size(px(side))
            .flex_none()
            .rounded(px(7.))
            .bg(p.chip)
            .flex()
            .items_center()
            .justify_center()
            .child(Self::render_palette_glyph(
                &node.node_type,
                (side * 0.58).round(),
                p.text,
            ))
            .into_any_element()
    }

    fn render_top_overlay(&self, compact: bool, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let org_letter: String = self
            .workspace
            .chars()
            .next()
            .unwrap_or('N')
            .to_uppercase()
            .collect();
        let env_badge = if self.branch.as_ref() == "Main" {
            tr(cx, "PRODUCTION", "生产")
        } else {
            tr(cx, "STAGING", "预发")
        };

        let org_open = self.top_menu == Some(TopMenu::Org);
        let proj_open = self.top_menu == Some(TopMenu::Project);
        let branch_open = self.top_menu == Some(TopMenu::Branch);

        let mut path_nav = div()
            .h(px(36.))
            .px(px(4.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(2.))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation());

        if !compact {
            path_nav = path_nav
                .child(
                    div()
                        .relative()
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, "crumb-org"))
                                .role(Role::Button)
                                .aria_expanded(org_open)
                                .tab_index(0)
                                .h(px(28.))
                                .pl(px(6.))
                                .pr(px(6.))
                                .rounded(px(6.))
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover))
                                .child(
                                    div()
                                        .size(px(18.))
                                        .rounded(px(5.))
                                        .bg(p.ink)
                                        .text_color(p.panel)
                                        .text_size(px(10.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(org_letter),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(self.workspace.clone()),
                                )
                                .child(
                                    div()
                                        .px(px(5.))
                                        .rounded(px(4.))
                                        .bg(p.accent_soft)
                                        .text_size(px(10.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.blue)
                                        .child("TEAM"),
                                )
                                .child(pro_icon(ProIcon::ChevronDown, 12., p.faint)),
                            cx,
                            |this, _, cx| {
                                this.top_menu = if this.top_menu == Some(TopMenu::Org) {
                                    None
                                } else {
                                    Some(TopMenu::Org)
                                };
                                cx.notify();
                            },
                        ))
                        .child({
                            let mut m = div()
                                .occlude()
                                .absolute()
                                .left_0()
                                .w(px(196.))
                                .p(px(4.))
                                .rounded(px(10.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line_strong)
                                .shadow_md()
                                .flex()
                                .flex_col();
                            for ws in ["Northwind", "Acme", "Fieldwork"] {
                                let ws_s: SharedString = ws.into();
                                let chk = self.workspace.as_ref() == ws;
                                m = m.child(activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("org-{ws}")))
                                        .role(Role::MenuItem)
                                        .tab_index(0)
                                        .h(px(30.))
                                        .px(px(10.))
                                        .rounded(px(7.))
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .text_size(px(13.))
                                        .text_color(if chk { p.ink } else { p.text })
                                        .cursor_pointer()
                                        .hover(move |s| s.bg(p.hover))
                                        .child(ws)
                                        .when(chk, |x| {
                                            x.child(pro_svg("0 0 24 24", SVG_CHECK_SM, 13., p.ink))
                                        }),
                                    cx,
                                    move |this, _, cx| {
                                        this.workspace = ws_s.clone();
                                        this.top_menu = None;
                                        this.sync_shell(cx);
                                        cx.notify();
                                    },
                                ));
                            }
                            pro_presence(
                                pro_eid(&self.id, "wf-org-pres"),
                                org_open,
                                0.18,
                                0.12,
                                move |a| {
                                    m.top(px(34. - 4. * (1. - a))).opacity(a).into_any_element()
                                },
                            )
                        }),
                )
                .child(
                    div()
                        .px(px(2.))
                        .text_size(px(13.))
                        .text_color(p.faint)
                        .child("/"),
                );
        }

        // Project crumb
        path_nav = path_nav.child(
            div()
                .relative()
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "crumb-proj"))
                        .role(Role::Button)
                        .aria_expanded(proj_open)
                        .tab_index(0)
                        .h(px(28.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.hover))
                        .child(div().max_w(px(180.)).truncate().child(self.project.clone()))
                        .child(pro_icon(ProIcon::ChevronDown, 12., p.faint)),
                    cx,
                    |this, _, cx| {
                        this.top_menu = if this.top_menu == Some(TopMenu::Project) {
                            None
                        } else {
                            Some(TopMenu::Project)
                        };
                        cx.notify();
                    },
                ))
                .child({
                    let mut m = div()
                        .occlude()
                        .absolute()
                        .left_0()
                        .w(px(224.))
                        .p(px(4.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line_strong)
                        .shadow_md()
                        .flex()
                        .flex_col();
                    for prj in WORKFLOW_PROJECTS {
                        let ps: SharedString = prj.into();
                        let chk = self.project.as_ref() == prj;
                        m = m.child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("wfproj-{prj}")))
                                .role(Role::MenuItem)
                                .tab_index(0)
                                .h(px(30.))
                                .px(px(10.))
                                .rounded(px(7.))
                                .flex()
                                .items_center()
                                .justify_between()
                                .text_size(px(13.))
                                .text_color(if chk { p.ink } else { p.text })
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover))
                                .child(prj)
                                .when(chk, |x| {
                                    x.child(pro_svg("0 0 24 24", SVG_CHECK_SM, 13., p.ink))
                                }),
                            cx,
                            move |this, _, cx| {
                                this.project = ps.clone();
                                this.top_menu = None;
                                cx.emit(WorkflowBuilderEvent::ProjectChanged(ps.clone()));
                                cx.notify();
                            },
                        ));
                    }
                    pro_presence(
                        pro_eid(&self.id, "wf-proj-pres"),
                        proj_open,
                        0.18,
                        0.12,
                        move |a| m.top(px(34. - 4. * (1. - a))).opacity(a).into_any_element(),
                    )
                }),
        );

        // Branch crumb
        path_nav = path_nav
            .child(
                div()
                    .px(px(2.))
                    .text_size(px(13.))
                    .text_color(p.faint)
                    .child("/"),
            )
            .child(
                div()
                    .relative()
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "crumb-branch"))
                            .role(Role::Button)
                            .aria_expanded(branch_open)
                            .tab_index(0)
                            .h(px(28.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .cursor_pointer()
                            .hover(move |s| s.bg(p.hover))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(self.branch.clone()),
                            )
                            .when(!compact, |d| {
                                d.child(
                                    div()
                                        .px(px(5.))
                                        .rounded(px(4.))
                                        .bg(p.chip)
                                        .text_size(px(10.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.muted)
                                        .child(env_badge),
                                )
                            })
                            .child(pro_icon(ProIcon::ChevronDown, 12., p.faint)),
                        cx,
                        |this, _, cx| {
                            this.top_menu = if this.top_menu == Some(TopMenu::Branch) {
                                None
                            } else {
                                Some(TopMenu::Branch)
                            };
                            cx.notify();
                        },
                    ))
                    .child({
                        let mut m = div()
                            .occlude()
                            .absolute()
                            .left_0()
                            .w(px(196.))
                            .p(px(4.))
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line_strong)
                            .shadow_md()
                            .flex()
                            .flex_col();
                        for (br, env) in WORKFLOW_BRANCHES {
                            let bs: SharedString = br.into();
                            let chk = self.branch.as_ref() == br;
                            m = m.child(activate(
                                div()
                                    .id(pro_eid(&self.id, format!("wfbr-{br}")))
                                    .role(Role::MenuItem)
                                    .tab_index(0)
                                    .h(px(30.))
                                    .px(px(10.))
                                    .rounded(px(7.))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .text_size(px(13.))
                                    .text_color(if chk { p.ink } else { p.text })
                                    .cursor_pointer()
                                    .hover(move |s| s.bg(p.hover))
                                    .child(div().flex().items_center().gap(px(6.)).child(br).child(
                                        div().text_size(px(10.5)).text_color(p.faint).child(env),
                                    ))
                                    .when(chk, |x| {
                                        x.child(pro_svg("0 0 24 24", SVG_CHECK_SM, 13., p.ink))
                                    }),
                                cx,
                                move |this, _, cx| {
                                    this.branch = bs.clone();
                                    this.top_menu = None;
                                    cx.emit(WorkflowBuilderEvent::BranchChanged(bs.clone()));
                                    cx.notify();
                                },
                            ));
                        }
                        pro_presence(
                            pro_eid(&self.id, "wf-branch-pres"),
                            branch_open,
                            0.18,
                            0.12,
                            move |a| m.top(px(34. - 4. * (1. - a))).opacity(a).into_any_element(),
                        )
                    }),
            );

        // Publish primary button
        let is_pub = self.published;
        let pub_btn = activate(
            div()
                .id(pro_eid(&self.id, "publish-btn"))
                .role(Role::Button)
                .aria_label(tr(cx, "Publish", "发布"))
                .tab_index(0)
                .h(px(36.))
                .px(px(12.))
                .rounded(px(10.))
                .bg(if is_pub { p.green_soft } else { p.primary })
                .text_color(if is_pub { p.green_ink } else { p.on_primary })
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .flex_none()
                .flex()
                .items_center()
                .gap(px(6.))
                .cursor_pointer()
                .hover(|s| s.opacity(0.90))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(if is_pub {
                    pro_svg("0 0 24 24", SVG_CHECK_SM, 14., p.green_ink)
                } else {
                    pro_svg("0 0 24 24", SVG_ROCKET, 14., p.on_primary)
                })
                .when(!compact, |d| {
                    d.child(if is_pub {
                        tr(cx, "Published", "已发布")
                    } else {
                        tr(cx, "Publish", "发布")
                    })
                }),
            cx,
            |this, _, cx| {
                this.published = !this.published;
                let msg: SharedString = if this.published {
                    tr(
                        cx,
                        "Workflow published to Production",
                        "工作流已发布至生产环境",
                    )
                    .into()
                } else {
                    tr(cx, "Switched back to draft", "已切回草稿模式").into()
                };
                this.toast = Some((msg, Instant::now()));
                cx.emit(WorkflowBuilderEvent::Published(this.published));
                cx.notify();
            },
        );

        div()
            .absolute()
            .top(px(8.))
            .left(px(if compact { 48. } else { 10. }))
            .flex()
            .items_center()
            .gap(px(8.))
            .child(path_nav)
            .child(pub_btn)
    }

    fn render_palette_panel(&self, compact: bool, cx: &mut Context<Self>) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let mut groups_el = div().flex().flex_col();

        for (gi, grp) in self.palette.iter().enumerate() {
            let mut sec = div()
                .when(gi > 0, |d| {
                    d.mt(px(6.))
                        .pt(px(6.))
                        .border_t_1()
                        .border_color(p.line_soft)
                })
                .flex()
                .flex_col()
                .child(
                    div()
                        .h(px(26.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(5.))
                        .text_size(px(10.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(grp.label.to_uppercase())
                        .child(
                            div()
                                .text_color(p.faint)
                                .child(format!("{}", grp.items.len())),
                        ),
                );

            for item in &grp.items {
                let tpl = item.clone();
                sec = sec.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("pal-{}", item.node_type)))
                        .role(Role::Button)
                        .tab_index(0)
                        .h(px(30.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.hover))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .child(
                            div()
                                .w(px(18.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(Self::render_palette_glyph(&item.node_type, 15., p.muted)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .child(item.label.clone()),
                        ),
                    cx,
                    move |this, _, cx| {
                        this.add_node_from_template(&tpl, None, cx);
                    },
                ));
            }
            groups_el = groups_el.child(sec);
        }

        div()
            .id(pro_eid(&self.id, "nodes-palette"))
            .occlude()
            .absolute()
            .when(!compact, |d| {
                d.top(px(54.)).bottom(px(10.)).left(px(10.)).w(px(216.))
            })
            .when(compact, |d| {
                d.left(px(8.)).right(px(8.)).bottom(px(54.)).max_h(px(340.))
            })
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .h(px(40.))
                    .pl(px(14.))
                    .pr(px(6.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(tr(cx, "Nodes", "节点库")),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "pal-close"),
                        tr(cx, "Close nodes", "收起节点库"),
                        ProIcon::Close,
                        28.,
                        false,
                        cx,
                        |this, _, cx| {
                            this.palette_open = false;
                            cx.notify();
                        },
                    )),
            )
            .child(
                div()
                    .id(pro_eid(&self.id, "pal-scroll"))
                    .flex_1()
                    .min_h_0()
                    .px(px(6.))
                    .pb(px(6.))
                    .overflow_y_scroll()
                    .child(groups_el),
            )
    }

    fn render_insert_picker(&self, picker: &InsertPickerState, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let parent_pos = self
            .nodes
            .iter()
            .find(|n| n.id == picker.after_node_id)
            .map(|n| {
                (
                    self.pan_x + (n.x + picker.port.offset_x()) * self.zoom - 120.,
                    self.pan_y + (n.y + NODE_H + 28.) * self.zoom,
                )
            })
            .unwrap_or((340., 220.));

        let q = self
            .picker_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_ascii_lowercase())
            .unwrap_or_default();

        let mut list = div()
            .id(pro_eid(&self.id, "picker-list"))
            .flex_1()
            .min_h_0()
            .max_h(px(260.))
            .p(px(5.))
            .flex()
            .flex_col()
            .gap(px(1.))
            .overflow_y_scroll();

        for grp in &self.palette {
            for item in &grp.items {
                if item.kind == WorkflowNodeKind::Trigger {
                    continue;
                }
                if !q.is_empty() && !item.label.to_ascii_lowercase().contains(&q) {
                    continue;
                }
                let tpl = item.clone();
                let after = (picker.after_node_id.clone(), picker.port);
                list = list.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("pick-{}", item.node_type)))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(30.))
                        .px(px(8.))
                        .rounded(px(7.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(13.))
                        .text_color(p.ink)
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.hover))
                        .child(Self::render_palette_glyph(&item.node_type, 14., p.muted))
                        .child(item.label.clone()),
                    cx,
                    move |this, _, cx| {
                        this.add_node_from_template(&tpl, Some(after.clone()), cx);
                    },
                ));
            }
        }

        div()
            .occlude()
            .absolute()
            .left(px(parent_pos.0.max(12.)))
            .top(px(parent_pos.1.max(52.)))
            .w(px(240.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .when_some(self.picker_editor.clone(), |d, ed| {
                d.child(
                    div()
                        .h(px(38.))
                        .px(px(10.))
                        .border_b_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_icon(ProIcon::Search, 14., p.faint))
                        .child(div().flex_1().min_w_0().child(ed)),
                )
            })
            .child(list)
    }

    fn render_bottom_toolbar(&self, compact: bool, w: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let zoom_pct = (self.zoom * 100.).round() as i32;
        let is_running = self.run_started_at.is_some();

        let pal_btn = activate(
            div()
                .id(pro_eid(&self.id, "tb-add"))
                .role(Role::Button)
                .aria_label(tr(cx, "Add node", "添加节点"))
                .tab_index(0)
                .size(px(28.))
                .rounded(px(6.))
                .bg(if self.palette_open {
                    p.accent_soft
                } else {
                    transparent_black()
                })
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(move |s| if !self.palette_open { s.bg(p.hover) } else { s })
                .child(pro_icon(
                    ProIcon::Plus,
                    15.,
                    if self.palette_open { p.blue } else { p.muted },
                )),
            cx,
            |this, _, cx| {
                this.palette_open = !this.palette_open;
                cx.notify();
            },
        );

        let sel_active = self.tool == WorkflowCanvasTool::Select;
        let cmt_active = self.tool == WorkflowCanvasTool::Comment;
        let tool_sensor = pro_selection_sensor();

        let sel_btn = activate(
            div()
                .id(pro_eid(&self.id, "tb-select"))
                .role(Role::Button)
                .aria_label(tr(cx, "Select (V)", "选择工具 (V)"))
                .tab_index(0)
                .relative()
                .size(px(28.))
                .rounded(px(6.))
                .when(sel_active, |d| d.child(pro_mark_selected(&tool_sensor)))
                .when(!sel_active, |d| d.hover(move |s| s.bg(p.hover)))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .child(pro_svg(
                    "0 0 24 24",
                    SVG_CURSOR_SELECT,
                    15.,
                    if sel_active { p.ink } else { p.muted },
                )),
            cx,
            |this, _, cx| {
                this.tool = WorkflowCanvasTool::Select;
                cx.notify();
            },
        );

        let cmt_btn = activate(
            div()
                .id(pro_eid(&self.id, "tb-comment"))
                .role(Role::Button)
                .aria_label(tr(cx, "Comment (C)", "批注工具 (C)"))
                .tab_index(0)
                .relative()
                .size(px(28.))
                .rounded(px(6.))
                .when(cmt_active, |d| d.child(pro_mark_selected(&tool_sensor)))
                .when(!cmt_active, |d| d.hover(move |s| s.bg(p.hover)))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .child(pro_svg(
                    "0 0 24 24",
                    SVG_COMMENT_TOOL,
                    15.,
                    if cmt_active { p.ink } else { p.muted },
                )),
            cx,
            |this, _, cx| {
                this.tool = WorkflowCanvasTool::Comment;
                cx.notify();
            },
        );
        let tool_group = div()
            .relative()
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "wf-tool-pill"),
                if sel_active { "select" } else { "comment" },
                tool_sensor,
                6.,
                p.chip,
                None,
                false,
            ))
            .child(sel_btn)
            .when(!compact, |d| d.child(cmt_btn));

        let fit_btn = activate(
            div()
                .id(pro_eid(&self.id, "tb-fit"))
                .role(Role::Button)
                .aria_label(tr(cx, "Fit to view", "适应画布"))
                .tab_index(0)
                .size(px(28.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover))
                .child(pro_svg("0 0 24 24", SVG_FIT_VIEW, 15., p.muted)),
            cx,
            |this, _, cx| this.fit_to_view(cx),
        );

        let zoom_out_btn = pro_icon_btn(
            pro_eid(&self.id, "tb-zoom-out"),
            tr(cx, "Zoom out", "缩小"),
            ProIcon::Minus,
            28.,
            false,
            cx,
            |this, _, cx| {
                this.zoom = ((this.zoom - 0.1).clamp(0.45, 1.4) * 100.).round() / 100.;
                cx.notify();
            },
        );

        let zoom_reset_btn = activate(
            div()
                .id(pro_eid(&self.id, "tb-zoom-pct"))
                .role(Role::Button)
                .aria_label(format!("Zoom {zoom_pct}%, reset to 100%"))
                .tab_index(0)
                .w(px(44.))
                .h(px(28.))
                .rounded(px(6.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(12.))
                .text_color(p.muted)
                .cursor_pointer()
                .hover(move |s| s.bg(p.hover).text_color(p.ink))
                .child(format!("{zoom_pct}%")),
            cx,
            |this, _, cx| {
                this.zoom = 1.0;
                cx.notify();
            },
        );

        let zoom_in_btn = pro_icon_btn(
            pro_eid(&self.id, "tb-zoom-in"),
            tr(cx, "Zoom in", "放大"),
            ProIcon::Plus,
            28.,
            false,
            cx,
            |this, _, cx| {
                this.zoom = ((this.zoom + 0.1).clamp(0.45, 1.4) * 100.).round() / 100.;
                cx.notify();
            },
        );

        let run_disabled = self.loading || self.empty;
        let run_btn = activate(
            div()
                .id(pro_eid(&self.id, "tb-run"))
                .role(Role::Button)
                .aria_label(tr(cx, "Run workflow", "运行测试工作流"))
                .tab_index(0)
                .size(px(28.))
                .rounded(px(6.))
                .bg(p.blue)
                .when(run_disabled, |d| d.opacity(0.45))
                .flex()
                .items_center()
                .justify_center()
                .when(run_disabled, |d| d.cursor_default())
                .when(!run_disabled, |d| d.cursor_pointer())
                .hover(move |s| if run_disabled { s } else { s.opacity(0.90) })
                .child(pro_icon(
                    if is_running {
                        ProIcon::Refresh
                    } else {
                        ProIcon::Play
                    },
                    13.5,
                    rgb(0xffffff).into(),
                )),
            cx,
            move |this, _, cx| {
                if !run_disabled {
                    this.start_test_run(cx);
                }
            },
        );

        let shift_x = if !compact {
            match (self.palette_open, self.selected_node_id.is_some()) {
                (true, false) => 108.,
                (false, true) => -160.,
                (true, true) => -52.,
                (false, false) => 0.,
            }
        } else {
            0.
        };
        let (cw, _) = self.canvas_size.get();
        let tb_w = if compact { 210. } else { 278. };
        let target_tb_left =
            ((cw * 0.5 + shift_x) - tb_w * 0.5).clamp(12., (cw - tb_w - 12.).max(12.));
        let tb_left = pro_spring_f32(pro_eid(&self.id, "tb-left-sp"), target_tb_left, 0.24, w, cx);

        div()
            .occlude()
            .absolute()
            .bottom(px(14.))
            .left(px(tb_left))
            .p(px(4.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(2.))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(pal_btn)
            .child(div().w(px(1.)).h(px(16.)).mx(px(3.)).bg(p.line_strong))
            .child(tool_group)
            .child(div().w(px(1.)).h(px(16.)).mx(px(3.)).bg(p.line_strong))
            .child(fit_btn)
            .child(zoom_out_btn)
            .when(!compact, |d| d.child(zoom_reset_btn))
            .child(zoom_in_btn)
            .child(div().w(px(1.)).h(px(16.)).mx(px(3.)).bg(p.line_strong))
            .child(run_btn)
    }

    fn render_node_inspector(
        &self,
        node_id: &SharedString,
        elapsed_ms: f32,
        compact: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let Some(node) = self.nodes.iter().find(|n| &n.id == node_id).cloned() else {
            return div().id(pro_eid(&self.id, "inspector-empty"));
        };
        let nid = node.id.clone();
        let nid_del = node.id.clone();
        let nid_del_foot = node.id.clone();
        let nid_toggle = node.id.clone();
        let nid_test = node.id.clone();
        let acc_open = self.top_menu == Some(TopMenu::InspectorAccount);
        let mod_open = self.top_menu == Some(TopMenu::InspectorModel);

        let kind_sub = format!(
            "{} · {}",
            node.kind.badge_label(cx),
            if !node.configured {
                tr(cx, "Not configured", "未配置")
            } else if !node.enabled {
                tr(cx, "Disabled", "已禁用")
            } else {
                tr(cx, "Active", "已就绪")
            }
        );

        let mut body = div().p(px(14.)).flex().flex_col().gap(px(14.));

        // Unconfigured warning banner with 1-click source configuration
        if !node.configured {
            let nid_cfg = node.id.clone();
            body = body.child(
                div()
                    .p(px(10.))
                    .rounded(px(8.))
                    .bg(p.red_soft)
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap(px(8.))
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.red_ink)
                            .child(pro_icon(ProIcon::AlertTriangle, 14., p.red))
                            .child(tr(
                                cx,
                                "Choose where document events come from before publishing.",
                                "发布前请选择文档上传事件的触发来源。",
                            )),
                    )
                    .child(
                        div().flex().flex_wrap().gap(px(6.)).children(
                            ["Google Drive", "S3 Uploads", "Portal Form"]
                                .into_iter()
                                .map(|src| {
                                    let nid_s = nid_cfg.clone();
                                    let src_s: SharedString = src.into();
                                    activate(
                                        div()
                                            .id(pro_eid(&self.id, format!("cfg-src-{src}")))
                                            .role(Role::Button)
                                            .tab_index(0)
                                            .h(px(24.))
                                            .px(px(8.))
                                            .rounded(px(6.))
                                            .bg(p.card)
                                            .border_1()
                                            .border_color(p.line_strong)
                                            .text_size(px(11.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.ink)
                                            .flex()
                                            .items_center()
                                            .cursor_pointer()
                                            .hover(move |s| s.bg(p.hover))
                                            .child(src),
                                        cx,
                                        move |this, _, cx| {
                                            if let Some(n) =
                                                this.nodes.iter_mut().find(|n| n.id == nid_s)
                                            {
                                                n.configured = true;
                                                n.chip = "Ready".into();
                                                n.sub = format!("From {src_s}").into();
                                                n.app = Some("google drive".into());
                                                let updated = n.clone();
                                                cx.emit(WorkflowBuilderEvent::NodeUpdated(updated));
                                            }
                                            cx.notify();
                                        },
                                    )
                                }),
                        ),
                    ),
            );
        }

        // Step name editor
        if let Some(ref title_ed) = self.node_title_editor {
            body = body.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.text)
                            .child(tr(cx, "Step name", "步骤名称")),
                    )
                    .child(
                        div()
                            .h(px(32.))
                            .px(px(10.))
                            .rounded(px(8.))
                            .bg(p.subtle)
                            .border_1()
                            .border_color(p.line_strong)
                            .flex()
                            .items_center()
                            .child(title_ed.clone()),
                    ),
            );
        }

        // Connected App Account selector
        let cur_app = node.app.as_deref().unwrap_or("slack");
        body = body.child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.text)
                        .child(tr(cx, "Connected account", "关联应用账号")),
                )
                .child(
                    div()
                        .relative()
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, "ins-account"))
                                .role(Role::Button)
                                .aria_expanded(acc_open)
                                .tab_index(0)
                                .h(px(34.))
                                .px(px(8.))
                                .rounded(px(8.))
                                .bg(p.subtle)
                                .border_1()
                                .border_color(p.line_strong)
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(8.))
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover))
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(pro_app_tile(cur_app, 20., cx))
                                        .child(
                                            div()
                                                .truncate()
                                                .text_size(px(12.5))
                                                .text_color(p.ink)
                                                .child(node.account.clone()),
                                        ),
                                )
                                .child(pro_icon(ProIcon::ChevronDown, 13., p.muted)),
                            cx,
                            |this, _, cx| {
                                this.top_menu = if this.top_menu == Some(TopMenu::InspectorAccount)
                                {
                                    None
                                } else {
                                    Some(TopMenu::InspectorAccount)
                                };
                                cx.notify();
                            },
                        ))
                        .child({
                            let mut m = div()
                                .occlude()
                                .absolute()
                                .left_0()
                                .right_0()
                                .p(px(4.))
                                .rounded(px(10.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line_strong)
                                .shadow_md()
                                .flex()
                                .flex_col();
                            for (app_slug, acc_label) in APP_ACCOUNTS {
                                let nid_a = nid.clone();
                                let app_s: SharedString = app_slug.into();
                                let acc_s: SharedString = acc_label.into();
                                m = m.child(activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("acc-{app_slug}")))
                                        .role(Role::MenuItem)
                                        .tab_index(0)
                                        .h(px(30.))
                                        .px(px(8.))
                                        .rounded(px(7.))
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .text_size(px(12.5))
                                        .text_color(p.ink)
                                        .cursor_pointer()
                                        .hover(move |s| s.bg(p.hover))
                                        .child(pro_app_tile(app_slug, 18., cx))
                                        .child(acc_label),
                                    cx,
                                    move |this, _, cx| {
                                        if let Some(n) =
                                            this.nodes.iter_mut().find(|n| n.id == nid_a)
                                        {
                                            if app_s.as_ref() != "database" {
                                                n.app = Some(app_s.clone());
                                            } else {
                                                n.app = None;
                                            }
                                            n.account = acc_s.clone();
                                            n.configured = true;
                                            if n.chip.as_ref() == "Not configured" {
                                                n.chip = "Ready".into();
                                            }
                                            let updated = n.clone();
                                            cx.emit(WorkflowBuilderEvent::NodeUpdated(updated));
                                        }
                                        this.top_menu = None;
                                        cx.notify();
                                    },
                                ));
                            }
                            pro_presence(
                                pro_eid(&self.id, "ins-acc-pres"),
                                acc_open,
                                0.18,
                                0.12,
                                move |a| {
                                    m.top(px(38. - 4. * (1. - a))).opacity(a).into_any_element()
                                },
                            )
                        }),
                ),
        );

        // Agent Model & Tool Chips (when Agent step)
        if node.kind == WorkflowNodeKind::Agent {
            let nid_model = node.id.clone();
            let nid_tools = node.id.clone();
            body = body
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.text)
                                .child(tr(cx, "Model", "模型选择")),
                        )
                        .child(
                            div()
                                .relative()
                                .child(activate(
                                    div()
                                        .id(pro_eid(&self.id, "ins-model"))
                                        .role(Role::Button)
                                        .aria_expanded(mod_open)
                                        .tab_index(0)
                                        .h(px(32.))
                                        .px(px(10.))
                                        .rounded(px(8.))
                                        .bg(p.subtle)
                                        .border_1()
                                        .border_color(p.line_strong)
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .text_size(px(12.5))
                                        .text_color(p.ink)
                                        .cursor_pointer()
                                        .hover(move |s| s.bg(p.hover))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(6.))
                                                .child(pro_icon(ProIcon::Sparkles, 13., p.purple))
                                                .child(node.model.clone()),
                                        )
                                        .child(pro_icon(ProIcon::ChevronDown, 13., p.muted)),
                                    cx,
                                    |this, _, cx| {
                                        this.top_menu =
                                            if this.top_menu == Some(TopMenu::InspectorModel) {
                                                None
                                            } else {
                                                Some(TopMenu::InspectorModel)
                                            };
                                        cx.notify();
                                    },
                                ))
                                .child({
                                    let mut m = div()
                                        .occlude()
                                        .absolute()
                                        .left_0()
                                        .right_0()
                                        .p(px(4.))
                                        .rounded(px(10.))
                                        .bg(p.card)
                                        .border_1()
                                        .border_color(p.line_strong)
                                        .shadow_md()
                                        .flex()
                                        .flex_col();
                                    for mdl in AGENT_MODELS {
                                        let nid_m = nid_model.clone();
                                        let ms: SharedString = mdl.into();
                                        m = m.child(activate(
                                            div()
                                                .id(pro_eid(&self.id, format!("mdl-{mdl}")))
                                                .role(Role::MenuItem)
                                                .tab_index(0)
                                                .h(px(29.))
                                                .px(px(8.))
                                                .rounded(px(7.))
                                                .flex()
                                                .items_center()
                                                .text_size(px(12.5))
                                                .text_color(p.ink)
                                                .cursor_pointer()
                                                .hover(move |s| s.bg(p.hover))
                                                .child(mdl),
                                            cx,
                                            move |this, _, cx| {
                                                if let Some(n) =
                                                    this.nodes.iter_mut().find(|n| n.id == nid_m)
                                                {
                                                    n.model = ms.clone();
                                                }
                                                this.top_menu = None;
                                                cx.notify();
                                            },
                                        ));
                                    }
                                    pro_presence(
                                        pro_eid(&self.id, "ins-mdl-pres"),
                                        mod_open,
                                        0.18,
                                        0.12,
                                        move |a| {
                                            m.top(px(36. - 4. * (1. - a)))
                                                .opacity(a)
                                                .into_any_element()
                                        },
                                    )
                                }),
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
                                .text_color(p.text)
                                .child(tr(cx, "Agent tools", "智能体工具")),
                        )
                        .child(div().flex().flex_wrap().gap(px(4.)).children(
                            AGENT_TOOLS.into_iter().map(|tname| {
                                let on = node.tools.iter().any(|t| t.as_ref() == tname);
                                let ts: SharedString = tname.into();
                                let nid_t = nid_tools.clone();
                                activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("ag-tool-{tname}")))
                                        .role(Role::CheckBox)
                                        .aria_toggled(on.into())
                                        .tab_index(0)
                                        .h(px(24.))
                                        .px(px(8.))
                                        .rounded(px(6.))
                                        .bg(if on { p.accent_soft } else { p.chip })
                                        .text_size(px(12.))
                                        .text_color(if on { p.blue } else { p.text })
                                        .flex()
                                        .items_center()
                                        .cursor_pointer()
                                        .child(tname),
                                    cx,
                                    move |this, _, cx| {
                                        if let Some(n) =
                                            this.nodes.iter_mut().find(|n| n.id == nid_t)
                                        {
                                            if let Some(pos) = n.tools.iter().position(|x| x == &ts)
                                            {
                                                n.tools.remove(pos);
                                            } else {
                                                n.tools.push(ts.clone());
                                            }
                                            n.chip = format!("{} tools", n.tools.len()).into();
                                        }
                                        cx.notify();
                                    },
                                )
                            }),
                        )),
                );
        }

        // Prompt / Condition / Field Mapping Editor
        let expr_heading = match node.kind {
            WorkflowNodeKind::Trigger => tr(cx, "Event filter & source", "事件筛选与来源"),
            WorkflowNodeKind::Agent => tr(cx, "System prompt & instructions", "提示词与指令"),
            WorkflowNodeKind::Tool => tr(cx, "Endpoint & query mapping", "接口地址与参数映射"),
            WorkflowNodeKind::Logic => tr(cx, "Branch condition", "分支判断条件"),
            WorkflowNodeKind::Action => tr(cx, "Field mapping / payload", "字段映射与消息模板"),
        };
        if let Some(ref expr_ed) = self.node_expr_editor {
            body = body.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.text)
                            .child(expr_heading),
                    )
                    .child(
                        div()
                            .min_h(px(76.))
                            .p(px(9.))
                            .rounded(px(8.))
                            .bg(p.subtle)
                            .border_1()
                            .border_color(p.line_strong)
                            .child(expr_ed.clone()),
                    ),
            );
        }

        // Outgoing connections list
        let outgoing: Vec<(SharedString, WorkflowPort, SharedString)> = self
            .edges
            .iter()
            .filter(|e| e.from == node.id)
            .filter_map(|e| {
                self.nodes
                    .iter()
                    .find(|n| n.id == e.to)
                    .map(|target| (e.id.clone(), e.port, target.title.clone()))
            })
            .collect();

        let mut conns_sec = div()
            .pt(px(12.))
            .border_t_1()
            .border_color(p.line_soft)
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.text)
                    .child(tr(cx, "Next steps", "后续连接")),
            );

        for (eid, port, target_title) in outgoing {
            let port_prefix = match port {
                WorkflowPort::Out => String::new(),
                WorkflowPort::Yes => "Yes → ".to_owned(),
                WorkflowPort::No => "No → ".to_owned(),
            };
            conns_sec = conns_sec.child(
                div()
                    .h(px(30.))
                    .pl(px(10.))
                    .pr(px(4.))
                    .rounded(px(8.))
                    .bg(p.subtle)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(6.))
                    .text_size(px(12.5))
                    .text_color(p.ink)
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .child(format!("{port_prefix}{target_title}")),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, format!("rm-edge-{eid}")),
                        tr(cx, "Remove connection", "移除连接"),
                        ProIcon::Close,
                        22.,
                        false,
                        cx,
                        move |this, _, cx| {
                            this.edges.retain(|e| e.id != eid);
                            cx.notify();
                        },
                    )),
            );
        }
        body = body.child(conns_sec);

        // Test Step Output JSON preview
        let tested_here = self
            .step_test_banner
            .as_ref()
            .filter(|(tid, _)| tid == &node.id)
            .map(|(_, msg)| msg.clone());

        body = body.child(
            div()
                .pt(px(12.))
                .border_t_1()
                .border_color(p.line_soft)
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
                                .text_size(px(10.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.muted)
                                .child(tr(cx, "STEP OUTPUT JSON", "单步测试输出 JSON")),
                        )
                        .child(pro_btn(
                            pro_eid(&self.id, "ins-test-step"),
                            tr(cx, "Test step", "测试此步"),
                            Some(ProIcon::Play),
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.step_test_banner = Some((
                                    nid_test.clone(),
                                    tr(cx, "200 OK · 84ms", "200 OK · 84ms").into(),
                                ));
                                cx.notify();
                            },
                        )),
                )
                .child({
                    let test_open = tested_here.is_some();
                    let test_msg = tested_here
                        .unwrap_or_else(|| tr(cx, "200 OK · 84ms", "200 OK · 84ms").into());
                    let banner = div()
                        .px(px(8.))
                        .py(px(4.))
                        .rounded(px(6.))
                        .bg(p.green_soft)
                        .text_size(px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.green_ink)
                        .child(test_msg);
                    pro_fold(
                        pro_eid(&self.id, format!("step-test-fold-{}", node.id)),
                        test_open,
                        28.,
                        0.,
                        move || banner.into_any_element(),
                    )
                })
                .child(
                    div()
                        .p(px(10.))
                        .rounded(px(8.))
                        .bg(p.subtle)
                        .border_1()
                        .border_color(p.line)
                        .text_size(px(11.5))
                        .line_height(px(17.))
                        .text_color(p.text)
                        .child(node.sample_output.clone()),
                )
                .child(div().pt(px(4.)).flex().justify_end().child(pro_btn(
                    pro_eid(&self.id, "ins-delete-step"),
                    tr(cx, "Delete step", "删除步骤"),
                    Some(ProIcon::Trash),
                    ProBtnStyle::Danger,
                    true,
                    cx,
                    move |this, _, cx| {
                        this.delete_node(&nid_del_foot, cx);
                    },
                ))),
        );

        let target_bottom_offset = if self.run_panel_open { 232. } else { 10. };
        let bottom_offset = pro_spring_f32(
            pro_eid(&self.id, "insp-bot-sp"),
            target_bottom_offset,
            0.24,
            w,
            cx,
        );
        let body_scroll = div()
            .id(pro_eid(&self.id, "inspector-scroll"))
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .child(pro_view_transition(
                pro_eid(&self.id, "wf-insp-vtx"),
                nid.clone(),
                body,
            ));
        div()
            .id(pro_eid(&self.id, "node-inspector"))
            .occlude()
            .absolute()
            .when(!compact, |d| {
                d.top(px(10.))
                    .right(px(10.))
                    .bottom(px(bottom_offset))
                    .w(px(320.))
            })
            .when(compact, |d| {
                d.left(px(8.)).right(px(8.)).bottom(px(8.)).h(px(390.))
            })
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .shadow_md()
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .h(px(52.))
                    .pl(px(12.))
                    .pr(px(8.))
                    .border_b_1()
                    .border_color(p.line)
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(Self::render_node_tile(
                        &node, 28., false, elapsed_ms, &p, cx,
                    ))
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
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(node.title.clone()),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .child(kind_sub),
                            ),
                    )
                    .child(pro_switch(
                        pro_eid(&self.id, "ins-enable-switch"),
                        tr(cx, "Enable step", "启用步骤"),
                        node.enabled,
                        cx,
                        move |this, _, cx| {
                            if let Some(n) = this.nodes.iter_mut().find(|n| n.id == nid_toggle) {
                                n.enabled = !n.enabled;
                            }
                            cx.notify();
                        },
                    ))
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "ins-trash-head"),
                        tr(cx, "Delete step", "删除步骤"),
                        ProIcon::Trash,
                        28.,
                        false,
                        cx,
                        move |this, _, cx| {
                            this.delete_node(&nid_del, cx);
                        },
                    ))
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "ins-close"),
                        tr(cx, "Close settings", "关闭设置"),
                        ProIcon::Close,
                        28.,
                        false,
                        cx,
                        |this, _, cx| this.select_node(None, cx),
                    )),
            )
            .child(body_scroll)
    }

    fn render_run_drawer(&self, run_elapsed_ms: Option<f32>, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let err_count = self.unconfigured_count();

        let tabs = [
            (
                WorkflowRunTab::Payload,
                ProIcon::Code,
                tr(cx, "Payload", "测试载荷"),
                0usize,
            ),
            (
                WorkflowRunTab::Triggers,
                ProIcon::Bolt,
                tr(cx, "Triggers", "触发器"),
                0usize,
            ),
            (
                WorkflowRunTab::Errors,
                ProIcon::AlertTriangle,
                tr(cx, "Errors", "告警"),
                err_count,
            ),
            (
                WorkflowRunTab::Timeline,
                ProIcon::Clock,
                tr(cx, "Timeline", "耗时时间线"),
                0usize,
            ),
            (
                WorkflowRunTab::Runs,
                ProIcon::Play,
                tr(cx, "Runs", "运行记录"),
                0usize,
            ),
        ];

        let rtab_sensor = pro_selection_sensor();
        let mut tab_bar =
            div()
                .relative()
                .flex()
                .items_center()
                .gap(px(2.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "run-tabs-pill"),
                    format!("{:?}", self.run_tab),
                    rtab_sensor.clone(),
                    6.,
                    p.chip,
                    None,
                    false,
                ));

        for (tab, ic, label, badge_n) in tabs {
            let sel = self.run_tab == tab;
            tab_bar = tab_bar.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("run-tab-{tab:?}")))
                    .role(Role::Tab)
                    .aria_selected(sel)
                    .tab_index(0)
                    .relative()
                    .h(px(28.))
                    .px(px(8.))
                    .rounded(px(6.))
                    .when(sel, |d| {
                        d.text_color(p.ink).child(pro_mark_selected(&rtab_sensor))
                    })
                    .when(!sel, |d| {
                        d.text_color(p.muted).hover(move |s| s.text_color(p.ink))
                    })
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .cursor_pointer()
                    .child(pro_icon(ic, 13.5, if sel { p.ink } else { p.muted }))
                    .child(label)
                    .when(badge_n > 0, |d| {
                        d.child(
                            div()
                                .min_w(px(16.))
                                .px(px(4.))
                                .rounded(px(4.))
                                .bg(p.red_soft)
                                .text_size(px(11.))
                                .text_color(p.red)
                                .child(format!("{badge_n}")),
                        )
                    }),
                cx,
                move |this, _, cx| {
                    this.run_tab = tab;
                    cx.notify();
                },
            ));
        }

        let body: AnyElement = match self.run_tab {
            WorkflowRunTab::Payload => div()
                .p(px(10.))
                .rounded(px(8.))
                .bg(p.subtle)
                .border_1()
                .border_color(p.line)
                .text_size(px(12.))
                .line_height(px(18.))
                .text_color(p.text)
                .child(
                    "{\n  \"event\": \"auth.user_created\",\n  \"user\": { \"name\": \"Elena Rostova\", \"email\": \"elena@linear.app\", \"domain\": \"linear.app\" },\n  \"plan\": \"Pro\"\n}",
                )
                .into_any_element(),
            WorkflowRunTab::Triggers => {
                let mut list = div().flex().flex_col().gap(px(4.));
                for n in self.nodes.iter().filter(|n| n.kind == WorkflowNodeKind::Trigger) {
                    let nid = n.id.clone();
                    list = list.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("trg-row-{nid}")))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(32.))
                            .px(px(10.))
                            .rounded(px(7.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .cursor_pointer()
                            .hover(move |s| s.bg(p.hover))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .text_size(px(13.))
                                    .text_color(p.ink)
                                    .child(n.title.clone())
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.muted)
                                            .child(n.sub.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .px(px(6.))
                                    .py(px(1.))
                                    .rounded(px(5.))
                                    .bg(if n.configured {
                                        p.green_soft
                                    } else {
                                        p.red_soft
                                    })
                                    .text_size(px(11.))
                                    .text_color(if n.configured {
                                        p.green_ink
                                    } else {
                                        p.red
                                    })
                                    .child(if n.configured {
                                        tr(cx, "Active", "已配置")
                                    } else {
                                        tr(cx, "Not configured", "未配置")
                                    }),
                            ),
                        cx,
                        move |this, _, cx| this.select_node(Some(nid.clone()), cx),
                    ));
                }
                list.into_any_element()
            }
            WorkflowRunTab::Errors => {
                let uncfg: Vec<&WorkflowNode> =
                    self.nodes.iter().filter(|n| !n.configured).collect();
                if uncfg.is_empty() {
                    div()
                        .py(px(24.))
                        .text_center()
                        .text_size(px(13.))
                        .text_color(p.muted)
                        .child(tr(cx, "No warnings or errors — all triggers and actions are configured.", "没有任何错误或未配置项。"))
                        .into_any_element()
                } else {
                    let mut list = div().flex().flex_col().gap(px(4.));
                    for n in uncfg {
                        let nid = n.id.clone();
                        list = list.child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("err-row-{nid}")))
                                .role(Role::Button)
                                .tab_index(0)
                                .h(px(34.))
                                .px(px(10.))
                                .rounded(px(7.))
                                .flex()
                                .items_center()
                                .justify_between()
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(pro_icon(ProIcon::AlertTriangle, 14., p.red))
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .child(n.title.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(p.red)
                                                .child(n.sub.clone()),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(p.blue)
                                        .child(tr(cx, "Configure →", "去配置 →")),
                                ),
                            cx,
                            move |this, _, cx| this.select_node(Some(nid.clone()), cx),
                        ));
                    }
                    list.into_any_element()
                }
            }
            WorkflowRunTab::Timeline => {
                let steps: Vec<(SharedString, u64)> = if let Some(latest) = self.runs.first() {
                    latest
                        .steps
                        .iter()
                        .map(|s| (s.title.clone(), s.duration_ms))
                        .collect()
                } else {
                    self.build_execution_plan()
                        .into_iter()
                        .filter_map(|(nid, _, ms, _)| {
                            self.nodes
                                .iter()
                                .find(|n| n.id == nid)
                                .map(|n| (n.title.clone(), ms))
                        })
                        .collect()
                };
                let total = steps.iter().map(|(_, ms)| *ms).sum::<u64>().max(1) as f32;
                let mut offset = 0.0_f32;
                let mut rows = div().flex().flex_col().gap(px(4.));
                for (title, ms) in steps {
                    let raw_w = (ms as f32 / total).max(0.06);
                    let frac = raw_w.min((1.0 - offset).max(0.02));
                    let left_pct = offset;
                    offset = (offset + (ms as f32 / total)).min(0.94);
                    rows = rows.child(
                        div()
                            .h(px(28.))
                            .px(px(8.))
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .text_size(px(12.5))
                            .child(div().w(px(148.)).truncate().text_color(p.ink).child(title))
                            .child(
                                div()
                                    .flex_1()
                                    .h(px(8.))
                                    .rounded(px(4.))
                                    .bg(p.subtle)
                                    .relative()
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .absolute()
                                            .top_0()
                                            .bottom_0()
                                            .left(relative(left_pct))
                                            .w(relative(frac))
                                            .rounded(px(4.))
                                            .bg(p.blue),
                                    ),
                            )
                            .child(
                                div()
                                    .w(px(56.))
                                    .text_right()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(format!("{ms}ms")),
                            ),
                    );
                }
                rows.into_any_element()
            }
            WorkflowRunTab::Runs => {
                if let Some(el_ms) = run_elapsed_ms {
                    let active_stage = (el_ms / RUN_STAGE_MS).floor() as usize;
                    let total_run_ms = (self.run_order.len().max(1) as f32) * RUN_STAGE_MS;
                    let run_frac = (el_ms / total_run_ms).clamp(0.04, 1.0);
                    let mut list = div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(
                            div().px(px(8.)).pb(px(2.)).child(pro_spring_bar(
                                pro_eid(&self.id, "live-run-bar"),
                                run_frac,
                                4.,
                                2.,
                                p.chip,
                                p.blue,
                            )),
                        );
                    for (si, (nid, _, ms, cred)) in self.run_order.iter().enumerate() {
                        if si > active_stage {
                            break;
                        }
                        if let Some(node) = self.nodes.iter().find(|n| &n.id == nid) {
                            let done = si < active_stage;
                            list = list.child(
                                div()
                                    .h(px(30.))
                                    .px(px(10.))
                                    .rounded(px(6.))
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .text_size(px(13.))
                                    .child(if done {
                                        div()
                                            .size(px(16.))
                                            .rounded_full()
                                            .bg(p.green_soft)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(pro_svg(
                                                "0 0 24 24",
                                                SVG_CHECK_SM,
                                                11.,
                                                p.green_ink,
                                            ))
                                            .into_any_element()
                                    } else {
                                        pro_icon(ProIcon::Play, 13., p.blue)
                                    })
                                    .child(
                                        div()
                                            .text_color(p.ink)
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(node.title.clone()),
                                    )
                                    .child(
                                        div()
                                            .px(px(6.))
                                            .rounded(px(4.))
                                            .bg(p.chip)
                                            .text_size(px(11.))
                                            .text_color(p.muted)
                                            .child(node.kind.badge_label(cx)),
                                    )
                                    .child(div().flex_1())
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.muted)
                                            .child(if done {
                                                format!("{ms}ms")
                                            } else {
                                                tr(cx, "Running…", "执行中…").to_owned()
                                            }),
                                    )
                                    .child(
                                        div()
                                            .w(px(74.))
                                            .text_right()
                                            .text_size(px(12.))
                                            .text_color(p.faint)
                                            .child(format!("{cred} cr")),
                                    ),
                            );
                        }
                    }
                    list.into_any_element()
                } else if let Some(latest) = self.runs.first() {
                    let mut list = div()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .h(px(30.))
                                .px(px(10.))
                                .flex()
                                .items_center()
                                .justify_between()
                                .text_size(px(12.5))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(
                                            div()
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.strong)
                                                .child(format!("Test run {}", latest.id)),
                                        )
                                        .child(
                                            div()
                                                .text_color(p.faint)
                                                .child(latest.started_at.clone()),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_color(p.muted)
                                        .child(format!(
                                            "{}ms · {} credits",
                                            latest.total_ms, latest.total_credits
                                        )),
                                ),
                        );
                    for step in &latest.steps {
                        let nid = step.node_id.clone();
                        list = list.child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("run-step-{nid}")))
                                .role(Role::Button)
                                .tab_index(0)
                                .h(px(30.))
                                .pl(px(22.))
                                .pr(px(10.))
                                .rounded(px(6.))
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .text_size(px(12.5))
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover))
                                .child(
                                    div()
                                        .size(px(16.))
                                        .rounded_full()
                                        .bg(p.green_soft)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(pro_svg("0 0 24 24", SVG_CHECK_SM, 11., p.green_ink)),
                                )
                                .child(div().text_color(p.ink).child(step.title.clone()))
                                .child(
                                    div()
                                        .px(px(5.))
                                        .rounded(px(4.))
                                        .bg(p.chip)
                                        .text_size(px(10.5))
                                        .text_color(p.muted)
                                        .child(step.kind.badge_label(cx)),
                                )
                                .child(div().flex_1())
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(p.muted)
                                        .child(format!("{}ms", step.duration_ms)),
                                )
                                .child(
                                    div()
                                        .w(px(68.))
                                        .text_right()
                                        .text_size(px(12.))
                                        .text_color(p.faint)
                                        .child(format!("{} cr", step.credits)),
                                ),
                            cx,
                            move |this, _, cx| this.select_node(Some(nid.clone()), cx),
                        ));
                    }
                    list.into_any_element()
                } else {
                    div()
                        .py(px(16.))
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(6.))
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(tr(cx, "No runs yet", "暂无运行记录")),
                        )
                        .child(
                            div()
                                .text_size(px(12.5))
                                .text_color(p.muted)
                                .child(tr(
                                    cx,
                                    "Press Run to test the workflow with the sample payload.",
                                    "点击运行测试以使用示例载荷验证工作流。",
                                )),
                        )
                        .child(
                            div()
                                .pt(px(4.))
                                .child(pro_btn(
                                    pro_eid(&self.id, "run-empty-start"),
                                    tr(cx, "Start test run", "启动测试运行"),
                                    Some(ProIcon::Play),
                                    ProBtnStyle::Secondary,
                                    true,
                                    cx,
                                    |this, _, cx| this.start_test_run(cx),
                                )),
                        )
                        .into_any_element()
                }
            }
        };

        div()
            .h(px(220.))
            .flex_none()
            .bg(p.panel)
            .border_t_1()
            .border_color(p.line_strong)
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .h(px(42.))
                    .px(px(10.))
                    .border_b_1()
                    .border_color(p.line)
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(tab_bar)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(pro_btn(
                                pro_eid(&self.id, "run-again-btn"),
                                tr(cx, "Test run", "运行测试"),
                                Some(ProIcon::Play),
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                |this, _, cx| this.start_test_run(cx),
                            ))
                            .child(pro_icon_btn(
                                pro_eid(&self.id, "run-close-btn"),
                                tr(cx, "Close run panel", "关闭运行面板"),
                                ProIcon::Close,
                                28.,
                                false,
                                cx,
                                |this, _, cx| {
                                    this.run_panel_open = false;
                                    cx.notify();
                                },
                            )),
                    ),
            )
            .child(
                div()
                    .id(pro_eid(&self.id, "run-drawer-scroll"))
                    .flex_1()
                    .min_h_0()
                    .p(px(8.))
                    .overflow_y_scroll()
                    .child(pro_view_transition(
                        pro_eid(&self.id, "run-drawer-vtx"),
                        format!("{:?}", self.run_tab),
                        body,
                    )),
            )
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editors(w, cx);
        if let Some((_, started)) = &self.toast
            && started.elapsed().as_secs_f32() >= 2.45
        {
            self.toast = None;
        }
        if let Some(ref sid) = self.selected_node_id {
            self.last_inspected_id = Some(sid.clone());
        }
        if let Some(ref pk) = self.insert_picker {
            self.last_picker = Some(pk.clone());
        }
        let reduced = Theme::global(cx).reduced_motion;
        let boot_ms = self.boot_at.elapsed().as_secs_f32() * 1000.;

        // Advance or finish live execution run
        let mut run_elapsed_ms: Option<f32> = None;
        if let Some(started) = self.run_started_at {
            let ms = started.elapsed().as_secs_f32() * 1000.;
            let total_anim = (self.run_order.len() as f32) * RUN_STAGE_MS;
            if ms >= total_anim {
                self.finalize_run(cx);
            } else {
                run_elapsed_ms = Some(ms);
                if !reduced {
                    w.request_animation_frame();
                }
            }
        }
        if self.loading && !reduced {
            w.request_animation_frame();
        }
        let skel_phase = if self.loading && !reduced {
            (boot_ms / 1100.) % 1.0
        } else {
            0.35
        };

        let width_cell = self.measured_width.clone();
        let compact = width_cell.get() < 680.;
        let p = ProPalette::get(cx);

        // Compute initial canvas centering once measured
        let (cw, _ch) = self.canvas_size.get();
        if !self.centered_once && cw > 300. {
            self.centered_once = true;
            let left_clear = if self.palette_open && !compact {
                228.
            } else {
                20.
            };
            let avail = (cw - left_clear).max(280.);
            self.pan_x = left_clear + avail * 0.5 - (NODE_W * 0.5) * self.zoom - 24.;
            self.pan_y = 76.;
        }

        let active_run_stage = run_elapsed_ms.map(|ms| (ms / RUN_STAGE_MS).floor() as usize);
        let stage_progress = run_elapsed_ms
            .map(|ms| ((ms % RUN_STAGE_MS) / RUN_STAGE_MS).clamp(0., 1.))
            .unwrap_or(0.);

        // Prepare edge specs for canvas painting
        let mut edge_specs = Vec::with_capacity(self.edges.len());
        let mut branch_tags: Vec<(SharedString, f32, f32)> = Vec::new();

        if !self.empty {
            for edge in &self.edges {
                let Some(src) = self.nodes.iter().find(|n| n.id == edge.from) else {
                    continue;
                };
                let Some(dst) = self.nodes.iter().find(|n| n.id == edge.to) else {
                    continue;
                };
                let sx = self.pan_x + (src.x + edge.port.offset_x()) * self.zoom;
                let sy = self.pan_y + (src.y + NODE_H) * self.zoom;
                let dx = self.pan_x + (dst.x + NODE_W * 0.5) * self.zoom;
                let dy = self.pan_y + dst.y * self.zoom;

                if let Some(tag_txt) = edge.port.tag_label(cx) {
                    branch_tags.push((tag_txt.into(), (sx + dx) * 0.5, (sy + dy) * 0.5));
                }

                let mut flowing = false;
                let mut lit_done = false;
                if let Some(st_idx) = active_run_stage {
                    for (ri, (_, inc_edge, _, _)) in self.run_order.iter().enumerate() {
                        if inc_edge.as_ref() == Some(&edge.id) {
                            if ri == st_idx {
                                flowing = true;
                            } else if ri < st_idx {
                                lit_done = true;
                            }
                        }
                    }
                } else if !self.runs.is_empty() {
                    lit_done = self
                        .run_order
                        .iter()
                        .any(|(_, inc_edge, _, _)| inc_edge.as_ref() == Some(&edge.id));
                }

                let (_, sender_c) = src.kind.colors(&p);
                edge_specs.push(EdgePaintSpec {
                    p0: (sx, sy),
                    p3: (dx, dy),
                    selected: self.selected_edge_id.as_ref() == Some(&edge.id),
                    lit_done,
                    flowing,
                    flow_t: stage_progress,
                    sender_color: sender_c,
                });
            }
        }

        let dot_color: Hsla = if p.dark {
            hsla(0., 0., 1., 0.12)
        } else {
            hsla(0., 0., 0., 0.15)
        };
        let edge_rest_c: Hsla = if p.dark {
            hsla(0., 0., 1., 0.22)
        } else {
            hsla(0., 0., 0., 0.20)
        };
        let zoom_val = self.zoom;
        let pan_x_val = self.pan_x;
        let pan_y_val = self.pan_y;
        let is_loading = self.loading;
        let canvas_size_cell = self.canvas_size.clone();
        let canvas_origin_cell = self.canvas_origin.clone();

        let backdrop = canvas(
            move |bounds, _, _| {
                canvas_size_cell.set((f32::from(bounds.size.width), f32::from(bounds.size.height)));
                canvas_origin_cell.set((f32::from(bounds.origin.x), f32::from(bounds.origin.y)));
            },
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 4. || bh <= 4. {
                    return;
                }

                // 1. Zoom-linked dot grid
                let step = (20. * zoom_val).clamp(11., 32.);
                let ox = pan_x_val.rem_euclid(step);
                let oy = pan_y_val.rem_euclid(step);
                let mut gx = ox;
                while gx < bw {
                    let mut gy = oy;
                    while gy < bh {
                        window.paint_quad(
                            fill(
                                Bounds::new(
                                    point(bounds.left() + px(gx), bounds.top() + px(gy)),
                                    size(px(1.5), px(1.5)),
                                ),
                                dot_color,
                            )
                            .corner_radii(px(0.8)),
                        );
                        gy += step;
                    }
                    gx += step;
                }

                // 2. Smooth cubic Bezier edges + live pulse
                for spec in &edge_specs {
                    let (x0, y0) = spec.p0;
                    let (x3, y3) = spec.p3;
                    let c_dy = ((y3 - y0).abs() * 0.5).max(22. * zoom_val);
                    let (x1, y1) = (x0, y0 + c_dy);
                    let (x2, y2) = (x3, y3 - c_dy);

                    let sample_cubic = |t: f32| -> Point<Pixels> {
                        let inv = 1. - t;
                        let b0 = inv * inv * inv;
                        let b1 = 3. * inv * inv * t;
                        let b2 = 3. * inv * t * t;
                        let b3 = t * t * t;
                        point(
                            bounds.left() + px(b0 * x0 + b1 * x1 + b2 * x2 + b3 * x3),
                            bounds.top() + px(b0 * y0 + b1 * y1 + b2 * y2 + b3 * y3),
                        )
                    };

                    let segs = 24usize;
                    let base_c = if is_loading {
                        edge_rest_c.opacity(0.12)
                    } else if spec.selected || spec.lit_done {
                        spec.sender_color.opacity(0.75)
                    } else {
                        edge_rest_c
                    };
                    let mut pb = PathBuilder::stroke(px(if spec.selected || spec.lit_done {
                        1.7
                    } else {
                        1.3
                    }));
                    for s in 0..=segs {
                        let pt = sample_cubic(s as f32 / segs as f32);
                        if s == 0 {
                            pb.move_to(pt);
                        } else {
                            pb.line_to(pt);
                        }
                    }
                    if let Ok(path) = pb.build() {
                        window.paint_path(path, base_c);
                    }

                    if spec.flowing && !reduced && !is_loading {
                        let head = (spec.flow_t * 1.35).clamp(0., 1.);
                        let tail = (head - 0.35).max(0.);
                        if head > tail + 0.02 {
                            let mut fpb = PathBuilder::stroke(px(2.4));
                            let pulse_steps = 12usize;
                            for k in 0..=pulse_steps {
                                let t = tail + (head - tail) * (k as f32 / pulse_steps as f32);
                                let pt = sample_cubic(t);
                                if k == 0 {
                                    fpb.move_to(pt);
                                } else {
                                    fpb.line_to(pt);
                                }
                            }
                            if let Ok(fpath) = fpb.build() {
                                window.paint_path(fpath, spec.sender_color);
                            }
                            let dot_pt = sample_cubic(head);
                            window.paint_quad(
                                fill(
                                    Bounds::new(
                                        point(dot_pt.x - px(3.5), dot_pt.y - px(3.5)),
                                        size(px(7.), px(7.)),
                                    ),
                                    spec.sender_color,
                                )
                                .corner_radii(px(3.5)),
                            );
                        }
                    }
                }
            },
        )
        .size_full();

        let is_panning = matches!(self.drag, Some(CanvasDrag::Pan { .. }));
        let mut vp = div()
            .id(pro_eid(&self.id, "wf-viewport"))
            .relative()
            .flex_1()
            .min_h_0()
            .bg(p.panel)
            .overflow_hidden()
            .when(self.tool == WorkflowCanvasTool::Comment, |d| {
                d.cursor_pointer()
            })
            .when(self.tool == WorkflowCanvasTool::Select && is_panning, |d| {
                d.cursor(CursorStyle::ClosedHand)
            })
            .when(
                self.tool == WorkflowCanvasTool::Select && !is_panning,
                |d| d.cursor(CursorStyle::OpenHand),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, w, cx| {
                    this.top_menu = None;
                    this.insert_picker = None;
                    if this.tool == WorkflowCanvasTool::Comment {
                        let (ox, oy) = this.canvas_origin.get();
                        let wx = (f32::from(ev.position.x) - ox - this.pan_x) / this.zoom;
                        let wy = (f32::from(ev.position.y) - oy - this.pan_y) / this.zoom;
                        let cid: SharedString = format!("pin-{}", this.comments.len() + 1).into();
                        this.comments.push(WorkflowComment {
                            id: cid.clone(),
                            x: wx,
                            y: wy,
                            author: this.user_name.clone(),
                            text: "Check retry policy before going live".into(),
                        });
                        this.open_comment_id = Some(cid);
                        this.tool = WorkflowCanvasTool::Select;
                        if let Some(ed) = this.comment_editor.clone() {
                            ed.update(cx, |e, cx| {
                                e.set_value("Check retry policy before going live", w, cx);
                                e.focus(w, cx);
                            });
                        }
                        cx.notify();
                        return;
                    }
                    this.open_comment_id = None;
                    this.drag = Some(CanvasDrag::Pan {
                        start_mouse: ev.position,
                        orig_pan_x: this.pan_x,
                        orig_pan_y: this.pan_y,
                    });
                    cx.notify();
                }),
            )
            .on_scroll(cx.listener(|this, ev: &ScrollEvent, _, cx| {
                let delta = ev.delta.pixel_delta(px(18.));
                if ev.modifiers.platform || ev.modifiers.control {
                    let dy = f32::from(delta.y);
                    let next_z = (this.zoom + dy * 0.002).clamp(0.45, 1.4);
                    this.zoom = (next_z * 100.).round() / 100.;
                } else {
                    this.pan_x += f32::from(delta.x);
                    this.pan_y += f32::from(delta.y);
                }
                cx.notify();
            }))
            .child(backdrop);

        // Render midpoint YES / NO tags on branch edges
        if !self.empty && !self.loading {
            for (tag_lbl, tx, ty) in branch_tags {
                vp = vp.child(
                    div()
                        .absolute()
                        .left(px(tx - 15.))
                        .top(px(ty - 9.))
                        .px(px(5.))
                        .py(px(1.))
                        .rounded(px(4.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .text_size(px(9.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(tag_lbl),
                );
            }
        }

        // Render empty state OR skeleton nodes OR interactive nodes
        if self.empty && !self.loading {
            let empty_card = pro_empty_state(
                ProIcon::Bolt,
                tr(cx, "Add a trigger", "添加触发器"),
                tr(
                    cx,
                    "Start this workflow with a signup, document upload, schedule, or incoming webhook.",
                    "选择用户注册、文档上传、定时计划或 Webhook 作为工作流起点。",
                ),
                Some((
                    pro_eid(&self.id, "empty-add-trigger"),
                    tr(cx, "Add trigger", "添加触发器").into(),
                    Box::new(|this: &mut Self, _, cx| {
                        let tpl = WorkflowPaletteItem {
                            node_type: "new-signup".into(),
                            label: "New signup".into(),
                            kind: WorkflowNodeKind::Trigger,
                            sub: "When a user signs up".into(),
                            default_expr: "auth.user_created · Sign-up form".into(),
                        };
                        this.add_node_from_template(&tpl, None, cx);
                    }),
                )),
                cx,
            );
            vp = vp.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .p(px(24.))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .max_w(px(360.))
                            .rounded(px(14.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .child(empty_card),
                    ),
            );
        } else {
            let nodes_snapshot = self.nodes.clone();
            for (ni, node) in nodes_snapshot.iter().enumerate() {
                let sx = self.pan_x + node.x * self.zoom;
                let sy = self.pan_y + node.y * self.zoom;
                let sw = (NODE_W * self.zoom).round();
                let sh = (NODE_H * self.zoom).round();

                if self.loading {
                    let ph = (skel_phase + ni as f32 * 0.09) % 1.0;
                    vp = vp.child(
                        div()
                            .absolute()
                            .left(px(sx))
                            .top(px(sy))
                            .w(px(sw))
                            .h(px(sh))
                            .p(px(12. * self.zoom))
                            .rounded(px(10. * self.zoom))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(pro_skeleton_bar(22., 22., 6., ph, cx))
                                    .child(pro_skeleton_bar(112., 12., 4., ph, cx)),
                            )
                            .child(pro_skeleton_bar(164., 10., 4., ph, cx))
                            .child(pro_skeleton_bar(sw - 24., 26., 6., ph, cx)),
                    );
                    continue;
                }

                let (kind_bg, kind_fg) = node.kind.colors(&p);
                let is_selected = self.selected_node_id.as_ref() == Some(&node.id);

                // Check run status of this node
                let mut is_step_running = false;
                let mut step_done_ms: Option<u64> = None;
                if let Some(st_idx) = active_run_stage {
                    if let Some((pos, (_, _, ms, _))) = self
                        .run_order
                        .iter()
                        .enumerate()
                        .find(|(_, (nid, _, _, _))| nid == &node.id)
                    {
                        if pos == st_idx {
                            is_step_running = true;
                        } else if pos < st_idx {
                            step_done_ms = Some(*ms);
                        }
                    }
                } else if let Some(latest) = self.runs.first()
                    && let Some(done_step) = latest.steps.iter().find(|s| s.node_id == node.id)
                {
                    step_done_ms = Some(done_step.duration_ms);
                }

                let border_c = if is_step_running {
                    kind_fg
                } else if is_selected {
                    p.blue
                } else {
                    p.line
                };

                let nid = node.id.clone();
                let orig_x = node.x;
                let orig_y = node.y;

                // Chip in top-right of node head
                let head_chip = if is_step_running {
                    div()
                        .h(px(19.))
                        .px(px(6.))
                        .rounded(px(5.))
                        .bg(p.accent_soft)
                        .text_size(px(10.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.blue)
                        .flex()
                        .items_center()
                        .child(tr(cx, "Running", "运行中"))
                } else if let Some(ms) = step_done_ms {
                    div()
                        .h(px(19.))
                        .px(px(6.))
                        .rounded(px(5.))
                        .bg(p.green_soft)
                        .text_size(px(10.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.green_ink)
                        .flex()
                        .items_center()
                        .gap(px(3.))
                        .child(pro_svg("0 0 24 24", SVG_CHECK_SM, 10., p.green_ink))
                        .child(format!("{ms}ms"))
                } else {
                    div()
                        .h(px(19.))
                        .px(px(6.))
                        .rounded(px(5.))
                        .bg(if !node.configured { p.red_soft } else { p.chip })
                        .text_size(px(10.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if !node.configured { p.red } else { p.muted })
                        .flex()
                        .items_center()
                        .child(if !node.configured {
                            tr(cx, "Not configured", "未配置").into()
                        } else {
                            node.chip.clone()
                        })
                };

                // Bottom well content: file tags, agent chips, or mono expression
                let well_bg: Hsla = if p.dark {
                    hsla(0., 0., 1., 0.04)
                } else {
                    hsla(0., 0., 0., 0.03)
                };
                let well_inner: AnyElement = if !node.files.is_empty() {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .children(node.files.iter().map(|ext| {
                            div()
                                .h(px(22.))
                                .px(px(7.))
                                .rounded(px(6.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line)
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .text_size(px(11.))
                                .text_color(p.text)
                                .child(pro_svg("0 0 12 12", SVG_DOC_MINI, 11., p.muted))
                                .child(ext.clone())
                        }))
                        .into_any_element()
                } else if node.kind == WorkflowNodeKind::Agent && !node.tools.is_empty() {
                    let short_model = node
                        .model
                        .split_whitespace()
                        .next()
                        .unwrap_or("Auto")
                        .to_owned();
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .child(
                            div()
                                .h(px(22.))
                                .px(px(6.))
                                .rounded(px(6.))
                                .bg(kind_bg)
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap(px(3.))
                                .text_size(px(11.))
                                .text_color(kind_fg)
                                .child(pro_icon(ProIcon::Sparkles, 10., kind_fg))
                                .child(short_model),
                        )
                        .children(node.tools.iter().take(2).map(|tl| {
                            div()
                                .h(px(22.))
                                .px(px(6.))
                                .rounded(px(6.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line)
                                .flex_none()
                                .flex()
                                .items_center()
                                .text_size(px(11.))
                                .text_color(p.text)
                                .child(tl.clone())
                        }))
                        .into_any_element()
                } else {
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(10.5))
                        .text_color(p.muted)
                        .child(node.expression.clone())
                        .into_any_element()
                };

                // Kind pill floating above top-left of card
                vp = vp.child(
                    div()
                        .absolute()
                        .left(px(sx))
                        .top(px(sy - 20.))
                        .px(px(6.))
                        .py(px(1.5))
                        .rounded(px(5.))
                        .bg(kind_bg)
                        .text_size(px(9.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(kind_fg)
                        .child(node.kind.badge_label(cx)),
                );

                // Main node card
                let mut card_el = div()
                    .id(pro_eid(&self.id, format!("wf-node-{nid}")))
                    .role(Role::Button)
                    .aria_label(format!("{}, {}", node.title, node.kind.badge_label(cx)))
                    .tab_index(0)
                    .absolute()
                    .left(px(sx))
                    .top(px(sy))
                    .w(px(sw))
                    .h(px(sh))
                    .pt(px(10.))
                    .pb(px(6.))
                    .px(px(11.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(border_c)
                    .when(!node.enabled, |d| d.opacity(0.55))
                    .flex()
                    .flex_col()
                    .justify_between()
                    .cursor_pointer()
                    .hover(move |s| {
                        if !is_selected && !is_step_running {
                            s.border_color(p.line_strong)
                        } else {
                            s
                        }
                    })
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                            this.top_menu = None;
                            this.insert_picker = None;
                            this.drag = Some(CanvasDrag::Node {
                                node_id: nid.clone(),
                                start_mouse: ev.position,
                                orig_x,
                                orig_y,
                                moved: false,
                            });
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .h(px(24.))
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(Self::render_node_tile(
                                node,
                                22.,
                                is_step_running,
                                boot_ms,
                                &p,
                                cx,
                            ))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(node.title.clone()),
                            )
                            .child(head_chip),
                    )
                    .child(
                        div()
                            .pl(px(30.))
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.))
                            .text_color(if !node.configured { p.red } else { p.text })
                            .child(node.sub.clone()),
                    )
                    .child(
                        div()
                            .h(px(32.))
                            .px(px(8.))
                            .rounded(px(7.))
                            .bg(well_bg)
                            .flex()
                            .items_center()
                            .overflow_hidden()
                            .child(well_inner),
                    );

                // Input port dot (non-trigger nodes)
                if node.kind != WorkflowNodeKind::Trigger {
                    card_el = card_el.child(
                        div()
                            .absolute()
                            .top(px(-4.))
                            .left(px(sw * 0.5 - 4.))
                            .size(px(8.))
                            .rounded_full()
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line_strong),
                    );
                }

                // Output port dot(s)
                let ports = if node.node_type.as_ref() == "branch" {
                    vec![WorkflowPort::Yes, WorkflowPort::No]
                } else if node.node_type.as_ref() == "end" {
                    Vec::new()
                } else {
                    vec![WorkflowPort::Out]
                };
                for port in &ports {
                    let px_off = port.offset_x() * self.zoom - 4.;
                    card_el = card_el.child(
                        div()
                            .absolute()
                            .bottom(px(-4.))
                            .left(px(px_off))
                            .size(px(8.))
                            .rounded_full()
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line_strong),
                    );
                }

                vp = vp.child(card_el);

                // Accent blue `+` insertion button below unconnected output ports
                for port in ports {
                    if !self.has_outgoing_port(&node.id, port) {
                        let add_x = sx + port.offset_x() * self.zoom - 8.;
                        let add_y = sy + sh + 8.;
                        let after_nid = node.id.clone();
                        let add_aria = format!("Add node after {}", node.title);
                        vp = vp.child(
                            div()
                                .absolute()
                                .left(px(add_x))
                                .top(px(add_y))
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .child(activate(
                                    div()
                                        .id(pro_eid(
                                            &self.id,
                                            format!("add-after-{}-{port:?}", node.id),
                                        ))
                                        .role(Role::Button)
                                        .aria_label(add_aria)
                                        .tab_index(0)
                                        .size(px(16.))
                                        .rounded_full()
                                        .bg(p.blue)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .hover(|s| s.opacity(0.88))
                                        .child(pro_icon(ProIcon::Plus, 10., rgb(0xffffff).into())),
                                    cx,
                                    move |this, w, cx| {
                                        this.insert_picker = Some(InsertPickerState {
                                            after_node_id: after_nid.clone(),
                                            port,
                                        });
                                        if let Some(ed) = this.picker_editor.clone() {
                                            ed.update(cx, |e, cx| {
                                                e.set_value("", w, cx);
                                                e.focus(w, cx);
                                            });
                                        }
                                        cx.notify();
                                    },
                                )),
                        );
                    }
                }
            }

            // Render comment pins & open comment popover
            for cmt in &self.comments {
                let cx_px = self.pan_x + cmt.x * self.zoom;
                let cy_px = self.pan_y + cmt.y * self.zoom;
                let cid = cmt.id.clone();
                let cid_del = cmt.id.clone();
                let is_open = self.open_comment_id.as_ref() == Some(&cid);

                vp = vp.child(
                    div()
                        .absolute()
                        .left(px(cx_px))
                        .top(px(cy_px - 28.))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, format!("cmt-pin-{cid}")))
                                .role(Role::Button)
                                .tab_index(0)
                                .size(px(26.))
                                .rounded_full()
                                .bg(p.card)
                                .border_1()
                                .border_color(p.blue)
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .child(pro_person_avatar(&cmt.author, 20., false, None, cx)),
                            cx,
                            move |this, w, cx| {
                                if this.open_comment_id.as_ref() == Some(&cid) {
                                    this.open_comment_id = None;
                                } else {
                                    this.open_comment_id = Some(cid.clone());
                                    if let Some(txt) = this
                                        .comments
                                        .iter()
                                        .find(|c| c.id == cid)
                                        .map(|c| c.text.to_string())
                                        && let Some(ed) = this.comment_editor.clone()
                                    {
                                        ed.update(cx, |e, cx| e.set_value(txt, w, cx));
                                    }
                                }
                                cx.notify();
                            },
                        ))
                        .when(is_open, |d| {
                            d.child(
                                div()
                                    .occlude()
                                    .mt(px(4.))
                                    .w(px(232.))
                                    .rounded(px(10.))
                                    .bg(p.card)
                                    .border_1()
                                    .border_color(p.line_strong)
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .h(px(34.))
                                            .px(px(8.))
                                            .border_b_1()
                                            .border_color(p.line)
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_size(px(12.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.strong)
                                                    .child(cmt.author.clone()),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(2.))
                                                    .child(pro_icon_btn(
                                                        pro_eid(&self.id, "cmt-resolve"),
                                                        tr(cx, "Resolve", "解决"),
                                                        ProIcon::Trash,
                                                        24.,
                                                        false,
                                                        cx,
                                                        move |this, _, cx| {
                                                            this.comments
                                                                .retain(|c| c.id != cid_del);
                                                            this.open_comment_id = None;
                                                            cx.notify();
                                                        },
                                                    ))
                                                    .child(pro_icon_btn(
                                                        pro_eid(&self.id, "cmt-close"),
                                                        tr(cx, "Close", "关闭"),
                                                        ProIcon::Close,
                                                        24.,
                                                        false,
                                                        cx,
                                                        |this, _, cx| {
                                                            this.open_comment_id = None;
                                                            cx.notify();
                                                        },
                                                    )),
                                            ),
                                    )
                                    .when_some(self.comment_editor.clone(), |el, ed| {
                                        el.child(div().min_h(px(56.)).p(px(8.)).child(ed))
                                    }),
                            )
                        }),
                );
            }
        }

        // Overlays inside canvas area: top path bar, left palette, `+` picker, bottom toolbar, toast, right inspector
        vp = vp.child(self.render_top_overlay(compact, cx));

        {
            let pal = self.render_palette_panel(compact, cx).shadow_md();
            vp = vp.child(pro_presence(
                pro_eid(&self.id, "wf-pal-pres"),
                self.palette_open,
                0.22,
                0.15,
                move |a| {
                    let styled = if compact {
                        pal.bottom(px(54. - 12. * (1. - a))).opacity(a)
                    } else {
                        pal.left(px(10. - 14. * (1. - a))).opacity(a)
                    };
                    styled.into_any_element()
                },
            ));
        }

        if let Some(ref picker) = self
            .insert_picker
            .clone()
            .or_else(|| self.last_picker.clone())
        {
            let picker_open = self.insert_picker.is_some();
            let picker_el = self.render_insert_picker(picker, cx).shadow_md();
            let base_top = self
                .nodes
                .iter()
                .find(|n| n.id == picker.after_node_id)
                .map(|n| (self.pan_y + (n.y + NODE_H + 28.) * self.zoom).max(52.))
                .unwrap_or(220.);
            vp = vp.child(pro_presence(
                pro_eid(&self.id, "wf-picker-pres"),
                picker_open,
                0.18,
                0.12,
                move |a| {
                    picker_el
                        .top(px(base_top - 5. * (1. - a)))
                        .opacity(a)
                        .into_any_element()
                },
            ));
        }

        vp = vp.child(self.render_bottom_toolbar(compact, w, cx));
        vp = vp.child(pro_toast_banner(self.toast.clone()));

        let width_probe = self.measured_width.clone();
        let ruler = canvas(
            move |bounds, _, _| width_probe.set(f32::from(bounds.size.width)),
            |_, _, _, _| {},
        )
        .w_full()
        .h(px(0.));

        let mut root = div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key))
            .on_mouse_move(
                cx.listener(|this, ev: &MouseMoveEvent, _, cx| match &mut this.drag {
                    Some(CanvasDrag::Pan {
                        start_mouse,
                        orig_pan_x,
                        orig_pan_y,
                    }) => {
                        this.pan_x = *orig_pan_x + f32::from(ev.position.x - start_mouse.x);
                        this.pan_y = *orig_pan_y + f32::from(ev.position.y - start_mouse.y);
                        cx.notify();
                    }
                    Some(CanvasDrag::Node {
                        node_id,
                        start_mouse,
                        orig_x,
                        orig_y,
                        moved,
                    }) => {
                        let dx = f32::from(ev.position.x - start_mouse.x);
                        let dy = f32::from(ev.position.y - start_mouse.y);
                        if *moved || dx.hypot(dy) > 4.0 {
                            *moved = true;
                            let nx = *orig_x + dx / this.zoom;
                            let ny = *orig_y + dy / this.zoom;
                            let target_id = node_id.clone();
                            if let Some(n) = this.nodes.iter_mut().find(|n| n.id == target_id) {
                                n.x = nx;
                                n.y = ny;
                            }
                            cx.notify();
                        }
                    }
                    None => {}
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if let Some(drag) = this.drag.take() {
                        if let CanvasDrag::Node { node_id, moved, .. } = drag
                            && !moved
                        {
                            this.select_node(Some(node_id), cx);
                        } else {
                            cx.notify();
                        }
                    }
                }),
            )
            .relative()
            .size_full()
            .min_h_0()
            .bg(p.panel)
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(ruler)
            .child(vp);

        {
            let run_drawer = self.render_run_drawer(run_elapsed_ms, cx);
            root = root.child(pro_fold(
                pro_eid(&self.id, "wf-runs-fold"),
                self.run_panel_open,
                220.,
                0.,
                move || run_drawer.into_any_element(),
            ));
        }

        if let Some(insp_id) = self
            .selected_node_id
            .clone()
            .or_else(|| self.last_inspected_id.clone())
            && self.nodes.iter().any(|n| n.id == insp_id)
        {
            let insp_open = self.selected_node_id.is_some();
            let insp = self.render_node_inspector(&insp_id, boot_ms, compact, w, cx);
            root = root.child(pro_presence(
                pro_eid(&self.id, "wf-insp-pres"),
                insp_open,
                0.22,
                0.15,
                move |a| {
                    let styled = if compact {
                        insp.bottom(px(8. - 12. * (1. - a))).opacity(a)
                    } else {
                        insp.right(px(10. - 14. * (1. - a))).opacity(a)
                    };
                    styled.into_any_element()
                },
            ));
        }

        root
    }
}

impl Render for WorkflowBuilderScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

fn sample_palette() -> Vec<WorkflowPaletteGroup> {
    vec![
        WorkflowPaletteGroup {
            id: "triggers".into(),
            label: "Triggers".into(),
            items: vec![
                WorkflowPaletteItem {
                    node_type: "new-signup".into(),
                    label: "New signup".into(),
                    kind: WorkflowNodeKind::Trigger,
                    sub: "When a user signs up".into(),
                    default_expr: "auth.user_created · Sign-up form".into(),
                },
                WorkflowPaletteItem {
                    node_type: "document-upload".into(),
                    label: "Document upload".into(),
                    kind: WorkflowNodeKind::Trigger,
                    sub: "When a file is uploaded".into(),
                    default_expr: "drive.file_uploaded · .pdf, .docx".into(),
                },
                WorkflowPaletteItem {
                    node_type: "schedule".into(),
                    label: "Schedule".into(),
                    kind: WorkflowNodeKind::Trigger,
                    sub: "Every weekday at 09:00 UTC".into(),
                    default_expr: "cron(0 9 * * 1-5)".into(),
                },
                WorkflowPaletteItem {
                    node_type: "webhook".into(),
                    label: "Webhook".into(),
                    kind: WorkflowNodeKind::Trigger,
                    sub: "Incoming signed POST event".into(),
                    default_expr: "POST /hooks/customer-events".into(),
                },
            ],
        },
        WorkflowPaletteGroup {
            id: "tools".into(),
            label: "Tools".into(),
            items: vec![
                WorkflowPaletteItem {
                    node_type: "ai-agent".into(),
                    label: "AI Agent".into(),
                    kind: WorkflowNodeKind::Agent,
                    sub: "Classifies and summarizes input".into(),
                    default_expr: "Classify customer tier and company size from signup context."
                        .into(),
                },
                WorkflowPaletteItem {
                    node_type: "http-request".into(),
                    label: "HTTP request".into(),
                    kind: WorkflowNodeKind::Tool,
                    sub: "Fetch JSON from REST endpoint".into(),
                    default_expr: "GET https://api.clearbit.com/v2/companies/find".into(),
                },
                WorkflowPaletteItem {
                    node_type: "calculation".into(),
                    label: "Calculation".into(),
                    kind: WorkflowNodeKind::Tool,
                    sub: "Compute derived score".into(),
                    default_expr: "seats * 24 + usage_tier".into(),
                },
                WorkflowPaletteItem {
                    node_type: "file-parser".into(),
                    label: "File parser".into(),
                    kind: WorkflowNodeKind::Tool,
                    sub: "Extract structured sections".into(),
                    default_expr: "parse_document(file.url, format='markdown')".into(),
                },
                WorkflowPaletteItem {
                    node_type: "validation".into(),
                    label: "Validation".into(),
                    kind: WorkflowNodeKind::Tool,
                    sub: "Verify required schema fields".into(),
                    default_expr: "require(user.email, company.domain)".into(),
                },
            ],
        },
        WorkflowPaletteGroup {
            id: "logic".into(),
            label: "Logic".into(),
            items: vec![
                WorkflowPaletteItem {
                    node_type: "branch".into(),
                    label: "Branch".into(),
                    kind: WorkflowNodeKind::Logic,
                    sub: "plan = Pro".into(),
                    default_expr: "user.plan == \"Pro\"".into(),
                },
                WorkflowPaletteItem {
                    node_type: "merge".into(),
                    label: "Merge".into(),
                    kind: WorkflowNodeKind::Logic,
                    sub: "Combine branch outputs".into(),
                    default_expr: "await_any(yes, no)".into(),
                },
                WorkflowPaletteItem {
                    node_type: "wait".into(),
                    label: "Wait".into(),
                    kind: WorkflowNodeKind::Logic,
                    sub: "Pause for 15 minutes".into(),
                    default_expr: "sleep(15m)".into(),
                },
                WorkflowPaletteItem {
                    node_type: "loop".into(),
                    label: "Loop".into(),
                    kind: WorkflowNodeKind::Logic,
                    sub: "For each item in batch".into(),
                    default_expr: "for item in payload.records".into(),
                },
            ],
        },
        WorkflowPaletteGroup {
            id: "actions".into(),
            label: "Actions".into(),
            items: vec![
                WorkflowPaletteItem {
                    node_type: "database".into(),
                    label: "Database".into(),
                    kind: WorkflowNodeKind::Action,
                    sub: "Insert customer row".into(),
                    default_expr: "customers · insert".into(),
                },
                WorkflowPaletteItem {
                    node_type: "notification".into(),
                    label: "Notification".into(),
                    kind: WorkflowNodeKind::Action,
                    sub: "#sales on Slack".into(),
                    default_expr: "New Pro signup: {{user.name}} from {{company.name}}".into(),
                },
                WorkflowPaletteItem {
                    node_type: "send-email".into(),
                    label: "Send email".into(),
                    kind: WorkflowNodeKind::Action,
                    sub: "Welcome onboarding email".into(),
                    default_expr: "to: {{user.email}} · template: welcome-pro".into(),
                },
                WorkflowPaletteItem {
                    node_type: "webhook-call".into(),
                    label: "Webhook call".into(),
                    kind: WorkflowNodeKind::Action,
                    sub: "Dispatch signed event".into(),
                    default_expr: "POST https://hooks.northwind.co/onboarding".into(),
                },
                WorkflowPaletteItem {
                    node_type: "end".into(),
                    label: "End".into(),
                    kind: WorkflowNodeKind::Action,
                    sub: "Return workflow output".into(),
                    default_expr: "return { onboarded: true }".into(),
                },
            ],
        },
    ]
}

fn sample_nodes() -> Vec<WorkflowNode> {
    let mut n1 = WorkflowNode::new(
        "n1",
        "new-signup",
        WorkflowNodeKind::Trigger,
        "New signup",
        0.,
        0.,
    );
    n1.sub = "When a user signs up".into();
    n1.chip = "Ready".into();
    n1.expression = "auth.user_created · Sign-up form".into();
    n1.sample_output = "{\n  \"event\": \"auth.user_created\",\n  \"user\": { \"name\": \"Elena Rostova\", \"domain\": \"linear.app\" },\n  \"plan\": \"Pro\"\n}".into();

    let mut n2 = WorkflowNode::new(
        "n2",
        "document-upload",
        WorkflowNodeKind::Trigger,
        "Document upload",
        320.,
        0.,
    );
    n2.configured = false;
    n2.chip = "Not configured".into();
    n2.sub = "Choose where events come from".into();
    n2.files = vec![".pdf".into(), ".docx".into()];
    n2.expression = "drive.file_uploaded · .pdf, .docx".into();
    n2.sample_output =
        "{\n  \"warning\": \"Select an upload bucket or Google Drive folder\"\n}".into();

    let mut n3 = WorkflowNode::new(
        "n3",
        "ai-agent",
        WorkflowNodeKind::Agent,
        "Classify customer",
        0.,
        150.,
    );
    n3.sub = "Classifies plan and size".into();
    n3.chip = "2 tools".into();
    n3.tools = vec!["CRM lookup".into(), "Web search".into()];
    n3.expression =
        "Classify {{user.domain}} into segment (Startup, Growth, Enterprise) and verify plan tier."
            .into();
    n3.sample_output = "{\n  \"segment\": \"Growth\",\n  \"seats_estimate\": 65,\n  \"plan\": \"Pro\",\n  \"confidence\": 0.96\n}".into();

    let mut n4 = WorkflowNode::new(
        "n4",
        "http-request",
        WorkflowNodeKind::Tool,
        "Enrich company",
        0.,
        300.,
    );
    n4.sub = "Looks the company up by its domain".into();
    n4.chip = "GET".into();
    n4.expression = "GET api.clearbit.com/v2/companies/find?domain={{user.domain}}".into();
    n4.sample_output =
        "{\n  \"company\": { \"name\": \"Linear\", \"employees\": 80, \"country\": \"US\" }\n}"
            .into();

    let mut n5 = WorkflowNode::new("n5", "branch", WorkflowNodeKind::Logic, "Branch", 0., 450.);
    n5.sub = "plan = Pro".into();
    n5.chip = "2 paths".into();
    n5.expression = "Yes → Notify team · No → Create record".into();
    n5.sample_output = "{\n  \"matched\": true,\n  \"port\": \"yes\"\n}".into();

    let mut n6 = WorkflowNode::new(
        "n6",
        "notification",
        WorkflowNodeKind::Action,
        "Notify team",
        -160.,
        600.,
    );
    n6.sub = "#sales on Slack".into();
    n6.chip = "Ready".into();
    n6.app = Some("slack".into());
    n6.account = "Slack · northwind-prod".into();
    n6.expression = "New Pro signup: {{user.name}} from {{company.name}}".into();
    n6.sample_output =
        "{\n  \"channel\": \"#sales\",\n  \"ts\": \"1732349102.0042\",\n  \"delivered\": true\n}"
            .into();

    let mut n7 = WorkflowNode::new(
        "n7",
        "database",
        WorkflowNodeKind::Action,
        "Create record",
        160.,
        600.,
    );
    n7.sub = "Adds the customer".into();
    n7.chip = "Ready".into();
    n7.account = "Postgres · primary-db".into();
    n7.expression = "customers · insert".into();
    n7.sample_output =
        "{\n  \"table\": \"customers\",\n  \"inserted_id\": \"cus_94f81a\"\n}".into();

    vec![n1, n2, n3, n4, n5, n6, n7]
}

fn sample_edges() -> Vec<WorkflowEdge> {
    vec![
        WorkflowEdge::new("e1", "n1", WorkflowPort::Out, "n3"),
        WorkflowEdge::new("e2", "n3", WorkflowPort::Out, "n4"),
        WorkflowEdge::new("e3", "n4", WorkflowPort::Out, "n5"),
        WorkflowEdge::new("e4", "n5", WorkflowPort::Yes, "n6"),
        WorkflowEdge::new("e5", "n5", WorkflowPort::No, "n7"),
    ]
}

fn compute_execution_plan(
    nodes: &[WorkflowNode],
    edges: &[WorkflowEdge],
) -> Vec<(SharedString, Option<SharedString>, u64, u32)> {
    if nodes.is_empty() {
        return Vec::new();
    }
    let start = nodes
        .iter()
        .find(|n| n.kind == WorkflowNodeKind::Trigger && n.configured && n.enabled)
        .or_else(|| nodes.iter().find(|n| n.enabled));
    let Some(first) = start else {
        return Vec::new();
    };

    let mut plan = Vec::new();
    let mut visited = Vec::new();
    let mut cur_id = first.id.clone();
    let mut incoming_edge: Option<SharedString> = None;

    while !visited.contains(&cur_id) && visited.len() < 32 {
        visited.push(cur_id.clone());
        let Some(node) = nodes.iter().find(|n| n.id == cur_id) else {
            break;
        };
        if node.enabled {
            let (ms, credits) = match node.kind {
                WorkflowNodeKind::Trigger => (12, 0),
                WorkflowNodeKind::Agent => (640, 18),
                WorkflowNodeKind::Tool => (184, 2),
                WorkflowNodeKind::Logic => (4, 0),
                WorkflowNodeKind::Action => (92, 1),
            };
            plan.push((cur_id.clone(), incoming_edge.take(), ms, credits));
        }
        let next_edge = edges
            .iter()
            .find(|e| e.from == cur_id && matches!(e.port, WorkflowPort::Out | WorkflowPort::Yes))
            .or_else(|| edges.iter().find(|e| e.from == cur_id));
        if let Some(edge) = next_edge {
            incoming_edge = Some(edge.id.clone());
            cur_id = edge.to.clone();
        } else {
            break;
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::{
        WorkflowNodeKind, WorkflowPort, compute_execution_plan, sample_edges, sample_nodes,
        sample_palette,
    };

    #[test]
    fn sample_workflow_matches_spec_graph_and_palette() {
        let palette = sample_palette();
        assert_eq!(palette.len(), 4);
        let group_counts: Vec<usize> = palette.iter().map(|g| g.items.len()).collect();
        assert_eq!(group_counts, vec![4, 5, 4, 5]);

        let nodes = sample_nodes();
        let edges = sample_edges();
        assert_eq!(nodes.len(), 7);
        assert_eq!(edges.len(), 5);

        let unconfigured: Vec<&str> = nodes
            .iter()
            .filter(|n| !n.configured)
            .map(|n| n.title.as_ref())
            .collect();
        assert_eq!(unconfigured, vec!["Document upload"]);

        let yes_edge = edges.iter().find(|e| e.port == WorkflowPort::Yes);
        let no_edge = edges.iter().find(|e| e.port == WorkflowPort::No);
        assert!(yes_edge.is_some_and(|e| e.from.as_ref() == "n5" && e.to.as_ref() == "n6"));
        assert!(no_edge.is_some_and(|e| e.from.as_ref() == "n5" && e.to.as_ref() == "n7"));
    }

    #[test]
    fn execution_plan_traces_configured_trigger_and_branch_routing() {
        let mut nodes = sample_nodes();
        let edges = sample_edges();

        let plan = compute_execution_plan(&nodes, &edges);
        let visited_ids: Vec<&str> = plan.iter().map(|(id, _, _, _)| id.as_ref()).collect();
        assert_eq!(visited_ids, vec!["n1", "n3", "n4", "n5", "n6"]);

        let total_ms: u64 = plan.iter().map(|(_, _, ms, _)| *ms).sum();
        let total_credits: u32 = plan.iter().map(|(_, _, _, c)| *c).sum();
        assert_eq!(total_ms, 932);
        assert_eq!(total_credits, 21);

        // Disabling the enrich step skips it in execution while keeping downstream routing intact.
        nodes
            .iter_mut()
            .find(|n| n.id.as_ref() == "n4")
            .expect("n4 exists")
            .enabled = false;
        let skipped_plan = compute_execution_plan(&nodes, &edges);
        let skipped: Vec<&str> = skipped_plan
            .iter()
            .map(|(id, _, _, _)| id.as_ref())
            .collect();
        assert_eq!(skipped, vec!["n1", "n3", "n5", "n6"]);

        assert!(WorkflowPort::Yes.offset_x() < WorkflowPort::Out.offset_x());
        assert!(WorkflowPort::Out.offset_x() < WorkflowPort::No.offset_x());
        assert_eq!(WorkflowNodeKind::default(), WorkflowNodeKind::Trigger);
    }
}
