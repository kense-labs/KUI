//! Agent roster screen with twinkling pixel marks, run charts, and agent creation.
//!
//! Displays every autonomous agent on the team in a two-column card grid or compact list,
//! pairing a right-hand detail inspector (live status, 14-day run bars, instructions, skills,
//! connected apps, and recent tasks) with a `New agent` dialog whose pixel mark updates live.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProPalette, pro_agent_avatar, pro_ease, pro_eid, pro_empty_state, pro_fold,
            pro_icon_btn, pro_mark_selected, pro_pill_indicator, pro_presence,
            pro_selection_sensor, pro_skeleton_bar, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, EditorShortcutEvent, InputMode},
        pro_icons::{ProIcon, person_initials, pro_app_artwork, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

/// Operational state of an [`AgentItem`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AgentRunStatus {
    /// Actively executing a task right now.
    Working,
    /// Available and waiting for new work.
    #[default]
    Idle,
    /// Paused by a team member; does not pick up tasks until resumed.
    Paused,
}

/// Accent swatch color for an agent's 3x3 pixel mark and 14-day run bars.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AgentColor {
    #[default]
    Blue,
    Purple,
    Pink,
    Green,
    Amber,
    Teal,
}

impl AgentColor {
    const ALL: [Self; 6] = [
        Self::Blue,
        Self::Purple,
        Self::Pink,
        Self::Green,
        Self::Amber,
        Self::Teal,
    ];

    fn resolve(self, p: &ProPalette) -> Hsla {
        match self {
            Self::Blue => p.blue,
            Self::Purple => p.purple,
            Self::Pink => p.pink,
            Self::Green => p.green,
            Self::Amber => p.amber,
            Self::Teal => p.teal,
        }
    }

    fn name(self, cx: &App) -> &'static str {
        match self {
            Self::Blue => tr(cx, "Blue", "蓝色"),
            Self::Purple => tr(cx, "Purple", "紫色"),
            Self::Pink => tr(cx, "Pink", "粉色"),
            Self::Green => tr(cx, "Green", "绿色"),
            Self::Amber => tr(cx, "Amber", "橙黄"),
            Self::Teal => tr(cx, "Teal", "青色"),
        }
    }
}

/// Layout presentation mode for [`AgentsScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AgentsViewMode {
    /// Two-column responsive card grid (`max-width: 720px`).
    #[default]
    Grid,
    /// Single-line tabular list rows.
    List,
}

/// Status filter scope in [`AgentsScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AgentStatusFilter {
    #[default]
    All,
    Working,
    Idle,
    Paused,
}

impl AgentStatusFilter {
    const ALL: [Self; 4] = [Self::All, Self::Working, Self::Idle, Self::Paused];

    fn next(self) -> Self {
        match self {
            Self::All => Self::Working,
            Self::Working => Self::Idle,
            Self::Idle => Self::Paused,
            Self::Paused => Self::All,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::All => Self::Paused,
            Self::Working => Self::All,
            Self::Idle => Self::Working,
            Self::Paused => Self::Idle,
        }
    }
}

/// Execution state dot for a row in `Recent tasks`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AgentTaskState {
    Running,
    #[default]
    Done,
    Failed,
}

/// One recent task entry shown at the bottom of the agent inspector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentRecentTask {
    /// Task summary title.
    pub title: SharedString,
    /// Current state (`Running`, `Done`, or `Failed`).
    pub state: AgentTaskState,
    /// Relative timestamp (`"now"`, `"3 hours ago"`, `"yesterday"`).
    pub at: SharedString,
}

impl AgentRecentTask {
    /// Construct a recent task entry.
    pub fn new(
        title: impl Into<SharedString>,
        state: AgentTaskState,
        at: impl Into<SharedString>,
    ) -> Self {
        Self {
            title: title.into(),
            state,
            at: at.into(),
        }
    }
}

/// Model family preset selectable in the `New agent` creation modal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AgentModelPreset {
    #[default]
    Claude,
    Gpt,
    Gemini,
    Grok,
}

impl AgentModelPreset {
    const ALL: [Self; 4] = [Self::Claude, Self::Gpt, Self::Gemini, Self::Grok];

    fn picker_label(self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Gpt => "GPT",
            Self::Gemini => "Gemini",
            Self::Grok => "Grok",
        }
    }

    fn model_name(self) -> &'static str {
        match self {
            Self::Claude => "Sonnet 5.5",
            Self::Gpt => "GPT-6 mini",
            Self::Gemini => "Gemini 3.8 Pro",
            Self::Grok => "Grok 5",
        }
    }
}

/// One autonomous agent in [`AgentsScreen`].
#[derive(Clone, Debug, PartialEq)]
pub struct AgentItem {
    /// Stable identifier (for example `"builder"`).
    pub id: SharedString,
    /// Agent display name (also seeds its deterministic 3x3 mirrored pixel mark).
    pub name: SharedString,
    /// Concise one-line role summary.
    pub role: SharedString,
    /// Mark and chart accent color.
    pub color: AgentColor,
    /// Backing foundation model label (`"Opus 5.5"`, `"GPT-6 mini"`, etc.).
    pub model: SharedString,
    /// Current operational status.
    pub status: AgentRunStatus,
    /// Live activity description shown when `status == Working`.
    pub doing: Option<SharedString>,
    /// Status caption shown when `Idle` or `Paused`.
    pub last_run: Option<SharedString>,
    /// Number of runs this week.
    pub runs: u32,
    /// Spend in euros this week (`46.20` formats as `€46` on cards and `€46.20` in the panel).
    pub cost: f32,
    /// Daily run counts across the past 14 days (oldest first; last 7 are this week).
    pub daily: [u32; 14],
    /// Reporting owner display name (`"Alex Morgan"`).
    pub owner: SharedString,
    /// System instructions paragraph shown in the inspector well.
    pub instructions: SharedString,
    /// Enabled skill names.
    pub skills: Vec<SharedString>,
    /// Connected apps this agent is permitted to use.
    pub apps: Vec<SharedString>,
    /// Recent task runs.
    pub recent: Vec<AgentRecentTask>,
}

impl AgentItem {
    /// Construct a new [`AgentItem`] with defaults suitable for further builder customization.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        role: impl Into<SharedString>,
        color: AgentColor,
        model: impl Into<SharedString>,
        status: AgentRunStatus,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            role: role.into(),
            color,
            model: model.into(),
            status,
            doing: None,
            last_run: None,
            runs: 0,
            cost: 0.0,
            daily: [0; 14],
            owner: "Alex Morgan".into(),
            instructions: SharedString::default(),
            skills: Vec::new(),
            apps: Vec::new(),
            recent: Vec::new(),
        }
    }

    /// Set what a working agent is doing right now.
    pub fn with_doing(mut self, doing: impl Into<SharedString>) -> Self {
        self.doing = Some(doing.into());
        self
    }

    /// Set the idle/paused status caption.
    pub fn with_last_run(mut self, last_run: impl Into<SharedString>) -> Self {
        self.last_run = Some(last_run.into());
        self
    }

    /// Set weekly run count and euro spend.
    pub fn with_weekly_stats(mut self, runs: u32, cost: f32) -> Self {
        self.runs = runs;
        self.cost = cost;
        self
    }

    /// Set the 14-day run count series.
    pub fn with_daily(mut self, daily: [u32; 14]) -> Self {
        self.daily = daily;
        self
    }

    /// Set the human owner to whom the agent reports.
    pub fn with_owner(mut self, owner: impl Into<SharedString>) -> Self {
        self.owner = owner.into();
        self
    }

    /// Set the agent's operating instructions.
    pub fn with_instructions(mut self, instructions: impl Into<SharedString>) -> Self {
        self.instructions = instructions.into();
        self
    }

    /// Set the agent's skills list.
    pub fn with_skills<I, S>(mut self, skills: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.skills = skills.into_iter().map(Into::into).collect();
        self
    }

    /// Set the connected applications this agent may access.
    pub fn with_apps<I, S>(mut self, apps: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.apps = apps.into_iter().map(Into::into).collect();
        self
    }

    /// Set the recent tasks list.
    pub fn with_recent(mut self, recent: Vec<AgentRecentTask>) -> Self {
        self.recent = recent;
        self
    }

    /// Return `true` if the agent matches `filter`.
    pub fn matches_filter(&self, filter: AgentStatusFilter) -> bool {
        match filter {
            AgentStatusFilter::All => true,
            AgentStatusFilter::Working => self.status == AgentRunStatus::Working,
            AgentStatusFilter::Idle => self.status == AgentRunStatus::Idle,
            AgentStatusFilter::Paused => self.status == AgentRunStatus::Paused,
        }
    }

    /// Return `true` if the agent matches a case-insensitive search `query`.
    pub fn matches_query(&self, query: &str) -> bool {
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            return true;
        }
        self.name.to_ascii_lowercase().contains(&q)
            || self.role.to_ascii_lowercase().contains(&q)
            || self.model.to_ascii_lowercase().contains(&q)
            || self.owner.to_ascii_lowercase().contains(&q)
            || self
                .doing
                .as_ref()
                .is_some_and(|d| d.to_ascii_lowercase().contains(&q))
            || self
                .apps
                .iter()
                .any(|a| a.to_ascii_lowercase().contains(&q))
            || self
                .skills
                .iter()
                .any(|s| s.to_ascii_lowercase().contains(&q))
    }
}

/// Events emitted by [`AgentsScreen`].
#[derive(Clone, Debug, PartialEq)]
pub enum AgentsEvent {
    /// User paused (`paused == true`) or resumed (`paused == false`) an agent.
    Pause { agent: AgentItem, paused: bool },
    /// User clicked `Message` on an agent in the detail inspector.
    Message(AgentItem),
    /// User created a new agent via the `New agent` modal.
    Create(AgentItem),
    /// User selected/opened an agent in the detail inspector.
    Select(SharedString),
    /// User navigated via the sidebar or inspector links (`"Skills"`, `"Permissions"`).
    Navigate(SharedString),
}

impl EventEmitter<AgentsEvent> for AgentsScreen {}

