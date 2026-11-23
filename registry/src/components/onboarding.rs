//! Guided workspace onboarding screen with setup checklist, tool connections, invites, and starter agents.
//!
//! Renders an in-app getting-started flow hosted inside [`AppShell`] (or standalone),
//! pairing a sticky setup checklist and progress bar on the left with interactive
//! workspace configuration, app connection switches, team invitations, and first-agent
//! templates on the right.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellLayout, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_btn, pro_eid, pro_empty_state, pro_fold,
            pro_mark_selected, pro_pill_indicator, pro_presence, pro_progress_ring,
            pro_selection_sensor, pro_skeleton_bar, pro_spring_bar, pro_spring_f32,
            pro_step_transition, pro_switch, pro_toast_banner, pro_view_transition, tr,
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
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const NOTION_SVG: &str = r#"<path d="M4.459 4.208c.746.606 1.026.56 2.428.466l13.215-.793c.28 0 .047-.28-.046-.326L17.86 1.968c-.42-.326-.981-.7-2.055-.607L3.01 2.295c-.466.046-.56.28-.374.466zm.793 3.08v13.904c0 .747.373 1.027 1.214.98l14.523-.84c.841-.046.935-.56.935-1.167V6.354c0-.606-.233-.933-.748-.887l-15.177.887c-.56.047-.747.327-.747.933zm14.337.745c.093.42 0 .84-.42.888l-.7.14v10.264c-.608.327-1.168.514-1.635.514-.748 0-.935-.234-1.495-.933l-4.577-7.186v6.952L12.21 19s0 .84-1.168.84l-3.222.186c-.093-.186 0-.653.327-.746l.84-.233V9.854L7.822 9.76c-.094-.42.14-1.026.793-1.073l3.456-.233 4.764 7.279v-6.44l-1.215-.139c-.093-.514.28-.887.747-.933zM1.936 1.035l13.31-.98c1.634-.14 2.055-.047 3.082.7l4.249 2.986c.7.513.934.653.934 1.213v16.378c0 1.026-.373 1.634-1.68 1.726l-15.458.934c-.98.047-1.448-.093-1.962-.747l-3.129-4.06c-.56-.747-.793-1.306-.793-1.96V2.667c0-.839.374-1.54 1.447-1.632z" fill="currentColor"></path>"#;

const LINEAR_SVG: &str = r#"<path d="M2.886 4.18A11.982 11.982 0 0 1 11.99 0C18.624 0 24 5.376 24 12.009c0 3.64-1.62 6.903-4.18 9.105L2.887 4.18ZM1.817 5.626l16.556 16.556c-.524.33-1.075.62-1.65.866L.951 7.277c.247-.575.537-1.126.866-1.65ZM.322 9.163l14.515 14.515c-.71.172-1.443.282-2.195.322L0 11.358a12 12 0 0 1 .322-2.195Zm-.17 4.862 9.823 9.824a12.02 12.02 0 0 1-9.824-9.824Z" fill="currentColor"></path>"#;

const HEADSET_SVG: &str = r#"<path d="M17 10.8045C17 10.4588 17 10.286 17.052 10.132C17.2032 9.68444 17.6018 9.51076 18.0011 9.32888C18.45 9.12442 18.6744 9.02219 18.8968 9.0042C19.1493 8.98378 19.4022 9.03818 19.618 9.15929C19.9041 9.31984 20.1036 9.62493 20.3079 9.87302C21.2513 11.0188 21.7229 11.5918 21.8955 12.2236C22.0348 12.7334 22.0348 13.2666 21.8955 13.7764C21.6438 14.6979 20.8485 15.4704 20.2598 16.1854C19.9587 16.5511 19.8081 16.734 19.618 16.8407C19.4022 16.9618 19.1493 17.0162 18.8968 16.9958C18.6744 16.9778 18.45 16.8756 18.0011 16.6711C17.6018 16.4892 17.2032 16.3156 17.052 15.868C17 15.714 17 15.5412 17 15.1955V10.8045Z" stroke="currentColor" stroke-width="1.9"></path><path d="M7 10.8046C7 10.3694 6.98778 9.97821 6.63591 9.6722C6.50793 9.5609 6.33825 9.48361 5.99891 9.32905C5.55001 9.12458 5.32556 9.02235 5.10316 9.00436C4.43591 8.9504 4.07692 9.40581 3.69213 9.87318C2.74875 11.019 2.27706 11.5919 2.10446 12.2237C1.96518 12.7336 1.96518 13.2668 2.10446 13.7766C2.3562 14.6981 3.15152 15.4705 3.74021 16.1856C4.11129 16.6363 4.46577 17.0475 5.10316 16.996C5.32556 16.978 5.55001 16.8757 5.99891 16.6713C6.33825 16.5167 6.50793 16.4394 6.63591 16.3281C6.98778 16.0221 7 15.631 7 15.1957V10.8046Z" stroke="currentColor" stroke-width="1.9"></path><path d="M5 9C5 5.68629 8.13401 3 12 3C15.866 3 19 5.68629 19 9" stroke="currentColor" stroke-linecap="square" stroke-linejoin="round" stroke-width="1.9"></path><path d="M19 17V17.8C19 19.5673 17.2091 21 15 21H13" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const BOLD_CHECK_SVG: &str = r#"<path d="M5 14L8.5 17.5L19 6.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2.6"></path>"#;

/// Ordered setup stages in [`OnboardingScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OnboardingStep {
    /// Step 1: workspace name, URL slug, and team headcount.
    Workspace,
    /// Step 2 (default initial view): external app and tool connections.
    #[default]
    Tools,
    /// Step 3: teammate email invitations and shareable join link.
    InviteTeam,
    /// Step 4: starter AI agent template selection.
    FirstAgent,
}

impl OnboardingStep {
    /// All four steps in canonical checklist order.
    pub const ALL: [Self; 4] = [
        Self::Workspace,
        Self::Tools,
        Self::InviteTeam,
        Self::FirstAgent,
    ];

    /// Return the 1-based step number (`1..=4`).
    pub fn number(self) -> u8 {
        match self {
            Self::Workspace => 1,
            Self::Tools => 2,
            Self::InviteTeam => 3,
            Self::FirstAgent => 4,
        }
    }

    /// Resolve a 1-based step number (`1..=4`), clamping out-of-range inputs.
    pub fn from_number(step: u8) -> Self {
        match step {
            0 | 1 => Self::Workspace,
            2 => Self::Tools,
            3 => Self::InviteTeam,
            _ => Self::FirstAgent,
        }
    }

    /// Return the 0-based checklist index (`0..4`).
    pub fn index(self) -> usize {
        usize::from(self.number() - 1)
    }

    /// Resolve a 0-based checklist index (`0..4`).
    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => Self::Workspace,
            1 => Self::Tools,
            2 => Self::InviteTeam,
            _ => Self::FirstAgent,
        }
    }
}

/// Completion state for a single checklist entry in [`OnboardingScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OnboardingStepState {
    #[default]
    Todo,
    Current,
    Done,
    Skipped,
}

/// Team size choice on the workspace setup step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TeamSize {
    Solo,
    #[default]
    Small,
    Medium,
    Large,
}

impl TeamSize {
    const ALL: [Self; 4] = [Self::Solo, Self::Small, Self::Medium, Self::Large];

    /// Localized display label for the team size band.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Solo => tr(cx, "Just me", "仅自己"),
            Self::Small => tr(cx, "2–10 people", "2–10 人"),
            Self::Medium => tr(cx, "11–50 people", "11–50 人"),
            Self::Large => tr(cx, "50+ people", "50 人以上"),
        }
    }
}

/// Default permission role assigned to invited teammates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InviteRole {
    #[default]
    Member,
    Admin,
    Viewer,
}

impl InviteRole {
    const ALL: [Self; 3] = [Self::Member, Self::Admin, Self::Viewer];

    /// Localized display label for the invite role.
    pub fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Member => tr(cx, "Member", "成员"),
            Self::Admin => tr(cx, "Admin", "管理员"),
            Self::Viewer => tr(cx, "Viewer", "访客"),
        }
    }
}

/// One connectable integration row in Step 2 of [`OnboardingScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OnboardingTool {
    pub id: SharedString,
    pub name: SharedString,
    pub description: SharedString,
    pub connected: bool,
}

impl OnboardingTool {
    /// Construct a disconnected integration option.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        description: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            connected: false,
        }
    }

    /// Set the initial connection state.
    pub fn with_connected(mut self, connected: bool) -> Self {
        self.connected = connected;
        self
    }
}

/// Starter AI agent template card in Step 4 of [`OnboardingScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentTemplate {
    pub id: SharedString,
    pub name: SharedString,
    pub description: SharedString,
    pub meta: SharedString,
    pub icon: ProIcon,
}

impl AgentTemplate {
    /// Construct an agent template option.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        description: impl Into<SharedString>,
        meta: impl Into<SharedString>,
        icon: ProIcon,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            meta: meta.into(),
            icon,
        }
    }
}

/// Collected configuration snapshot emitted when onboarding completes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OnboardingValues {
    pub workspace: SharedString,
    pub slug: SharedString,
    pub team_size: TeamSize,
    pub connected_tools: Vec<SharedString>,
    pub invites: Vec<SharedString>,
    pub role: InviteRole,
    pub template: SharedString,
}

/// Events emitted by [`OnboardingScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OnboardingEvent {
    StepChanged(OnboardingStep),
    ToolToggled { id: SharedString, connected: bool },
    InviteAdded(SharedString),
    InviteRemoved(SharedString),
    RoleChanged(InviteRole),
    TemplateSelected(SharedString),
    Complete(OnboardingValues),
    SkipStep(OnboardingStep),
    Dismissed,
    BookCall,
    Navigate(SharedString),
}

impl EventEmitter<OnboardingEvent> for OnboardingScreen {}

/// Normalize a workspace display name into a URL-safe lowercase slug.
pub fn workspace_slug(name: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if (ch.is_whitespace() || ch == '-' || ch == '_') && !out.is_empty() && !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "workspace".to_string()
    } else {
        trimmed
    }
}

