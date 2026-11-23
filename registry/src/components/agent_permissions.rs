//! Autonomous agent permission presets, per-app capability rules, spending limits, and red lines.
//!
//! Lets workspace owners set an overall autonomy posture (`Careful`, `Balanced`,
//! `Hands-off`), inspect connected apps (`Gmail`, `Slack`, `GitHub`, `Google Drive`,
//! `Stripe`, `Google Calendar`, `Web`), adjust per-action capability levels and
//! spending caps, and enforce locked red lines that never run unprompted.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset},
        pro_chrome::{
            ProPalette, pro_app_tile, pro_ease, pro_eid, pro_empty_state, pro_fold,
            pro_mark_selected, pro_pill_indicator, pro_presence, pro_selection_sensor,
            pro_skeleton_bar, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashSet, rc::Rc};
use web_time::Instant;

const BAR_TRANSITION_MS: f32 = 520.;

const SVG_CAREFUL: &str = r##"<path d="M18.7088 3.49534C16.8165 2.55382 14.5009 2 12 2C9.4991 2 7.1835 2.55382 5.29116 3.49534C4.36318 3.95706 3.89919 4.18792 3.4496 4.91378C3 5.63965 3 6.34248 3 7.74814V11.2371C3 16.9205 7.54236 20.0804 10.173 21.4338C10.9067 21.8113 11.2735 22 12 22C12.7265 22 13.0933 21.8113 13.8269 21.4338C16.4576 20.0804 21 16.9205 21 11.2371L21 7.74814C21 6.34249 21 5.63966 20.5504 4.91378C20.1008 4.18791 19.6368 3.95706 18.7088 3.49534Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M9 11.5C9 11.5 10.4079 11.7519 11 13.5C11 13.5 12.5 10.5 15 9.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"##;

const SVG_BALANCED: &str = r##"<circle cx="12" cy="5" r="2" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></circle><path d="M10 5H4M14 5H20" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M17 21H7" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M12 7V21" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M16 14L18.5 8H19.5L22 14C22 15.6569 20.6569 17 19 17C17.3431 17 16 15.6569 16 14ZM22 14H16" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M2 14L4.5 8H5.5L8 14C8 15.6569 6.65685 17 5 17C3.34315 17 2 15.6569 2 14ZM8 14H2" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"##;

const SVG_REFUND: &str = r##"<path d="M14.4998 14.001C14.4998 15.3817 13.3805 16.501 11.9998 16.501C10.619 16.501 9.49976 15.3817 9.49976 14.001C9.49976 12.6203 10.619 11.501 11.9998 11.501C13.3805 11.501 14.4998 12.6203 14.4998 14.001Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M17 7.00293C18.9452 7.08767 20.3228 7.40675 21.1329 7.6578C21.6756 7.82599 22 8.34037 22 8.90853V18.6833C22 19.7984 20.7719 20.6375 19.6762 20.4305C18.7361 20.253 17.5107 20.1104 16 20.1104C11.2491 20.1104 10.1096 21.9161 3.1448 20.3802C2.47265 20.232 2 19.6275 2 18.9392V8.92217C2 7.94632 2.92079 7.23467 3.87798 7.42461C5.31598 7.70997 6.49012 7.8435 7.5 7.87953" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M2 11.001C3.95133 11.001 5.70483 9.40605 5.92901 7.75514M18.5005 7.50098C18.5005 9.54062 20.2655 11.47 22 11.47M22 17.001C20.1009 17.001 18.2601 18.3112 18.102 20.0993M6.00049 20.4971C6.00049 18.2879 4.20963 16.4971 2.00049 16.4971" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M12 8.50098L12 3.00098M12 8.50098C12.7002 8.50098 14.5 6.00098 14.5 6.00098M12 8.50098C11.2998 8.50098 9.5 6.00098 9.5 6.00098" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"##;

const SVG_PRICE_TAG: &str = r##"<circle cx="17.5" cy="6.5" r="1.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></circle><path d="M2.77423 11.1439C1.77108 12.2643 1.7495 13.9546 2.67016 15.1437C4.49711 17.5033 6.49674 19.5029 8.85633 21.3298C10.0454 22.2505 11.7357 22.2289 12.8561 21.2258C15.8979 18.5022 18.6835 15.6559 21.3719 12.5279C21.6377 12.2187 21.8039 11.8397 21.8412 11.4336C22.0062 9.63798 22.3452 4.46467 20.9403 3.05974C19.5353 1.65481 14.362 1.99377 12.5664 2.15876C12.1603 2.19608 11.7813 2.36233 11.472 2.62811C8.34412 5.31646 5.49781 8.10211 2.77423 11.1439Z" stroke="currentColor" stroke-width="1.9"></path><path d="M7.00002 14.0002L10 17.0002" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"##;

/// Top-level autonomy posture applied across unlocked [`PermissionRule`]s.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutonomyPreset {
    /// Reads on its own, asks before changing anything (`Careful`).
    Cautious,
    /// Reads and routine writes on its own, asks before risky actions (`Balanced`).
    #[default]
    Balanced,
    /// Everything on its own except locked red lines (`Hands-off`).
    HandsOff,
}

impl AutonomyPreset {
    fn all() -> [Self; 3] {
        [Self::Cautious, Self::Balanced, Self::HandsOff]
    }
}

/// Three-state autonomy level assigned to a [`PermissionRule`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PermissionLevel {
    /// Agent executes without prompting (`"On its own"`).
    #[default]
    Allow,
    /// Agent pauses for approval in the Inbox (`"Asks first"`).
    Ask,
    /// Agent never performs this action (`"Never"`).
    Never,
}

impl PermissionLevel {
    /// Human-readable pill label matching the reference screen (`On its own`, `Asks first`, `Never`).
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Allow => tr(cx, "On its own", "自主执行"),
            Self::Ask => tr(cx, "Asks first", "先询问"),
            Self::Never => tr(cx, "Never", "从不"),
        }
    }
}

/// Action impact tier used by [`AutonomyPreset`] mapping and inspector grouping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PermissionRuleKind {
    /// Read-only inspection (`Reads`).
    #[default]
    Read,
    /// Routine workspace updates (`Changes`).
    Write,
    /// High-impact, external, or irreversible actions (`Needs care`).
    Risky,
}

impl PermissionRuleKind {
    fn heading(self, cx: &App) -> &'static str {
        match self {
            Self::Read => tr(cx, "Reads", "读取"),
            Self::Write => tr(cx, "Changes", "变更"),
            Self::Risky => tr(cx, "Needs care", "高风险"),
        }
    }
}

/// Line glyph displayed beside an individual [`PermissionRule`] in the app inspector.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PermissionActionIcon {
    /// Read/view glyph (`Eye`).
    #[default]
    Eye,
    /// Refund banknote glyph.
    Refund,
    /// Pricing tag glyph.
    PriceTag,
    /// Email envelope glyph.
    Mail,
    /// Conversation/message glyph.
    Thread,
    /// Delete/trash glyph.
    Trash,
    /// Source code glyph.
    Code,
    /// Branch/merge glyph.
    GitBranch,
    /// Folder/file glyph.
    Folder,
    /// External share/link glyph.
    Link,
    /// Calendar event glyph.
    Calendar,
    /// Web browser globe glyph.
    Globe,
    /// Card/purchase glyph.
    CreditCard,
    /// Security/deploy shield glyph.
    Shield,
}

impl PermissionActionIcon {
    fn render(self, side: f32, color: Hsla) -> AnyElement {
        match self {
            Self::Eye => pro_icon(ProIcon::Eye, side, color),
            Self::Refund => pro_svg("0 0 24 24", SVG_REFUND, side, color),
            Self::PriceTag => pro_svg("0 0 24 24", SVG_PRICE_TAG, side, color),
            Self::Mail => pro_icon(ProIcon::Mail, side, color),
            Self::Thread => pro_icon(ProIcon::Thread, side, color),
            Self::Trash => pro_icon(ProIcon::Trash, side, color),
            Self::Code => pro_icon(ProIcon::Code, side, color),
            Self::GitBranch => pro_icon(ProIcon::GitBranch, side, color),
            Self::Folder => pro_icon(ProIcon::Folder, side, color),
            Self::Link => pro_icon(ProIcon::Link, side, color),
            Self::Calendar => pro_icon(ProIcon::Calendar, side, color),
            Self::Globe => pro_icon(ProIcon::Globe, side, color),
            Self::CreditCard => pro_icon(ProIcon::CreditCard, side, color),
            Self::Shield => pro_icon(ProIcon::Shield, side, color),
        }
    }
}

/// Monetary or quantitative guardrail cap (`On its own up to <value> <unit> asks above`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PermissionLimit {
    /// Numeric cap below which the agent acts autonomously.
    pub value: u64,
    /// Currency or unit suffix (for example `"€"` or `"$"`).
    pub unit: SharedString,
}

impl PermissionLimit {
    /// Construct a guardrail cap with `value` and `unit`.
    pub fn new(value: u64, unit: impl Into<SharedString>) -> Self {
        Self {
            value,
            unit: unit.into(),
        }
    }
}

/// Outcome status of a recent permission check under `Lately`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PermissionEventOutcome {
    /// Completed autonomously (`Done`).
    #[default]
    Done,
    /// Approved by the user after asking (`Approved`).
    Approved,
    /// Declined by the user (`Declined`).
    Declined,
    /// Stopped by a `Never` rule (`Blocked`).
    Blocked,
}

impl PermissionEventOutcome {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Done => tr(cx, "Done", "已完成"),
            Self::Approved => tr(cx, "Approved", "已批准"),
            Self::Declined => tr(cx, "Declined", "已拒绝"),
            Self::Blocked => tr(cx, "Blocked", "已阻止"),
        }
    }
}

/// Recent audit entry shown under `Lately` inside an app's inspector panel.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PermissionRecentEvent {
    /// Human-readable description of the attempted or completed action.
    pub what: SharedString,
    /// Rule action name associated with this event.
    pub action: SharedString,
    /// Resolution outcome (`Done`, `Approved`, `Declined`, or `Blocked`).
    pub outcome: PermissionEventOutcome,
    /// Relative timestamp (for example `"2 hours ago"` or `"yesterday"`).
    pub at: SharedString,
}