/// Default six sample agents (`Builder`, `Reviewer`, `Writer`, `Ops`, `Researcher`, `Support`).
pub fn default_agents() -> Vec<AgentItem> {
    vec![
        AgentItem::new(
            "builder",
            "Builder",
            "Writes and ships code",
            AgentColor::Blue,
            "Opus 5.5",
            AgentRunStatus::Working,
        )
        .with_doing("Running the migration on staging")
        .with_last_run("Idle · 2m ago")
        .with_weekly_stats(38, 46.20)
        .with_daily([2, 5, 3, 4, 2, 2, 5, 9, 3, 8, 8, 3, 4, 3])
        .with_owner("Alex Morgan")
        .with_instructions(
            "You work in the Fieldwork monorepo. Keep pull requests small, write tests first and never deploy to production without a person approving it. When a migration touches user data, do a dry run on staging first and post the result.",
        )
        .with_skills(["Code review", "Release notes", "Spreadsheet analysis"])
        .with_apps(["GitHub", "Linear", "Slack"])
        .with_recent(vec![
            AgentRecentTask::new("Migrate users table", AgentTaskState::Running, "now"),
            AgentRecentTask::new("Fix flaky checkout test", AgentTaskState::Done, "3 hours ago"),
            AgentRecentTask::new("Billing webhooks", AgentTaskState::Done, "yesterday"),
        ]),
        AgentItem::new(
            "reviewer",
            "Reviewer",
            "Reviews pull requests",
            AgentColor::Purple,
            "Sonnet 5.5",
            AgentRunStatus::Working,
        )
        .with_doing("Writing comments on PR #482")
        .with_last_run("Idle · 8m ago")
        .with_weekly_stats(64, 22.40)
        .with_daily([5, 8, 6, 9, 4, 3, 7, 11, 9, 12, 8, 10, 7, 7])
        .with_owner("Alex Morgan")
        .with_instructions(
            "Review every pull request opened against main within ten minutes. Point out missing edge-case tests, SQL lock risks and breaking API changes concisely, and approve clean fixes without nitpicking formatting.",
        )
        .with_skills(["Code review", "Security audit", "Test coverage"])
        .with_apps(["GitHub", "Slack"])
        .with_recent(vec![
            AgentRecentTask::new(
                "Review PR #482 – Retry failed billing webhooks",
                AgentTaskState::Running,
                "now",
            ),
            AgentRecentTask::new(
                "Audit auth token rotation PR #480",
                AgentTaskState::Done,
                "1 hour ago",
            ),
            AgentRecentTask::new("Verify API v2 schema diff", AgentTaskState::Done, "yesterday"),
        ]),
        AgentItem::new(
            "writer",
            "Writer",
            "Drafts posts, emails and notes",
            AgentColor::Pink,
            "Sonnet 5.5",
            AgentRunStatus::Working,
        )
        .with_doing("Drafting the release highlights")
        .with_last_run("Idle · 15m ago")
        .with_weekly_stats(27, 12.10)
        .with_daily([2, 3, 4, 2, 1, 3, 2, 4, 5, 3, 6, 4, 2, 3])
        .with_owner("Priya Shah")
        .with_instructions(
            "Write in Fieldwork's plain, direct voice – short sentences, concrete numbers, no hype. Save customer email drafts in Gmail without sending until a person approves.",
        )
        .with_skills(["Release notes", "Customer emails", "Weekly briefs"])
        .with_apps(["Google Drive", "Slack", "Gmail"])
        .with_recent(vec![
            AgentRecentTask::new(
                "Drafting week 46 release highlights",
                AgentTaskState::Running,
                "now",
            ),
            AgentRecentTask::new("Weekly update to Priya Shah", AgentTaskState::Done, "2 hours ago"),
            AgentRecentTask::new(
                "Onboarding email sequence copy",
                AgentTaskState::Done,
                "yesterday",
            ),
        ]),
        AgentItem::new(
            "ops",
            "Ops",
            "Keeps data and tools tidy",
            AgentColor::Green,
            "GPT-6 mini",
            AgentRunStatus::Idle,
        )
        .with_last_run("Idle · last run 38m ago")
        .with_weekly_stats(112, 9.40)
        .with_daily([12, 15, 14, 16, 11, 13, 15, 18, 16, 14, 19, 15, 16, 14])
        .with_owner("Alex Morgan")
        .with_instructions(
            "Reconcile Stripe payouts with HubSpot deals every morning, refund duplicate webhook captures under €50, and prune merged staging branches older than 14 days.",
        )
        .with_skills(["Webhook triage", "CRM sync", "Billing reconciliation"])
        .with_apps(["HubSpot", "Stripe", "GitHub"])
        .with_recent(vec![
            AgentRecentTask::new("Refund duplicate €38 charge", AgentTaskState::Done, "38m ago"),
            AgentRecentTask::new("Sync 14 HubSpot renewals", AgentTaskState::Done, "4 hours ago"),
            AgentRecentTask::new(
                "Prune 6 merged staging branches",
                AgentTaskState::Done,
                "yesterday",
            ),
        ]),
        AgentItem::new(
            "researcher",
            "Researcher",
            "Reads widely, writes cited briefs",
            AgentColor::Amber,
            "Gemini 3.8 Pro",
            AgentRunStatus::Idle,
        )
        .with_last_run("Idle · last run 3 hours ago")
        .with_weekly_stats(9, 31.80)
        .with_daily([1, 2, 0, 1, 2, 1, 0, 2, 1, 2, 0, 1, 2, 1])
        .with_owner("Priya Shah")
        .with_instructions(
            "Synthesize primary papers, benchmarks and changelogs into cited two-page briefs in Google Drive. Always link original sources and quote exact numbers.",
        )
        .with_skills(["Literature review", "Market briefs", "Citation check"])
        .with_apps(["Web", "Google Drive"])
        .with_recent(vec![
            AgentRecentTask::new("EU AI Act compliance brief", AgentTaskState::Done, "3 hours ago"),
            AgentRecentTask::new(
                "Vector index latency comparison",
                AgentTaskState::Done,
                "2 days ago",
            ),
            AgentRecentTask::new("Q4 pricing teardown", AgentTaskState::Done, "4 days ago"),
        ]),
        AgentItem::new(
            "support",
            "Support",
            "Answers customer questions",
            AgentColor::Teal,
            "Grok 5",
            AgentRunStatus::Paused,
        )
        .with_last_run("Paused · yesterday")
        .with_weekly_stats(0, 0.0)
        .with_daily([4, 6, 5, 3, 4, 2, 0, 0, 0, 0, 0, 0, 0, 0])
        .with_owner("Tom Becker")
        .with_instructions(
            "Answer incoming support threads using verified product docs and recent changelog entries. Escalate billing disputes or account deletions to Tom Becker in Slack immediately.",
        )
        .with_skills(["Ticket triage", "Docs lookup"])
        .with_apps(["Gmail", "Slack"])
        .with_recent(vec![
            AgentRecentTask::new("SSO SAML metadata question", AgentTaskState::Done, "2 days ago"),
            AgentRecentTask::new(
                "Webhook signature mismatch #109",
                AgentTaskState::Failed,
                "3 days ago",
            ),
        ]),
    ]
}

fn count_by_filter(agents: &[AgentItem], filter: AgentStatusFilter) -> usize {
    agents.iter().filter(|a| a.matches_filter(filter)).count()
}

fn model_brand_key(model: &str) -> &'static str {
    let lower = model.trim().to_ascii_lowercase();
    if lower.contains("opus")
        || lower.contains("sonnet")
        || lower.contains("haiku")
        || lower.contains("claude")
    {
        "claude"
    } else if lower.contains("gpt")
        || lower.starts_with("o1")
        || lower.starts_with("o3")
        || lower.starts_with("o4")
        || lower.contains("openai")
    {
        "openai"
    } else if lower.contains("gemini") {
        "gemini"
    } else if lower.contains("grok") || lower.contains("xai") {
        "grok"
    } else {
        "claude"
    }
}

fn render_model_logo(model: &str, side: f32, cx: &App) -> AnyElement {
    let p = ProPalette::get(cx);
    let key = model_brand_key(model);
    if let Some(mark) = pro_app_artwork(key, side, cx) {
        mark
    } else {
        pro_icon(ProIcon::Sparkles, side, p.ink)
    }
}

fn render_mini_app_tile(app: &str, side: f32, overlap: bool, cx: &App) -> Div {
    let p = ProPalette::get(cx);
    let lower = app.trim().to_ascii_lowercase();
    let glyph_side = (side * 0.62).round().max(10.);
    let inner = if matches!(lower.as_str(), "web" | "browser") {
        pro_icon(ProIcon::Globe, glyph_side, p.ink)
    } else if lower == "stripe"
        && let Some(mark) = pro_app_artwork("stripe", side, cx)
    {
        return div()
            .size(px(side))
            .flex_none()
            .when(overlap, |d| d.ml(px(-4.)))
            .rounded(px(5.))
            .overflow_hidden()
            .child(mark);
    } else if let Some(mark) = pro_app_artwork(app, glyph_side, cx) {
        mark
    } else {
        let ch: String = person_initials(app).chars().take(1).collect();
        div()
            .text_size(px(9.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.muted)
            .child(if ch.is_empty() { "A".to_owned() } else { ch })
            .into_any_element()
    };

    let tile_bg: Hsla = if p.dark {
        if matches!(
            lower.as_str(),
            "gmail" | "google drive" | "drive" | "slack" | "google calendar"
        ) {
            rgb(0x1b1b1e).into()
        } else {
            p.card
        }
    } else {
        rgb(0xffffff).into()
    };

    div()
        .size(px(side))
        .flex_none()
        .when(overlap, |d| d.ml(px(-4.)))
        .rounded(px(5.))
        .bg(tile_bg)
        .border_1()
        .border_color(p.line)
        .flex()
        .items_center()
        .justify_center()
        .child(inner)
}

fn render_apps_cluster(apps: &[SharedString], show_text: bool, cx: &App) -> Div {
    let p = ProPalette::get(cx);
    let max_tiles = 4usize;
    let mut tiles = div().flex_none().flex().items_center().py(px(1.));
    for (i, app) in apps.iter().take(max_tiles).enumerate() {
        tiles = tiles.child(render_mini_app_tile(app, 18., i > 0, cx));
    }
    if apps.len() > max_tiles {
        tiles = tiles.child(
            div()
                .min_w(px(18.))
                .h(px(18.))
                .ml(px(-4.))
                .px(px(3.))
                .rounded(px(5.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(9.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.muted)
                .child(format!("+{}", apps.len() - max_tiles)),
        );
    }

    let summary: SharedString = match apps.len() {
        0 => tr(cx, "No app access", "未授权应用").into(),
        1 => tr(cx, "Access to 1 app", "可访问 1 个应用").into(),
        n => format!("Access to {n} apps").into(),
    };

    div()
        .h(px(20.))
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(8.))
        .child(tiles)
        .when(show_text, |d| {
            d.child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(px(12.5))
                    .text_color(p.muted)
                    .child(summary),
            )
        })
}

fn render_status_tag(status: AgentRunStatus, cx: &App) -> Div {
    let p = ProPalette::get(cx);
    let (bg, fg, label) = match status {
        AgentRunStatus::Working => (
            p.accent_soft,
            if p.dark {
                rgb(0x7cc3fe).into()
            } else {
                rgb(0x0a6fc2).into()
            },
            tr(cx, "Working", "运行中"),
        ),
        AgentRunStatus::Idle => (p.chip, p.muted, tr(cx, "Idle", "空闲")),
        AgentRunStatus::Paused => (
            p.amber_soft,
            if p.dark {
                p.amber
            } else {
                rgb(0x8a6a00).into()
            },
            tr(cx, "Paused", "已暂停"),
        ),
    };

    div()
        .h(px(20.))
        .px(px(7.))
        .flex_none()
        .rounded(px(6.))
        .bg(bg)
        .flex()
        .items_center()
        .text_size(px(11.5))
        .font_weight(FontWeight::MEDIUM)
        .text_color(fg)
        .whitespace_nowrap()
        .child(label)
}

/// Pro Agents team screen with card/list switcher, right-hand agent inspector, and `New agent` dialog.
pub struct AgentsScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    standalone: bool,
    focus: FocusHandle,
    agents: Vec<AgentItem>,
    filter: AgentStatusFilter,
    view: AgentsViewMode,
    selected_id: Option<SharedString>,
    focused_id: Option<SharedString>,
    expanded_instructions_id: Option<SharedString>,
    search_open: bool,
    search_editor: Option<Entity<Editor>>,
    modal_open: bool,
    modal_focus_pending: bool,
    name_editor: Option<Entity<Editor>>,
    role_editor: Option<Entity<Editor>>,
    draft_model: AgentModelPreset,
    draft_color: AgentColor,
    created_seq: usize,
    fresh_agent: Option<(SharedString, Instant)>,
    chart_opened_at: Instant,
    loading: bool,
    empty: bool,
    clock: Instant,
    measured_width: Rc<Cell<f32>>,
}