fn is_valid_email(raw: &str) -> bool {
    let s = raw.trim();
    let Some((local, domain)) = s.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !local.contains(char::is_whitespace)
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains(char::is_whitespace)
}

fn first_name_of(full_name: &str) -> &str {
    full_name
        .split_whitespace()
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("Alex")
}

fn default_tools(cx: &App) -> Vec<OnboardingTool> {
    vec![
        OnboardingTool::new(
            "slack",
            "Slack",
            tr(
                cx,
                "Ask your agents from any channel and get their reports there.",
                "在任意频道向智能体提问并直接接收汇报。",
            ),
        )
        .with_connected(true),
        OnboardingTool::new(
            "notion",
            "Notion",
            tr(
                cx,
                "Search pages while you chat and save answers as docs.",
                "对话时检索页面并将回答保存为文档。",
            ),
        ),
        OnboardingTool::new(
            "google-drive",
            "Google Drive",
            tr(
                cx,
                "Pick files from Drive and let agents read them for context.",
                "从云端硬盘选择文件，供智能体读取上下文。",
            ),
        ),
        OnboardingTool::new(
            "hubspot",
            "HubSpot",
            tr(
                cx,
                "Look up contacts and deals, and log what agents find.",
                "查询联系人与商机，并自动记录跟进结果。",
            ),
        ),
        OnboardingTool::new(
            "linear",
            "Linear",
            tr(
                cx,
                "Turn requests from chat into issues and follow them to release.",
                "将对话需求转为工单并持续追踪至发布。",
            ),
        ),
        OnboardingTool::new(
            "google-calendar",
            "Google Calendar",
            tr(
                cx,
                "Schedule agent runs and prepare briefs before meetings.",
                "按日程触发智能体并在会前准备简报。",
            ),
        ),
    ]
}

fn default_templates(cx: &App) -> Vec<AgentTemplate> {
    vec![
        AgentTemplate::new(
            "support-triage",
            tr(cx, "Support triage", "工单分诊助手"),
            tr(
                cx,
                "Drafts replies, tags urgency, and routes billing questions.",
                "起草回复、标记紧急度并路由账单咨询。",
            ),
            tr(cx, "Uses Slack · Notion", "使用 Slack · Notion"),
            ProIcon::Inbox,
        ),
        AgentTemplate::new(
            "research-brief",
            tr(cx, "Research brief", "研究简报助手"),
            tr(
                cx,
                "Summarizes company news, docs, and competitor changes.",
                "汇总行业动态、内部文档与同行变化。",
            ),
            tr(cx, "Uses Drive · Notion", "使用 Drive · Notion"),
            ProIcon::Globe,
        ),
        AgentTemplate::new(
            "meeting-prep",
            tr(cx, "Meeting prep", "会前准备助手"),
            tr(
                cx,
                "Sends attendee context and open action items before calls.",
                "通话前自动推送参会背景与待办事项。",
            ),
            tr(cx, "Uses Calendar · HubSpot", "使用 Calendar · HubSpot"),
            ProIcon::Calendar,
        ),
    ]
}

fn sidebar_groups(cx: &App) -> Vec<ShellNavGroup> {
    vec![
        ShellNavGroup::primary(vec![
            ShellNavItem::new("home", tr(cx, "Home", "首页"), ProIcon::Home),
            ShellNavItem::new("chat", tr(cx, "Chat", "对话"), ProIcon::Thread),
            ShellNavItem::new(
                "analytics",
                tr(cx, "Analytics", "分析"),
                ProIcon::UsageChart,
            ),
            ShellNavItem::new(
                "integrations",
                tr(cx, "Integrations", "集成"),
                ProIcon::Grid,
            ),
        ]),
        ShellNavGroup::named(
            tr(cx, "Projects", "项目"),
            vec![
                ShellNavItem::new(
                    "q3",
                    tr(cx, "Q3 Campaign", "Q3 推广活动"),
                    ProIcon::ProjectFolder,
                ),
                ShellNavItem::new(
                    "brand",
                    tr(cx, "Brand assets", "品牌资产"),
                    ProIcon::ProjectFolder,
                ),
                ShellNavItem::new(
                    "hiring",
                    tr(cx, "Hiring", "团队招新"),
                    ProIcon::ProjectFolder,
                ),
                ShellNavItem::new(
                    "research",
                    tr(cx, "Research", "研究"),
                    ProIcon::ProjectFolder,
                ),
            ],
        ),
    ]
}

/// First-run workspace setup screen with interactive checklist and step wizard.
pub struct OnboardingScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    focus: FocusHandle,
    standalone: bool,
    loading: bool,
    empty: bool,
    workspace: SharedString,
    slug: SharedString,
    slug_customized: bool,
    user_name: SharedString,
    user_email: SharedString,
    team_size: TeamSize,
    current_step: OnboardingStep,
    step_states: [OnboardingStepState; 4],
    completed: bool,
    tools: Vec<OnboardingTool>,
    invites: Vec<SharedString>,
    invite_error: Option<SharedString>,
    last_invite_error: SharedString,
    role: InviteRole,
    role_menu_open: bool,
    link_copied_at: Option<Instant>,
    templates: Vec<AgentTemplate>,
    selected_template: SharedString,
    workspace_editor: Option<Entity<Editor>>,
    slug_editor: Option<Entity<Editor>>,
    invite_editor: Option<Entity<Editor>>,
    measured_width: Rc<Cell<f32>>,
    started_at: Instant,
}