impl PermissionRecentEvent {
    /// Create a recent event record.
    pub fn new(
        what: impl Into<SharedString>,
        action: impl Into<SharedString>,
        outcome: PermissionEventOutcome,
        at: impl Into<SharedString>,
    ) -> Self {
        Self {
            what: what.into(),
            action: action.into(),
            outcome,
            at: at.into(),
        }
    }
}

/// Connected integration displayed in the `Apps` list of [`AgentPermissionsScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectedApp {
    /// Stable identifier (for example `"stripe"` or `"gmail"`).
    pub id: SharedString,
    /// Display name (for example `"Stripe"` or `"Gmail"`).
    pub name: SharedString,
    /// Connected account or workspace subtitle (for example `"Fieldwork GmbH"`).
    pub account: Option<SharedString>,
    /// Optional fallback line icon when the app has no brand artwork (such as `Web`).
    pub icon: Option<ProIcon>,
}

impl ConnectedApp {
    /// Construct a connected app with `id` and `name`.
    pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            account: None,
            icon: None,
        }
    }

    /// Attach the connected account or workspace subtitle.
    pub fn with_account(mut self, account: impl Into<SharedString>) -> Self {
        self.account = Some(account.into());
        self
    }

    /// Attach a semantic fallback icon for non-brand tools such as `Web`.
    pub fn with_icon(mut self, icon: ProIcon) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// One capability rule governing an action inside a [`ConnectedApp`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionRule {
    /// Stable rule identifier.
    pub id: SharedString,
    /// Owning [`ConnectedApp::id`].
    pub app: SharedString,
    /// Action title (for example `"Issue refunds"` or `"Delete email"`).
    pub action: SharedString,
    /// Optional clarifying subtitle (for example `"1,284 members"`).
    pub detail: Option<SharedString>,
    /// Leading line glyph inside the app inspector.
    pub icon: PermissionActionIcon,
    /// Current autonomy level (`Allow`, `Ask`, or `Never`).
    pub level: PermissionLevel,
    /// Baseline level restored by `Reset`.
    pub default_level: PermissionLevel,
    /// Impact tier (`Read`, `Write`, or `Risky`).
    pub kind: PermissionRuleKind,
    /// When `true`, appears under `Red lines`, ignores presets, and disallows `On its own`.
    pub locked: bool,
    /// Optional spending or quantity cap when `level` is [`PermissionLevel::Allow`].
    pub limit: Option<PermissionLimit>,
    /// Recent activity entries associated with this rule.
    pub recent: Vec<PermissionRecentEvent>,
}

impl PermissionRule {
    /// Construct a capability rule whose `default_level` matches `level`.
    pub fn new(
        id: impl Into<SharedString>,
        app: impl Into<SharedString>,
        action: impl Into<SharedString>,
        kind: PermissionRuleKind,
        level: PermissionLevel,
    ) -> Self {
        Self {
            id: id.into(),
            app: app.into(),
            action: action.into(),
            detail: None,
            icon: PermissionActionIcon::Eye,
            level,
            default_level: level,
            kind,
            locked: false,
            limit: None,
            recent: Vec::new(),
        }
    }

    /// Set a secondary detail caption on this rule.
    pub fn with_detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Set the action line icon.
    pub fn with_icon(mut self, icon: PermissionActionIcon) -> Self {
        self.icon = icon;
        self
    }

    /// Mark this rule as a locked red line (`locked: true` forces `Allow` down to `Ask`).
    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        if locked && self.level == PermissionLevel::Allow {
            self.level = PermissionLevel::Ask;
            self.default_level = PermissionLevel::Ask;
        }
        self
    }

    /// Attach a spending or numeric cap (`On its own up to <value> <unit> asks above`).
    pub fn with_limit(mut self, value: u64, unit: impl Into<SharedString>) -> Self {
        self.limit = Some(PermissionLimit::new(value, unit));
        self
    }

    /// Attach recent audit outcomes for the `Lately` card.
    pub fn with_recent(mut self, recent: Vec<PermissionRecentEvent>) -> Self {
        self.recent = recent;
        self
    }
}

/// Events emitted by [`AgentPermissionsScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentPermissionsEvent {
    /// A rule's autonomy level or spending limit changed.
    Change(PermissionRule),
    /// User selected one of the top-level autonomy presets.
    Preset(AutonomyPreset),
    /// User opened a connected app in the right-hand inspector.
    SelectApp(SharedString),
    /// User clicked `Connect app` in the empty state.
    Connect,
    /// User navigated via the [`AppShell`] sidebar.
    Navigate(SharedString),
}

impl EventEmitter<AgentPermissionsEvent> for AgentPermissionsScreen {}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MenuTarget {
    Rule {
        id: SharedString,
        in_red_lines: bool,
    },
    Group {
        app: SharedString,
        kind: PermissionRuleKind,
    },
}

/// Default seven connected apps matching the reference `Agent / Permissions` screen.
pub fn default_connected_apps() -> Vec<ConnectedApp> {
    vec![
        ConnectedApp::new("gmail", "Gmail").with_account("alex@fieldwork.co"),
        ConnectedApp::new("slack", "Slack").with_account("Fieldwork workspace"),
        ConnectedApp::new("github", "GitHub").with_account("fieldwork-labs"),
        ConnectedApp::new("drive", "Google Drive").with_account("Shared drive · Clients"),
        ConnectedApp::new("stripe", "Stripe").with_account("Fieldwork GmbH"),
        ConnectedApp::new("calendar", "Google Calendar").with_account("alex@fieldwork.co"),
        ConnectedApp::new("web", "Web")
            .with_account("Browser")
            .with_icon(ProIcon::Globe),
    ]
}