impl AgentsScreen {
    /// Construct an [`AgentsScreen`] with the `Fieldwork` agent sidebar, 6 sample agents, and `"builder"` open.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let focus = cx.focus_handle().tab_stop(true);
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let nav_groups = vec![
                ShellNavGroup::primary(vec![
                    ShellNavItem::new("thread", tr(shell_cx, "Thread", "会话"), ProIcon::Thread),
                    ShellNavItem::new("inbox", tr(shell_cx, "Inbox", "收件箱"), ProIcon::Inbox)
                        .with_count(2),
                    ShellNavItem::new("agents", tr(shell_cx, "Agents", "智能体"), ProIcon::Users),
                    ShellNavItem::new("tasks", tr(shell_cx, "Tasks", "任务"), ProIcon::Tasks),
                    ShellNavItem::new("memory", tr(shell_cx, "Memory", "记忆"), ProIcon::Memory),
                    ShellNavItem::new("skills", tr(shell_cx, "Skills", "技能"), ProIcon::Sparkles),
                ]),
                ShellNavGroup::named(
                    tr(shell_cx, "Workspace", "工作区"),
                    vec![
                        ShellNavItem::new(
                            "schedules",
                            tr(shell_cx, "Schedules", "定时计划"),
                            ProIcon::Calendar,
                        ),
                        ShellNavItem::new(
                            "activity",
                            tr(shell_cx, "Activity", "活动记录"),
                            ProIcon::Activity,
                        ),
                        ShellNavItem::new(
                            "usage",
                            tr(shell_cx, "Usage", "用量统计"),
                            ProIcon::UsageChart,
                        ),
                        ShellNavItem::new(
                            "permissions",
                            tr(shell_cx, "Permissions", "权限策略"),
                            ProIcon::Shield,
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
                .with_groups(nav_groups)
                .with_active("agents")
                .with_work_area(true, true)
                .with_panel_width(348.);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s.bind_panel(&this, |screen, w, cx| screen.render_inspector(w, cx));
            s
        });

        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                cx.emit(AgentsEvent::Navigate(label.clone()));
            }
            AppShellEvent::PanelOpen(false) => {
                cx.notify();
            }
            _ => {}
        })
        .detach();

        let now = Instant::now();
        Self {
            id: ElementId::Name(id_str),
            shell,
            standalone: false,
            focus,
            agents: default_agents(),
            filter: AgentStatusFilter::All,
            view: AgentsViewMode::Grid,
            selected_id: Some("builder".into()),
            focused_id: Some("builder".into()),
            expanded_instructions_id: None,
            search_open: false,
            search_editor: None,
            modal_open: false,
            modal_focus_pending: false,
            name_editor: None,
            role_editor: None,
            draft_model: AgentModelPreset::Claude,
            draft_color: AgentColor::Blue,
            created_seq: 0,
            fresh_agent: None,
            chart_opened_at: now,
            loading: false,
            empty: false,
            clock: now,
            measured_width: Rc::new(Cell::new(680.)),
        }
    }

    /// Render only the inner roster view without the surrounding [`AppShell`].
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Override the agent roster.
    pub fn with_agents(mut self, agents: Vec<AgentItem>) -> Self {
        if self
            .selected_id
            .as_ref()
            .is_some_and(|sel| !agents.iter().any(|a| &a.id == sel))
        {
            self.selected_id = agents.first().map(|a| a.id.clone());
        }
        self.focused_id = self.selected_id.clone();
        self.agents = agents;
        self
    }

    /// Set the initial view mode (`Grid` or `List`).
    pub fn with_view(mut self, view: AgentsViewMode) -> Self {
        self.view = view;
        self
    }

    /// Set the initial status filter (`All`, `Working`, `Idle`, `Paused`).
    pub fn with_filter(mut self, filter: AgentStatusFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Set the agent initially open in the right-hand inspector (`Some("builder")` by default).
    pub fn with_selected(mut self, open: Option<SharedString>, cx: &mut Context<Self>) -> Self {
        let has_open = open.is_some();
        self.selected_id = open.clone();
        self.focused_id = open;
        self.shell
            .update(cx, |s, cx| s.set_panel_open(has_open, cx));
        self
    }

    /// Configure the skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the zero-agents empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Toggle the skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Return whether loading skeletons are active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle the empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    /// Return whether the screen is forced into its empty state.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// Replace the agents slice at runtime.
    pub fn set_agents(&mut self, agents: Vec<AgentItem>, cx: &mut Context<Self>) {
        if self
            .selected_id
            .as_ref()
            .is_some_and(|sel| !agents.iter().any(|a| &a.id == sel))
        {
            self.selected_id = agents.first().map(|a| a.id.clone());
        }
        self.agents = agents;
        cx.notify();
    }

    /// Return the current slice of agents.
    pub fn agents(&self) -> &[AgentItem] {
        &self.agents
    }

    /// Return the active view mode (`Grid` or `List`).
    pub fn view(&self) -> AgentsViewMode {
        self.view
    }

    /// Switch between `Grid` and `List` view modes.
    pub fn set_view(&mut self, view: AgentsViewMode, cx: &mut Context<Self>) {
        if self.view != view {
            self.view = view;
            cx.notify();
        }
    }

    /// Return the id of the agent currently open in the inspector.
    pub fn selected_id(&self) -> Option<&SharedString> {
        self.selected_id.as_ref()
    }

    /// Select and open an agent in the right-hand inspector.
    pub fn select(&mut self, id: Option<SharedString>, cx: &mut Context<Self>) {
        let changed = self.selected_id != id;
        self.selected_id = id.clone();
        if id.is_some() {
            self.focused_id = id.clone();
            if changed {
                self.chart_opened_at = Instant::now();
            }
            self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        }
        if let Some(sel) = id {
            cx.emit(AgentsEvent::Select(sel));
        }
        cx.notify();
    }

    /// Pause (`paused == true`) or resume (`paused == false`) the agent with `id`.
    pub fn set_paused(&mut self, id: &str, paused: bool, cx: &mut Context<Self>) -> bool {
        let Some(agent) = self.agents.iter_mut().find(|a| a.id.as_ref() == id) else {
            return false;
        };
        let next_status = if paused {
            AgentRunStatus::Paused
        } else if agent.doing.is_some() {
            AgentRunStatus::Working
        } else {
            AgentRunStatus::Idle
        };
        if agent.status == next_status {
            return false;
        }
        agent.status = next_status;
        if paused {
            agent.last_run = Some("Paused · just now".into());
        } else if next_status == AgentRunStatus::Idle {
            agent.last_run = Some("Idle · resumed just now".into());
        }
        let snapshot = agent.clone();
        cx.emit(AgentsEvent::Pause {
            agent: snapshot,
            paused,
        });
        cx.notify();
        true
    }

    /// Open the `New agent` creation modal.
    pub fn open_new_agent_modal(&mut self, cx: &mut Context<Self>) {
        self.modal_open = true;
        self.modal_focus_pending = true;
        self.draft_model = AgentModelPreset::Claude;
        self.draft_color = AgentColor::Blue;
        cx.notify();
    }

    /// Access the hosting [`AppShell`] entity.
    pub fn shell(&self) -> &Entity<AppShell> {
        &self.shell
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.search_editor.is_none() {
            let ph: SharedString = tr(cx, "Search agents", "搜索智能体").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed | EditEvent::Submitted => cx.notify(),
                EditEvent::Escape => {
                    if !ed_ent.read(cx).value().is_empty() {
                        ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                    } else {
                        this.search_open = false;
                        this.focus.focus(w, cx);
                    }
                    cx.notify();
                }
                EditEvent::Navigate(down) if *down => {
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.search_editor = Some(ed);
        }

        if self.name_editor.is_none() {
            let ph: SharedString = tr(cx, "e.g. Analyst", "例如：Analyst").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.set_command_submit(true);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |this, _, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => cx.notify(),
                EditEvent::Submitted => {
                    if let Some(role_ed) = this.role_editor.clone() {
                        role_ed.read(cx).focus.clone().focus(w, cx);
                    }
                }
                EditEvent::Escape => {
                    this.close_modal(w, cx);
                }
                _ => {}
            })
            .detach();
            cx.subscribe_in(&ed, w, |this, _, _: &EditorShortcutEvent, w, cx| {
                this.submit_new_agent(w, cx);
            })
            .detach();
            self.name_editor = Some(ed);
        }

        if self.role_editor.is_none() {
            let ph: SharedString = tr(
                cx,
                "Summarizes customer calls and files follow-ups…",
                "总结客户通话并整理后续跟进事项…",
            )
            .into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Multi, w, cx);
                e.set_placeholder(ph);
                e.set_command_submit(true);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |this, _, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => cx.notify(),
                EditEvent::Escape => {
                    this.close_modal(w, cx);
                }
                _ => {}
            })
            .detach();
            cx.subscribe_in(&ed, w, |this, _, _: &EditorShortcutEvent, w, cx| {
                this.submit_new_agent(w, cx);
            })
            .detach();
            self.role_editor = Some(ed);
        }

        if self.modal_focus_pending {
            self.modal_focus_pending = false;
            if let Some(ed) = self.name_editor.clone() {
                ed.read(cx).focus.clone().focus(w, cx);
            }
        }
    }

    fn close_modal(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.modal_open = false;
        if let Some(ed) = self.name_editor.clone() {
            ed.update(cx, |e, cx| e.set_value("", w, cx));
        }
        if let Some(ed) = self.role_editor.clone() {
            ed.update(cx, |e, cx| e.set_value("", w, cx));
        }
        self.focus.focus(w, cx);
        cx.notify();
    }

    fn submit_new_agent(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let name = self
            .name_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_owned())
            .unwrap_or_default();
        if name.is_empty() {
            return;
        }
        let raw_role = self
            .role_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_owned())
            .unwrap_or_default();
        let role: SharedString = if raw_role.is_empty() {
            tr(
                cx,
                "Handles assigned workspace tasks",
                "负责分配的工作区任务",
            )
            .into()
        } else {
            raw_role
                .lines()
                .next()
                .unwrap_or(&raw_role)
                .trim()
                .to_owned()
                .into()
        };
        let instructions: SharedString = if raw_role.is_empty() {
            format!(
                "You are {name} in the Fieldwork workspace. Ask before taking irreversible external actions and keep updates concise."
            )
            .into()
        } else {
            raw_role.into()
        };

        self.created_seq += 1;
        let slug: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();
        let new_id: SharedString = format!("{}-{slug}", self.created_seq).into();

        let created = AgentItem::new(
            new_id.clone(),
            name,
            role,
            self.draft_color,
            self.draft_model.model_name(),
            AgentRunStatus::Idle,
        )
        .with_last_run("Idle · created just now")
        .with_weekly_stats(0, 0.0)
        .with_daily([0; 14])
        .with_owner("Alex Morgan")
        .with_instructions(instructions)
        .with_skills(["Task triage", "Workspace search"])
        .with_apps(["Slack", "Google Drive"]);

        self.empty = false;
        if matches!(
            self.filter,
            AgentStatusFilter::Working | AgentStatusFilter::Paused
        ) {
            self.filter = AgentStatusFilter::All;
        }
        self.agents.insert(0, created.clone());
        self.fresh_agent = Some((new_id.clone(), Instant::now()));
        self.chart_opened_at = Instant::now();
        self.selected_id = Some(new_id.clone());
        self.focused_id = Some(new_id);
        self.shell.update(cx, |s, cx| s.set_panel_open(true, cx));
        cx.emit(AgentsEvent::Create(created));
        self.close_modal(w, cx);
    }

    fn current_query(&self, cx: &App) -> String {
        self.search_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_owned())
            .unwrap_or_default()
    }

    fn filtered_indices(&self, query: &str) -> Vec<usize> {
        self.agents
            .iter()
            .enumerate()
            .filter(|(_, a)| a.matches_filter(self.filter) && a.matches_query(query))
            .map(|(i, _)| i)
            .collect()
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        let m = ev.keystroke.modifiers;

        if self.modal_open {
            if key == "escape" {
                self.close_modal(w, cx);
                cx.stop_propagation();
            } else if key == "enter" && (m.platform || m.control) {
                self.submit_new_agent(w, cx);
                cx.stop_propagation();
            }
            return;
        }

        if self.loading {
            return;
        }

        let search_focused = self
            .search_editor
            .as_ref()
            .is_some_and(|e| e.read(cx).focus.is_focused(w));
        if search_focused {
            if key == "escape" {
                if let Some(ed) = self.search_editor.clone() {
                    if !ed.read(cx).value().is_empty() {
                        ed.update(cx, |e, cx| e.set_value("", w, cx));
                    } else {
                        self.search_open = false;
                        self.focus.focus(w, cx);
                    }
                }
                cx.stop_propagation();
                cx.notify();
            } else if key == "down" {
                self.focus.focus(w, cx);
                cx.stop_propagation();
                cx.notify();
            }
            return;
        }

        if m.platform || m.control || m.alt {
            return;
        }

        let query = self.current_query(cx);
        let visible = self.filtered_indices(&query);
        let two_col_grid = self.view == AgentsViewMode::Grid && self.measured_width.get() >= 580.;

        match key {
            "/" => {
                self.search_open = true;
                if let Some(ed) = self.search_editor.clone() {
                    ed.read(cx).focus.clone().focus(w, cx);
                }
                w.prevent_default();
                cx.stop_propagation();
                cx.notify();
            }
            "escape" => {
                if !query.is_empty() || self.search_open {
                    self.search_open = false;
                    if let Some(ed) = self.search_editor.clone() {
                        ed.update(cx, |e, cx| e.set_value("", w, cx));
                    }
                    cx.stop_propagation();
                    cx.notify();
                }
            }
            "[" => {
                self.filter = self.filter.prev();
                cx.stop_propagation();
                cx.notify();
            }
            "]" => {
                self.filter = self.filter.next();
                cx.stop_propagation();
                cx.notify();
            }
            "v" | "V" => {
                self.view = match self.view {
                    AgentsViewMode::Grid => AgentsViewMode::List,
                    AgentsViewMode::List => AgentsViewMode::Grid,
                };
                cx.stop_propagation();
                cx.notify();
            }
            "n" | "N" => {
                self.open_new_agent_modal(cx);
                cx.stop_propagation();
            }
            "j" | "right" | "down" => {
                if !visible.is_empty() {
                    let step = if key == "down" && two_col_grid { 2 } else { 1 };
                    let cur = self
                        .focused_id
                        .as_ref()
                        .or(self.selected_id.as_ref())
                        .and_then(|fid| {
                            visible.iter().position(|&idx| &self.agents[idx].id == fid)
                        });
                    let next = match cur {
                        Some(p) => (p + step).min(visible.len() - 1),
                        None => 0,
                    };
                    self.focused_id = Some(self.agents[visible[next]].id.clone());
                    cx.stop_propagation();
                    cx.notify();
                }
            }
            "k" | "left" | "up" => {
                if !visible.is_empty() {
                    let step = if key == "up" && two_col_grid { 2 } else { 1 };
                    let cur = self
                        .focused_id
                        .as_ref()
                        .or(self.selected_id.as_ref())
                        .and_then(|fid| {
                            visible.iter().position(|&idx| &self.agents[idx].id == fid)
                        });
                    let prev = match cur {
                        Some(p) => p.saturating_sub(step),
                        None => 0,
                    };
                    self.focused_id = Some(self.agents[visible[prev]].id.clone());
                    cx.stop_propagation();
                    cx.notify();
                }
            }
            "enter" => {
                let target = self
                    .focused_id
                    .clone()
                    .or_else(|| self.selected_id.clone())
                    .or_else(|| visible.first().map(|&idx| self.agents[idx].id.clone()));
                if target.is_some() {
                    self.select(target, cx);
                    cx.stop_propagation();
                }
            }
            "p" | "P" => {
                if let Some(tid) = self.focused_id.clone().or_else(|| self.selected_id.clone())
                    && let Some(ag) = self.agents.iter().find(|a| a.id == tid)
                {
                    let should_pause = ag.status != AgentRunStatus::Paused;
                    self.set_paused(&tid, should_pause, cx);
                    cx.stop_propagation();
                }
            }
            _ => {}
        }
    }

    fn render_toolbar(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let width = self.measured_width.get();
        let narrow = width < 560.;
        let panel_open = self.standalone || self.shell.read(cx).is_panel_open();
        let query = self.current_query(cx);
        let search_focused = self
            .search_editor
            .as_ref()
            .is_some_and(|e| e.read(cx).focus.is_focused(w));
        let show_search_box = self.search_open || search_focused || !query.is_empty();

        let n_all = count_by_filter(&self.agents, AgentStatusFilter::All);
        let n_working = count_by_filter(&self.agents, AgentStatusFilter::Working);
        let n_idle = count_by_filter(&self.agents, AgentStatusFilter::Idle);
        let n_paused = count_by_filter(&self.agents, AgentStatusFilter::Paused);

        let scope_sensor = pro_selection_sensor();
        let selected_scope_slug = match self.filter {
            AgentStatusFilter::All => "all",
            AgentStatusFilter::Working => "working",
            AgentStatusFilter::Idle => "idle",
            AgentStatusFilter::Paused => "paused",
        };
        let mut scopes = div()
            .relative()
            .flex()
            .items_center()
            .gap(px(4.))
            .min_w_0()
            .overflow_hidden()
            .child(pro_pill_indicator(
                pro_eid(&self.id, "scope-tabs"),
                selected_scope_slug,
                scope_sensor.clone(),
                8.,
                p.chip,
                None,
                false,
            ));

        for scope in AgentStatusFilter::ALL {
            let selected = self.filter == scope;
            let (label, count, slug) = match scope {
                AgentStatusFilter::All => (tr(cx, "All agents", "全部智能体"), n_all, "all"),
                AgentStatusFilter::Working => (tr(cx, "Working", "运行中"), n_working, "working"),
                AgentStatusFilter::Idle => (tr(cx, "Idle", "空闲"), n_idle, "idle"),
                AgentStatusFilter::Paused => (tr(cx, "Paused", "已暂停"), n_paused, "paused"),
            };
            scopes = scopes.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("scope-{slug}")))
                    .role(Role::Tab)
                    .aria_label(format!("{label} {count}"))
                    .tab_index(0)
                    .relative()
                    .h(px(28.))
                    .px(px(10.))
                    .rounded(px(8.))
                    .when(!selected, |d| {
                        d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                    })
                    .when(selected, |d| d.child(pro_mark_selected(&scope_sensor)))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .font_weight(if selected {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if selected { p.ink } else { p.muted })
                    .whitespace_nowrap()
                    .cursor_pointer()
                    .child(label)
                    .when(!self.loading, |d| {
                        d.child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::NORMAL)
                                .text_color(p.faint)
                                .child(count.to_string()),
                        )
                    }),
                cx,
                move |this, _, cx| {
                    this.filter = scope;
                    cx.notify();
                },
            ));
        }

        let search_ctrl = if show_search_box {
            let ed = self.search_editor.clone();
            let ed_click = ed.clone();
            let ed_clear = ed.clone();
            div()
                .id(pro_eid(&self.id, "search-pill"))
                .w(px(if narrow { 136. } else { 172. }))
                .h(px(28.))
                .px(px(8.))
                .rounded(px(7.))
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
                .cursor_text()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |_, _, w, cx| {
                        if let Some(e) = &ed_click {
                            e.read(cx).focus.clone().focus(w, cx);
                        }
                    }),
                )
                .child(pro_icon(ProIcon::Search, 13.5, p.faint))
                .when_some(ed, |d, ed_ent| {
                    d.child(div().flex_1().min_w_0().overflow_hidden().child(ed_ent))
                })
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "search-close"))
                        .role(Role::Button)
                        .aria_label(tr(cx, "Clear search", "关闭搜索"))
                        .tab_index(0)
                        .size(px(18.))
                        .rounded(px(4.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .child(pro_icon(ProIcon::Close, 11., p.muted)),
                    cx,
                    move |this, w, cx| {
                        this.search_open = false;
                        if let Some(e) = &ed_clear {
                            e.update(cx, |ed_inner, cx| ed_inner.set_value("", w, cx));
                        }
                        this.focus.focus(w, cx);
                        cx.notify();
                    },
                ))
                .into_any_element()
        } else {
            pro_icon_btn(
                pro_eid(&self.id, "search-btn"),
                tr(cx, "Search agents", "搜索智能体"),
                ProIcon::Search,
                28.,
                false,
                cx,
                |this, w, cx| {
                    this.search_open = true;
                    if let Some(ed) = this.search_editor.clone() {
                        ed.read(cx).focus.clone().focus(w, cx);
                    }
                    cx.notify();
                },
            )
            .into_any_element()
        };

        let view_sensor = pro_selection_sensor();
        let selected_view_key = match self.view {
            AgentsViewMode::Grid => "grid",
            AgentsViewMode::List => "list",
        };
        let is_grid = self.view == AgentsViewMode::Grid;
        let is_list = self.view == AgentsViewMode::List;
        let view_toggle = div()
            .relative()
            .flex_none()
            .mr(px(4.))
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "view-tabs"),
                selected_view_key,
                view_sensor.clone(),
                7.,
                p.chip,
                None,
                false,
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "view-grid"))
                    .role(Role::RadioButton)
                    .aria_label(tr(cx, "Grid", "网格视图"))
                    .tab_index(0)
                    .relative()
                    .size(px(28.))
                    .rounded(px(7.))
                    .when(!is_grid, |d| {
                        d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                    })
                    .when(is_grid, |d| d.child(pro_mark_selected(&view_sensor)))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(pro_icon(
                        ProIcon::Grid,
                        15.,
                        if is_grid { p.ink } else { p.muted },
                    )),
                cx,
                |this, _, cx| this.set_view(AgentsViewMode::Grid, cx),
            ))
            .child(activate(
                div()
                    .id(pro_eid(&self.id, "view-list"))
                    .role(Role::RadioButton)
                    .aria_label(tr(cx, "List", "列表视图"))
                    .tab_index(0)
                    .relative()
                    .size(px(28.))
                    .rounded(px(7.))
                    .when(!is_list, |d| {
                        d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                    })
                    .when(is_list, |d| d.child(pro_mark_selected(&view_sensor)))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(pro_icon(
                        ProIcon::List,
                        15.,
                        if is_list { p.ink } else { p.muted },
                    )),
                cx,
                |this, _, cx| this.set_view(AgentsViewMode::List, cx),
            ));

        let new_btn = activate(
            div()
                .id(pro_eid(&self.id, "new-agent-btn"))
                .role(Role::Button)
                .aria_label(tr(cx, "New agent", "新建智能体"))
                .tab_index(0)
                .h(px(28.))
                .when(narrow, |d| d.w(px(28.)).justify_center())
                .when(!narrow, |d| d.pl(px(10.)).pr(px(12.)))
                .rounded(px(8.))
                .bg(p.primary)
                .hover(move |s| s.opacity(0.88))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .flex_none()
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.on_primary)
                .whitespace_nowrap()
                .cursor_pointer()
                .child(pro_icon(ProIcon::Plus, 14., p.on_primary))
                .when(!narrow, |d| d.child(tr(cx, "New agent", "新建智能体"))),
            cx,
            |this, _, cx| this.open_new_agent_modal(cx),
        );

        let tools = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(4.))
            .child(search_ctrl)
            .child(view_toggle)
            .child(new_btn);

        div()
            .min_h(px(48.))
            .py(px(10.))
            .pl(px(if narrow { 44. } else { 14. }))
            .pr(px(if !panel_open { 52. } else { 12. }))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .child(scopes)
            .child(tools)
    }

    fn render_agent_card(&self, agent: &AgentItem, elapsed_ms: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let is_selected = self.selected_id.as_ref() == Some(&agent.id);
        let is_focused = self.focused_id.as_ref() == Some(&agent.id);
        let is_paused = agent.status == AgentRunStatus::Paused;
        let is_working = agent.status == AgentRunStatus::Working;

        let fresh_glow = self
            .fresh_agent
            .as_ref()
            .filter(|(fid, _)| fid == &agent.id)
            .and_then(|(_, started)| {
                let ms = started.elapsed().as_secs_f32() * 1000.;
                if ms < 1100. && !Theme::global(cx).reduced_motion {
                    Some((1.0 - ms / 1100.).clamp(0., 1.) * 0.22)
                } else {
                    None
                }
            })
            .unwrap_or(0.);

        let mark_color = if is_paused {
            p.faint
        } else {
            agent.color.resolve(&p)
        };
        let status_summary = match agent.status {
            AgentRunStatus::Working => {
                agent
                    .doing
                    .as_deref()
                    .unwrap_or(tr(cx, "Working", "运行中"))
            }
            AgentRunStatus::Idle => tr(cx, "Idle", "空闲"),
            AgentRunStatus::Paused => tr(cx, "Paused", "已暂停"),
        };
        let aria = format!("{}, {}. {}", agent.name, agent.role, status_summary);
        let select_id = agent.id.clone();

        let hover_card_bg: Hsla = if p.dark {
            rgb(0x202024).into()
        } else {
            rgb(0xfcfcfd).into()
        };
        let border_c = if fresh_glow > 0.01 {
            p.blue.opacity(0.45)
        } else if is_selected || is_focused {
            p.line_strong
        } else {
            p.line
        };

        div()
            .flex_1()
            .min_w_0()
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(border_c)
            .relative()
            .overflow_hidden()
            .hover(move |s| s.bg(hover_card_bg))
            .when(fresh_glow > 0.01, |d| {
                d.child(div().absolute().inset_0().bg(p.blue.opacity(fresh_glow)))
            })
            .child(activate(
                div()
                    .id(pro_eid(&self.id, format!("card-{}", agent.id)))
                    .role(Role::Button)
                    .aria_label(aria)
                    .tab_index(0)
                    .w_full()
                    .p(px(14.))
                    .rounded(px(12.))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .cursor_pointer()
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(pro_agent_avatar(
                                &agent.name,
                                mark_color,
                                24.,
                                is_working,
                                elapsed_ms,
                                cx,
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
                                            .line_height(px(19.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(if is_paused { p.faint } else { p.ink })
                                            .child(agent.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .truncate()
                                            .text_size(px(12.))
                                            .line_height(px(16.))
                                            .text_color(if is_paused { p.faint } else { p.muted })
                                            .child(agent.role.clone()),
                                    ),
                            )
                            .child(render_status_tag(agent.status, cx)),
                    )
                    .child(render_apps_cluster(&agent.apps, true, cx))
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .max_w(px(168.))
                                    .h(px(19.))
                                    .pl(px(5.))
                                    .pr(px(7.))
                                    .rounded(px(5.))
                                    .bg(p.chip)
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .text_size(px(11.5))
                                    .text_color(p.muted)
                                    .child(render_model_logo(&agent.model, 12., cx))
                                    .child(div().truncate().child(agent.model.clone())),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .text_size(px(12.))
                                    .text_color(p.faint)
                                    .child(div().pr(px(8.)).child(format!(
                                        "{} {}",
                                        agent.runs,
                                        tr(cx, "runs", "次")
                                    )))
                                    .child(div().w(px(1.)).h(px(11.)).bg(p.line_strong))
                                    .child(div().pl(px(8.)).child(format!("€{:.0}", agent.cost))),
                            ),
                    ),
                cx,
                move |this, _, cx| {
                    this.select(Some(select_id.clone()), cx);
                },
            ))
    }

    fn render_agent_row(
        &self,
        agent: &AgentItem,
        width: f32,
        elapsed_ms: f32,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let is_selected = self.selected_id.as_ref() == Some(&agent.id);
        let is_focused = self.focused_id.as_ref() == Some(&agent.id);
        let is_paused = agent.status == AgentRunStatus::Paused;
        let is_working = agent.status == AgentRunStatus::Working;
        let show_role_model = width >= 680.;
        let show_nums = width >= 540.;

        let mark_color = if is_paused {
            p.faint
        } else {
            agent.color.resolve(&p)
        };
        let select_id = agent.id.clone();

        activate(
            div()
                .id(pro_eid(&self.id, format!("row-{}", agent.id)))
                .role(Role::Button)
                .aria_label(format!("{}, {}", agent.name, agent.role))
                .tab_index(0)
                .w_full()
                .h(px(42.))
                .px(px(12.))
                .rounded(px(10.))
                .bg(if is_selected {
                    p.chip
                } else {
                    transparent_black()
                })
                .when(is_focused && !is_selected, |d| {
                    d.border_1().border_color(p.line_strong)
                })
                .hover(move |s| s.bg(if is_selected { p.chip } else { p.hover }))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .flex()
                .items_center()
                .gap(px(12.))
                .cursor_pointer()
                .child(pro_agent_avatar(
                    &agent.name,
                    mark_color,
                    22.,
                    is_working,
                    elapsed_ms,
                    cx,
                ))
                .child(
                    div()
                        .when(show_nums, |d| d.w(px(96.)).flex_none())
                        .when(!show_nums, |d| d.flex_1().min_w_0())
                        .truncate()
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if is_paused { p.faint } else { p.ink })
                        .child(agent.name.clone()),
                )
                .when(show_role_model, |d| {
                    d.child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(13.))
                            .text_color(p.muted)
                            .child(agent.role.clone()),
                    )
                })
                .child(div().w(px(72.)).flex_none().child(render_apps_cluster(
                    &agent.apps,
                    false,
                    cx,
                )))
                .child(
                    div()
                        .w(px(74.))
                        .flex_none()
                        .child(render_status_tag(agent.status, cx)),
                )
                .when(show_role_model, |d| {
                    d.child(
                        div()
                            .w(px(108.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(render_model_logo(&agent.model, 12., cx))
                            .child(div().truncate().child(agent.model.clone())),
                    )
                })
                .when(show_nums, |d| {
                    d.child(
                        div()
                            .w(px(52.))
                            .flex_none()
                            .flex()
                            .justify_end()
                            .text_size(px(12.))
                            .text_color(p.faint)
                            .child(format!("{}", agent.runs)),
                    )
                    .child(
                        div()
                            .w(px(40.))
                            .flex_none()
                            .flex()
                            .justify_end()
                            .text_size(px(12.))
                            .text_color(p.faint)
                            .child(format!("€{:.0}", agent.cost)),
                    )
                }),
            cx,
            move |this, _, cx| {
                this.select(Some(select_id.clone()), cx);
            },
        )
    }

    fn render_loading_grid(&self, two_cols: bool, phase: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let skel_card = || {
            div()
                .flex_1()
                .min_w_0()
                .p(px(14.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(24., 24., 6., phase, cx))
                        .child(
                            div()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(5.))
                                .child(pro_skeleton_bar(92., 13., 4., phase, cx))
                                .child(pro_skeleton_bar(136., 10., 4., phase, cx)),
                        )
                        .child(pro_skeleton_bar(54., 20., 6., phase, cx)),
                )
                .child(pro_skeleton_bar(124., 16., 5., phase, cx))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(pro_skeleton_bar(78., 18., 5., phase, cx))
                        .child(pro_skeleton_bar(84., 12., 4., phase, cx)),
                )
        };

        let mut wrap = div()
            .w_full()
            .max_w(px(720.))
            .flex()
            .flex_col()
            .gap(px(10.));
        if two_cols {
            for _ in 0..3usize {
                wrap = wrap.child(
                    div()
                        .flex()
                        .gap(px(10.))
                        .child(skel_card())
                        .child(skel_card()),
                );
            }
        } else {
            for _ in 0..4usize {
                wrap = wrap.child(skel_card());
            }
        }
        wrap
    }

    fn render_loading_inspector(&self, phase: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let well_bg: Hsla = if p.dark {
            rgb(0x222226).into()
        } else {
            rgb(0xf6f6f7).into()
        };
        div()
            .px(px(16.))
            .pt(px(14.))
            .pb(px(28.))
            .flex()
            .flex_col()
            .gap(px(26.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(pro_skeleton_bar(24., 24., 6., phase, cx))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(5.))
                            .child(pro_skeleton_bar(80., 13., 4., phase, cx))
                            .child(pro_skeleton_bar(120., 10., 4., phase, cx)),
                    )
                    .child(pro_skeleton_bar(72., 28., 8., phase, cx))
                    .child(pro_skeleton_bar(28., 28., 8., phase, cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .children((0..3usize).map(|_| {
                        div()
                            .h(px(22.))
                            .flex()
                            .items_center()
                            .gap(px(16.))
                            .child(pro_skeleton_bar(60., 11., 4., phase, cx))
                            .child(pro_skeleton_bar(140., 12., 4., phase, cx))
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .child(pro_skeleton_bar(42., 11., 4., phase, cx))
                            .child(pro_skeleton_bar(96., 11., 4., phase, cx)),
                    )
                    .child(pro_skeleton_bar(260., 72., 8., phase, cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(pro_skeleton_bar(72., 11., 4., phase, cx))
                    .child(
                        div()
                            .p(px(12.))
                            .rounded(px(10.))
                            .bg(well_bg)
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(pro_skeleton_bar(220., 12., 4., phase, cx))
                            .child(pro_skeleton_bar(196., 12., 4., phase, cx))
                            .child(pro_skeleton_bar(164., 12., 4., phase, cx)),
                    ),
            )
    }

    fn render_new_agent_modal(
        &mut self,
        elapsed_ms: f32,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let well_bg: Hsla = if p.dark {
            rgb(0x222226).into()
        } else {
            rgb(0xf5f5f7).into()
        };

        let draft_name = self
            .name_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_owned())
            .unwrap_or_default();
        let draft_role = self
            .role_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_owned())
            .unwrap_or_default();

        let seed = if draft_name.is_empty() {
            "new-agent"
        } else {
            draft_name.as_str()
        };
        let preview_sub: SharedString = if draft_role.is_empty() {
            tr(
                cx,
                "Name it and pick its model – the mark draws as you type",
                "输入名称并选择模型——像素徽标随输入实时生成",
            )
            .into()
        } else {
            draft_role
                .lines()
                .next()
                .unwrap_or(&draft_role)
                .to_owned()
                .into()
        };

        let name_focused = self
            .name_editor
            .as_ref()
            .is_some_and(|e| e.read(cx).focus.is_focused(w));
        let role_focused = self
            .role_editor
            .as_ref()
            .is_some_and(|e| e.read(cx).focus.is_focused(w));

        let name_ed = self.name_editor.clone();
        let name_click = name_ed.clone();
        let role_ed = self.role_editor.clone();
        let role_click = role_ed.clone();

        let model_sensor = pro_selection_sensor();
        let mut model_pills =
            div()
                .relative()
                .flex()
                .flex_wrap()
                .gap(px(6.))
                .child(pro_pill_indicator(
                    pro_eid(&self.id, "model-pills"),
                    self.draft_model.picker_label(),
                    model_sensor.clone(),
                    7.,
                    p.chip,
                    None,
                    false,
                ));
        for preset in AgentModelPreset::ALL {
            let checked = self.draft_model == preset;
            model_pills = model_pills.child(activate(
                div()
                    .id(pro_eid(
                        &self.id,
                        format!("pick-model-{}", preset.picker_label()),
                    ))
                    .role(Role::RadioButton)
                    .aria_label(preset.picker_label())
                    .tab_index(0)
                    .relative()
                    .h(px(30.))
                    .pl(px(8.))
                    .pr(px(10.))
                    .rounded(px(7.))
                    .when(!checked, |d| {
                        d.hover(move |s| s.bg(p.hover).text_color(p.ink))
                    })
                    .when(checked, |d| d.child(pro_mark_selected(&model_sensor)))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .font_weight(if checked {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if checked { p.ink } else { p.muted })
                    .cursor_pointer()
                    .child(render_model_logo(preset.model_name(), 14., cx))
                    .child(preset.picker_label()),
                cx,
                move |this, _, cx| {
                    this.draft_model = preset;
                    cx.notify();
                },
            ));
        }

        let mut swatches = div().flex().items_center().gap(px(8.));
        for (si, col) in AgentColor::ALL.into_iter().enumerate() {
            let checked = self.draft_color == col;
            let fill = col.resolve(&p);
            swatches = swatches.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("swatch-{si}")))
                    .role(Role::RadioButton)
                    .aria_label(col.name(cx))
                    .tab_index(0)
                    .size(px(24.))
                    .rounded_full()
                    .when(checked, |d| d.border_2().border_color(fill))
                    .p(px(2.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(div().size_full().rounded_full().bg(fill)),
                cx,
                move |this, _, cx| {
                    this.draft_color = col;
                    cx.notify();
                },
            ));
        }

        let can_create = !draft_name.is_empty();
        let scrim_id = pro_eid(&self.id, "modal-scrim");
        let modal_aria: SharedString = tr(cx, "New agent", "新建智能体").into();
        let scrim_alpha = if p.dark { 0.42 } else { 0.16 };
        let on_dismiss = cx.listener(|this, _, w, cx| {
            this.close_modal(w, cx);
        });

        let dialog_card = div()
            .w_full()
            .max_w(px(452.))
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
                    .pl(px(20.))
                    .pr(px(12.))
                    .pt(px(18.))
                    .pb(px(4.))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(pro_agent_avatar(
                        seed,
                        self.draft_color.resolve(&p),
                        36.,
                        true,
                        elapsed_ms,
                        cx,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(15.))
                                    .line_height(px(22.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(if draft_name.is_empty() {
                                        SharedString::from(tr(cx, "New agent", "新建智能体"))
                                    } else {
                                        SharedString::from(draft_name)
                                    }),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(12.5))
                                    .line_height(px(18.))
                                    .text_color(p.muted)
                                    .child(preview_sub),
                            ),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "modal-close"),
                        tr(cx, "Close", "关闭"),
                        ProIcon::Close,
                        28.,
                        false,
                        cx,
                        |this, w, cx| this.close_modal(w, cx),
                    )),
            )
            .child(
                div()
                    .px(px(20.))
                    .pt(px(16.))
                    .pb(px(20.))
                    .flex()
                    .flex_col()
                    .gap(px(15.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(tr(cx, "Name", "名称")),
                            )
                            .child(
                                div()
                                    .h(px(36.))
                                    .px(px(10.))
                                    .rounded(px(8.))
                                    .bg(if name_focused { p.card } else { well_bg })
                                    .border_1()
                                    .border_color(if name_focused {
                                        p.line_strong
                                    } else {
                                        transparent_black()
                                    })
                                    .flex()
                                    .items_center()
                                    .cursor_text()
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |_, _, w, cx| {
                                            if let Some(ed) = &name_click {
                                                ed.read(cx).focus.clone().focus(w, cx);
                                            }
                                        }),
                                    )
                                    .when_some(name_ed, |d, ed| {
                                        d.child(div().flex_1().min_w_0().child(ed))
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
                                    .text_color(p.muted)
                                    .child(tr(cx, "What it does", "职责描述")),
                            )
                            .child(
                                div()
                                    .min_h(px(68.))
                                    .px(px(10.))
                                    .py(px(8.))
                                    .rounded(px(8.))
                                    .bg(if role_focused { p.card } else { well_bg })
                                    .border_1()
                                    .border_color(if role_focused {
                                        p.line_strong
                                    } else {
                                        transparent_black()
                                    })
                                    .cursor_text()
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |_, _, w, cx| {
                                            if let Some(ed) = &role_click {
                                                ed.read(cx).focus.clone().focus(w, cx);
                                            }
                                        }),
                                    )
                                    .when_some(role_ed, |d, ed| d.child(div().w_full().child(ed))),
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
                                    .text_color(p.muted)
                                    .child(tr(cx, "Model", "模型")),
                            )
                            .child(model_pills),
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
                                    .text_color(p.muted)
                                    .child(tr(cx, "Color", "颜色")),
                            )
                            .child(swatches),
                    ),
            )
            .child(
                div()
                    .px(px(16.))
                    .py(px(12.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(8.))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "modal-cancel"))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(32.))
                            .px(px(12.))
                            .rounded(px(8.))
                            .hover(move |s| s.bg(p.hover).text_color(p.ink))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .text_size(px(13.))
                            .text_color(p.text)
                            .cursor_pointer()
                            .child(tr(cx, "Cancel", "取消")),
                        cx,
                        |this, w, cx| this.close_modal(w, cx),
                    ))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "modal-submit"))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(32.))
                            .px(px(14.))
                            .rounded(px(8.))
                            .bg(p.primary)
                            .opacity(if can_create { 1.0 } else { 0.35 })
                            .when(can_create, |d| {
                                d.cursor_pointer().hover(move |s| s.opacity(0.88))
                            })
                            .when(!can_create, |d| d.cursor_default())
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.on_primary)
                            .child(tr(cx, "Create agent", "创建智能体")),
                        cx,
                        |this, w, cx| this.submit_new_agent(w, cx),
                    )),
            );

        pro_presence(
            pro_eid(&self.id, "new-agent-pres"),
            self.modal_open,
            0.22,
            0.16,
            move |amount| {
                div()
                    .id(scrim_id)
                    .role(Role::Dialog)
                    .aria_label(modal_aria)
                    .absolute()
                    .inset_0()
                    .px(px(16.))
                    .py(px(24.))
                    .bg(hsla(0., 0., 0., scrim_alpha * amount))
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_mouse_down(MouseButton::Left, on_dismiss)
                    .child(
                        div()
                            .w_full()
                            .max_w(px(452.))
                            .relative()
                            .top(px(8. * (1. - amount)))
                            .opacity(amount)
                            .child(dialog_card),
                    )
                    .into_any_element()
            },
        )
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let reduced_motion = Theme::global(cx).reduced_motion;

        if self
            .fresh_agent
            .as_ref()
            .is_some_and(|(_, t)| t.elapsed().as_millis() >= 1150)
        {
            self.fresh_agent = None;
        }

        let any_working = self
            .agents
            .iter()
            .any(|a| a.status == AgentRunStatus::Working);
        let chart_animating = self.chart_opened_at.elapsed().as_millis() < 680;
        let fresh_animating = self.fresh_agent.is_some();

        if !reduced_motion
            && (self.loading
                || self.modal_open
                || (!self.empty && any_working)
                || chart_animating
                || fresh_animating)
        {
            w.request_animation_frame();
        }

        let elapsed_ms = self.clock.elapsed().as_secs_f32() * 1000.;
        let phase = (elapsed_ms / 1333.).fract();
        let measure_cell = self.measured_width.clone();
        let width = measure_cell.get();
        let two_cols = width >= 580.;

        let toolbar = self.render_toolbar(w, cx);

        let body = if self.loading {
            div()
                .w_full()
                .px(px(16.))
                .pt(px(8.))
                .pb(px(32.))
                .flex()
                .justify_center()
                .child(self.render_loading_grid(two_cols, phase, cx))
                .into_any_element()
        } else if self.empty || self.agents.is_empty() {
            let cta_id = pro_eid(&self.id, "empty-new-agent");
            let cta_label: SharedString = tr(cx, "New agent", "新建智能体").into();
            div()
                .min_h_full()
                .flex()
                .items_center()
                .justify_center()
                .child(pro_empty_state(
                    ProIcon::Users,
                    tr(cx, "No agents yet", "尚未创建智能体"),
                    tr(
                        cx,
                        "Create your first agent, pick its model and connect the apps it may work in.",
                        "创建你的第一个智能体，为其选择模型并配置可访问的应用。",
                    ),
                    Some((
                        cta_id,
                        cta_label,
                        Box::new(|this: &mut Self, _, cx| {
                            this.open_new_agent_modal(cx);
                        }),
                    )),
                    cx,
                ))
                .into_any_element()
        } else {
            let query = self.current_query(cx);
            let visible = self.filtered_indices(&query);
            if visible.is_empty() {
                let clear_id = pro_eid(&self.id, "clear-agent-filters");
                let clear_label: SharedString = tr(cx, "Show all agents", "显示全部智能体").into();
                div()
                    .min_h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_empty_state(
                        ProIcon::Search,
                        tr(cx, "No matching agents", "没有匹配的智能体"),
                        tr(
                            cx,
                            "Try clearing the search query or switching to All agents.",
                            "请清除搜索词，或切换回全部智能体标签。",
                        ),
                        Some((
                            clear_id,
                            clear_label,
                            Box::new(|this: &mut Self, w, cx| {
                                this.filter = AgentStatusFilter::All;
                                this.search_open = false;
                                if let Some(ed) = this.search_editor.clone() {
                                    ed.update(cx, |e, cx| e.set_value("", w, cx));
                                }
                                cx.notify();
                            }),
                        )),
                        cx,
                    ))
                    .into_any_element()
            } else {
                let inner = match self.view {
                    AgentsViewMode::Grid => {
                        let mut grid = div()
                            .w_full()
                            .max_w(px(720.))
                            .flex()
                            .flex_col()
                            .gap(px(10.));
                        if two_cols {
                            for pair in visible.chunks(2) {
                                let first =
                                    self.render_agent_card(&self.agents[pair[0]], elapsed_ms, cx);
                                let second = if pair.len() > 1 {
                                    self.render_agent_card(&self.agents[pair[1]], elapsed_ms, cx)
                                        .into_any_element()
                                } else {
                                    div().flex_1().min_w_0().into_any_element()
                                };
                                grid = grid.child(
                                    div()
                                        .w_full()
                                        .flex()
                                        .gap(px(10.))
                                        .child(first)
                                        .child(second),
                                );
                            }
                        } else {
                            for idx in visible {
                                grid = grid.child(self.render_agent_card(
                                    &self.agents[idx],
                                    elapsed_ms,
                                    cx,
                                ));
                            }
                        }
                        grid
                    }
                    AgentsViewMode::List => {
                        let mut rows = div().w_full().max_w(px(720.)).flex().flex_col().gap(px(2.));
                        for idx in visible {
                            rows = rows.child(self.render_agent_row(
                                &self.agents[idx],
                                width,
                                elapsed_ms,
                                cx,
                            ));
                        }
                        rows
                    }
                };

                div()
                    .w_full()
                    .px(px(if width < 560. { 12. } else { 16. }))
                    .pt(px(8.))
                    .pb(px(32.))
                    .flex()
                    .justify_center()
                    .child(inner)
                    .into_any_element()
            }
        };

        let body = if self.loading {
            body
        } else {
            let body_key = format!("agents-{:?}-{:?}-{}", self.filter, self.view, self.empty);
            pro_view_transition(pro_eid(&self.id, "roster-vtx"), body_key, body)
        };

        let n_working = count_by_filter(&self.agents, AgentStatusFilter::Working);
        let n_idle = count_by_filter(&self.agents, AgentStatusFilter::Idle);
        let n_paused = count_by_filter(&self.agents, AgentStatusFilter::Paused);
        let _live_summary = format!("{n_working} working, {n_idle} idle, {n_paused} paused");

        let modal = self.render_new_agent_modal(elapsed_ms, w, cx);

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .size_full()
            .min_h_0()
            .bg(p.panel)
            .overflow_hidden()
            .relative()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                this.handle_key_down(ev, w, cx);
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
            .child(toolbar)
            .child(
                div()
                    .id(pro_eid(&self.id, "agents-scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(body),
            )
            .child(div().absolute().size_0())
            .child(modal)
    }

    fn render_inspector(
        &mut self,
        _w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SharedString, ProIcon, AnyElement)> {
        let p = ProPalette::get(cx);
        let reduced_motion = Theme::global(cx).reduced_motion;
        let elapsed_ms = self.clock.elapsed().as_secs_f32() * 1000.;

        if self.loading {
            let phase = (elapsed_ms / 1333.).fract();
            return Some((
                tr(cx, "Agent", "智能体").into(),
                ProIcon::Users,
                self.render_loading_inspector(phase, cx).into_any_element(),
            ));
        }

        if self.empty || self.agents.is_empty() {
            let empty_el = div()
                .min_h(px(240.))
                .p(px(24.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .text_color(p.muted)
                .child(tr(cx, "No agent selected", "未选择智能体"));
            return Some((
                tr(cx, "Agent", "智能体").into(),
                ProIcon::Users,
                empty_el.into_any_element(),
            ));
        }

        let Some(sel_id) = self.selected_id.clone() else {
            let empty_el = div()
                .min_h(px(240.))
                .p(px(24.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .text_color(p.muted)
                .child(tr(
                    cx,
                    "Select an agent to inspect it",
                    "选择一个智能体查看详情",
                ));
            return Some((
                tr(cx, "Agent", "智能体").into(),
                ProIcon::Users,
                empty_el.into_any_element(),
            ));
        };

        let agent = self.agents.iter().find(|a| a.id == sel_id).cloned()?;

        let is_paused = agent.status == AgentRunStatus::Paused;
        let is_working = agent.status == AgentRunStatus::Working;
        let tone = agent.color.resolve(&p);
        let mark_color = if is_paused { p.faint } else { tone };

        // 1. Header: mark + name/role + Message + Pause/Resume
        let msg_agent = agent.clone();
        let pause_id = agent.id.clone();
        let pause_aria = if is_paused {
            tr(cx, "Resume", "恢复运行")
        } else {
            tr(cx, "Pause", "暂停")
        };

        let head = div()
            .flex()
            .items_center()
            .gap(px(10.))
            .child(pro_agent_avatar(
                &agent.name,
                mark_color,
                24.,
                is_working,
                elapsed_ms,
                cx,
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
                            .line_height(px(19.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(agent.name.clone()),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.muted)
                            .child(agent.role.clone()),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "panel-message-btn"))
                            .role(Role::Button)
                            .aria_label(tr(cx, "Message", "发消息"))
                            .tab_index(0)
                            .h(px(28.))
                            .pl(px(10.))
                            .pr(px(12.))
                            .rounded(px(8.))
                            .bg(p.primary)
                            .hover(move |s| s.opacity(0.88))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.on_primary)
                            .cursor_pointer()
                            .child(pro_icon(ProIcon::Thread, 13.5, p.on_primary))
                            .child(tr(cx, "Message", "发消息")),
                        cx,
                        move |_, _, cx| {
                            cx.emit(AgentsEvent::Message(msg_agent.clone()));
                        },
                    ))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "panel-pause-btn"))
                            .role(Role::Button)
                            .aria_label(pause_aria)
                            .tab_index(0)
                            .size(px(28.))
                            .rounded(px(8.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line_strong)
                            .hover(move |s| s.bg(p.hover))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .child(pro_icon(
                                if is_paused {
                                    ProIcon::Play
                                } else {
                                    ProIcon::Pause
                                },
                                13.5,
                                p.muted,
                            )),
                        cx,
                        move |this, _, cx| {
                            this.set_paused(&pause_id, !is_paused, cx);
                        },
                    )),
            );

        // 2. Props: Status / Model / Reports to
        let breathe_alpha = if is_working && !reduced_motion {
            let s = ((elapsed_ms / 2000.) * std::f32::consts::TAU).sin() * 0.5 + 0.5;
            0.10 + 0.16 * s
        } else {
            0.18
        };
        let status_dot = match agent.status {
            AgentRunStatus::Working => div()
                .size(px(13.))
                .flex_none()
                .rounded_full()
                .bg(p.green.opacity(breathe_alpha))
                .flex()
                .items_center()
                .justify_center()
                .child(div().size(px(7.)).rounded_full().bg(p.green)),
            AgentRunStatus::Idle => div()
                .size(px(13.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(div().size(px(7.)).rounded_full().bg(p.faint)),
            AgentRunStatus::Paused => div()
                .size(px(13.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .size(px(7.))
                        .rounded_full()
                        .border_1()
                        .border_color(p.faint),
                ),
        };
        let status_line: SharedString = match agent.status {
            AgentRunStatus::Working => agent
                .doing
                .clone()
                .unwrap_or_else(|| tr(cx, "Working", "运行中").into()),
            AgentRunStatus::Idle => agent
                .last_run
                .clone()
                .unwrap_or_else(|| tr(cx, "Idle", "空闲").into()),
            AgentRunStatus::Paused => agent
                .last_run
                .clone()
                .unwrap_or_else(|| tr(cx, "Paused", "已暂停").into()),
        };

        let owner_initials = person_initials(&agent.owner);

        let props = div()
            .flex()
            .flex_col()
            .gap(px(4.))
            .text_size(px(13.))
            .child(
                div()
                    .min_h(px(24.))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w(px(82.))
                            .flex_none()
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
                            .text_color(if is_working { p.ink } else { p.muted })
                            .child(status_dot)
                            .child(div().truncate().child(status_line)),
                    ),
            )
            .child(
                div()
                    .min_h(px(24.))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w(px(82.))
                            .flex_none()
                            .text_color(p.muted)
                            .child(tr(cx, "Model", "模型")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_color(p.ink)
                            .child(render_model_logo(&agent.model, 14., cx))
                            .child(div().truncate().child(agent.model.clone())),
                    ),
            )
            .child(
                div()
                    .min_h(px(24.))
                    .flex()
                    .items_center()
                    .child(div().w(px(82.)).flex_none().text_color(p.muted).child(tr(
                        cx,
                        "Reports to",
                        "汇报对象",
                    )))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .text_color(p.ink)
                            .child(
                                div()
                                    .size(px(20.))
                                    .flex_none()
                                    .rounded_full()
                                    .bg(p.chip)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(9.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.muted)
                                    .child(owner_initials),
                            )
                            .child(div().truncate().child(agent.owner.clone())),
                    ),
            );

        // 3. Runs 14-day bar chart
        let max_daily = agent.daily.iter().copied().max().unwrap_or(0);
        let y_max = max_daily.div_ceil(5).saturating_mul(5).max(10);
        let y_mid = y_max / 2;
        let chart_elapsed_ms = self.chart_opened_at.elapsed().as_secs_f32() * 1000.;
        let guide_color = p.line;

        let mut bars_row = div()
            .absolute()
            .inset_0()
            .px(px(4.))
            .flex()
            .items_end()
            .justify_between();

        for (bi, &val) in agent.daily.iter().enumerate() {
            let reveal = if reduced_motion {
                1.0
            } else {
                let delay = bi as f32 * 18.;
                pro_ease(((chart_elapsed_ms - delay) / 420.).clamp(0., 1.))
            };
            let target_h = if val == 0 {
                2.0
            } else {
                ((val as f32 / y_max as f32) * 70.).clamp(3., 70.)
            };
            let bar_h = (target_h * reveal).max(if val == 0 { 1.5 } else { 2.0 });
            let bar_color = if is_paused || val == 0 {
                p.ink.opacity(0.11)
            } else if bi >= 7 {
                tone.opacity(0.76)
            } else {
                tone.opacity(0.24)
            };
            bars_row = bars_row.child(div().w(px(9.)).h(px(bar_h)).rounded(px(2.5)).bg(bar_color));
        }

        let runs_section = div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Runs", "运行次数")),
                    )
                    .child(div().text_size(px(12.)).text_color(p.faint).child(format!(
                        "{} {} · €{:.2}",
                        agent.runs,
                        tr(cx, "this week", "本周"),
                        agent.cost
                    ))),
            )
            .child(
                div()
                    .mt(px(4.))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(
                                div()
                                    .w(px(20.))
                                    .h(px(76.))
                                    .flex_none()
                                    .flex()
                                    .flex_col()
                                    .justify_between()
                                    .items_end()
                                    .text_size(px(11.))
                                    .line_height(px(12.))
                                    .text_color(p.faint)
                                    .child(y_max.to_string())
                                    .child(y_mid.to_string())
                                    .child("0"),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .h(px(72.))
                                    .relative()
                                    .child(
                                        canvas(
                                            |_, _, _| (),
                                            move |bounds, _, window, _| {
                                                let w_px = f32::from(bounds.size.width);
                                                let h_px = f32::from(bounds.size.height);
                                                for frac in [0.0f32, 0.5f32] {
                                                    let y = bounds.top() + px(h_px * frac);
                                                    let mut x = 0.0f32;
                                                    while x < w_px {
                                                        let seg = (w_px - x).min(3.5);
                                                        window.paint_quad(fill(
                                                            Bounds::new(
                                                                point(bounds.left() + px(x), y),
                                                                size(px(seg), px(1.)),
                                                            ),
                                                            guide_color,
                                                        ));
                                                        x += 6.5;
                                                    }
                                                }
                                                window.paint_quad(fill(
                                                    Bounds::new(
                                                        point(
                                                            bounds.left(),
                                                            bounds.bottom() - px(1.),
                                                        ),
                                                        size(bounds.size.width, px(1.)),
                                                    ),
                                                    guide_color,
                                                ));
                                            },
                                        )
                                        .size_full(),
                                    )
                                    .child(bars_row),
                            ),
                    )
                    .child(
                        div()
                            .pl(px(30.))
                            .pr(px(2.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(11.))
                            .text_color(p.faint)
                            .child(tr(cx, "2 weeks ago", "两周前"))
                            .child(tr(cx, "Today", "今天")),
                    ),
            );

        // 4. Instructions well
        let well_bg: Hsla = if p.dark {
            rgb(0x222226).into()
        } else {
            rgb(0xf6f6f7).into()
        };
        let inst_raw = agent.instructions.clone();
        let inst_long = inst_raw.chars().count() > 164;
        let inst_expanded = self.expanded_instructions_id.as_ref() == Some(&agent.id);
        let (inst_head, inst_rest): (SharedString, Option<SharedString>) = if inst_long {
            let chars: Vec<char> = inst_raw.chars().collect();
            let split_at = chars[..158.min(chars.len())]
                .iter()
                .rposition(|c| c.is_whitespace())
                .unwrap_or(158.min(chars.len()));
            let head_str: String = chars[..split_at].iter().collect();
            let rest_str: SharedString = chars[split_at..]
                .iter()
                .collect::<String>()
                .trim_start()
                .to_owned()
                .into();
            let shown_head: SharedString = if inst_expanded {
                head_str.into()
            } else {
                format!("{}…", head_str.trim_end()).into()
            };
            (shown_head, Some(rest_str))
        } else {
            (inst_raw, None)
        };
        let inst_toggle_id = agent.id.clone();

        let mut inst_well = div()
            .px(px(14.))
            .py(px(12.))
            .rounded(px(10.))
            .bg(well_bg)
            .flex()
            .flex_col()
            .items_start()
            .gap(px(6.))
            .child(
                div()
                    .w_full()
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.text)
                    .child(inst_head),
            );

        if let Some(rest_txt) = inst_rest {
            let rest_el = div()
                .w_full()
                .text_size(px(13.))
                .line_height(px(20.))
                .text_color(p.text)
                .child(rest_txt)
                .into_any_element();
            inst_well = inst_well.child(pro_fold(
                pro_eid(&self.id, format!("inst-fold-{}", agent.id)),
                inst_expanded,
                40.,
                6.,
                move || rest_el,
            ));
        }

        if inst_long {
            inst_well = inst_well.child(activate(
                div()
                    .id(pro_eid(&self.id, "panel-inst-more"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(22.))
                    .px(px(6.))
                    .ml(px(-6.))
                    .rounded(px(6.))
                    .hover(move |s| s.bg(p.chip).text_color(p.ink))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .cursor_pointer()
                    .child(if inst_expanded {
                        tr(cx, "Show less", "收起")
                    } else {
                        tr(cx, "Show all", "展开全部")
                    }),
                cx,
                move |this, _, cx| {
                    if this.expanded_instructions_id.as_ref() == Some(&inst_toggle_id) {
                        this.expanded_instructions_id = None;
                    } else {
                        this.expanded_instructions_id = Some(inst_toggle_id.clone());
                    }
                    cx.notify();
                },
            ));
        }

        let instructions_section = div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(tr(cx, "Instructions", "运行指令")),
            )
            .child(inst_well);

        // 5. Skills & Can use
        let mut skill_tags = div().flex().flex_wrap().gap(px(6.));
        for sk in &agent.skills {
            skill_tags = skill_tags.child(
                div()
                    .h(px(26.))
                    .pl(px(7.))
                    .pr(px(9.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.5))
                    .text_color(p.text)
                    .child(pro_icon(ProIcon::Sparkles, 12., p.muted))
                    .child(sk.clone()),
            );
        }

        let mut app_tags = div().flex().flex_wrap().gap(px(6.));
        for app in &agent.apps {
            app_tags = app_tags.child(
                div()
                    .h(px(26.))
                    .pl(px(5.))
                    .pr(px(9.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(12.5))
                    .text_color(p.text)
                    .child(render_mini_app_tile(app, 16., false, cx))
                    .child(app.clone()),
            );
        }

        let skills_and_apps_section = div()
            .flex()
            .flex_col()
            .gap(px(10.))
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
                            .child(tr(cx, "Skills", "技能")),
                    )
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "panel-skills-link"))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(22.))
                            .pl(px(6.))
                            .pr(px(4.))
                            .rounded(px(6.))
                            .hover(move |s| s.bg(p.chip).text_color(p.ink))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .cursor_pointer()
                            .child(tr(cx, "Skills", "技能"))
                            .child(pro_icon(ProIcon::ChevronRight, 12., p.muted)),
                        cx,
                        |_, _, cx| cx.emit(AgentsEvent::Navigate("Skills".into())),
                    )),
            )
            .child(skill_tags)
            .child(
                div()
                    .mt(px(8.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Can use", "可用应用")),
                    )
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "panel-perms-link"))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(22.))
                            .pl(px(6.))
                            .pr(px(4.))
                            .rounded(px(6.))
                            .hover(move |s| s.bg(p.chip).text_color(p.ink))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .gap(px(2.))
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .cursor_pointer()
                            .child(tr(cx, "Permissions", "权限策略"))
                            .child(pro_icon(ProIcon::ChevronRight, 12., p.muted)),
                        cx,
                        |_, _, cx| cx.emit(AgentsEvent::Navigate("Permissions".into())),
                    )),
            )
            .child(app_tags);

        // 6. Recent tasks
        let recent_section = if !agent.recent.is_empty() {
            let mut list = div().flex().flex_col().gap(px(8.));
            for rt in &agent.recent {
                let state_dot = match rt.state {
                    AgentTaskState::Running => div()
                        .size(px(8.))
                        .flex_none()
                        .rounded_full()
                        .border_2()
                        .border_color(p.blue),
                    AgentTaskState::Done => {
                        div().size(px(8.)).flex_none().rounded_full().bg(p.green)
                    }
                    AgentTaskState::Failed => {
                        div().size(px(8.)).flex_none().rounded_full().bg(p.red)
                    }
                };
                list = list.child(
                    div()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .child(state_dot)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_color(p.text)
                                .child(rt.title.clone()),
                        )
                        .child(
                            div()
                                .flex_none()
                                .text_size(px(12.))
                                .text_color(p.faint)
                                .child(rt.at.clone()),
                        ),
                );
            }
            Some(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Recent tasks", "最近任务")),
                    )
                    .child(list),
            )
        } else {
            None
        };

        let panel_body = div()
            .px(px(16.))
            .pt(px(14.))
            .pb(px(28.))
            .flex()
            .flex_col()
            .gap(px(26.))
            .child(head)
            .child(props)
            .child(runs_section)
            .child(instructions_section)
            .child(skills_and_apps_section)
            .when_some(recent_section, |d, rs| d.child(rs));

        let panel_el = pro_view_transition(pro_eid(&self.id, "inspector-vtx"), sel_id, panel_body);

        Some((agent.name, ProIcon::Users, panel_el))
    }
}