impl OnboardingScreen {
    /// Create a new [`OnboardingScreen`] defaulting to Step 2 (`Connect your tools`) inside `"Acme"`.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let this = cx.entity();
        let groups = sidebar_groups(cx);
        let active_home: SharedString = tr(cx, "Home", "首页").into();
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_workspace("Acme")
                .with_user("Alex Morgan", "alex@acme.ai")
                .with_groups(groups)
                .with_active(active_home)
                .with_side_cards(true)
                .with_work_area(false, false);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s
        });
        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| {
            if let AppShellEvent::Navigate(label) = ev {
                cx.emit(OnboardingEvent::Navigate(label.clone()));
            }
        })
        .detach();

        let tools = default_tools(cx);
        let templates = default_templates(cx);
        let selected_template = templates
            .first()
            .map(|t| t.id.clone())
            .unwrap_or_else(|| "support-triage".into());

        let mut screen = Self {
            id: ElementId::Name(id_str),
            shell,
            focus: cx.focus_handle(),
            standalone: false,
            loading: false,
            empty: false,
            workspace: "Acme".into(),
            slug: "acme".into(),
            slug_customized: false,
            user_name: "Alex Morgan".into(),
            user_email: "alex@acme.ai".into(),
            team_size: TeamSize::Small,
            current_step: OnboardingStep::Tools,
            step_states: [
                OnboardingStepState::Done,
                OnboardingStepState::Current,
                OnboardingStepState::Todo,
                OnboardingStepState::Todo,
            ],
            completed: false,
            tools,
            invites: vec!["jamie@acme.com".into()],
            invite_error: None,
            last_invite_error: "".into(),
            role: InviteRole::Member,
            role_menu_open: false,
            link_copied_at: None,
            templates,
            selected_template,
            workspace_editor: None,
            slug_editor: None,
            invite_editor: None,
            measured_width: Rc::new(Cell::new(980.)),
            started_at: Instant::now(),
        };
        screen.apply_initial_step(OnboardingStep::Tools);
        screen
    }

    fn apply_initial_step(&mut self, step: OnboardingStep) {
        self.current_step = step;
        self.completed = false;
        let target = step.index();
        for (idx, slot) in self.step_states.iter_mut().enumerate() {
            *slot = if idx < target {
                OnboardingStepState::Done
            } else if idx == target {
                OnboardingStepState::Current
            } else {
                OnboardingStepState::Todo
            };
        }
    }

    /// Render only the onboarding canvas without the surrounding [`AppShell`] sidebar.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Switch the host [`AppShell`] between `Inset` and `Flat` chrome.
    pub fn with_layout(self, layout: AppShellLayout, cx: &mut Context<Self>) -> Self {
        let ws = self.workspace.clone();
        let name = self.user_name.clone();
        let email = self.user_email.clone();
        let groups = sidebar_groups(cx);
        let active: SharedString = tr(cx, "Home", "首页").into();
        let this = cx.entity();
        self.shell.update(cx, |s, shell_cx| {
            let mut next = AppShell::new("onb-shell", shell_cx)
                .with_layout(layout)
                .with_workspace(ws)
                .with_user(name, email)
                .with_groups(groups)
                .with_active(active)
                .with_side_cards(true)
                .with_work_area(false, false);
            next.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            *s = next;
        });
        self
    }

    /// Override the initial workspace name and derived URL slug.
    pub fn with_workspace(
        mut self,
        workspace: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let ws: SharedString = workspace.into();
        self.slug = workspace_slug(&ws).into();
        self.workspace = ws.clone();
        self.shell.update(cx, |s, shell_cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", shell_cx)).with_workspace(ws);
        });
        self
    }

    /// Override the signed-in user displayed in the greeting and sidebar footer.
    pub fn with_user(
        mut self,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let name: SharedString = name.into();
        let email: SharedString = email.into();
        self.user_name = name.clone();
        self.user_email = email.clone();
        self.shell.update(cx, |s, shell_cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", shell_cx)).with_user(name, email);
        });
        self
    }

    /// Set the step shown first; earlier steps are marked [`OnboardingStepState::Done`].
    pub fn with_default_step(mut self, step: OnboardingStep) -> Self {
        self.apply_initial_step(step);
        self
    }

    /// Seed initial invite email chips on Step 3.
    pub fn with_default_invites<I, S>(mut self, invites: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.invites = invites.into_iter().map(Into::into).collect();
        self
    }

    /// Override the connectable integration list on Step 2.
    pub fn with_tools(mut self, tools: Vec<OnboardingTool>) -> Self {
        self.tools = tools;
        self
    }

    /// Configure initial skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure initial dismissed/empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Toggle skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            cx.notify();
        }
    }

    /// Return whether the screen is showing loading skeletons.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    /// Return whether the screen is showing its empty/dismissed state.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// Return the currently active onboarding step.
    pub fn current_step(&self) -> OnboardingStep {
        self.current_step
    }

    /// Return whether the user has finished (or skipped past) the final step.
    pub fn is_completed(&self) -> bool {
        self.completed
    }

    /// Count how many of the 4 steps are marked [`OnboardingStepState::Done`].
    pub fn done_count(&self) -> usize {
        self.step_states
            .iter()
            .filter(|&&st| st == OnboardingStepState::Done)
            .count()
    }

    /// Access the underlying [`AppShell`] entity.
    pub fn shell(&self) -> &Entity<AppShell> {
        &self.shell
    }

    /// Snapshot all current onboarding answers.
    pub fn values(&self) -> OnboardingValues {
        OnboardingValues {
            workspace: self.workspace.clone(),
            slug: self.slug.clone(),
            team_size: self.team_size,
            connected_tools: self
                .tools
                .iter()
                .filter(|t| t.connected)
                .map(|t| t.id.clone())
                .collect(),
            invites: self.invites.clone(),
            role: self.role,
            template: self.selected_template.clone(),
        }
    }

    /// Jump directly to any step in the checklist.
    pub fn jump_to_step(&mut self, step: OnboardingStep, cx: &mut Context<Self>) {
        let prev_idx = self.current_step.index();
        let next_idx = step.index();
        if !self.completed && prev_idx == next_idx {
            return;
        }
        if !self.completed && self.step_states[prev_idx] == OnboardingStepState::Current {
            self.step_states[prev_idx] = OnboardingStepState::Todo;
        }
        self.completed = false;
        self.role_menu_open = false;
        self.current_step = step;
        if self.step_states[next_idx] != OnboardingStepState::Done {
            self.step_states[next_idx] = OnboardingStepState::Current;
        }
        cx.emit(OnboardingEvent::StepChanged(step));
        cx.notify();
    }

    /// Toggle connection state for a tool by `id`, returning the new state if found.
    pub fn toggle_tool(&mut self, id: &str, cx: &mut Context<Self>) -> Option<bool> {
        let tool = self.tools.iter_mut().find(|t| t.id.as_ref() == id)?;
        tool.connected = !tool.connected;
        let connected = tool.connected;
        let tool_id = tool.id.clone();
        cx.emit(OnboardingEvent::ToolToggled {
            id: tool_id,
            connected,
        });
        cx.notify();
        Some(connected)
    }

    /// Add a teammate email address to Step 3 invites after syntax validation.
    pub fn try_add_invite(
        &mut self,
        raw: &str,
        cx: &mut Context<Self>,
    ) -> Result<SharedString, SharedString> {
        let cleaned = raw
            .trim()
            .trim_matches(|c| c == ',' || c == ';')
            .to_ascii_lowercase();
        if cleaned.is_empty() {
            return Err(tr(cx, "Enter an email address.", "请输入邮箱地址。").into());
        }
        if !is_valid_email(&cleaned) {
            let msg: SharedString = tr(
                cx,
                "Enter a valid email like teammate@acme.com",
                "请输入有效邮箱，例如 teammate@acme.com",
            )
            .into();
            self.last_invite_error = msg.clone();
            self.invite_error = Some(msg.clone());
            cx.notify();
            return Err(msg);
        }
        self.invite_error = None;
        if let Some(existing) = self
            .invites
            .iter()
            .find(|e| e.eq_ignore_ascii_case(&cleaned))
            .cloned()
        {
            cx.notify();
            return Ok(existing);
        }
        let shared: SharedString = cleaned.into();
        self.invites.push(shared.clone());
        cx.emit(OnboardingEvent::InviteAdded(shared.clone()));
        cx.notify();
        Ok(shared)
    }

    /// Remove an invite chip by index.
    pub fn remove_invite(&mut self, index: usize, cx: &mut Context<Self>) -> Option<SharedString> {
        if index < self.invites.len() {
            let removed = self.invites.remove(index);
            self.invite_error = None;
            cx.emit(OnboardingEvent::InviteRemoved(removed.clone()));
            cx.notify();
            Some(removed)
        } else {
            None
        }
    }

    /// Complete the current step and advance to the next step (or completion view).
    pub fn advance_step(&mut self, cx: &mut Context<Self>) {
        if self.completed {
            return;
        }
        let idx = self.current_step.index();
        self.step_states[idx] = OnboardingStepState::Done;
        self.role_menu_open = false;
        if idx + 1 < 4 {
            let next = OnboardingStep::from_index(idx + 1);
            self.current_step = next;
            if self.step_states[idx + 1] != OnboardingStepState::Done {
                self.step_states[idx + 1] = OnboardingStepState::Current;
            }
            cx.emit(OnboardingEvent::StepChanged(next));
        } else {
            self.completed = true;
            cx.emit(OnboardingEvent::Complete(self.values()));
        }
        cx.notify();
    }

    /// Mark the current step as skipped and move forward.
    pub fn skip_current_step(&mut self, cx: &mut Context<Self>) {
        if self.completed {
            return;
        }
        let step = self.current_step;
        let idx = step.index();
        if self.step_states[idx] != OnboardingStepState::Done {
            self.step_states[idx] = OnboardingStepState::Skipped;
        }
        self.role_menu_open = false;
        cx.emit(OnboardingEvent::SkipStep(step));
        if idx + 1 < 4 {
            let next = OnboardingStep::from_index(idx + 1);
            self.current_step = next;
            if self.step_states[idx + 1] != OnboardingStepState::Done {
                self.step_states[idx + 1] = OnboardingStepState::Current;
            }
            cx.emit(OnboardingEvent::StepChanged(next));
        } else {
            self.completed = true;
            cx.emit(OnboardingEvent::Complete(self.values()));
        }
        cx.notify();
    }

    /// Return to the previous step in the wizard.
    pub fn previous_step(&mut self, cx: &mut Context<Self>) {
        if self.completed {
            self.completed = false;
            self.current_step = OnboardingStep::FirstAgent;
            cx.emit(OnboardingEvent::StepChanged(OnboardingStep::FirstAgent));
            cx.notify();
            return;
        }
        let idx = self.current_step.index();
        if idx == 0 {
            return;
        }
        if self.step_states[idx] == OnboardingStepState::Current {
            self.step_states[idx] = OnboardingStepState::Todo;
        }
        let prev = OnboardingStep::from_index(idx - 1);
        self.current_step = prev;
        self.role_menu_open = false;
        cx.emit(OnboardingEvent::StepChanged(prev));
        cx.notify();
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.workspace_editor.is_none() {
            let initial = self.workspace.to_string();
            let label: SharedString = tr(cx, "Workspace name", "工作区名称").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(label, InputMode::Single, w, cx);
                e.set_placeholder("Acme");
                e.font_size = 13.;
                e.set_value(initial, w, cx);
                e
            });
            cx.subscribe_in(&ed, w, |this, e, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => {
                    let raw = e.read(cx).value().trim().to_string();
                    let ws: SharedString = if raw.is_empty() {
                        "Acme".into()
                    } else {
                        raw.into()
                    };
                    this.workspace = ws.clone();
                    if !this.slug_customized {
                        let next_slug = workspace_slug(&ws);
                        this.slug = next_slug.clone().into();
                        if let Some(slug_ed) = &this.slug_editor {
                            slug_ed.update(cx, |se, cx| se.set_value(next_slug, w, cx));
                        }
                    }
                    cx.notify();
                }
                EditEvent::Submitted => {
                    this.advance_step(cx);
                }
                _ => {}
            })
            .detach();
            self.workspace_editor = Some(ed);
        }

        if self.slug_editor.is_none() {
            let initial = self.slug.to_string();
            let label: SharedString = tr(cx, "Workspace URL", "工作区地址").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(label, InputMode::Single, w, cx);
                e.set_placeholder("acme");
                e.font_size = 13.;
                e.set_value(initial, w, cx);
                e
            });
            cx.subscribe(&ed, |this, e, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => {
                    let v = e.read(cx).value().trim().to_string();
                    if !v.is_empty() {
                        this.slug_customized = true;
                        this.slug = workspace_slug(&v).into();
                    }
                    cx.notify();
                }
                EditEvent::Submitted => {
                    this.advance_step(cx);
                }
                _ => {}
            })
            .detach();
            self.slug_editor = Some(ed);
        }

        if self.invite_editor.is_none() {
            let ph: SharedString = tr(cx, "colleague@acme.com", "colleague@acme.com").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(
                    tr(cx, "Invite by email", "通过邮箱邀请"),
                    InputMode::Single,
                    w,
                    cx,
                );
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |this, e, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed => {
                    let val = e.read(cx).value().to_string();
                    if val.ends_with(',') || val.ends_with(' ') {
                        let candidate = val.trim_end_matches([',', ' ']).trim().to_string();
                        if !candidate.is_empty() && this.try_add_invite(&candidate, cx).is_ok() {
                            e.update(cx, |ed, cx| ed.set_value("", w, cx));
                        }
                    } else if this.invite_error.is_some() {
                        this.invite_error = None;
                        cx.notify();
                    }
                }
                EditEvent::Submitted => {
                    let val = e.read(cx).value().trim().to_string();
                    if val.is_empty() {
                        this.advance_step(cx);
                    } else if this.try_add_invite(&val, cx).is_ok() {
                        e.update(cx, |ed, cx| ed.set_value("", w, cx));
                    }
                }
                _ => {}
            })
            .detach();
            self.invite_editor = Some(ed);
        }
    }

    fn step_display_state(&self, step: OnboardingStep) -> OnboardingStepState {
        let idx = step.index();
        if !self.completed && self.current_step == step {
            OnboardingStepState::Current
        } else {
            self.step_states[idx]
        }
    }

    fn step_label(&self, step: OnboardingStep, cx: &App) -> SharedString {
        match step {
            OnboardingStep::Workspace => tr(cx, "Workspace", "工作区设置").into(),
            OnboardingStep::Tools => tr(cx, "Connect your tools", "连接常用工具").into(),
            OnboardingStep::InviteTeam => tr(cx, "Invite your team", "邀请团队成员").into(),
            OnboardingStep::FirstAgent => tr(cx, "Your first agent", "创建首个智能体").into(),
        }
    }

    fn step_sublabel(
        &self,
        step: OnboardingStep,
        state: OnboardingStepState,
        cx: &App,
    ) -> SharedString {
        match state {
            OnboardingStepState::Current => tr(cx, "In progress", "进行中").into(),
            OnboardingStepState::Skipped => tr(cx, "Skipped", "已跳过").into(),
            OnboardingStepState::Todo => match step {
                OnboardingStep::Workspace => tr(cx, "About 30 sec", "约 30 秒").into(),
                OnboardingStep::Tools => tr(cx, "Under a minute", "不到 1 分钟").into(),
                OnboardingStep::InviteTeam => tr(cx, "Under a minute", "不到 1 分钟").into(),
                OnboardingStep::FirstAgent => tr(cx, "About 1 min", "约 1 分钟").into(),
            },
            OnboardingStepState::Done => match step {
                OnboardingStep::Workspace => {
                    format!("{} · {}", self.workspace, self.team_size.label(cx)).into()
                }
                OnboardingStep::Tools => {
                    let count = self.tools.iter().filter(|t| t.connected).count();
                    if count == 0 {
                        tr(cx, "No tools yet", "暂未连接").into()
                    } else {
                        format!("{count} {}", tr(cx, "connected", "项已连接")).into()
                    }
                }
                OnboardingStep::InviteTeam => {
                    let n = self.invites.len();
                    if n == 0 {
                        tr(cx, "Link ready", "链接已就绪").into()
                    } else {
                        format!("{n} {}", tr(cx, "invited", "人已邀请")).into()
                    }
                }
                OnboardingStep::FirstAgent => self
                    .templates
                    .iter()
                    .find(|t| t.id == self.selected_template)
                    .map(|t| t.name.clone())
                    .unwrap_or_else(|| tr(cx, "Support triage", "工单分诊助手").into()),
            },
        }
    }

    fn render_tool_logo(tool_name: &str, cx: &App) -> AnyElement {
        let lower = tool_name.trim().to_ascii_lowercase();
        let inner = if lower == "notion" {
            pro_svg("0 0 24 24", NOTION_SVG, 15., rgb(0x171717).into())
        } else if lower == "linear" {
            pro_svg("0 0 24 24", LINEAR_SVG, 15., rgb(0x5e6ad2).into())
        } else if let Some(art) = pro_app_artwork(tool_name, 15., cx) {
            art
        } else {
            pro_icon(ProIcon::Grid, 15., rgb(0x171717).into())
        };
        div()
            .size(px(28.))
            .flex_none()
            .rounded(px(8.))
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(hsla(0., 0., 0., 0.10))
            .shadow(vec![BoxShadow {
                color: hsla(0., 0., 0., 0.06),
                offset: point(px(0.), px(1.)),
                blur_radius: px(1.5),
                spread_radius: px(0.),
                inset: false,
            }])
            .flex()
            .items_center()
            .justify_center()
            .child(inner)
            .into_any_element()
    }

    fn render_tool_switch(&self, tool: &OnboardingTool, cx: &Context<Self>) -> Stateful<Div> {
        let checked = tool.connected;
        let tool_id = tool.id.clone();
        pro_switch(
            pro_eid(&self.id, format!("swc-{}", tool.id)),
            format!("{} {}", tr(cx, "Connect", "连接"), tool.name),
            checked,
            cx,
            move |this, _, cx| {
                this.toggle_tool(&tool_id, cx);
            },
        )
    }

    fn render_checklist_aside(
        &self,
        stacked: bool,
        mobile: bool,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let first_name = first_name_of(&self.user_name).to_owned();
        let done = self.done_count();
        let progress = (done as f32 / 4.0).clamp(0., 1.);
        let pct_spring = pro_spring_f32(
            pro_eid(&self.id, "check-pct-sp"),
            progress * 100.,
            0.26,
            w,
            cx,
        )
        .clamp(0., 100.);

        let step_sensor = pro_selection_sensor();
        let active_step_key: SharedString = if self.completed {
            "completed".into()
        } else {
            format!("step-{}", self.current_step.number()).into()
        };

        let mut list = div()
            .relative()
            .flex()
            .flex_col()
            .gap(px(1.))
            .when(stacked && !mobile, |d| d.flex_row().flex_wrap().gap(px(6.)))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "check-pill"),
                active_step_key,
                step_sensor.clone(),
                9.,
                p.hover,
                None,
                false,
            ));

        for step in OnboardingStep::ALL {
            let state = self.step_display_state(step);
            let is_current = state == OnboardingStepState::Current;
            let label = self.step_label(step, cx);
            let sub = self.step_sublabel(step, state, cx);

            let mark = match state {
                OnboardingStepState::Done => div()
                    .size(px(16.))
                    .mt(px(1.))
                    .flex_none()
                    .rounded_full()
                    .bg(p.ink.opacity(0.09))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_svg("0 0 24 24", BOLD_CHECK_SVG, 11., p.ink)),
                OnboardingStepState::Current => div()
                    .size(px(16.))
                    .mt(px(1.))
                    .flex_none()
                    .rounded_full()
                    .border_2()
                    .border_color(p.ink),
                OnboardingStepState::Skipped => div()
                    .size(px(16.))
                    .mt(px(1.))
                    .flex_none()
                    .rounded_full()
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().w(px(6.)).h(px(1.5)).rounded(px(1.)).bg(p.faint)),
                OnboardingStepState::Todo => div()
                    .size(px(16.))
                    .mt(px(1.))
                    .flex_none()
                    .rounded_full()
                    .border_1()
                    .border_color(p.line_strong),
            };

            let label_color = match state {
                OnboardingStepState::Current => p.ink,
                OnboardingStepState::Done => p.muted,
                OnboardingStepState::Todo | OnboardingStepState::Skipped => p.text,
            };
            let label_weight = match state {
                OnboardingStepState::Current | OnboardingStepState::Done => FontWeight::MEDIUM,
                OnboardingStepState::Todo | OnboardingStepState::Skipped => FontWeight::NORMAL,
            };
            let sub_color = if state == OnboardingStepState::Todo {
                p.faint
            } else {
                p.muted
            };

            let sub_tx = pro_view_transition(
                pro_eid(&self.id, format!("step-sub-{}", step.number())),
                sub.clone(),
                div()
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(sub_color)
                    .truncate()
                    .child(sub.clone()),
            );

            let item_btn = activate(
                div()
                    .id(pro_eid(&self.id, format!("check-step-{}", step.number())))
                    .role(Role::Button)
                    .aria_label(format!("{label} — {sub}"))
                    .tab_index(0)
                    .relative()
                    .when(stacked && !mobile, |d| d.w(px(260.)))
                    .when(!(stacked && !mobile), |d| d.w_full())
                    .p(px(8.))
                    .rounded(px(9.))
                    .when(!is_current, |d| {
                        d.hover(move |s| s.bg(p.hover.opacity(0.65)))
                    })
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .cursor_pointer()
                    .flex()
                    .items_start()
                    .gap(px(10.))
                    .when(is_current, |d| d.child(pro_mark_selected(&step_sensor)))
                    .child(mark)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(1.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(18.))
                                    .font_weight(label_weight)
                                    .text_color(label_color)
                                    .child(label),
                            )
                            .child(sub_tx),
                    ),
                cx,
                move |this, _, cx| {
                    this.jump_to_step(step, cx);
                },
            );
            list = list.child(item_btn);
        }

        let welcome_heading: SharedString = if tr(cx, "en", "zh") == "zh" {
            format!("欢迎来到 {}，{}", self.workspace, first_name).into()
        } else {
            format!("Welcome to {}, {}", self.workspace, first_name).into()
        };

        div()
            .when(stacked, |d| d.w_full().max_w(px(600.)))
            .when(!stacked, |d| d.w(px(248.)).flex_none().pt(px(8.)))
            .flex()
            .flex_col()
            .gap(px(20.))
            // Greeting header
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Getting started", "快速开始")),
                    )
                    .child(pro_view_transition(
                        pro_eid(&self.id, "welcome-head-tx"),
                        self.workspace.clone(),
                        div()
                            .mt(px(6.))
                            .text_size(px(20.))
                            .line_height(px(26.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(welcome_heading),
                    ))
                    .child(
                        div()
                            .mt(px(4.))
                            .text_size(px(13.))
                            .line_height(px(19.))
                            .text_color(p.muted)
                            .child(tr(
                                cx,
                                "A few quick steps – about two minutes.",
                                "只需几个简单步骤，约两分钟完成。",
                            )),
                    ),
            )
            // Checklist progress bar + count + step buttons
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(10.))
                    .child(
                        div()
                            .px(px(8.))
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(pro_spring_bar(
                                pro_eid(&self.id, "check-pbar"),
                                progress,
                                3.,
                                999.,
                                p.chip,
                                p.ink,
                            ))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(3.))
                                            .text_size(px(12.))
                                            .child(
                                                div()
                                                    .text_color(p.ink)
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child(format!("{done}")),
                                            )
                                            .child(div().text_color(p.muted).child(tr(
                                                cx,
                                                "of 4 done",
                                                "/ 4 已完成",
                                            )))
                                            .child(div().text_color(p.faint).child(format!(
                                                "· {}%",
                                                pct_spring.round() as u32
                                            ))),
                                    )
                                    .child(pro_progress_ring(progress, 14., 1.8, p.ink, p.line)),
                            ),
                    )
                    .child(list),
            )
            // Need a hand? card (hidden when stacked per responsive spec)
            .when(!stacked, |d| {
                d.child(activate(
                    div()
                        .id(pro_eid(&self.id, "book-call"))
                        .role(Role::Button)
                        .tab_index(0)
                        .p(px(8.))
                        .rounded(px(12.))
                        .hover(move |s| s.bg(p.hover))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(
                            div()
                                .size(px(28.))
                                .flex_none()
                                .rounded(px(8.))
                                .bg(p.chip)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(pro_svg("0 0 24 24", HEADSET_SVG, 15., p.text)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(1.))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(tr(cx, "Need a hand?", "需要帮助？")),
                                )
                                .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                                    cx,
                                    "Book a free 15-minute setup call",
                                    "预约 15 分钟免费配置指导",
                                ))),
                        ),
                    cx,
                    |_, _, cx| {
                        cx.emit(OnboardingEvent::BookCall);
                    },
                ))
            })
    }

    fn render_step_head(
        &self,
        step_num: u8,
        title: SharedString,
        subtitle: SharedString,
        mobile: bool,
        cx: &App,
    ) -> Div {
        let p = ProPalette::get(cx);
        let kicker: SharedString = if tr(cx, "en", "zh") == "zh" {
            format!("第 {step_num} 步，共 4 步").into()
        } else {
            format!("Step {step_num} of 4").into()
        };
        div()
            .mb(px(20.))
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.muted)
                    .child(kicker),
            )
            .child(
                div()
                    .mt(px(6.))
                    .text_size(px(if mobile { 20. } else { 22. }))
                    .line_height(px(if mobile { 26. } else { 30. }))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(title),
            )
            .child(
                div()
                    .mt(px(4.))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.muted)
                    .child(subtitle),
            )
    }

    fn render_step_footer(
        &self,
        can_back: bool,
        primary_label: SharedString,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let skip_btn = activate(
            div()
                .id(pro_eid(&self.id, "skip-step"))
                .role(Role::Button)
                .tab_index(0)
                .py(px(4.))
                .rounded(px(6.))
                .text_size(px(13.))
                .text_color(p.muted)
                .hover(move |s| s.text_color(p.ink))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .cursor_pointer()
                .child(tr(cx, "Skip for now", "暂时跳过")),
            cx,
            |this, _, cx| {
                this.skip_current_step(cx);
            },
        );

        let back_btn = can_back.then(|| {
            activate(
                div()
                    .id(pro_eid(&self.id, "step-back"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(28.))
                    .pl(px(8.))
                    .pr(px(10.))
                    .rounded(px(6.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .hover(move |s| s.border_color(p.line_strong).bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(pro_icon(ProIcon::ChevronLeft, 14., p.muted))
                    .child(tr(cx, "Back", "上一步")),
                cx,
                |this, _, cx| {
                    this.previous_step(cx);
                },
            )
        });

        let next_btn = activate(
            div()
                .id(pro_eid(&self.id, "step-continue"))
                .role(Role::Button)
                .tab_index(0)
                .h(px(28.))
                .pl(px(10.))
                .pr(px(8.))
                .rounded(px(6.))
                .bg(p.primary)
                .hover(move |s| s.bg(p.primary.opacity(0.88)))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .cursor_pointer()
                .flex()
                .items_center()
                .gap(px(5.))
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.on_primary)
                .child(primary_label)
                .child(pro_icon(ProIcon::ChevronRight, 14., p.on_primary)),
            cx,
            |this, _, cx| {
                this.advance_step(cx);
            },
        );

        div()
            .mt(px(24.))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .child(skip_btn)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .when_some(back_btn, |d, b| d.child(b))
                    .child(next_btn),
            )
    }

    fn render_workspace_step(&self, mobile: bool, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let ws_ed = self
            .workspace_editor
            .clone()
            .map(|e| e.into_any_element())
            .unwrap_or_else(|| div().into_any_element());
        let slug_ed = self
            .slug_editor
            .clone()
            .map(|e| e.into_any_element())
            .unwrap_or_else(|| div().into_any_element());

        let sz_sensor = pro_selection_sensor();
        let sz_key: SharedString = format!("sz-{}", self.team_size as u8).into();
        let mut size_pills = div()
            .relative()
            .p(px(3.))
            .rounded(px(10.))
            .bg(p.chip)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_wrap()
            .gap(px(4.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "team-sz-pill"),
                sz_key.clone(),
                sz_sensor.clone(),
                7.,
                p.card,
                Some(p.ink),
                !p.dark,
            ));
        for sz in TeamSize::ALL {
            let selected = self.team_size == sz;
            let lbl = sz.label(cx);
            size_pills = size_pills.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("team-sz-{}", sz as u8)))
                    .role(Role::RadioButton)
                    .aria_toggled(selected.into())
                    .aria_label(lbl)
                    .tab_index(0)
                    .relative()
                    .h(px(30.))
                    .px(px(12.))
                    .rounded(px(7.))
                    .when(!selected, |d| d.hover(move |s| s.text_color(p.ink)))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .text_size(px(13.))
                    .font_weight(if selected {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if selected { p.ink } else { p.text })
                    .when(selected, |d| d.child(pro_mark_selected(&sz_sensor)))
                    .child(lbl),
                cx,
                move |this, _, cx| {
                    this.team_size = sz;
                    cx.notify();
                },
            ));
        }

        let team_note_txt: SharedString = match self.team_size {
            TeamSize::Solo => tr(
                cx,
                "Private workspace defaults; invite collaborators anytime later.",
                "默认开启私有工作区，随时可邀请成员加入。",
            )
            .into(),
            TeamSize::Small => tr(
                cx,
                "Shared folders and member-level tool permissions enabled for 2–10 seats.",
                "已为 2–10 人团队启用共享文件夹与成员工具权限。",
            )
            .into(),
            TeamSize::Medium => tr(
                cx,
                "Role-based access and workspace audit logs ready for 11–50 teammates.",
                "已为 11–50 人团队准备角色权限控制与工作区审计日志。",
            )
            .into(),
            TeamSize::Large => tr(
                cx,
                "SSO-ready permissions and departmental folder controls enabled for 50+ seats.",
                "已为 50 人以上团队启用部门级目录与权限管控。",
            )
            .into(),
        };
        let team_note_el = div()
            .px(px(12.))
            .py(px(9.))
            .rounded(px(9.))
            .bg(p.subtle)
            .border_1()
            .border_color(p.line)
            .child(pro_view_transition(
                pro_eid(&self.id, "team-note-tx"),
                sz_key,
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.5))
                    .line_height(px(18.))
                    .text_color(p.muted)
                    .child(pro_icon(ProIcon::Sparkles, 13., p.blue))
                    .child(team_note_txt),
            ))
            .into_any_element();

        let live_url_txt: SharedString = format!("https://app.kui.dev/{}", self.slug).into();

        div()
            .flex()
            .flex_col()
            .child(
                self.render_step_head(
                    1,
                    tr(cx, "Name your workspace", "设置工作区名称").into(),
                    tr(
                        cx,
                        "This appears in your sidebar, shared artifacts, and teammate invites.",
                        "该名称将显示在侧边栏、共享产物及成员邀请中。",
                    )
                    .into(),
                    mobile,
                    cx,
                ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(20.))
                    // Workspace name
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(7.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(tr(cx, "Workspace name", "工作区名称")),
                            )
                            .child(
                                div()
                                    .h(px(36.))
                                    .px(px(11.))
                                    .rounded(px(9.))
                                    .bg(p.card)
                                    .border_1()
                                    .border_color(p.line_strong)
                                    .flex()
                                    .items_center()
                                    .child(ws_ed),
                            ),
                    )
                    // Workspace URL
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(7.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(tr(cx, "Workspace URL", "工作区地址")),
                            )
                            .child(
                                div()
                                    .h(px(36.))
                                    .px(px(11.))
                                    .rounded(px(9.))
                                    .bg(p.card)
                                    .border_1()
                                    .border_color(p.line_strong)
                                    .flex()
                                    .items_center()
                                    .gap(px(2.))
                                    .child(
                                        div()
                                            .flex_none()
                                            .text_size(px(13.))
                                            .text_color(p.muted)
                                            .child("app.kui.dev/"),
                                    )
                                    .child(div().flex_1().min_w_0().h_full().child(slug_ed)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(8.))
                                    .text_size(px(12.5))
                                    .text_color(p.muted)
                                    .child(tr(
                                        cx,
                                        "Used in shared links and agent webhooks.",
                                        "用于共享链接与智能体回调地址。",
                                    ))
                                    .child(
                                        div().flex_none().child(pro_view_transition(
                                            pro_eid(&self.id, "slug-live-tx"),
                                            self.slug.clone(),
                                            div()
                                                .px(px(6.))
                                                .py(px(1.))
                                                .rounded(px(5.))
                                                .bg(p.chip)
                                                .text_size(px(11.5))
                                                .text_color(p.text)
                                                .child(live_url_txt),
                                        )),
                                    ),
                            ),
                    )
                    // Team size
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(7.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(tr(cx, "How many people on your team?", "团队规模")),
                            )
                            .child(size_pills)
                            .child(pro_fold(
                                pro_eid(&self.id, "team-collab-fold"),
                                self.team_size != TeamSize::Solo,
                                42.,
                                7.,
                                move || team_note_el,
                            )),
                    ),
            )
            .child(self.render_step_footer(false, tr(cx, "Continue", "继续").into(), cx))
    }

    fn render_tools_step(&self, mobile: bool, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let mut tools_card = div()
            .py(px(4.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .shadow(vec![BoxShadow {
                color: hsla(0., 0., 0., if p.dark { 0.35 } else { 0.05 }),
                offset: point(px(0.), px(1.)),
                blur_radius: px(2.),
                spread_radius: px(0.),
                inset: false,
            }])
            .flex()
            .flex_col();

        let connected_names: Vec<SharedString> = self
            .tools
            .iter()
            .filter(|t| t.connected)
            .map(|t| t.name.clone())
            .collect();
        let connected_count = connected_names.len();

        for (idx, tool) in self.tools.iter().enumerate() {
            let swc = self.render_tool_switch(tool, cx);
            let logo = Self::render_tool_logo(&tool.name, cx);
            let connected = tool.connected;
            let conn_lbl: SharedString = tr(cx, "Connected", "已连接").into();
            let green_soft = p.green_soft;
            let green_ink = p.green_ink;
            let conn_badge = pro_presence(
                pro_eid(&self.id, format!("tool-conn-{}", tool.id)),
                connected,
                0.20,
                0.16,
                move |amt| {
                    div()
                        .relative()
                        .top(px(2. * (1. - amt)))
                        .opacity(amt)
                        .px(px(7.))
                        .py(px(1.))
                        .rounded(px(6.))
                        .bg(green_soft)
                        .text_size(px(11.5))
                        .line_height(px(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(green_ink)
                        .child(conn_lbl)
                        .into_any_element()
                },
            );
            let row = div()
                .min_h(px(56.))
                .px(px(14.))
                .when(mobile, |d| d.py(px(8.)))
                .when(idx > 0, |d| d.border_t_1().border_color(p.line_soft))
                .flex()
                .items_center()
                .gap(px(12.))
                .child(logo)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(18.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(tool.name.clone()),
                                )
                                .child(conn_badge),
                        )
                        .child(
                            div()
                                .text_size(px(12.5))
                                .line_height(px(18.))
                                .text_color(p.muted)
                                .when(!mobile, |d| d.truncate())
                                .child(tool.description.clone()),
                        ),
                )
                .child(swc);
            tools_card = tools_card.child(row);
        }

        let summary_key: SharedString = connected_names
            .iter()
            .map(SharedString::as_ref)
            .collect::<Vec<_>>()
            .join(",")
            .into();
        let summary_txt: SharedString = if tr(cx, "en", "zh") == "zh" {
            format!(
                "已就绪 {connected_count} 项集成：{}",
                connected_names
                    .iter()
                    .map(SharedString::as_ref)
                    .collect::<Vec<_>>()
                    .join(" · ")
            )
            .into()
        } else {
            format!(
                "{connected_count} {} ready for agents: {}",
                if connected_count == 1 { "app" } else { "apps" },
                connected_names
                    .iter()
                    .map(SharedString::as_ref)
                    .collect::<Vec<_>>()
                    .join(" · ")
            )
            .into()
        };
        let tools_summary_el = div()
            .mt(px(10.))
            .px(px(12.))
            .py(px(8.))
            .rounded(px(9.))
            .bg(p.subtle)
            .border_1()
            .border_color(p.line)
            .child(pro_view_transition(
                pro_eid(&self.id, "tools-sum-tx"),
                summary_key,
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.5))
                    .text_color(p.muted)
                    .child(div().size(px(6.)).rounded_full().bg(p.green))
                    .child(div().flex_1().min_w_0().truncate().child(summary_txt)),
            ))
            .into_any_element();

        div()
            .flex()
            .flex_col()
            .child(self.render_step_head(
                2,
                tr(cx, "Connect your tools", "连接常用工具").into(),
                tr(
                    cx,
                    "Your agents can read from and act in the apps you connect. Change this anytime.",
                    "智能体可在已连接的应用中读取上下文并执行操作，随时可更改。",
                )
                .into(),
                mobile,
                cx,
            ))
            .child(tools_card)
            .child(pro_fold(
                pro_eid(&self.id, "tools-ready-fold"),
                connected_count > 0,
                38.,
                0.,
                move || tools_summary_el,
            ))
            .child(self.render_step_footer(
                true,
                tr(cx, "Continue", "继续").into(),
                cx,
            ))
    }

    fn render_invites_step(&self, mobile: bool, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let inv_ed = self
            .invite_editor
            .clone()
            .map(|e| e.into_any_element())
            .unwrap_or_else(|| div().into_any_element());

        let mut chips_wrap = div()
            .min_h(px(36.))
            .px(px(8.))
            .py(px(5.))
            .flex_1()
            .min_w_0()
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(if self.invite_error.is_some() {
                p.red
            } else {
                p.line_strong
            })
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(6.));

        for (idx, email) in self.invites.iter().enumerate() {
            let email_label = email.clone();
            chips_wrap = chips_wrap.child(
                div()
                    .h(px(24.))
                    .pl(px(8.))
                    .pr(px(4.))
                    .rounded(px(6.))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(email_label.clone())
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("rm-invite-{idx}")))
                            .role(Role::Button)
                            .aria_label(format!("{} {email_label}", tr(cx, "Remove", "移除")))
                            .tab_index(0)
                            .size(px(18.))
                            .rounded(px(4.))
                            .hover(move |s| s.bg(p.hover))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_icon(ProIcon::Close, 11., p.muted)),
                        cx,
                        move |this, _, cx| {
                            this.remove_invite(idx, cx);
                        },
                    )),
            );
        }

        chips_wrap = chips_wrap.child(
            div()
                .flex_1()
                .min_w(px(140.))
                .h(px(24.))
                .flex()
                .items_center()
                .child(inv_ed),
        );

        let role_trigger = activate(
            div()
                .id(pro_eid(&self.id, "role-trigger"))
                .role(Role::Button)
                .aria_label(tr(cx, "Invite role", "邀请角色"))
                .tab_index(0)
                .h(px(36.))
                .when(mobile, |d| d.w_full())
                .when(!mobile, |d| d.min_w(px(104.)))
                .px(px(10.))
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .hover(move |s| s.bg(p.hover))
                .focus_visible(move |s| s.border_1().border_color(p.focus))
                .cursor_pointer()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(6.))
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink)
                .child(pro_view_transition(
                    pro_eid(&self.id, "role-lbl-tx"),
                    format!("r-{}", self.role as u8),
                    div().child(self.role.label(cx)),
                ))
                .child(pro_icon(
                    if self.role_menu_open {
                        ProIcon::ChevronUp
                    } else {
                        ProIcon::ChevronDown
                    },
                    13.,
                    p.faint,
                )),
            cx,
            |this, _, cx| {
                this.role_menu_open = !this.role_menu_open;
                cx.notify();
            },
        );

        let role_sensor = pro_selection_sensor();
        let role_menu_div = div()
            .id(pro_eid(&self.id, "role-menu"))
            .role(Role::Menu)
            .absolute()
            .right_0()
            .w(px(148.))
            .p(px(4.))
            .rounded(px(10.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow(vec![BoxShadow {
                color: hsla(0., 0., 0., if p.dark { 0.5 } else { 0.14 }),
                offset: point(px(0.), px(8.)),
                blur_radius: px(20.),
                spread_radius: px(-4.),
                inset: false,
            }])
            .flex()
            .flex_col()
            .gap(px(1.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "role-menu-pill"),
                format!("role-{}", self.role as u8),
                role_sensor.clone(),
                7.,
                p.hover,
                None,
                false,
            ))
            .children(InviteRole::ALL.into_iter().map(|r| {
                let active = self.role == r;
                let label = r.label(cx);
                activate(
                    div()
                        .id(pro_eid(&self.id, format!("role-opt-{}", r as u8)))
                        .role(Role::MenuItem)
                        .tab_index(0)
                        .relative()
                        .h(px(30.))
                        .px(px(10.))
                        .rounded(px(7.))
                        .when(!active, |d| d.hover(move |s| s.bg(p.hover.opacity(0.6))))
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .justify_between()
                        .text_size(px(13.))
                        .font_weight(if active {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(if active { p.ink } else { p.text })
                        .when(active, |d| d.child(pro_mark_selected(&role_sensor)))
                        .child(label)
                        .when(active, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink))),
                    cx,
                    move |this, _, cx| {
                        this.role = r;
                        this.role_menu_open = false;
                        cx.emit(OnboardingEvent::RoleChanged(r));
                        cx.notify();
                    },
                )
            }));

        let err_msg = self
            .invite_error
            .clone()
            .unwrap_or_else(|| self.last_invite_error.clone());
        let red_ink = p.red;
        let err_fold = pro_fold(
            pro_eid(&self.id, "inv-err-fold"),
            self.invite_error.is_some(),
            18.,
            7.,
            move || {
                div()
                    .text_size(px(12.))
                    .text_color(red_ink)
                    .child(err_msg)
                    .into_any_element()
            },
        );

        let inv_count = self.invites.len();
        let role_name = self.role.label(cx);
        let queued_key: SharedString = format!("{inv_count}-{}", self.role as u8).into();
        let queued_txt: SharedString = if tr(cx, "en", "zh") == "zh" {
            format!("待发送 {inv_count} 封邀请 · 默认角色：{role_name}").into()
        } else {
            format!(
                "{inv_count} {} queued · {} access",
                if inv_count == 1 {
                    "teammate"
                } else {
                    "teammates"
                },
                role_name
            )
            .into()
        };
        let queued_el = div()
            .px(px(12.))
            .py(px(8.))
            .rounded(px(9.))
            .bg(p.subtle)
            .border_1()
            .border_color(p.line)
            .child(pro_view_transition(
                pro_eid(&self.id, "inv-queued-tx"),
                queued_key,
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.5))
                    .text_color(p.muted)
                    .child(pro_icon(ProIcon::Check, 13., p.green_ink))
                    .child(queued_txt),
            ))
            .into_any_element();

        let copied = self
            .link_copied_at
            .is_some_and(|t| Instant::now().duration_since(t).as_secs_f32() < 2.2);
        let join_url = format!("https://app.kui.dev/join/{}", self.slug);
        let join_url_copy = join_url.clone();

        let link_card = div()
            .p(px(12.))
            .pl(px(14.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(12.))
            .child(
                div()
                    .size(px(28.))
                    .flex_none()
                    .rounded(px(8.))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_icon(ProIcon::Link, 15., p.text)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tr(cx, "Invite link", "邀请链接")),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .text_color(p.muted)
                            .truncate()
                            .child(join_url),
                    ),
            )
            .child(pro_btn(
                pro_eid(&self.id, "copy-join-link"),
                if copied {
                    tr(cx, "Copied", "已复制")
                } else {
                    tr(cx, "Copy link", "复制链接")
                },
                Some(if copied {
                    ProIcon::Check
                } else {
                    ProIcon::Copy
                }),
                ProBtnStyle::Secondary,
                true,
                cx,
                move |this, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(join_url_copy.clone()));
                    this.link_copied_at = Some(Instant::now());
                    cx.notify();
                },
            ));

        div()
            .capture_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                if ev.keystroke.key == "backspace"
                    && let Some(ed) = &this.invite_editor
                    && ed.read(cx).focus.is_focused(w)
                    && !ed.read(cx).is_composing()
                    && ed.read(cx).value().is_empty()
                    && !this.invites.is_empty()
                {
                    let last = this.invites.len() - 1;
                    this.remove_invite(last, cx);
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }))
            .flex()
            .flex_col()
            .child(
                self.render_step_head(
                    3,
                    tr(cx, "Invite your team", "邀请团队成员").into(),
                    tr(
                        cx,
                        "Share agents, folders, and connected tools with your teammates.",
                        "与团队成员共享智能体、文件夹及已连接的工具。",
                    )
                    .into(),
                    mobile,
                    cx,
                ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .child(
                        div()
                            .relative()
                            .flex()
                            .flex_col()
                            .gap(px(7.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(tr(cx, "Email addresses", "成员邮箱")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .when(mobile, |d| d.flex_col().items_stretch())
                                    .when(!mobile, |d| d.items_start())
                                    .gap(px(8.))
                                    .child(chips_wrap)
                                    .child(role_trigger),
                            )
                            .child(err_fold)
                            .when(self.invite_error.is_none(), |d| {
                                d.child(div().text_size(px(12.5)).text_color(p.muted).child(tr(
                                    cx,
                                    "Press Enter or comma to add multiple addresses.",
                                    "按回车或空格可连续添加多个邮箱。",
                                )))
                            })
                            .child(pro_presence(
                                pro_eid(&self.id, "role-menu-pres"),
                                self.role_menu_open,
                                0.18,
                                0.14,
                                move |amt| {
                                    role_menu_div
                                        .top(px(66. - 5. * (1. - amt)))
                                        .opacity(amt)
                                        .into_any_element()
                                },
                            )),
                    )
                    .child(pro_fold(
                        pro_eid(&self.id, "inv-queued-fold"),
                        inv_count > 0,
                        38.,
                        16.,
                        move || queued_el,
                    ))
                    .child(link_card),
            )
            .child(self.render_step_footer(true, tr(cx, "Continue", "继续").into(), cx))
    }

    fn render_first_agent_step(&self, mobile: bool, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let tpl_sensor = pro_selection_sensor();
        let mut grid = div()
            .relative()
            .flex()
            .when(mobile, |d| d.flex_col())
            .when(!mobile, |d| d.flex_row())
            .gap(px(12.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "tpl-pill"),
                self.selected_template.clone(),
                tpl_sensor.clone(),
                12.,
                p.card,
                Some(p.line_strong),
                !p.dark,
            ));

        for tpl in &self.templates {
            let checked = self.selected_template == tpl.id;
            let tpl_id = tpl.id.clone();
            let p_ink = p.ink;
            let p_on_primary = p.on_primary;
            let check_badge = pro_presence(
                pro_eid(&self.id, format!("tpl-chk-{}", tpl.id)),
                checked,
                0.18,
                0.14,
                move |amt| {
                    div()
                        .size(px(16.))
                        .rounded_full()
                        .bg(p_ink.opacity(amt))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_svg(
                            "0 0 24 24",
                            BOLD_CHECK_SVG,
                            10.,
                            p_on_primary.opacity(amt),
                        ))
                        .into_any_element()
                },
            );
            let card = activate(
                div()
                    .id(pro_eid(&self.id, format!("tpl-{}", tpl.id)))
                    .role(Role::RadioButton)
                    .aria_toggled(checked.into())
                    .aria_label(tpl.name.clone())
                    .tab_index(0)
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .p(px(14.))
                    .pb(px(13.))
                    .rounded(px(12.))
                    .when(!checked, |d| {
                        d.bg(p.subtle)
                            .border_1()
                            .border_color(p.line_soft)
                            .hover(move |s| s.bg(p.hover))
                    })
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .cursor_pointer()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap(px(4.))
                    .when(checked, |d| d.child(pro_mark_selected(&tpl_sensor)))
                    .child(
                        div()
                            .w_full()
                            .mb(px(10.))
                            .flex()
                            .items_start()
                            .justify_between()
                            .child(
                                div()
                                    .size(px(28.))
                                    .rounded(px(8.))
                                    .bg(if checked { p.chip } else { p.card })
                                    .when(!checked, |d| d.border_1().border_color(p.line))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(pro_icon(tpl.icon, 15., p.ink)),
                            )
                            .child(
                                div()
                                    .size(px(16.))
                                    .rounded_full()
                                    .when(!checked, |d| d.border_1().border_color(p.line_strong))
                                    .child(check_badge),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(18.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tpl.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .line_height(px(18.))
                            .text_color(p.muted)
                            .child(tpl.description.clone()),
                    )
                    .child(
                        div()
                            .mt_auto()
                            .pt(px(if mobile { 6. } else { 10. }))
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.faint)
                            .child(tpl.meta.clone()),
                    ),
                cx,
                move |this, _, cx| {
                    this.selected_template = tpl_id.clone();
                    cx.emit(OnboardingEvent::TemplateSelected(tpl_id.clone()));
                    cx.notify();
                },
            );
            grid = grid.child(card);
        }

        let active_tpl = self
            .templates
            .iter()
            .find(|t| t.id == self.selected_template)
            .or_else(|| self.templates.first());
        let tpl_preview = active_tpl.map(|tpl| {
            div()
                .mt(px(12.))
                .px(px(14.))
                .py(px(10.))
                .rounded(px(10.))
                .bg(p.subtle)
                .border_1()
                .border_color(p.line)
                .child(pro_view_transition(
                    pro_eid(&self.id, "tpl-preview-tx"),
                    tpl.id.clone(),
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(10.))
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(pro_icon(tpl.icon, 14., p.blue))
                                .child(
                                    div()
                                        .text_size(px(12.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(tpl.name.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(format!("· {}", tpl.meta)),
                                ),
                        )
                        .child(
                            div()
                                .flex_none()
                                .px(px(7.))
                                .py(px(1.))
                                .rounded(px(5.))
                                .bg(p.green_soft)
                                .text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.green_ink)
                                .child(tr(cx, "Starter ready", "模板就绪")),
                        ),
                ))
        });

        div()
            .flex()
            .flex_col()
            .child(self.render_step_head(
                4,
                tr(cx, "Create your first agent", "创建首个智能体").into(),
                tr(
                    cx,
                    "Pick a starter workflow. You can edit its prompt, triggers, and tools right after.",
                    "选择一个起始工作流，稍后即可自由调整提示词、触发器与工具。",
                )
                .into(),
                mobile,
                cx,
            ))
            .child(grid)
            .when_some(tpl_preview, |d, el| d.child(el))
            .child(self.render_step_footer(
                true,
                tr(cx, "Create agent", "创建智能体").into(),
                cx,
            ))
    }

    fn render_success_view(&self, cx: &mut Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let first_name = first_name_of(&self.user_name);
        let heading: SharedString = if tr(cx, "en", "zh") == "zh" {
            format!("一切就绪，{first_name}").into()
        } else {
            format!("You're all set, {first_name}").into()
        };
        let sub: SharedString = if tr(cx, "en", "zh") == "zh" {
            format!("{} 已配置完成，选择下一步开始使用。", self.workspace).into()
        } else {
            format!(
                "{} is ready. Pick where you'd like to start.",
                self.workspace
            )
            .into()
        };

        let next_items = [
            (
                "next-chat",
                ProIcon::Thread,
                SharedString::from(tr(cx, "Open Chat", "打开对话")),
                SharedString::from(tr(
                    cx,
                    "Ask your new agent its first question in the workspace.",
                    "在工作区向新智能体提出第一个问题。",
                )),
                SharedString::from("Chat"),
            ),
            (
                "next-integrations",
                ProIcon::Grid,
                SharedString::from(tr(cx, "Browse Integrations", "浏览集成")),
                SharedString::from(tr(
                    cx,
                    "Connect more data sources and configure tool permissions.",
                    "连接更多数据源并配置工具权限策略。",
                )),
                SharedString::from("Integrations"),
            ),
            (
                "next-analytics",
                ProIcon::UsageChart,
                SharedString::from(tr(cx, "View Analytics", "查看分析")),
                SharedString::from(tr(
                    cx,
                    "Monitor agent runs, response quality, and team usage.",
                    "追踪智能体运行情况、响应质量与团队用量。",
                )),
                SharedString::from("Analytics"),
            ),
        ];

        let mut next_card = div()
            .mt(px(20.))
            .py(px(4.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col();

        for (idx, (id_key, icon, title, desc, target)) in next_items.into_iter().enumerate() {
            let row = activate(
                div()
                    .id(pro_eid(&self.id, id_key))
                    .role(Role::Button)
                    .tab_index(0)
                    .min_h(px(56.))
                    .px(px(14.))
                    .py(px(8.))
                    .when(idx > 0, |d| d.border_t_1().border_color(p.line_soft))
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        div()
                            .size(px(28.))
                            .flex_none()
                            .rounded(px(8.))
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_icon(icon, 15., p.ink)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(1.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(18.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(title),
                            )
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .line_height(px(18.))
                                    .text_color(p.muted)
                                    .child(desc),
                            ),
                    )
                    .child(pro_icon(ProIcon::ChevronRight, 15., p.faint)),
                cx,
                move |_, _, cx| {
                    cx.emit(OnboardingEvent::Navigate(target.clone()));
                },
            );
            next_card = next_card.child(row);
        }

        div()
            .flex()
            .flex_col()
            .child(
                div()
                    .size(px(36.))
                    .mb(px(16.))
                    .rounded_full()
                    .bg(rgb(0x1e8a4f))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_svg(
                        "0 0 24 24",
                        BOLD_CHECK_SVG,
                        18.,
                        rgb(0xffffff).into(),
                    )),
            )
            .child(
                div()
                    .text_size(px(22.))
                    .line_height(px(30.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(heading),
            )
            .child(
                div()
                    .mt(px(4.))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.muted)
                    .child(sub),
            )
            .child(next_card)
    }

    fn render_skeleton(&self, stacked: bool, w: &mut Window, cx: &App) -> Div {
        let p = ProPalette::get(cx);
        let phase = if Theme::global(cx).reduced_motion {
            0.25
        } else {
            w.request_animation_frame();
            (self.started_at.elapsed().as_secs_f32() / 1.4).fract()
        };

        let aside_skel = div()
            .when(stacked, |d| d.w_full().max_w(px(600.)))
            .when(!stacked, |d| d.w(px(248.)).flex_none())
            .flex()
            .flex_col()
            .gap(px(18.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(pro_skeleton_bar(88., 11., 4., phase, cx))
                    .child(pro_skeleton_bar(196., 20., 6., phase + 0.08, cx))
                    .child(pro_skeleton_bar(172., 12., 4., phase + 0.14, cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .children((0..4).map(|i| {
                        div()
                            .p(px(8.))
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(pro_skeleton_bar(
                                16.,
                                16.,
                                999.,
                                phase + i as f32 * 0.06,
                                cx,
                            ))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(5.))
                                    .child(pro_skeleton_bar(
                                        118.,
                                        12.,
                                        4.,
                                        phase + i as f32 * 0.06,
                                        cx,
                                    ))
                                    .child(pro_skeleton_bar(
                                        80.,
                                        10.,
                                        4.,
                                        phase + i as f32 * 0.08,
                                        cx,
                                    )),
                            )
                    })),
            );

        let card_skel = div()
            .flex_1()
            .max_w(px(600.))
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(pro_skeleton_bar(74., 11., 4., phase, cx))
                    .child(pro_skeleton_bar(210., 22., 6., phase + 0.05, cx))
                    .child(pro_skeleton_bar(340., 12., 4., phase + 0.1, cx)),
            )
            .child(
                div()
                    .py(px(6.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .children((0..6).map(|i| {
                        div()
                            .h(px(56.))
                            .px(px(14.))
                            .when(i > 0, |d| d.border_t_1().border_color(p.line_soft))
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(pro_skeleton_bar(28., 28., 8., phase + i as f32 * 0.05, cx))
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(pro_skeleton_bar(
                                        104.,
                                        12.,
                                        4.,
                                        phase + i as f32 * 0.05,
                                        cx,
                                    ))
                                    .child(pro_skeleton_bar(
                                        236.,
                                        11.,
                                        4.,
                                        phase + i as f32 * 0.07,
                                        cx,
                                    )),
                            )
                            .child(pro_skeleton_bar(
                                32.,
                                20.,
                                999.,
                                phase + i as f32 * 0.05,
                                cx,
                            ))
                    })),
            );

        div()
            .w_full()
            .flex()
            .when(stacked, |d| d.flex_col().items_center().gap(px(28.)))
            .when(!stacked, |d| {
                d.flex_row().justify_center().items_start().gap(px(64.))
            })
            .pt(px(16.))
            .px(px(40.))
            .pb(px(64.))
            .child(aside_skel)
            .child(card_skel)
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);

        let measure_cell = self.measured_width.clone();
        let main_w = measure_cell.get();
        let stacked = main_w < 920.;
        let mobile = main_w < 620.;

        let toast = self.link_copied_at.map(|t| {
            (
                SharedString::from(tr(
                    cx,
                    "Invite link copied to clipboard",
                    "邀请链接已复制到剪贴板",
                )),
                t,
            )
        });

        // Top sticky breadcrumb bar (`Home / Getting started`)
        let top_bar = div()
            .h(px(48.))
            .flex_none()
            .px(px(20.))
            .bg(p.panel)
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(13.))
                    .whitespace_nowrap()
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "crumb-home"))
                            .role(Role::Button)
                            .tab_index(0)
                            .rounded(px(4.))
                            .text_color(p.muted)
                            .hover(move |s| s.text_color(p.ink))
                            .cursor_pointer()
                            .child(tr(cx, "Home", "首页")),
                        cx,
                        |_, _, cx| {
                            cx.emit(OnboardingEvent::Navigate("Home".into()));
                        },
                    ))
                    .child(div().text_color(p.faint).child("/"))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tr(cx, "Getting started", "快速开始")),
                    ),
            )
            .when(!self.empty && !self.loading, |d| {
                d.child(activate(
                    div()
                        .id(pro_eid(&self.id, "dismiss-setup"))
                        .role(Role::Button)
                        .aria_label(tr(cx, "Dismiss setup", "稍后设置"))
                        .tab_index(0)
                        .px(px(8.))
                        .h(px(26.))
                        .rounded(px(6.))
                        .hover(move |s| s.bg(p.hover).text_color(p.ink))
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .text_size(px(12.))
                        .text_color(p.faint)
                        .child(tr(cx, "Dismiss", "关闭向导")),
                    cx,
                    |this, _, cx| {
                        this.empty = true;
                        cx.emit(OnboardingEvent::Dismissed);
                        cx.notify();
                    },
                ))
            });

        let body = if self.loading {
            self.render_skeleton(stacked, w, cx)
        } else if self.empty {
            div()
                .w_full()
                .pt(px(48.))
                .child(pro_empty_state(
                    ProIcon::Sparkles,
                    tr(cx, "Workspace setup dismissed", "工作区向导已收起"),
                    tr(
                        cx,
                        "You can resume connecting tools, inviting teammates, or creating starter agents anytime.",
                        "随时可重新打开向导，继续连接工具、邀请成员或创建智能体。",
                    ),
                    Some((
                        pro_eid(&self.id, "restart-onboarding"),
                        tr(cx, "Resume setup", "继续配置").into(),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
        } else {
            let step_idx = if self.completed {
                4
            } else {
                self.current_step.index() as i32
            };
            let step_content = if self.completed {
                self.render_success_view(cx)
            } else {
                match self.current_step {
                    OnboardingStep::Workspace => self.render_workspace_step(mobile, cx),
                    OnboardingStep::Tools => self.render_tools_step(mobile, cx),
                    OnboardingStep::InviteTeam => self.render_invites_step(mobile, cx),
                    OnboardingStep::FirstAgent => self.render_first_agent_step(mobile, cx),
                }
            };

            let aside = self.render_checklist_aside(stacked, mobile, w, cx);
            let card =
                div()
                    .flex_1()
                    .w_full()
                    .max_w(px(600.))
                    .min_w_0()
                    .child(pro_step_transition(
                        pro_eid(&self.id, "wizard-step-tx"),
                        step_idx,
                        step_content,
                    ));

            div()
                .w_full()
                .flex()
                .when(stacked, |d| {
                    d.flex_col()
                        .items_center()
                        .gap(px(28.))
                        .pt(px(12.))
                        .px(px(if mobile { 16. } else { 32. }))
                        .pb(px(48.))
                })
                .when(!stacked, |d| {
                    d.flex_row()
                        .justify_center()
                        .items_start()
                        .gap(px(64.))
                        .pt(px(16.))
                        .px(px(40.))
                        .pb(px(64.))
                })
                .child(aside)
                .child(card)
        };

        div()
            .id(pro_eid(&self.id, "page"))
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .bg(p.panel)
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                let any_editor_focused = this
                    .workspace_editor
                    .as_ref()
                    .is_some_and(|e| e.read(cx).focus.is_focused(w))
                    || this
                        .slug_editor
                        .as_ref()
                        .is_some_and(|e| e.read(cx).focus.is_focused(w))
                    || this
                        .invite_editor
                        .as_ref()
                        .is_some_and(|e| e.read(cx).focus.is_focused(w));
                if any_editor_focused {
                    return;
                }
                match ev.keystroke.key.as_str() {
                    "1" => this.jump_to_step(OnboardingStep::Workspace, cx),
                    "2" => this.jump_to_step(OnboardingStep::Tools, cx),
                    "3" => this.jump_to_step(OnboardingStep::InviteTeam, cx),
                    "4" => this.jump_to_step(OnboardingStep::FirstAgent, cx),
                    "enter" => this.advance_step(cx),
                    "escape" if this.role_menu_open => {
                        this.role_menu_open = false;
                        cx.notify();
                    }
                    "[" => this.previous_step(cx),
                    "]" => this.advance_step(cx),
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
                .left_0()
                .right_0()
                .top_0()
                .h(px(0.)),
            )
            .child(top_bar)
            .child(
                div()
                    .id(pro_eid(&self.id, "scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(body),
            )
            .child(pro_toast_banner(toast))
    }
}

impl Render for OnboardingScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{OnboardingScreen, OnboardingStep, workspace_slug};
    use gpui::TestAppContext;

    #[test]
    fn workspace_slug_normalizes_punctuation_and_spaces() {
        assert_eq!(workspace_slug("Acme Studio"), "acme-studio");
        assert_eq!(
            workspace_slug("  Northwind_AI  Labs!! "),
            "northwind-ai-labs"
        );
        assert_eq!(workspace_slug("   "), "workspace");
    }

    #[test]
    fn step_progression_tool_toggle_and_invites_update_snapshot() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (screen, w) = app.add_window_view(|_w, cx| OnboardingScreen::new("test-onb", cx));

        w.update(|_w, cx| {
            screen.update(cx, |s, cx| {
                // Default step is Step 2 (Tools) with Step 1 already done and Slack connected.
                assert_eq!(s.current_step(), OnboardingStep::Tools);
                assert_eq!(s.done_count(), 1);
                assert_eq!(s.values().connected_tools, vec!["slack"]);

                // Toggle Notion on and Slack off.
                assert_eq!(s.toggle_tool("notion", cx), Some(true));
                assert_eq!(s.toggle_tool("slack", cx), Some(false));
                assert_eq!(s.values().connected_tools, vec!["notion"]);

                // Advance to Step 3 (InviteTeam), validate email syntax, and add a teammate.
                s.advance_step(cx);
                assert_eq!(s.current_step(), OnboardingStep::InviteTeam);
                assert_eq!(s.done_count(), 2);
                assert!(s.try_add_invite("not-an-email", cx).is_err());
                assert!(s.try_add_invite("Taylor@Acme.com", cx).is_ok());
                assert_eq!(s.values().invites.len(), 2);

                // Skip Step 3 and finish Step 4 into completion state.
                s.skip_current_step(cx);
                assert_eq!(s.current_step(), OnboardingStep::FirstAgent);
                s.advance_step(cx);
                assert!(s.is_completed());
                assert_eq!(s.done_count(), 3);
            });
        });
    }
}