/// Default 26 capability rules (14 `Allow`, 6 `Ask`, 6 `Never`, 7 locked red lines) matching the reference specification.
pub fn default_permission_rules() -> Vec<PermissionRule> {
    vec![
        // 1. Gmail (3 allow · 1 ask · 1 never)
        PermissionRule::new(
            "gmail-read-threads",
            "gmail",
            "Read emails and threads",
            PermissionRuleKind::Read,
            PermissionLevel::Allow,
        )
        .with_icon(PermissionActionIcon::Eye)
        .with_recent(vec![PermissionRecentEvent::new(
            "Summarised 6 replies from Northwind procurement",
            "Read emails and threads",
            PermissionEventOutcome::Done,
            "35 minutes ago",
        )]),
        PermissionRule::new(
            "gmail-search-contacts",
            "gmail",
            "Search contacts and labels",
            PermissionRuleKind::Read,
            PermissionLevel::Allow,
        )
        .with_icon(PermissionActionIcon::Mail),
        PermissionRule::new(
            "gmail-save-drafts",
            "gmail",
            "Create and update drafts",
            PermissionRuleKind::Write,
            PermissionLevel::Allow,
        )
        .with_icon(PermissionActionIcon::Thread)
        .with_recent(vec![PermissionRecentEvent::new(
            "Drafted Monday follow-up to Priya Shah",
            "Create and update drafts",
            PermissionEventOutcome::Done,
            "3 hours ago",
        )]),
        PermissionRule::new(
            "gmail-send-email",
            "gmail",
            "Send email",
            PermissionRuleKind::Risky,
            PermissionLevel::Ask,
        )
        .with_detail("External recipients")
        .with_icon(PermissionActionIcon::Mail)
        .with_recent(vec![PermissionRecentEvent::new(
            "Send Q3 invoice summary to finance@northwind.co",
            "Send email",
            PermissionEventOutcome::Approved,
            "yesterday",
        )]),
        PermissionRule::new(
            "gmail-delete",
            "gmail",
            "Delete email",
            PermissionRuleKind::Risky,
            PermissionLevel::Never,
        )
        .with_icon(PermissionActionIcon::Trash)
        .with_locked(true),
        // 2. Slack (2 allow · 1 ask · 1 never)
        PermissionRule::new(
            "slack-read-channels",
            "slack",
            "Read public channels",
            PermissionRuleKind::Read,
            PermissionLevel::Allow,
        )
        .with_icon(PermissionActionIcon::Eye),
        PermissionRule::new(
            "slack-reply-threads",
            "slack",
            "Reply in threads and DMs",
            PermissionRuleKind::Write,
            PermissionLevel::Allow,
        )
        .with_icon(PermissionActionIcon::Thread)
        .with_recent(vec![PermissionRecentEvent::new(
            "Posted staging test summary in #release-ops",
            "Reply in threads and DMs",
            PermissionEventOutcome::Done,
            "1 hour ago",
        )]),
        PermissionRule::new(
            "slack-manage-channels",
            "slack",
            "Create or archive channels",
            PermissionRuleKind::Risky,
            PermissionLevel::Ask,
        )
        .with_icon(PermissionActionIcon::Folder),
        PermissionRule::new(
            "slack-post-general",
            "slack",
            "Post in #general",
            PermissionRuleKind::Risky,
            PermissionLevel::Never,
        )
        .with_detail("1,284 members")
        .with_icon(PermissionActionIcon::Thread)
        .with_locked(true),
        // 3. GitHub (2 allow · 1 ask · 1 never)
        PermissionRule::new(
            "gh-read-code",
            "github",
            "Read code, issues and pull requests",
            PermissionRuleKind::Read,
            PermissionLevel::Allow,
        )
        .with_icon(PermissionActionIcon::Code),
        PermissionRule::new(
            "gh-open-pr",
            "github",
            "Push branches and open draft PRs",
            PermissionRuleKind::Write,
            PermissionLevel::Allow,
        )
        .with_icon(PermissionActionIcon::GitBranch)
        .with_recent(vec![PermissionRecentEvent::new(
            "Opened draft PR #419 for v1 traffic deprecation notice",
            "Push branches and open draft PRs",
            PermissionEventOutcome::Done,
            "4 hours ago",
        )]),
        PermissionRule::new(
            "gh-merge-main",
            "github",
            "Merge to main",
            PermissionRuleKind::Risky,
            PermissionLevel::Ask,
        )
        .with_icon(PermissionActionIcon::GitBranch)
        .with_locked(true)
        .with_recent(vec![PermissionRecentEvent::new(
            "Merge PR #414 into main after CI green",
            "Merge to main",
            PermissionEventOutcome::Approved,
            "2 days ago",
        )]),
        PermissionRule::new(
            "gh-deploy-prod",
            "github",
            "Deploy to production",
            PermissionRuleKind::Risky,
            PermissionLevel::Never,
        )
        .with_icon(PermissionActionIcon::Shield)
        .with_locked(true),
        // 4. Google Drive (2 allow · 1 ask · 1 never)
        PermissionRule::new(
            "drive-read-docs",
            "drive",
            "Read documents and sheets",
            PermissionRuleKind::Read,
            PermissionLevel::Allow,
        )
        .with_icon(PermissionActionIcon::Eye),
        PermissionRule::new(
            "drive-edit-docs",
            "drive",
            "Create and edit workspace files",
            PermissionRuleKind::Write,
            PermissionLevel::Allow,
        )
        .with_icon(PermissionActionIcon::Folder)
        .with_recent(vec![PermissionRecentEvent::new(
            "Updated Q3 client rollout tracker sheet",
            "Create and edit workspace files",
            PermissionEventOutcome::Done,
            "5 hours ago",
        )]),
        PermissionRule::new(
            "drive-share-external",
            "drive",
            "Share files outside Fieldwork",
            PermissionRuleKind::Risky,
            PermissionLevel::Ask,
        )
        .with_icon(PermissionActionIcon::Link),
        PermissionRule::new(
            "drive-delete-files",
            "drive",
            "Delete files",
            PermissionRuleKind::Risky,
            PermissionLevel::Never,
        )
        .with_icon(PermissionActionIcon::Trash)
        .with_locked(true),
        // 5. Stripe (2 allow · 0 ask · 1 never) — default open app in inspector
        PermissionRule::new(
            "stripe-read-payments",
            "stripe",
            "Read payments",
            PermissionRuleKind::Read,
            PermissionLevel::Allow,
        )
        .with_icon(PermissionActionIcon::Eye),
        PermissionRule::new(
            "stripe-issue-refunds",
            "stripe",
            "Issue refunds",
            PermissionRuleKind::Risky,
            PermissionLevel::Allow,
        )
        .with_icon(PermissionActionIcon::Refund)
        .with_limit(50, "€")
        .with_recent(vec![
            PermissionRecentEvent::new(
                "Refund 38 € to Hannah Kim – charged twice",
                "Issue refunds",
                PermissionEventOutcome::Done,
                "2 hours ago",
            ),
            PermissionRecentEvent::new(
                "Refund 240 € to Lumen Studio",
                "Issue refunds",
                PermissionEventOutcome::Approved,
                "yesterday",
            ),
            PermissionRecentEvent::new(
                "Refund 12 € to Jo Park",
                "Issue refunds",
                PermissionEventOutcome::Done,
                "3 days ago",
            ),
        ]),
        PermissionRule::new(
            "stripe-change-prices",
            "stripe",
            "Change prices",
            PermissionRuleKind::Risky,
            PermissionLevel::Never,
        )
        .with_icon(PermissionActionIcon::PriceTag)
        .with_locked(true),
        // 6. Google Calendar (2 allow · 1 ask · 0 never)
        PermissionRule::new(
            "cal-read-events",
            "calendar",
            "Read calendar and availability",
            PermissionRuleKind::Read,
            PermissionLevel::Allow,
        )
        .with_icon(PermissionActionIcon::Eye),
        PermissionRule::new(
            "cal-hold-blocks",
            "calendar",
            "Hold internal focus blocks",
            PermissionRuleKind::Write,
            PermissionLevel::Allow,
        )
        .with_icon(PermissionActionIcon::Calendar)
        .with_recent(vec![PermissionRecentEvent::new(
            "Held 10:00 CET prep block before Northwind review",
            "Hold internal focus blocks",
            PermissionEventOutcome::Done,
            "yesterday",
        )]),
        PermissionRule::new(
            "cal-external-invites",
            "calendar",
            "Send or move external invites",
            PermissionRuleKind::Risky,
            PermissionLevel::Ask,
        )
        .with_icon(PermissionActionIcon::Mail),
        // 7. Web (1 allow · 1 ask · 1 never)
        PermissionRule::new(
            "web-read-pages",
            "web",
            "Read public pages and search",
            PermissionRuleKind::Read,
            PermissionLevel::Allow,
        )
        .with_icon(PermissionActionIcon::Globe)
        .with_recent(vec![PermissionRecentEvent::new(
            "Checked EU VAT rate changes on ec.europa.eu",
            "Read public pages and search",
            PermissionEventOutcome::Done,
            "6 hours ago",
        )]),
        PermissionRule::new(
            "web-submit-forms",
            "web",
            "Submit forms and sign in",
            PermissionRuleKind::Risky,
            PermissionLevel::Ask,
        )
        .with_icon(PermissionActionIcon::Link),
        PermissionRule::new(
            "web-buy-things",
            "web",
            "Buy things",
            PermissionRuleKind::Risky,
            PermissionLevel::Never,
        )
        .with_detail("With a saved card")
        .with_icon(PermissionActionIcon::CreditCard)
        .with_locked(true),
    ]
}

fn target_level_for_preset(rule: &PermissionRule, preset: AutonomyPreset) -> PermissionLevel {
    if rule.locked {
        return rule.level;
    }
    match preset {
        AutonomyPreset::Cautious => match rule.kind {
            PermissionRuleKind::Read => PermissionLevel::Allow,
            PermissionRuleKind::Write | PermissionRuleKind::Risky => PermissionLevel::Ask,
        },
        AutonomyPreset::Balanced => match rule.kind {
            PermissionRuleKind::Read | PermissionRuleKind::Write => PermissionLevel::Allow,
            PermissionRuleKind::Risky => {
                if rule.limit.is_some() {
                    PermissionLevel::Allow
                } else {
                    PermissionLevel::Ask
                }
            }
        },
        AutonomyPreset::HandsOff => PermissionLevel::Allow,
    }
}

fn apply_preset_rules(rules: &mut [PermissionRule], preset: AutonomyPreset) -> Vec<PermissionRule> {
    let mut changed = Vec::new();
    for r in rules.iter_mut() {
        if r.locked {
            continue;
        }
        let target = target_level_for_preset(r, preset);
        if r.level != target {
            r.level = target;
            changed.push(r.clone());
        }
    }
    changed
}

fn detect_matching_preset(rules: &[PermissionRule]) -> Option<AutonomyPreset> {
    AutonomyPreset::all().into_iter().find(|&preset| {
        rules
            .iter()
            .filter(|r| !r.locked)
            .all(|r| r.level == target_level_for_preset(r, preset))
    })
}

fn count_levels<'a>(rules: impl IntoIterator<Item = &'a PermissionRule>) -> (usize, usize, usize) {
    let mut allow = 0usize;
    let mut ask = 0usize;
    let mut never = 0usize;
    for r in rules {
        match r.level {
            PermissionLevel::Allow => allow += 1,
            PermissionLevel::Ask => ask += 1,
            PermissionLevel::Never => never += 1,
        }
    }
    (allow, ask, never)
}

fn group_uniform_level(rules: &[&PermissionRule]) -> Option<PermissionLevel> {
    let first = rules.first()?.level;
    rules.iter().all(|r| r.level == first).then_some(first)
}

fn render_app_mark(app: &ConnectedApp, side: f32, cx: &App) -> AnyElement {
    let p = ProPalette::get(cx);
    let radius = (side * 0.25).clamp(5., 8.);
    if pro_app_artwork(&app.name, side, cx).is_some() {
        return pro_app_tile(&app.name, side, cx);
    }
    if let Some(ic) = app.icon {
        let tone = if app.id.eq_ignore_ascii_case("web") {
            p.pink
        } else {
            p.blue
        };
        return div()
            .size(px(side))
            .flex_none()
            .rounded(px(radius))
            .bg(tone.opacity(if p.dark { 0.18 } else { 0.13 }))
            .flex()
            .items_center()
            .justify_center()
            .child(pro_icon(ic, (side * 0.58).round().clamp(11., 18.), tone))
            .into_any_element();
    }
    pro_app_tile(&app.name, side, cx)
}

fn render_level_dot(
    level: Option<PermissionLevel>,
    locked: bool,
    size_px: f32,
    p: &ProPalette,
) -> AnyElement {
    if locked && level == Some(PermissionLevel::Never) {
        return pro_icon(ProIcon::Lock, 11., p.faint);
    }
    let never_c = if p.dark {
        hsla(0., 0., 1., 0.25)
    } else {
        hsla(0., 0., 0., 0.22)
    };
    match level {
        Some(PermissionLevel::Allow) => div()
            .size(px(size_px))
            .flex_none()
            .rounded_full()
            .bg(p.blue)
            .into_any_element(),
        Some(PermissionLevel::Ask) => div()
            .size(px(size_px))
            .flex_none()
            .rounded_full()
            .bg(p.amber)
            .into_any_element(),
        Some(PermissionLevel::Never) => div()
            .size(px(size_px))
            .flex_none()
            .rounded_full()
            .bg(never_c)
            .into_any_element(),
        None => {
            // Mixed two-tone dot (left half blue, right half amber)
            div()
                .size(px(size_px))
                .flex_none()
                .rounded_full()
                .overflow_hidden()
                .flex()
                .child(div().w(px(size_px * 0.5)).h_full().bg(p.blue))
                .child(div().w(px(size_px * 0.5)).h_full().bg(p.amber))
                .into_any_element()
        }
    }
}

/// Interactive `Agent / Permissions` governance screen hosted in [`AppShell`].
pub struct AgentPermissionsScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    standalone: bool,
    loading: bool,
    empty: bool,
    apps: Vec<ConnectedApp>,
    rules: Vec<PermissionRule>,
    selected_app: Option<SharedString>,
    last_preset: AutonomyPreset,
    open_menu: Option<MenuTarget>,
    collapsed_kinds: HashSet<SharedString>,
    editing_limit_id: Option<SharedString>,
    limit_editor: Option<Entity<Editor>>,
    search_open: bool,
    search_editor: Option<Entity<Editor>>,
    bar_anim: Option<([f32; 3], [f32; 3], Instant)>,
    focus: FocusHandle,
    measured_width: Rc<Cell<f32>>,
    boot_at: Instant,
}