impl Render for AgentsScreen {
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
        AgentColor, AgentModelPreset, AgentRunStatus, AgentStatusFilter, count_by_filter,
        default_agents, model_brand_key,
    };

    #[test]
    fn default_roster_counts_and_weekly_runs_match_spec() {
        let agents = default_agents();
        assert_eq!(agents.len(), 6);
        assert_eq!(count_by_filter(&agents, AgentStatusFilter::All), 6);
        assert_eq!(count_by_filter(&agents, AgentStatusFilter::Working), 3);
        assert_eq!(count_by_filter(&agents, AgentStatusFilter::Idle), 2);
        assert_eq!(count_by_filter(&agents, AgentStatusFilter::Paused), 1);

        // Builder's last-7-day bars equal its 38 runs this week
        let builder = &agents[0];
        let this_week_sum: u32 = builder.daily[7..].iter().sum();
        assert_eq!(this_week_sum, builder.runs);
        assert_eq!(builder.runs, 38);
    }

    #[test]
    fn search_and_model_brand_mapping_cover_all_families() {
        let agents = default_agents();
        let stripe_agents: Vec<&str> = agents
            .iter()
            .filter(|a| a.matches_query("stripe"))
            .map(|a| a.id.as_ref())
            .collect();
        assert_eq!(stripe_agents, vec!["ops"]);

        let working_claude: Vec<&str> = agents
            .iter()
            .filter(|a| {
                a.matches_filter(AgentStatusFilter::Working)
                    && model_brand_key(&a.model) == "claude"
            })
            .map(|a| a.id.as_ref())
            .collect();
        assert_eq!(working_claude, vec!["builder", "reviewer", "writer"]);

        assert_eq!(
            model_brand_key(AgentModelPreset::Claude.model_name()),
            "claude"
        );
        assert_eq!(
            model_brand_key(AgentModelPreset::Gpt.model_name()),
            "openai"
        );
        assert_eq!(
            model_brand_key(AgentModelPreset::Gemini.model_name()),
            "gemini"
        );
        assert_eq!(model_brand_key(AgentModelPreset::Grok.model_name()), "grok");
        assert_eq!(agents[5].status, AgentRunStatus::Paused);
        assert_eq!(agents[5].color, AgentColor::Teal);
    }
}