impl AgentPermissionsScreen {
    /// Construct a new [`AgentPermissionsScreen`] with [`default_connected_apps`], [`default_permission_rules`], and `"stripe"` open on the right.
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
                .with_active("Permissions")
                .with_work_area(true, true)
                .with_panel_width(360.);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_inspector(w, cx));
            s
        });
        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                cx.emit(AgentPermissionsEvent::Navigate(label.clone()));
            }
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
            apps: default_connected_apps(),
            rules: default_permission_rules(),
            selected_app: Some("stripe".into()),
            last_preset: AutonomyPreset::Balanced,
            open_menu: None,
            collapsed_kinds: HashSet::new(),
            editing_limit_id: None,
            limit_editor: None,
            search_open: false,
            search_editor: None,
            bar_anim: None,
            focus: cx.focus_handle(),
            measured_width: Rc::new(Cell::new(920.)),
            boot_at: Instant::now(),
        }
    }

    /// Replace the connected apps list.
    pub fn with_apps(mut self, apps: Vec<ConnectedApp>) -> Self {
        self.apps = apps;
        self
    }

    /// Replace the capability rules list.
    pub fn with_rules(mut self, rules: Vec<PermissionRule>) -> Self {
        if let Some(matched) = detect_matching_preset(&rules) {
            self.last_preset = matched;
        }
        self.rules = rules;
        self
    }

    /// Set the initially open app id in the right inspector (`None` folds the inspector).
    pub fn with_default_app(
        mut self,
        app_id: Option<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let open = app_id.is_some();
        self.selected_app = app_id;
        self.shell.update(cx, |s, cx| s.set_panel_open(open, cx));
        self
    }

    /// Override the workspace name shown in the sidebar.
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

    /// Override the account footer name and email in the sidebar.
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

    /// Render without the surrounding [`AppShell`] frame when `standalone` is `true`.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Configure the skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the empty (no connected apps) state.
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

    fn current_ratios(&self) -> [f32; 3] {
        let (allow, ask, never) = count_levels(self.rules.iter());
        let total = (allow + ask + never).max(1) as f32;
        [
            allow as f32 / total,
            ask as f32 / total,
            never as f32 / total,
        ]
    }

    fn record_bar_transition(&mut self, prev: [f32; 3], cx: &App) {
        let next = self.current_ratios();
        if !Theme::global(cx).reduced_motion
            && ((prev[0] - next[0]).abs() > 0.005
                || (prev[1] - next[1]).abs() > 0.005
                || (prev[2] - next[2]).abs() > 0.005)
        {
            self.bar_anim = Some((prev, next, Instant::now()));
        } else {
            self.bar_anim = None;
        }
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.limit_editor.is_none() {
            let ph: SharedString = "50".into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 12.;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Submitted => {
                    if let Some(rule_id) = this.editing_limit_id.take() {
                        let raw: String = ed_ent
                            .read(cx)
                            .value()
                            .chars()
                            .filter(|c| c.is_ascii_digit())
                            .collect();
                        if let Ok(parsed) = raw.parse::<u64>()
                            && parsed > 0
                            && let Some(rule) = this.rules.iter_mut().find(|r| r.id == rule_id)
                            && let Some(ref mut lim) = rule.limit
                        {
                            lim.value = parsed;
                            let updated = rule.clone();
                            cx.emit(AgentPermissionsEvent::Change(updated));
                        }
                    }
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                EditEvent::Escape => {
                    this.editing_limit_id = None;
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                EditEvent::Changed => cx.notify(),
                _ => {}
            })
            .detach();
            self.limit_editor = Some(ed);
        }

        if self.search_editor.is_none() {
            let ph: SharedString = tr(cx, "Filter apps or actions", "筛选应用或操作").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 12.5;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed | EditEvent::Submitted => cx.notify(),
                EditEvent::Escape => {
                    ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                    this.search_open = false;
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.search_editor = Some(ed);
        }
    }

    fn current_query(&self, cx: &App) -> String {
        self.search_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_ascii_lowercase())
            .unwrap_or_default()
    }

    fn choose_preset(&mut self, preset: AutonomyPreset, cx: &mut Context<Self>) {
        let prev = self.current_ratios();
        self.last_preset = preset;
        self.open_menu = None;
        let changed = apply_preset_rules(&mut self.rules, preset);
        self.record_bar_transition(prev, cx);
        for rule in changed {
            cx.emit(AgentPermissionsEvent::Change(rule));
        }
        cx.emit(AgentPermissionsEvent::Preset(preset));
        cx.notify();
    }

    fn set_rule_level(&mut self, id: &str, level: PermissionLevel, cx: &mut Context<Self>) {
        let prev = self.current_ratios();
        if let Some(rule) = self.rules.iter_mut().find(|r| r.id.as_ref() == id) {
            if rule.locked && level == PermissionLevel::Allow {
                return;
            }
            if rule.level != level {
                rule.level = level;
                let updated = rule.clone();
                cx.emit(AgentPermissionsEvent::Change(updated));
            }
        }
        if let Some(matched) = detect_matching_preset(&self.rules) {
            self.last_preset = matched;
        }
        self.open_menu = None;
        self.record_bar_transition(prev, cx);
        cx.notify();
    }

    fn reset_rule(&mut self, id: &str, cx: &mut Context<Self>) {
        let prev = self.current_ratios();
        if let Some(rule) = self.rules.iter_mut().find(|r| r.id.as_ref() == id)
            && rule.level != rule.default_level
        {
            rule.level = rule.default_level;
            let updated = rule.clone();
            cx.emit(AgentPermissionsEvent::Change(updated));
        }
        if let Some(matched) = detect_matching_preset(&self.rules) {
            self.last_preset = matched;
        }
        self.open_menu = None;
        self.record_bar_transition(prev, cx);
        cx.notify();
    }

    fn set_group_level(
        &mut self,
        app_id: &str,
        kind: PermissionRuleKind,
        level: PermissionLevel,
        cx: &mut Context<Self>,
    ) {
        let prev = self.current_ratios();
        let mut changed = Vec::new();
        for rule in self.rules.iter_mut() {
            if rule.app.as_ref() == app_id && rule.kind == kind {
                if rule.locked && level == PermissionLevel::Allow {
                    continue;
                }
                if rule.level != level {
                    rule.level = level;
                    changed.push(rule.clone());
                }
            }
        }
        if let Some(matched) = detect_matching_preset(&self.rules) {
            self.last_preset = matched;
        }
        self.open_menu = None;
        self.record_bar_transition(prev, cx);
        for r in changed {
            cx.emit(AgentPermissionsEvent::Change(r));
        }
        cx.notify();
    }

    fn select_app(&mut self, app_id: SharedString, cx: &mut Context<Self>) {
        self.selected_app = Some(app_id.clone());
        self.open_menu = None;
        self.editing_limit_id = None;
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        cx.emit(AgentPermissionsEvent::SelectApp(app_id));
        cx.notify();
    }

    fn handle_key(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        if ev.keystroke.modifiers.platform || ev.keystroke.modifiers.control {
            return;
        }
        let key = ev.keystroke.key.as_str();
        match key {
            "/" => {
                self.search_open = true;
                if let Some(ref ed) = self.search_editor {
                    {
                        let fh = ed.read(cx).focus.clone();
                        fh.focus(w, cx);
                    }
                }
                w.prevent_default();
                cx.stop_propagation();
                cx.notify();
            }
            "left" => {
                let active = detect_matching_preset(&self.rules).unwrap_or(self.last_preset);
                let prev = match active {
                    AutonomyPreset::Cautious => AutonomyPreset::Cautious,
                    AutonomyPreset::Balanced => AutonomyPreset::Cautious,
                    AutonomyPreset::HandsOff => AutonomyPreset::Balanced,
                };
                self.choose_preset(prev, cx);
                w.prevent_default();
                cx.stop_propagation();
            }
            "right" => {
                let active = detect_matching_preset(&self.rules).unwrap_or(self.last_preset);
                let next = match active {
                    AutonomyPreset::Cautious => AutonomyPreset::Balanced,
                    AutonomyPreset::Balanced => AutonomyPreset::HandsOff,
                    AutonomyPreset::HandsOff => AutonomyPreset::HandsOff,
                };
                self.choose_preset(next, cx);
                w.prevent_default();
                cx.stop_propagation();
            }
            "j" | "J" | "down" => {
                if !self.apps.is_empty() && !self.empty {
                    let next = self
                        .selected_app
                        .as_ref()
                        .and_then(|cur| self.apps.iter().position(|a| &a.id == cur))
                        .map(|i| (i + 1).min(self.apps.len() - 1))
                        .unwrap_or(0);
                    let id = self.apps[next].id.clone();
                    self.select_app(id, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }
            "k" | "K" | "up" => {
                if !self.apps.is_empty() && !self.empty {
                    let prev = self
                        .selected_app
                        .as_ref()
                        .and_then(|cur| self.apps.iter().position(|a| &a.id == cur))
                        .map(|i| i.saturating_sub(1))
                        .unwrap_or(0);
                    let id = self.apps[prev].id.clone();
                    self.select_app(id, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }
            "escape" => {
                if self.open_menu.is_some() || self.editing_limit_id.is_some() {
                    self.open_menu = None;
                    self.editing_limit_id = None;
                    cx.notify();
                } else if self.search_open {
                    if let Some(ref ed) = self.search_editor {
                        ed.update(cx, |e, cx| e.set_value("", w, cx));
                    }
                    self.search_open = false;
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    fn render_rule_menu(&self, rule: &PermissionRule, suffix: &str, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let rid = rule.id.clone();
        let mut menu = div()
            .mt(px(4.))
            .p(px(4.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_col()
            .gap(px(1.));

        if rule.locked {
            menu = menu.child(
                div()
                    .px(px(8.))
                    .py(px(5.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(11.5))
                    .text_color(p.faint)
                    .child(pro_icon(ProIcon::Lock, 12., p.faint))
                    .child(tr(
                        cx,
                        "Red line – never runs on its own",
                        "红线操作——绝不自主执行",
                    )),
            );
        }

        let options: &[PermissionLevel] = if rule.locked {
            &[PermissionLevel::Ask, PermissionLevel::Never]
        } else {
            &[
                PermissionLevel::Allow,
                PermissionLevel::Ask,
                PermissionLevel::Never,
            ]
        };

        for (oi, &lvl) in options.iter().enumerate() {
            let checked = rule.level == lvl;
            let rid_pick = rid.clone();
            menu = menu.child(activate(
                div()
                    .id(pro_eid(
                        &self.id,
                        format!("rmenu-{suffix}-{}-{oi}", rule.id),
                    ))
                    .role(Role::MenuItem)
                    .tab_index(0)
                    .h(px(30.))
                    .px(px(9.))
                    .rounded(px(7.))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.5))
                    .text_color(if checked { p.ink } else { p.text })
                    .font_weight(if checked {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .cursor_pointer()
                    .child(render_level_dot(Some(lvl), false, 7., &p))
                    .child(div().flex_1().child(lvl.label(cx)))
                    .when(checked, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink))),
                cx,
                move |this, _, cx| this.set_rule_level(&rid_pick, lvl, cx),
            ));
        }

        if rule.level != rule.default_level {
            let rid_reset = rid;
            menu = menu
                .child(div().my(px(3.)).mx(px(6.)).h(px(1.)).bg(p.line))
                .child(activate(
                    div()
                        .id(pro_eid(
                            &self.id,
                            format!("rmenu-reset-{suffix}-{}", rule.id),
                        ))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .h(px(28.))
                        .px(px(9.))
                        .rounded(px(7.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Undo, 12., p.faint))
                        .child(tr(cx, "Reset to default", "恢复默认")),
                    cx,
                    move |this, _, cx| this.reset_rule(&rid_reset, cx),
                ));
        }

        menu
    }

    fn render_loading_page(&self, phase: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        div()
            .w_full()
            .max_w(px(680.))
            .mx_auto()
            .pt(px(24.))
            .px(px(24.))
            .pb(px(48.))
            .flex()
            .flex_col()
            .gap(px(28.))
            .child(pro_skeleton_bar(340., 12., 4., phase, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(pro_skeleton_bar(148., 11., 4., phase, cx))
                    .child(
                        div()
                            .p(px(10.))
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .gap(px(12.))
                            .child(
                                div()
                                    .flex()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .flex_1()
                                            .child(pro_skeleton_bar(160., 58., 7., phase, cx)),
                                    )
                                    .child(div().flex_1().child(pro_skeleton_bar(
                                        160.,
                                        58.,
                                        7.,
                                        phase + 0.08,
                                        cx,
                                    )))
                                    .child(div().flex_1().child(pro_skeleton_bar(
                                        160.,
                                        58.,
                                        7.,
                                        phase + 0.16,
                                        cx,
                                    ))),
                            )
                            .child(pro_skeleton_bar(260., 11., 4., phase, cx)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(pro_skeleton_bar(56., 11., 4., phase, cx))
                    .child(
                        div()
                            .p(px(6.))
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .children((0..5usize).map(|i| {
                                div()
                                    .h(px(52.))
                                    .px(px(10.))
                                    .when(i > 0, |d| d.border_t_1().border_color(p.line_soft))
                                    .flex()
                                    .items_center()
                                    .gap(px(12.))
                                    .child(pro_skeleton_bar(28., 28., 7., phase, cx))
                                    .child(
                                        div()
                                            .flex_1()
                                            .flex()
                                            .flex_col()
                                            .gap(px(5.))
                                            .child(pro_skeleton_bar(
                                                96.,
                                                12.,
                                                4.,
                                                phase + 0.05 * i as f32,
                                                cx,
                                            ))
                                            .child(pro_skeleton_bar(124., 10., 4., phase, cx)),
                                    )
                                    .child(pro_skeleton_bar(140., 10., 4., phase, cx))
                            })),
                    ),
            )
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let reduced_motion = Theme::global(cx).reduced_motion;
        let width = self.measured_width.get();
        let stack_tiles = width < 620.;
        let panel_open = self.standalone || self.shell.read(cx).is_panel_open();

        // Interpolate segmented bar ratios
        let mut bar_ratios = self.current_ratios();
        if let Some((from, to, started)) = self.bar_anim {
            let elapsed = started.elapsed().as_secs_f32() * 1000.;
            if !reduced_motion && elapsed < BAR_TRANSITION_MS {
                let t = pro_ease(elapsed / BAR_TRANSITION_MS);
                bar_ratios = [
                    from[0] + (to[0] - from[0]) * t,
                    from[1] + (to[1] - from[1]) * t,
                    from[2] + (to[2] - from[2]) * t,
                ];
                w.request_animation_frame();
            } else {
                self.bar_anim = None;
            }
        }

        if self.loading && !reduced_motion {
            w.request_animation_frame();
        }
        let skel_phase = (self.boot_at.elapsed().as_secs_f32() * 0.85) % 1.0;

        let query = self.current_query(cx);
        let show_search = self.search_open || !query.is_empty();
        let search_ed = self.search_editor.clone();

        let search_pill_card = div()
            .w(px(204.))
            .h(px(28.))
            .px(px(8.))
            .rounded(px(6.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .items_center()
            .gap(px(6.))
            .child(pro_icon(ProIcon::Search, 13., p.faint))
            .when_some(search_ed, |d, ed| {
                d.child(div().flex_1().min_w_0().child(ed))
            })
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "close-filter"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Close filter", "关闭筛选"))
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
                    this.search_open = false;
                    cx.notify();
                },
            ));
        let search_pill = pro_presence(
            pro_eid(&self.id, "search-pill-pres"),
            show_search,
            0.18,
            0.14,
            move |amount| {
                div()
                    .relative()
                    .top(px(4. * (1. - amount)))
                    .opacity(amount)
                    .child(search_pill_card)
                    .into_any_element()
            },
        );

        // Top header (.ui-apm-head)
        let head = div()
            .h(px(48.))
            .flex_none()
            .pl(px(20.))
            .pr(px(if panel_open { 12. } else { 48. }))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(tr(cx, "Permissions", "权限策略")),
            )
            .child(search_pill);

        let content: AnyElement = if self.loading {
            self.render_loading_page(skel_phase, cx).into_any_element()
        } else {
            let active_preset = detect_matching_preset(&self.rules);
            let (total_allow, total_ask, total_never) = count_levels(self.rules.iter());
            let never_bar_c = if p.dark {
                hsla(0., 0., 1., 0.25)
            } else {
                hsla(0., 0., 0., 0.20)
            };

            // 1. Autonomy preset tiles (.ui-apm-auto)
            let preset_specs: [(AutonomyPreset, &'static str, &'static str); 3] = [
                (
                    AutonomyPreset::Cautious,
                    tr(cx, "Careful", "稳健"),
                    tr(
                        cx,
                        "Reads on its own, asks before it changes anything",
                        "自主读取信息，任何修改前均先询问",
                    ),
                ),
                (
                    AutonomyPreset::Balanced,
                    tr(cx, "Balanced", "均衡"),
                    tr(
                        cx,
                        "Works on its own, asks before anything that needs care",
                        "日常操作自主完成，敏感事项先询问",
                    ),
                ),
                (
                    AutonomyPreset::HandsOff,
                    tr(cx, "Hands-off", "全托管"),
                    tr(
                        cx,
                        "Does everything on its own – except the red lines",
                        "除红线操作外，其余全部自主完成",
                    ),
                ),
            ];

            let preset_sensor = pro_selection_sensor();
            let selected_preset = active_preset.unwrap_or(self.last_preset);
            let mut tiles_wrap = div()
                .relative()
                .p(px(3.))
                .rounded(px(9.))
                .bg(p.ink.opacity(0.035))
                .flex()
                .when(stack_tiles, |d| d.flex_col())
                .gap(px(2.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "preset-ind"),
                    format!("{selected_preset:?}"),
                    preset_sensor.clone(),
                    7.,
                    p.card,
                    Some(p.line),
                    true,
                ));

            for (pi, (preset, name, desc)) in preset_specs.into_iter().enumerate() {
                let checked = selected_preset == preset;
                let ic_color = if checked { p.blue } else { p.faint };
                let ic_el = match preset {
                    AutonomyPreset::Cautious => pro_svg("0 0 24 24", SVG_CAREFUL, 15., ic_color),
                    AutonomyPreset::Balanced => pro_svg("0 0 24 24", SVG_BALANCED, 15., ic_color),
                    AutonomyPreset::HandsOff => pro_icon(ProIcon::Bolt, 15., ic_color),
                };
                tiles_wrap = tiles_wrap.child(activate(
                    div()
                        .id(pro_eid(&self.id, format!("preset-{pi}")))
                        .role(Role::RadioButton)
                        .aria_label(name)
                        .tab_index(0)
                        .relative()
                        .flex_1()
                        .min_w_0()
                        .pt(px(10.))
                        .px(px(12.))
                        .pb(px(12.))
                        .rounded(px(7.))
                        .when(!checked, |d| d.hover(move |s| s.bg(p.hover)))
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .cursor_pointer()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .w(px(16.))
                                        .flex_none()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(ic_el),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(if checked { p.ink } else { p.text })
                                        .child(name),
                                ),
                        )
                        .child(
                            div()
                                .pl(px(24.))
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(desc),
                        )
                        .when(checked, |d| d.child(pro_mark_selected(&preset_sensor))),
                    cx,
                    move |this, _, cx| this.choose_preset(preset, cx),
                ));
            }

            let autonomy_section = div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(tr(cx, "How much it does on its own", "自主执行程度")),
                )
                .child(
                    div()
                        .p(px(6.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .child(tiles_wrap)
                        .child(
                            div()
                                .mt(px(6.))
                                .pt(px(12.))
                                .px(px(8.))
                                .pb(px(8.))
                                .border_t_1()
                                .border_color(p.line_soft)
                                .flex()
                                .flex_col()
                                .gap(px(10.))
                                .child(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .items_center()
                                        .gap(px(4.))
                                        .text_size(px(13.))
                                        .text_color(p.muted)
                                        .child(tr(cx, "Does", "自主完成"))
                                        .child(
                                            div()
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .child(format!("{total_allow}")),
                                        )
                                        .child(tr(cx, "things on its own", "项操作"))
                                        .child(div().text_color(p.faint).child("·"))
                                        .child(tr(cx, "asks before", "先询问"))
                                        .child(
                                            div()
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .child(format!("{total_ask}")),
                                        )
                                        .child(div().text_color(p.faint).child("·"))
                                        .child(tr(cx, "never does", "禁止"))
                                        .child(
                                            div()
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .child(format!("{total_never}")),
                                        ),
                                )
                                .child({
                                    let segs: [(f32, Hsla); 3] = [
                                        (bar_ratios[0], p.blue),
                                        (bar_ratios[1], p.amber),
                                        (bar_ratios[2], never_bar_c),
                                    ];
                                    let active_segs: Vec<(f32, Hsla)> =
                                        segs.into_iter().filter(|(r, _)| *r > 0.002).collect();
                                    let sum: f32 =
                                        active_segs.iter().map(|(r, _)| *r).sum::<f32>().max(1e-4);
                                    div().w_full().h(px(4.)).flex().gap(px(2.)).children(
                                        active_segs.into_iter().map(|(r, color)| {
                                            div()
                                                .w(relative((r / sum).clamp(0., 1.)))
                                                .min_w(px(3.))
                                                .h(px(4.))
                                                .rounded(px(2.))
                                                .bg(color)
                                        }),
                                    )
                                }),
                        ),
                );

            // 2. Connected Apps section (.ui-apm-apps)
            let filtered_apps: Vec<ConnectedApp> = if self.empty {
                Vec::new()
            } else {
                self.apps
                    .iter()
                    .filter(|app| {
                        if query.is_empty() {
                            return true;
                        }
                        app.name.to_ascii_lowercase().contains(&query)
                            || app
                                .account
                                .as_ref()
                                .is_some_and(|ac| ac.to_ascii_lowercase().contains(&query))
                            || self.rules.iter().any(|r| {
                                r.app == app.id && r.action.to_ascii_lowercase().contains(&query)
                            })
                    })
                    .cloned()
                    .collect()
            };

            let apps_section = div()
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
                        .child(tr(cx, "Apps", "已连接应用"))
                        .when(!self.empty, |d| {
                            d.child(
                                div()
                                    .text_color(p.faint)
                                    .child(format!("{}", filtered_apps.len())),
                            )
                        }),
                )
                .child(if self.empty || self.apps.is_empty() {
                    div()
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .child(pro_empty_state(
                            ProIcon::Grid,
                            tr(cx, "No apps connected", "尚未连接任何应用"),
                            tr(
                                cx,
                                "Connect the tools your agent works in to choose what it may read, change or ask about first.",
                                "连接智能体使用的工具，以配置其可自主读取、变更或需事先询问的权限。",
                            ),
                            Some((
                                pro_eid(&self.id, "connect-app"),
                                tr(cx, "Connect app", "连接应用").into(),
                                Box::new(|_: &mut Self, _: &mut Window, cx: &mut Context<Self>| {
                                    cx.emit(AgentPermissionsEvent::Connect);
                                    cx.emit(AgentPermissionsEvent::Navigate("Integrations".into()));
                                }),
                            )),
                            cx,
                        ))
                        .into_any_element()
                } else {
                    let app_sensor = pro_selection_sensor();
                    let sel_app_key = self
                        .selected_app
                        .clone()
                        .unwrap_or_else(|| "__none__".into());
                    div()
                        .relative()
                        .p(px(4.))
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .child(pro_pill_indicator(
                            pro_eid(&self.id, "app-sel-ind"),
                            sel_app_key,
                            app_sensor.clone(),
                            7.,
                            p.ink.opacity(0.045),
                            None,
                            false,
                        ))
                        .children(filtered_apps.into_iter().enumerate().map(|(ai, app)| {
                            let is_current = self.selected_app.as_ref() == Some(&app.id);
                            let (a_allow, a_ask, a_never) =
                                count_levels(self.rules.iter().filter(|r| r.app == app.id));
                            let mut parts = Vec::new();
                            if a_allow > 0 {
                                parts.push(format!(
                                    "{a_allow} {}",
                                    tr(cx, "on its own", "项自主")
                                ));
                            }
                            if a_ask > 0 {
                                parts.push(format!(
                                    "{} {a_ask}",
                                    tr(cx, "asks before", "需询问")
                                ));
                            }
                            if a_never > 0 {
                                parts.push(format!(
                                    "{} {a_never}",
                                    tr(cx, "never", "禁止")
                                ));
                            }
                            let words = parts.join(" · ");
                            let mark = render_app_mark(&app, 28., cx);
                            let app_id_click = app.id.clone();

                            // Mini proportional bar (36x4px)
                            let a_tot = (a_allow + a_ask + a_never).max(1) as f32;
                            let present_mini: Vec<(usize, Hsla)> = [
                                (a_allow, p.blue),
                                (a_ask, p.amber),
                                (a_never, never_bar_c),
                            ]
                            .into_iter()
                            .filter(|(c, _)| *c > 0)
                            .collect();
                            let mini_bar = div()
                                .w(px(36.))
                                .h(px(4.))
                                .flex_none()
                                .flex()
                                .gap(px(2.))
                                .children(present_mini.into_iter().map(|(cnt, col)| {
                                    div()
                                        .w(relative((cnt as f32 / a_tot).clamp(0., 1.)))
                                        .min_w(px(3.))
                                        .h(px(4.))
                                        .rounded(px(2.))
                                        .bg(col)
                                }));

                            div()
                                .when(ai > 0, |d| d.border_t_1().border_color(p.line_soft))
                                .child(activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("app-row-{}", app.id)))
                                        .role(Role::Button)
                                        .aria_label(format!("{}: {words}", app.name))
                                        .tab_index(0)
                                        .relative()
                                        .min_h(px(52.))
                                        .py(px(8.))
                                        .px(px(10.))
                                        .rounded(px(7.))
                                        .when(!is_current, |d| d.hover(move |s| s.bg(p.hover)))
                                        .flex()
                                        .items_center()
                                        .gap(px(12.))
                                        .cursor_pointer()
                                        .child(mark)
                                        .child(
                                            div()
                                                .when(stack_tiles, |d| d.flex_1())
                                                .when(!stack_tiles, |d| d.w(px(196.)).flex_none())
                                                .min_w_0()
                                                .flex()
                                                .flex_col()
                                                .child(
                                                    div()
                                                        .truncate()
                                                        .text_size(px(13.))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .line_height(px(20.))
                                                        .text_color(p.ink)
                                                        .child(app.name.clone()),
                                                )
                                                .when_some(app.account.clone(), |d, acc| {
                                                    d.child(
                                                        div()
                                                            .truncate()
                                                            .text_size(px(12.))
                                                            .line_height(px(16.))
                                                            .text_color(p.faint)
                                                            .child(acc),
                                                    )
                                                }),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .flex()
                                                .items_center()
                                                .justify_end()
                                                .gap(px(10.))
                                                .child(mini_bar)
                                                .when(!stack_tiles, |d| {
                                                    d.child(
                                                        div()
                                                            .min_w_0()
                                                            .truncate()
                                                            .text_size(px(12.))
                                                            .text_color(p.muted)
                                                            .child(words),
                                                    )
                                                }),
                                        )
                                        .child(pro_icon(ProIcon::ChevronRight, 12., p.faint))
                                        .when(is_current, |d| {
                                            d.child(pro_mark_selected(&app_sensor))
                                        }),
                                    cx,
                                    move |this, _, cx| {
                                        this.select_app(app_id_click.clone(), cx);
                                    },
                                ))
                        }))
                        .into_any_element()
                });

            // 3. Red lines section (.ui-apm-actions for locked rules)
            let red_lines: Vec<PermissionRule> = if self.empty || self.apps.is_empty() {
                Vec::new()
            } else {
                self.rules
                    .iter()
                    .filter(|r| {
                        r.locked
                            && (query.is_empty()
                                || r.action.to_ascii_lowercase().contains(&query)
                                || r.app.to_ascii_lowercase().contains(&query))
                    })
                    .cloned()
                    .collect()
            };

            let red_section = (!red_lines.is_empty()).then(|| {
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
                            .child(pro_icon(ProIcon::Lock, 12., p.muted))
                            .child(tr(cx, "Red lines", "安全红线")),
                    )
                    .child(
                        div()
                            .mt(px(-4.))
                            .mb(px(2.))
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.faint)
                            .child(tr(
                                cx,
                                "These always ask – or never happen – whatever you pick above",
                                "无论上方选择哪种模式，这些操作都必须先询问或严格禁止",
                            )),
                    )
                    .child(
                        div()
                            .px(px(4.))
                            .py(px(2.))
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .children(red_lines.into_iter().enumerate().map(|(ri, rule)| {
                                let app_obj = self
                                    .apps
                                    .iter()
                                    .find(|a| a.id == rule.app)
                                    .cloned()
                                    .unwrap_or_else(|| {
                                        ConnectedApp::new(rule.app.clone(), rule.app.clone())
                                    });
                                let sub = if let Some(ref det) = rule.detail {
                                    SharedString::from(format!("{} · {det}", app_obj.name))
                                } else {
                                    app_obj.name.clone()
                                };
                                let menu_open = self.open_menu
                                    == Some(MenuTarget::Rule {
                                        id: rule.id.clone(),
                                        in_red_lines: true,
                                    });
                                let rid_toggle = rule.id.clone();

                                div()
                                    .when(ri > 0, |d| d.border_t_1().border_color(p.line_soft))
                                    .py(px(6.))
                                    .pl(px(8.))
                                    .pr(px(4.))
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .min_h(px(36.))
                                            .flex()
                                            .items_center()
                                            .gap(px(10.))
                                            .child(render_app_mark(&app_obj, 22., cx))
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
                                                            .child(rule.action.clone()),
                                                    )
                                                    .child(
                                                        div()
                                                            .truncate()
                                                            .text_size(px(12.))
                                                            .line_height(px(16.))
                                                            .text_color(p.faint)
                                                            .child(sub),
                                                    ),
                                            )
                                            .child(activate(
                                                div()
                                                    .id(pro_eid(
                                                        &self.id,
                                                        format!("red-choice-{}", rule.id),
                                                    ))
                                                    .role(Role::Button)
                                                    .aria_label(format!(
                                                        "{}: {}",
                                                        rule.action,
                                                        rule.level.label(cx)
                                                    ))
                                                    .tab_index(0)
                                                    .h(px(28.))
                                                    .px(px(8.))
                                                    .rounded(px(6.))
                                                    .when(menu_open, |d| {
                                                        d.bg(p.chip).text_color(p.ink)
                                                    })
                                                    .hover(move |s| s.bg(p.chip).text_color(p.ink))
                                                    .flex_none()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(6.))
                                                    .text_size(px(12.5))
                                                    .text_color(p.text)
                                                    .cursor_pointer()
                                                    .child(render_level_dot(
                                                        Some(rule.level),
                                                        rule.locked,
                                                        7.,
                                                        &p,
                                                    ))
                                                    .child(rule.level.label(cx))
                                                    .child(pro_icon(
                                                        ProIcon::ChevronDown,
                                                        12.,
                                                        p.faint,
                                                    )),
                                                cx,
                                                move |this, _, cx| {
                                                    let target = MenuTarget::Rule {
                                                        id: rid_toggle.clone(),
                                                        in_red_lines: true,
                                                    };
                                                    if this.open_menu.as_ref() == Some(&target) {
                                                        this.open_menu = None;
                                                    } else {
                                                        this.open_menu = Some(target);
                                                    }
                                                    cx.notify();
                                                },
                                            )),
                                    )
                                    .child({
                                        let red_menu_el = div()
                                            .pl(px(32.))
                                            .child(self.render_rule_menu(&rule, "red", cx))
                                            .into_any_element();
                                        pro_fold(
                                            pro_eid(&self.id, format!("red-menu-fold-{}", rule.id)),
                                            menu_open,
                                            116.,
                                            0.,
                                            move || red_menu_el,
                                        )
                                    })
                            })),
                    )
            });

            let page_body = div()
                .w_full()
                .max_w(px(680.))
                .mx_auto()
                .pt(px(24.))
                .px(px(if stack_tiles { 16. } else { 24. }))
                .pb(px(48.))
                .flex()
                .flex_col()
                .gap(px(if stack_tiles { 24. } else { 28. }))
                .child(
                    div()
                        .mb(px(-8.))
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.muted)
                        .child(tr(
                            cx,
                            "What your agent – and every agent it starts – may do without asking you first",
                            "决定你的智能体及其启动的子智能体在无需询问的情况下可执行哪些操作",
                        )),
                )
                .child(autonomy_section)
                .child(apps_section)
                .when_some(red_section, |d, r| d.child(r));
            pro_view_transition(
                pro_eid(&self.id, "perm-page-vtx"),
                format!("{}", self.empty),
                page_body,
            )
        };

        let measure_cell = self.measured_width.clone();

        div()
            .id(pro_eid(&self.id, "page"))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| this.handle_key(ev, w, cx)))
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
            .child(
                div()
                    .id(pro_eid(&self.id, "scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(content),
            )
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
            let skel = div()
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(22.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .child(pro_skeleton_bar(36., 36., 8., phase, cx))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(5.))
                                .child(pro_skeleton_bar(84., 14., 4., phase, cx))
                                .child(pro_skeleton_bar(104., 10., 4., phase + 0.08, cx)),
                        ),
                )
                .child(pro_skeleton_bar(240., 64., 10., phase, cx))
                .child(pro_skeleton_bar(240., 104., 10., phase + 0.12, cx))
                .child(pro_skeleton_bar(240., 112., 10., phase + 0.2, cx))
                .into_any_element();
            return Some((tr(cx, "App", "应用").into(), ProIcon::Shield, skel));
        }

        if self.empty || self.apps.is_empty() {
            return None;
        }

        let app_id = self.selected_app.clone()?;
        let app = self.apps.iter().find(|a| a.id == app_id)?.clone();
        let app_rules: Vec<PermissionRule> = self
            .rules
            .iter()
            .filter(|r| r.app == app.id)
            .cloned()
            .collect();

        // Collect recent events across this app's rules
        let mut recent_events: Vec<PermissionRecentEvent> = Vec::new();
        for r in &app_rules {
            for ev in &r.recent {
                recent_events.push(ev.clone());
            }
        }

        let panel_head = div()
            .flex()
            .items_center()
            .gap(px(12.))
            .child(render_app_mark(&app, 36., cx))
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .truncate()
                            .text_size(px(15.))
                            .font_weight(FontWeight::MEDIUM)
                            .line_height(px(22.))
                            .text_color(p.ink)
                            .child(app.name.clone()),
                    )
                    .when_some(app.account.clone(), |d, acc| {
                        d.child(
                            div()
                                .truncate()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(acc),
                        )
                    }),
            );

        let mut panel_body = div()
            .pt(px(12.))
            .px(px(14.))
            .pb(px(20.))
            .flex()
            .flex_col()
            .gap(px(22.))
            .child(panel_head);

        for kind in [
            PermissionRuleKind::Read,
            PermissionRuleKind::Write,
            PermissionRuleKind::Risky,
        ] {
            let group_rules: Vec<PermissionRule> = app_rules
                .iter()
                .filter(|r| r.kind == kind)
                .cloned()
                .collect();
            if group_rules.is_empty() {
                continue;
            }
            let refs: Vec<&PermissionRule> = group_rules.iter().collect();
            let uniform = group_uniform_level(&refs);
            let group_count = group_rules.len();
            let group_menu_open = self.open_menu
                == Some(MenuTarget::Group {
                    app: app.id.clone(),
                    kind,
                });

            let group_choice = (group_count >= 2).then(|| {
                let g_app = app.id.clone();
                let g_lbl: SharedString = uniform
                    .map(|l| SharedString::from(l.label(cx)))
                    .unwrap_or_else(|| tr(cx, "Mixed", "混合").into());
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("grp-choice-{}-{kind:?}", app.id)))
                        .role(Role::Button)
                        .tab_index(0)
                        .mr(px(-4.))
                        .h(px(28.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .when(group_menu_open, |d| d.bg(p.chip).text_color(p.ink))
                        .hover(move |s| s.bg(p.chip).text_color(p.ink))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.5))
                        .text_color(p.text)
                        .cursor_pointer()
                        .child(render_level_dot(uniform, false, 7., &p))
                        .child(g_lbl)
                        .child(pro_icon(ProIcon::ChevronDown, 12., p.faint)),
                    cx,
                    move |this, _, cx| {
                        let target = MenuTarget::Group {
                            app: g_app.clone(),
                            kind,
                        };
                        if this.open_menu.as_ref() == Some(&target) {
                            this.open_menu = None;
                        } else {
                            this.open_menu = Some(target);
                        }
                        cx.notify();
                    },
                )
            });

            let group_dropdown = (group_count >= 2).then(|| {
                let g_app = app.id.clone();
                let root_id = self.id.clone();
                let items: Vec<_> = [
                    PermissionLevel::Allow,
                    PermissionLevel::Ask,
                    PermissionLevel::Never,
                ]
                .into_iter()
                .enumerate()
                .map(|(gi, lvl)| {
                    let checked = uniform == Some(lvl);
                    let g_app_pick = g_app.clone();
                    let lvl_lbl = lvl.label(cx);
                    activate(
                        div()
                            .id(pro_eid(&root_id, format!("gmenu-{}-{kind:?}-{gi}", g_app)))
                            .role(Role::MenuItem)
                            .tab_index(0)
                            .h(px(30.))
                            .px(px(9.))
                            .rounded(px(7.))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_size(px(12.5))
                            .text_color(if checked { p.ink } else { p.text })
                            .cursor_pointer()
                            .child(render_level_dot(Some(lvl), false, 7., &p))
                            .child(div().flex_1().child(lvl_lbl))
                            .when(checked, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink))),
                        cx,
                        move |this, _, cx| {
                            this.set_group_level(&g_app_pick, kind, lvl, cx);
                        },
                    )
                })
                .collect();
                let dd_el = div()
                    .p(px(4.))
                    .rounded(px(10.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .children(items)
                    .into_any_element();
                pro_fold(
                    pro_eid(&self.id, format!("grp-menu-fold-{}-{kind:?}", app.id)),
                    group_menu_open,
                    104.,
                    8.,
                    move || dd_el,
                )
            });

            let mut card = div()
                .mx(px(-6.))
                .px(px(4.))
                .py(px(2.))
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col();

            for (ri, rule) in group_rules.into_iter().enumerate() {
                let menu_open = self.open_menu
                    == Some(MenuTarget::Rule {
                        id: rule.id.clone(),
                        in_red_lines: false,
                    });
                let rid_toggle = rule.id.clone();
                let show_limit = rule.limit.is_some() && rule.level == PermissionLevel::Allow;
                let editing_this_limit = self.editing_limit_id.as_ref() == Some(&rule.id);
                let lim_ed = self.limit_editor.clone();

                let limit_row = rule.limit.as_ref().map(|lim| {
                    let edit_rid = rule.id.clone();
                    let cur_val = lim.value;
                    let unit = lim.unit.clone();
                    let lim_el = div()
                        .pl(px(26.))
                        .pb(px(4.))
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(tr(cx, "On its own up to", "自主执行上限"))
                        .child(if editing_this_limit {
                            div()
                                .h(px(24.))
                                .px(px(8.))
                                .rounded(px(6.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.muted)
                                .flex()
                                .items_center()
                                .gap(px(3.))
                                .when_some(lim_ed, |d, ed| d.child(div().w(px(40.)).child(ed)))
                                .child(div().text_color(p.muted).child(unit))
                                .into_any_element()
                        } else {
                            activate(
                                div()
                                    .id(pro_eid(&self.id, format!("lim-btn-{}", rule.id)))
                                    .role(Role::Button)
                                    .aria_label(format!("{cur_val} {unit}"))
                                    .tab_index(0)
                                    .h(px(24.))
                                    .px(px(8.))
                                    .rounded(px(6.))
                                    .bg(p.card)
                                    .border_1()
                                    .border_color(p.line_strong)
                                    .hover(move |s| s.border_color(p.muted))
                                    .flex()
                                    .items_center()
                                    .gap(px(3.))
                                    .cursor_pointer()
                                    .child(div().text_color(p.ink).child(format!("{cur_val}")))
                                    .child(div().text_color(p.muted).child(unit)),
                                cx,
                                move |this, w, cx| {
                                    this.editing_limit_id = Some(edit_rid.clone());
                                    if let Some(ref ed) = this.limit_editor {
                                        ed.update(cx, |e, cx| {
                                            e.set_value(cur_val.to_string(), w, cx);
                                            e.select_all(w, cx);
                                        });
                                        {
                                            let fh = ed.read(cx).focus.clone();
                                            fh.focus(w, cx);
                                        }
                                    }
                                    cx.notify();
                                },
                            )
                            .into_any_element()
                        })
                        .child(tr(cx, "asks above", "超出则询问"))
                        .into_any_element();
                    pro_fold(
                        pro_eid(&self.id, format!("lim-fold-{}", rule.id)),
                        show_limit,
                        28.,
                        2.,
                        move || lim_el,
                    )
                });

                let panel_menu_el = self.render_rule_menu(&rule, "panel", cx).into_any_element();
                let panel_menu_fold = pro_fold(
                    pro_eid(&self.id, format!("panel-menu-fold-{}", rule.id)),
                    menu_open,
                    124.,
                    2.,
                    move || panel_menu_el,
                );

                card = card.child(
                    div()
                        .when(ri > 0, |d| d.border_t_1().border_color(p.line_soft))
                        .py(px(6.))
                        .pl(px(8.))
                        .pr(px(4.))
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .min_h(px(34.))
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
                                        .child(rule.icon.render(15., p.muted)),
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
                                                .child(rule.action.clone()),
                                        )
                                        .when_some(rule.detail.clone(), |d, det| {
                                            d.child(
                                                div()
                                                    .truncate()
                                                    .text_size(px(12.))
                                                    .line_height(px(16.))
                                                    .text_color(p.faint)
                                                    .child(det),
                                            )
                                        }),
                                )
                                .child(activate(
                                    div()
                                        .id(pro_eid(&self.id, format!("pchoice-{}", rule.id)))
                                        .role(Role::Button)
                                        .aria_label(format!(
                                            "{}: {}",
                                            rule.action,
                                            rule.level.label(cx)
                                        ))
                                        .tab_index(0)
                                        .h(px(28.))
                                        .px(px(8.))
                                        .rounded(px(6.))
                                        .when(menu_open, |d| d.bg(p.chip).text_color(p.ink))
                                        .hover(move |s| s.bg(p.chip).text_color(p.ink))
                                        .flex_none()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .text_size(px(12.5))
                                        .text_color(p.text)
                                        .cursor_pointer()
                                        .child(render_level_dot(
                                            Some(rule.level),
                                            rule.locked,
                                            7.,
                                            &p,
                                        ))
                                        .child(rule.level.label(cx))
                                        .child(pro_icon(ProIcon::ChevronDown, 12., p.faint)),
                                    cx,
                                    move |this, _, cx| {
                                        let target = MenuTarget::Rule {
                                            id: rid_toggle.clone(),
                                            in_red_lines: false,
                                        };
                                        if this.open_menu.as_ref() == Some(&target) {
                                            this.open_menu = None;
                                        } else {
                                            this.open_menu = Some(target);
                                        }
                                        cx.notify();
                                    },
                                )),
                        )
                        .when_some(limit_row, |d, lr| d.child(lr))
                        .child(panel_menu_fold),
                );
            }

            let kind_key: SharedString = format!("{}:{kind:?}", app.id).into();
            let kind_collapsed = self.collapsed_kinds.contains(&kind_key);
            let kind_key_toggle = kind_key.clone();
            let card_el = card.into_any_element();
            let card_fold = pro_fold(
                pro_eid(&self.id, format!("grp-card-fold-{}-{kind:?}", app.id)),
                !kind_collapsed,
                (group_count as f32 * 48. + 8.).max(52.),
                8.,
                move || card_el,
            );

            panel_body = panel_body.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .min_h(px(24.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(activate(
                                div()
                                    .id(pro_eid(
                                        &self.id,
                                        format!("grp-fold-btn-{}-{kind:?}", app.id),
                                    ))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .ml(px(-6.))
                                    .px(px(6.))
                                    .h(px(24.))
                                    .rounded(px(5.))
                                    .hover(move |s| s.bg(p.hover))
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .cursor_pointer()
                                    .child(pro_icon(
                                        if kind_collapsed {
                                            ProIcon::ChevronRight
                                        } else {
                                            ProIcon::ChevronDown
                                        },
                                        11.,
                                        p.faint,
                                    ))
                                    .child(kind.heading(cx))
                                    .child(
                                        div().text_color(p.faint).child(format!("{group_count}")),
                                    ),
                                cx,
                                move |this, _, cx| {
                                    if !this.collapsed_kinds.remove(&kind_key_toggle) {
                                        this.collapsed_kinds.insert(kind_key_toggle.clone());
                                    }
                                    cx.notify();
                                },
                            ))
                            .when(!kind_collapsed, |d| {
                                d.when_some(group_choice, |d, gc| d.child(gc))
                            }),
                    )
                    .when(!kind_collapsed, |d| {
                        d.when_some(group_dropdown, |d, gd| d.child(gd))
                    })
                    .child(card_fold),
            );
        }

        // Lately section (.ui-apm-events)
        if !recent_events.is_empty() {
            panel_body = panel_body.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Lately", "近期记录")),
                    )
                    .child(
                        div()
                            .mx(px(-6.))
                            .px(px(12.))
                            .py(px(2.))
                            .rounded(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .children(recent_events.into_iter().enumerate().map(|(ei, ev)| {
                                let (badge_bg, badge_fg) = match ev.outcome {
                                    PermissionEventOutcome::Done
                                    | PermissionEventOutcome::Declined => (p.chip, p.muted),
                                    PermissionEventOutcome::Approved => (
                                        p.accent_soft,
                                        if p.dark {
                                            rgb(0x8dccfe).into()
                                        } else {
                                            p.accent_info
                                        },
                                    ),
                                    PermissionEventOutcome::Blocked => (p.red_soft, p.red_ink),
                                };
                                div()
                                    .py(px(8.))
                                    .when(ei > 0, |d| d.border_t_1().border_color(p.line_soft))
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
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
                                                    .child(ev.what),
                                            )
                                            .child(
                                                div()
                                                    .truncate()
                                                    .text_size(px(12.))
                                                    .line_height(px(16.))
                                                    .text_color(p.muted)
                                                    .child(format!("{} · {}", ev.action, ev.at)),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .h(px(18.))
                                            .px(px(6.))
                                            .flex_none()
                                            .rounded(px(4.))
                                            .bg(badge_bg)
                                            .text_color(badge_fg)
                                            .text_size(px(11.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .flex()
                                            .items_center()
                                            .child(ev.outcome.label(cx)),
                                    )
                            })),
                    ),
            );
        }

        let panel_body =
            pro_view_transition(pro_eid(&self.id, "perm-insp-vtx"), app_id, panel_body);
        Some((app.name, ProIcon::Shield, panel_body))
    }
}

impl Render for AgentPermissionsScreen {
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
        AutonomyPreset, PermissionLevel, PermissionRule, PermissionRuleKind, apply_preset_rules,
        count_levels, default_connected_apps, default_permission_rules, detect_matching_preset,
        group_uniform_level,
    };

    #[test]
    fn default_rules_match_balanced_totals_and_seven_locked_red_lines() {
        let apps = default_connected_apps();
        let rules = default_permission_rules();
        assert_eq!(apps.len(), 7);
        assert_eq!(rules.len(), 26);

        let (allow, ask, never) = count_levels(rules.iter());
        assert_eq!((allow, ask, never), (14, 6, 6));
        assert_eq!(
            detect_matching_preset(&rules),
            Some(AutonomyPreset::Balanced)
        );

        let locked_count = rules.iter().filter(|r| r.locked).count();
        assert_eq!(locked_count, 7);

        // Stripe has 2 on its own and 1 never, with Mixed level across its two Needs care rules.
        let stripe_risky: Vec<&PermissionRule> = rules
            .iter()
            .filter(|r| r.app.as_ref() == "stripe" && r.kind == PermissionRuleKind::Risky)
            .collect();
        assert_eq!(stripe_risky.len(), 2);
        assert_eq!(group_uniform_level(&stripe_risky), None);
    }

    #[test]
    fn switching_presets_recomputes_unlocked_rules_without_touching_red_lines() {
        let mut rules = default_permission_rules();

        // Careful: only Read rules remain Allow (8 reads across the 7 apps); locked red lines stay intact.
        let changed_to_cautious = apply_preset_rules(&mut rules, AutonomyPreset::Cautious);
        assert!(!changed_to_cautious.is_empty());
        assert_eq!(
            detect_matching_preset(&rules),
            Some(AutonomyPreset::Cautious)
        );
        let (_, _, never_cautious) = count_levels(rules.iter());
        assert_eq!(never_cautious, 6);

        // Hands-off: every unlocked rule becomes Allow (19 unlocked rules), while all 7 red lines stay Ask/Never.
        apply_preset_rules(&mut rules, AutonomyPreset::HandsOff);
        assert_eq!(
            detect_matching_preset(&rules),
            Some(AutonomyPreset::HandsOff)
        );
        let (allow_ho, ask_ho, never_ho) = count_levels(rules.iter());
        assert_eq!((allow_ho, ask_ho, never_ho), (19, 1, 6));
        assert!(
            rules
                .iter()
                .filter(|r| r.locked)
                .all(|r| r.level != PermissionLevel::Allow)
        );
    }
}
