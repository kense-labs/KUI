//! Collapsible application frame with workspace sidebar, command search, and tabbed work area.
//!
//! Supports both `Inset` (rounded panels on a tinted stage) and `Flat` (edge-to-edge
//! columns split by hairlines) layouts, drag-resizable sidebars, keyboard-accessible
//! work-area tabs, and built-in navigation presets for AI chat, agent workspaces,
//! settings, and product tools.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_app_tile, pro_fold, pro_icon_btn, pro_mark_selected, pro_person_avatar,
        pro_pill_indicator, pro_presence, pro_selection_sensor, pro_spring_bar,
        pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{Editor, InputMode},
        family::Slide,
        pro_icons::{ProIcon, pro_icon},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};

/// Visual framing mode of [`AppShell`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppShellLayout {
    #[default]
    Inset,
    Flat,
}

/// Built-in sidebar navigation presets used across KUI Pro screens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppShellPreset {
    Chat,
    Agent,
    Settings,
    Issues,
    Crm,
}

/// One clickable row in the [`AppShell`] sidebar.
#[derive(Clone, Debug)]
pub struct ShellNavItem {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: ProIcon,
    pub count: Option<u32>,
    pub unread: bool,
}

impl ShellNavItem {
    /// Create a sidebar item with a stable `id`, display `label`, and leading `icon`.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>, icon: ProIcon) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            count: None,
            unread: false,
        }
    }

    /// Show a tabular numeric badge on the trailing edge of the row.
    pub fn with_count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    /// Mark the row with an accent unread dot.
    pub fn with_unread(mut self, unread: bool) -> Self {
        self.unread = unread;
        self
    }
}

/// Titled or untitled section of [`ShellNavItem`]s in the sidebar.
#[derive(Clone, Debug)]
pub struct ShellNavGroup {
    pub heading: Option<SharedString>,
    pub items: Vec<ShellNavItem>,
}

impl ShellNavGroup {
    /// Primary untitled navigation group at the top of the sidebar.
    pub fn primary(items: Vec<ShellNavItem>) -> Self {
        Self {
            heading: None,
            items,
        }
    }

    /// Named navigation group (`Projects`, `Workspace`, `Personal`, `Chats`).
    pub fn named(heading: impl Into<SharedString>, items: Vec<ShellNavItem>) -> Self {
        Self {
            heading: Some(heading.into()),
            items,
        }
    }
}

/// Built-in artifact or tool tab inside the right-hand work area.
#[derive(Clone, Debug)]
pub enum WorkTabKind {
    NewTab,
    Terminal,
    Browser {
        url: SharedString,
    },
    Notes {
        body: SharedString,
    },
    Document {
        source: SharedString,
        heading: SharedString,
        paragraphs: Vec<SharedString>,
        checklist: Vec<(SharedString, bool)>,
    },
    Sheet {
        source: SharedString,
        headers: Vec<SharedString>,
        rows: Vec<Vec<SharedString>>,
    },
}

/// One open tab in the [`AppShell`] work area.
#[derive(Clone, Debug)]
pub struct WorkTab {
    pub id: SharedString,
    pub title: SharedString,
    pub icon: ProIcon,
    pub kind: WorkTabKind,
}

/// Events emitted by [`AppShell`] when the user navigates or opens artifacts.
#[derive(Clone, Debug)]
pub enum AppShellEvent {
    Navigate(SharedString),
    Back,
    OpenTool(SharedString),
    SidebarCollapsed(bool),
    PanelOpen(bool),
}

impl EventEmitter<AppShellEvent> for AppShell {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DragEdge {
    Left,
    Right,
}

type BoundPanelFn =
    Rc<dyn Fn(&mut Window, &mut App) -> Option<(SharedString, ProIcon, AnyElement)>>;

/// Interactive state and layout configuration for [`AppShell`].
pub struct AppShell {
    id: ElementId,
    layout: AppShellLayout,
    workspace: SharedString,
    back_label: Option<SharedString>,
    user_name: SharedString,
    user_email: SharedString,
    active: SharedString,
    header_title: Option<SharedString>,
    show_header: bool,
    page_scroll: bool,
    groups: Vec<ShellNavGroup>,
    side_cards: bool,
    news_count: u8,
    guide_open: bool,
    guide_checks: [bool; 4],
    ws_menu_open: bool,
    user_menu_open: bool,
    cmd_open: bool,
    cmd_editor: Option<Entity<Editor>>,
    url_editor: Option<Entity<Editor>>,
    sidebar_collapsed: bool,
    sidebar_width: f32,
    side_w_slide: Slide,
    has_work_area: bool,
    panel_open: bool,
    panel_width: f32,
    panel_w_slide: Slide,
    panel_title: SharedString,
    panel_icon: ProIcon,
    bound_panel: Option<BoundPanelFn>,
    custom_panel: Option<crate::components::pro_chrome::ProSlotFn>,
    custom_page: Option<crate::components::pro_chrome::ProSlotFn>,
    tabs: Vec<WorkTab>,
    active_tab: usize,
    dragging: Option<(DragEdge, f32, f32)>,
    measured_width: Rc<Cell<f32>>,
    mobile_side_open: bool,
    slides_ready: bool,
}

impl AppShell {
    /// Construct an [`AppShell`] with the default `Acme` AI assistant navigation.
    pub fn new(id: impl Into<SharedString>, cx: &App) -> Self {
        let mut s = Self {
            id: ElementId::Name(id.into()),
            layout: AppShellLayout::Inset,
            workspace: "Acme".into(),
            back_label: None,
            user_name: "Alex Morgan".into(),
            user_email: "alex@acme.ai".into(),
            active: tr(cx, "New chat", "新建对话").into(),
            header_title: None,
            show_header: false,
            page_scroll: true,
            groups: Vec::new(),
            side_cards: true,
            news_count: 2,
            guide_open: false,
            guide_checks: [true, false, false, false],
            ws_menu_open: false,
            user_menu_open: false,
            cmd_open: false,
            cmd_editor: None,
            url_editor: None,
            sidebar_collapsed: false,
            sidebar_width: 272.,
            side_w_slide: Slide::new(272.),
            has_work_area: true,
            panel_open: true,
            panel_width: 360.,
            panel_w_slide: Slide::new(360.),
            panel_title: tr(cx, "Workspace", "工作区").into(),
            panel_icon: ProIcon::ClipboardDoc,
            bound_panel: None,
            custom_panel: None,
            custom_page: None,
            tabs: vec![WorkTab {
                id: "new-1".into(),
                title: tr(cx, "New tab", "新标签页").into(),
                icon: ProIcon::Globe,
                kind: WorkTabKind::NewTab,
            }],
            active_tab: 0,
            dragging: None,
            measured_width: Rc::new(Cell::new(1280.)),
            mobile_side_open: false,
            slides_ready: false,
        };
        s.apply_preset(AppShellPreset::Chat, cx);
        s
    }

    /// Apply one of the canonical Pro screen navigation presets.
    pub fn with_preset(mut self, preset: AppShellPreset, cx: &App) -> Self {
        self.apply_preset(preset, cx);
        self
    }

    fn apply_preset(&mut self, preset: AppShellPreset, cx: &App) {
        match preset {
            AppShellPreset::Chat => {
                self.workspace = "Acme".into();
                self.back_label = None;
                self.side_cards = true;
                self.groups = vec![
                    ShellNavGroup::primary(vec![
                        ShellNavItem::new(
                            "new-chat",
                            tr(cx, "New chat", "新建对话"),
                            ProIcon::NewChat,
                        ),
                        ShellNavItem::new("folders", tr(cx, "Folders", "文件夹"), ProIcon::Folder),
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
                ];
            }
            AppShellPreset::Agent => {
                self.workspace = "Fieldwork".into();
                self.user_email = "alex@fieldwork.co".into();
                self.back_label = None;
                self.side_cards = false;
                self.groups = vec![
                    ShellNavGroup::primary(vec![
                        ShellNavItem::new("thread", tr(cx, "Thread", "会话"), ProIcon::Thread),
                        ShellNavItem::new("inbox", tr(cx, "Inbox", "收件箱"), ProIcon::Inbox)
                            .with_count(5),
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
            }
            AppShellPreset::Settings => {
                self.workspace = "Acme".into();
                self.back_label = Some(tr(cx, "Settings", "设置").into());
                self.side_cards = false;
                self.has_work_area = false;
                self.panel_open = false;
                self.groups = vec![
                    ShellNavGroup::named(
                        tr(cx, "Personal", "个人"),
                        vec![
                            ShellNavItem::new(
                                "profile",
                                tr(cx, "Profile", "个人资料"),
                                ProIcon::User,
                            ),
                            ShellNavItem::new(
                                "preferences",
                                tr(cx, "Preferences", "偏好设置"),
                                ProIcon::Sliders,
                            ),
                            ShellNavItem::new(
                                "notifications",
                                tr(cx, "Notifications", "通知"),
                                ProIcon::Bell,
                            ),
                            ShellNavItem::new(
                                "privacy",
                                tr(cx, "Privacy", "隐私"),
                                ProIcon::Shield,
                            ),
                        ],
                    ),
                    ShellNavGroup::named(
                        tr(cx, "Workspace", "工作区"),
                        vec![
                            ShellNavItem::new(
                                "general",
                                tr(cx, "General", "常规"),
                                ProIcon::Building,
                            ),
                            ShellNavItem::new(
                                "members",
                                tr(cx, "Members", "成员与团队"),
                                ProIcon::Users,
                            ),
                            ShellNavItem::new(
                                "models",
                                tr(cx, "Models", "模型"),
                                ProIcon::Sparkles,
                            ),
                            ShellNavItem::new(
                                "security",
                                tr(cx, "Security", "安全"),
                                ProIcon::KeyLock,
                            ),
                            ShellNavItem::new(
                                "billing",
                                tr(cx, "Billing", "账单与订阅"),
                                ProIcon::CreditCard,
                            ),
                        ],
                    ),
                ];
            }
            AppShellPreset::Issues => {
                self.workspace = "Northwind".into();
                self.back_label = None;
                self.side_cards = false;
                self.has_work_area = false;
                self.panel_open = false;
                self.groups = vec![
                    ShellNavGroup::primary(vec![
                        ShellNavItem::new("inbox", tr(cx, "Inbox", "收件箱"), ProIcon::Inbox)
                            .with_count(3),
                        ShellNavItem::new(
                            "activity",
                            tr(cx, "Activity", "动态"),
                            ProIcon::Activity,
                        )
                        .with_count(2),
                        ShellNavItem::new(
                            "my-issues",
                            tr(cx, "My issues", "我的问题"),
                            ProIcon::Tasks,
                        ),
                    ]),
                    ShellNavGroup::named(
                        tr(cx, "Workspace", "工作区"),
                        vec![
                            ShellNavItem::new("board", tr(cx, "Board", "看板"), ProIcon::Grid),
                            ShellNavItem::new(
                                "projects",
                                tr(cx, "Projects", "项目"),
                                ProIcon::Folder,
                            ),
                        ],
                    ),
                ];
            }
            AppShellPreset::Crm => {
                self.workspace = "Northwind".into();
                self.back_label = None;
                self.side_cards = true;
                self.has_work_area = false;
                self.panel_open = false;
                self.groups = vec![ShellNavGroup::primary(vec![
                    ShellNavItem::new("home", tr(cx, "Home", "首页"), ProIcon::Home),
                    ShellNavItem::new("campaigns", tr(cx, "Campaigns", "活动"), ProIcon::Megaphone),
                    ShellNavItem::new(
                        "companies",
                        tr(cx, "Companies", "公司客户"),
                        ProIcon::Building,
                    ),
                    ShellNavItem::new("deals", tr(cx, "Deals", "商机"), ProIcon::Coins),
                    ShellNavItem::new(
                        "analytics",
                        tr(cx, "Analytics", "分析"),
                        ProIcon::UsageChart,
                    ),
                    ShellNavItem::new("settings", tr(cx, "Settings", "设置"), ProIcon::Settings),
                ])];
            }
        }
    }

    /// Choose `Inset` or `Flat` framing.
    pub fn with_layout(mut self, layout: AppShellLayout) -> Self {
        self.layout = layout;
        if layout == AppShellLayout::Flat {
            self.show_header = true;
        }
        self
    }

    /// Override the workspace title shown at the top of the sidebar.
    pub fn with_workspace(mut self, workspace: impl Into<SharedString>) -> Self {
        self.workspace = workspace.into();
        self
    }

    /// Set the active navigation label or id.
    pub fn with_active(mut self, active: impl Into<SharedString>) -> Self {
        self.active = active.into();
        self
    }

    /// Override the sidebar navigation groups.
    pub fn with_groups(mut self, groups: Vec<ShellNavGroup>) -> Self {
        self.groups = groups;
        self
    }

    /// Show or hide the changelog and getting-started cards in the sidebar footer.
    pub fn with_side_cards(mut self, show: bool) -> Self {
        self.side_cards = show;
        self
    }

    /// Start with the left navigation rail collapsed (`48px` icon rail) or expanded (`272px`).
    pub fn with_sidebar_collapsed(mut self, collapsed: bool) -> Self {
        self.sidebar_collapsed = collapsed;
        self
    }

    /// Configure whether the right-hand work area is enabled and initially open.
    pub fn with_work_area(mut self, enabled: bool, open: bool) -> Self {
        self.has_work_area = enabled;
        self.panel_open = enabled && open;
        self
    }

    /// Set the initial width in pixels of the right-hand inspector/work area.
    pub fn with_panel_width(mut self, width: f32) -> Self {
        self.panel_width = width.clamp(240., 520.);
        self
    }

    /// Set an optional back-navigation label at the top of the sidebar (e.g. `"Settings"`).
    pub fn with_back_label(mut self, label: Option<SharedString>) -> Self {
        self.back_label = label;
        self
    }

    /// Override the flat-layout top bar title.
    pub fn with_header_title(mut self, title: Option<SharedString>) -> Self {
        self.header_title = title;
        self
    }

    /// Control whether the outer main canvas scrolls (`true`) or delegates layout/scrolling to its child (`false`).
    pub fn with_page_scroll(mut self, scroll: bool) -> Self {
        self.page_scroll = scroll;
        self
    }

    /// Open or fold the right-hand inspector/work area programmatically.
    pub fn set_panel_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if self.panel_open != open {
            self.panel_open = open;
            cx.notify();
        }
    }

    /// Return whether the right-hand inspector/work area is currently open.
    pub fn is_panel_open(&self) -> bool {
        self.panel_open
    }

    /// Update the active sidebar item label or id.
    pub fn set_active(&mut self, active: impl Into<SharedString>) {
        self.active = active.into();
    }

    /// Bind a host screen entity so `page_fn` renders with full `&mut Context<V>` whenever either view updates.
    pub fn bind_page<V: 'static>(
        &mut self,
        host: &Entity<V>,
        cx: &mut Context<Self>,
        page_fn: impl Fn(&mut V, &mut Window, &mut Context<V>) -> AnyElement + 'static,
    ) {
        cx.observe(host, |_, _, cx| cx.notify()).detach();
        let weak = host.downgrade();
        self.page_scroll = false;
        self.custom_page = Some(Rc::new(move |w, app| {
            weak.update(app, |v, v_cx| page_fn(v, w, v_cx))
                .unwrap_or_else(|_| div().into_any_element())
        }));
    }

    /// Bind a host screen inspector to the right-hand work area; returning `None` hides the inspector.
    pub fn bind_panel<V: 'static>(
        &mut self,
        host: &Entity<V>,
        panel_fn: impl Fn(
            &mut V,
            &mut Window,
            &mut Context<V>,
        ) -> Option<(SharedString, ProIcon, AnyElement)>
        + 'static,
    ) {
        let weak = host.downgrade();
        self.has_work_area = true;
        self.bound_panel = Some(Rc::new(move |w, app| {
            weak.update(app, |v, v_cx| panel_fn(v, w, v_cx))
                .ok()
                .flatten()
        }));
    }

    /// Mount custom inspector content inside the right-hand work area.
    pub fn with_panel(
        mut self,
        title: impl Into<SharedString>,
        render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.has_work_area = true;
        self.panel_open = true;
        self.panel_title = title.into();
        self.custom_panel = Some(Rc::new(render));
        self
    }

    /// Mount screen content inside the main canvas column.
    pub fn with_page(
        mut self,
        render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.custom_page = Some(Rc::new(render));
        self
    }

    /// Set the account footer display name and email.
    pub fn with_user(
        mut self,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
    ) -> Self {
        self.user_name = name.into();
        self.user_email = email.into();
        self
    }

    fn open_suggested(&mut self, key: &str, cx: &mut Context<Self>) {
        let (title, icon, kind) = match key {
            "terminal" => (
                SharedString::from("Terminal"),
                ProIcon::Terminal,
                WorkTabKind::Terminal,
            ),
            "browser" => (
                SharedString::from("Browser"),
                ProIcon::Globe,
                WorkTabKind::Browser {
                    url: "https://arxiv.org/abs/2611.04102".into(),
                },
            ),
            "notes" => (
                SharedString::from(tr(cx, "Notes", "笔记")),
                ProIcon::Note,
                WorkTabKind::Notes {
                    body: tr(
                        cx,
                        "Launch notes:\n• Confirm partner logos before Friday\n• Share updated Q3 spend table with finance",
                        "发布备忘：\n• 周五前确认合作方标志\n• 与财务同步最新 Q3 预算表",
                    )
                    .into(),
                },
            ),
            "geo" => (
                SharedString::from("GEO: Generative Engine..."),
                ProIcon::PageDoc,
                WorkTabKind::Document {
                    source: "arxiv.org".into(),
                    heading: "GEO: Generative Engine Optimization".into(),
                    paragraphs: vec![
                        "Generative engines synthesize cited answers from structured sources rather than ranking ten blue links.".into(),
                        "Adding verifiable statistics, concise schema headings, and primary quotes improved source visibility by 38% across evaluated benchmarks.".into(),
                    ],
                    checklist: vec![],
                },
            ),
            "tokens" => (
                SharedString::from("Design tokens – naming"),
                ProIcon::PageDoc,
                WorkTabKind::Document {
                    source: "notion.so".into(),
                    heading: "Design tokens – naming guide".into(),
                    paragraphs: vec![
                        "Semantic surface roles (--panel, --card, --tray, --chip) decouple product screens from light and dark hex values.".into(),
                    ],
                    checklist: vec![
                        ("Audit resting border hairlines in dark mode".into(), true),
                        ("Verify 3:1 keyboard focus rings on inset rows".into(), true),
                        ("Freeze ambient loops under reduced motion".into(), false),
                    ],
                },
            ),
            "checklist" => (
                SharedString::from("Q3 launch checklist"),
                ProIcon::ClipboardDoc,
                WorkTabKind::Document {
                    source: "linear.app".into(),
                    heading: "Q3 launch checklist".into(),
                    paragraphs: vec!["Owner: Alex Morgan · Target window: Thursday 09:00 UTC".into()],
                    checklist: vec![
                        ("Verify webhook idempotency under retry".into(), true),
                        ("Approve customer migration email draft".into(), true),
                        ("Enable EU model routing fallback".into(), false),
                        ("Publish full changelog note".into(), false),
                    ],
                },
            ),
            _ => (
                SharedString::from("Q3 launch budget"),
                ProIcon::ClipboardDoc,
                WorkTabKind::Sheet {
                    source: "sheets.google.com".into(),
                    headers: vec!["Channel".into(), "Planned".into(), "Committed".into(), "Delta".into()],
                    rows: vec![
                        vec!["Product Hunt & launch video".into(), "$18,000".into(), "$17,400".into(), "-$600".into()],
                        vec!["Partner integrations co-marketing".into(), "$24,500".into(), "$24,500".into(), "$0".into()],
                        vec!["Developer docs & benchmarks".into(), "$12,000".into(), "$11,250".into(), "-$750".into()],
                        vec!["Total".into(), "$54,500".into(), "$53,150".into(), "-$1,350".into()],
                    ],
                },
            ),
        };
        if let Some(pos) = self.tabs.iter().position(|t| t.id.as_ref() == key) {
            self.active_tab = pos;
        } else {
            let tab = WorkTab {
                id: key.to_owned().into(),
                title,
                icon,
                kind,
            };
            if self
                .tabs
                .get(self.active_tab)
                .is_some_and(|t| matches!(t.kind, WorkTabKind::NewTab))
            {
                self.tabs[self.active_tab] = tab;
            } else {
                self.tabs.push(tab);
                self.active_tab = self.tabs.len() - 1;
            }
        }
        self.panel_open = true;
        cx.emit(AppShellEvent::OpenTool(key.to_owned().into()));
        cx.notify();
    }

    fn add_new_tab(&mut self, cx: &mut Context<Self>) {
        let id = format!("new-{}", self.tabs.len() + 1);
        self.tabs.push(WorkTab {
            id: id.into(),
            title: tr(cx, "New tab", "新标签页").into(),
            icon: ProIcon::Globe,
            kind: WorkTabKind::NewTab,
        });
        self.active_tab = self.tabs.len() - 1;
        self.panel_open = true;
        cx.notify();
    }

    fn close_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.panel_open = false;
            self.tabs.push(WorkTab {
                id: "new-1".into(),
                title: tr(cx, "New tab", "新标签页").into(),
                icon: ProIcon::Globe,
                kind: WorkTabKind::NewTab,
            });
            self.active_tab = 0;
            cx.emit(AppShellEvent::PanelOpen(false));
        } else if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        cx.notify();
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.cmd_editor.is_none() {
            cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
            cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
                .detach();
            let ph: SharedString = tr(
                cx,
                "Jump to screen, project or command…",
                "跳转到页面、项目或命令…",
            )
            .into();
            self.cmd_editor = Some(cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e
            }));
        }
        if self.url_editor.is_none() {
            let ph: SharedString = tr(cx, "Search or enter URL", "搜索或输入网址").into();
            self.url_editor = Some(cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e
            }));
        }
    }

    fn render_sidebar(
        &mut self,
        compact: bool,
        collapsed_rail: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let collapsed = collapsed_rail && !compact;
        let side_w = if collapsed { 48. } else { self.sidebar_width };

        // Top workspace or back header
        let head = if let Some(back) = self.back_label.clone() {
            div()
                .h(px(30.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(4.))
                .child(when_not_collapsed(
                    collapsed,
                    activate(
                        div()
                            .id((self.id.clone(), "back-btn"))
                            .role(Role::Button)
                            .tab_index(0)
                            .h(px(30.))
                            .px(px(6.))
                            .rounded(px(8.))
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .cursor_pointer()
                            .hover(move |s| s.bg(p.hover))
                            .child(pro_icon(ProIcon::ChevronLeft, 14., p.muted))
                            .child(back),
                        cx,
                        |_, _, cx| {
                            cx.emit(AppShellEvent::Back);
                            cx.notify();
                        },
                    )
                    .into_any_element(),
                ))
                .child(pro_icon_btn(
                    (self.id.clone(), "toggle-side"),
                    tr(cx, "Toggle sidebar", "切换侧边栏"),
                    ProIcon::SidebarLeft,
                    28.,
                    false,
                    cx,
                    move |this, _, cx| {
                        if compact {
                            this.mobile_side_open = !this.mobile_side_open;
                        } else {
                            this.sidebar_collapsed = !this.sidebar_collapsed;
                            cx.emit(AppShellEvent::SidebarCollapsed(this.sidebar_collapsed));
                        }
                        cx.notify();
                    },
                ))
        } else {
            let ws_letter: String = self
                .workspace
                .chars()
                .next()
                .unwrap_or('A')
                .to_uppercase()
                .collect();
            div()
                .h(px(30.))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(4.))
                .child(activate(
                    div()
                        .id((self.id.clone(), "ws-btn"))
                        .role(Role::Button)
                        .tab_index(0)
                        .h(px(30.))
                        .px(px(6.))
                        .rounded(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .cursor_pointer()
                        .hover(move |s| s.bg(p.hover))
                        .child(
                            div()
                                .size(px(18.))
                                .rounded(px(5.))
                                .bg(if p.dark { rgb(0x2a2a2e) } else { rgb(0xffffff) })
                                .border_1()
                                .border_color(p.line_strong)
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(10.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.ink)
                                .child(ws_letter),
                        )
                        .when(!collapsed, |d| {
                            d.child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(self.workspace.clone()),
                            )
                            .child(pro_icon(
                                ProIcon::ChevronDown,
                                13.,
                                p.faint,
                            ))
                        }),
                    cx,
                    |this, _, cx| {
                        this.ws_menu_open = !this.ws_menu_open;
                        this.user_menu_open = false;
                        cx.notify();
                    },
                ))
                .when(!collapsed, |d| {
                    d.child(pro_icon_btn(
                        (self.id.clone(), "toggle-side"),
                        tr(cx, "Collapse sidebar", "收起侧边栏"),
                        ProIcon::SidebarLeft,
                        28.,
                        false,
                        cx,
                        move |this, _, cx| {
                            if compact {
                                this.mobile_side_open = false;
                            } else {
                                this.sidebar_collapsed = true;
                                cx.emit(AppShellEvent::SidebarCollapsed(true));
                            }
                            cx.notify();
                        },
                    ))
                })
        };

        // Search trigger button
        let search_btn = activate(
            div()
                .id((self.id.clone(), "side-search"))
                .role(Role::Button)
                .tab_index(0)
                .mt(px(
                    if self.show_header && self.layout == AppShellLayout::Flat {
                        4.
                    } else {
                        10.
                    },
                ))
                .h(px(30.))
                .px(px(8.))
                .rounded(px(8.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .gap(px(8.))
                .cursor_pointer()
                .hover(move |s| s.border_color(p.line_strong))
                .child(pro_icon(ProIcon::Search, 14., p.faint))
                .when(!collapsed, |d| {
                    d.child(
                        div()
                            .flex_1()
                            .text_size(px(13.))
                            .text_color(p.muted)
                            .child(tr(cx, "Search...", "搜索...")),
                    )
                    .child(
                        div()
                            .px(px(5.))
                            .h(px(18.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(p.line_strong)
                            .flex()
                            .items_center()
                            .text_size(px(10.5))
                            .text_color(p.faint)
                            .child("⌘K"),
                    )
                }),
            cx,
            |this, _, cx| {
                this.cmd_open = !this.cmd_open;
                cx.notify();
            },
        );

        // Nav groups with 2D sliding selection pill
        let nav_sensor = pro_selection_sensor();
        let active_key = self.active.to_ascii_lowercase();
        let mut nav_col = div()
            .id((self.id.clone(), "side-nav"))
            .mt(px(10.))
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .relative()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(pro_pill_indicator(
                (
                    self.id.clone(),
                    if compact { "m-nav-pill" } else { "nav-pill" },
                ),
                format!("{active_key}-{collapsed}"),
                nav_sensor.clone(),
                8.,
                p.select,
                None,
                false,
            ));

        for (gi, group) in self.groups.clone().into_iter().enumerate() {
            let mut grp = div().flex().flex_col().gap(px(1.));
            if let Some(heading) = group.heading {
                if collapsed {
                    grp = grp.child(
                        div()
                            .my(px(6.))
                            .mx(px(4.))
                            .h(px(1.))
                            .w(px(24.))
                            .bg(p.line_strong),
                    );
                } else {
                    grp = grp.child(
                        div()
                            .mt(px(if gi == 0 { 0. } else { 6. }))
                            .mb(px(4.))
                            .px(px(6.))
                            .text_size(px(12.))
                            .text_color(p.faint)
                            .child(heading),
                    );
                }
            }
            for item in group.items {
                let is_active = item.id.to_ascii_lowercase() == active_key
                    || item.label.to_ascii_lowercase() == active_key;
                let item_label = item.label.clone();
                let nav_id = item.id.clone();
                let icon_c = if is_active { p.ink } else { p.muted };
                let text_c = if is_active { p.ink } else { p.text };
                grp = grp.child(activate(
                    div()
                        .id((self.id.clone(), SharedString::from(format!("nav-{nav_id}"))))
                        .role(Role::Button)
                        .aria_label(item.label.clone())
                        .tab_index(0)
                        .relative()
                        .h(px(30.))
                        .px(px(7.))
                        .rounded(px(8.))
                        .when(is_active, |d| d.child(pro_mark_selected(&nav_sensor)))
                        .hover(move |s| if is_active { s } else { s.bg(p.hover) })
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .cursor_pointer()
                        .child(pro_icon(item.icon, 16., icon_c))
                        .when(!collapsed, |d| {
                            d.child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(13.))
                                    .font_weight(if is_active {
                                        FontWeight::MEDIUM
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .text_color(text_c)
                                    .child(item.label),
                            )
                            .when(item.unread, |d| {
                                d.child(div().size(px(6.)).rounded_full().bg(p.blue))
                            })
                            .when_some(item.count, |d, n| {
                                d.child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(p.faint)
                                        .child(n.to_string()),
                                )
                            })
                        }),
                    cx,
                    move |this, _, cx| {
                        this.active = item_label.clone();
                        this.mobile_side_open = false;
                        cx.emit(AppShellEvent::Navigate(item_label.clone()));
                        cx.notify();
                    },
                ));
            }
            nav_col = nav_col.child(grp);
        }

        // Bottom cards + account
        let mut foot = div().mt_auto().pt(px(8.)).flex().flex_col().gap(px(8.));
        if self.side_cards && !collapsed {
            if self.news_count > 0 {
                let mut stack = div().relative().flex().flex_col();
                if self.news_count > 1 {
                    stack = stack.child(
                        div()
                            .mx(px(8.))
                            .mb(px(-6.))
                            .h(px(26.))
                            .px(px(10.))
                            .pt(px(6.))
                            .rounded_t(px(10.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.blue.opacity(0.75))
                                    .child(format!(
                                        "{} {}",
                                        tr(cx, "New in", "最新于"),
                                        self.workspace
                                    )),
                            )
                            .child(pro_icon(ProIcon::Close, 10., p.faint)),
                    );
                }
                stack = stack.child(
                    div()
                        .rounded(px(11.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .p(px(10.))
                                .flex()
                                .flex_col()
                                .gap(px(3.))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_size(px(11.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.blue)
                                                .child(format!(
                                                    "{} {}",
                                                    tr(cx, "New in", "最新于"),
                                                    self.workspace
                                                )),
                                        )
                                        .child(activate(
                                            div()
                                                .id((self.id.clone(), "dismiss-news"))
                                                .role(Role::Button)
                                                .tab_index(0)
                                                .size(px(18.))
                                                .rounded(px(5.))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .cursor_pointer()
                                                .hover(move |s| s.bg(p.hover))
                                                .child(pro_icon(ProIcon::Close, 11., p.faint)),
                                            cx,
                                            |this, _, cx| {
                                                this.news_count = this.news_count.saturating_sub(1);
                                                cx.notify();
                                            },
                                        )),
                                )
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(if self.news_count > 1 {
                                            tr(cx, "Shared folders", "共享文件夹")
                                        } else {
                                            tr(cx, "Parallel background runs", "并行后台运行")
                                        }),
                                ),
                        )
                        .child(activate(
                            div()
                                .id((self.id.clone(), "changelog-link"))
                                .role(Role::Button)
                                .tab_index(0)
                                .h(px(32.))
                                .px(px(10.))
                                .border_t_1()
                                .border_color(p.line_soft)
                                .flex()
                                .items_center()
                                .justify_between()
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .cursor_pointer()
                                .hover(move |s| s.bg(p.hover).text_color(p.ink))
                                .child(tr(cx, "Full changelog", "完整更新日志"))
                                .child(pro_icon(ProIcon::ChevronRight, 12., p.faint)),
                            cx,
                            |this, _, cx| {
                                this.open_suggested("tokens", cx);
                            },
                        )),
                );
                foot = foot.child(stack);
            }

            let done_n = self.guide_checks.iter().filter(|&&b| b).count();
            let pct = match done_n {
                0 => 0,
                1 => 13,
                2 => 50,
                3 => 75,
                _ => 100,
            };
            let mut guide = div()
                .rounded(px(11.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .p(px(10.))
                .flex()
                .flex_col()
                .gap(px(5.))
                .child(activate(
                    div()
                        .id((self.id.clone(), "guide-head"))
                        .role(Role::Button)
                        .tab_index(0)
                        .flex()
                        .flex_col()
                        .gap(px(3.))
                        .cursor_pointer()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(tr(cx, "Getting started", "快速上手")),
                                )
                                .child(pro_icon(
                                    if self.guide_open {
                                        ProIcon::ChevronDown
                                    } else {
                                        ProIcon::ChevronRight
                                    },
                                    12.,
                                    p.faint,
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .text_size(px(12.))
                                .text_color(p.muted)
                                .child(format!("{pct}% {}", tr(cx, "done ·", "完成 ·")))
                                .child(div().text_color(p.blue).child(tr(
                                    cx,
                                    "Great start!",
                                    "开局顺利！",
                                ))),
                        )
                        .child(div().mt(px(3.)).w_full().child(pro_spring_bar(
                            (self.id.clone(), "guide-pbar"),
                            (pct as f32 / 100.).max(0.06),
                            4.,
                            2.,
                            p.tray,
                            p.blue,
                        ))),
                    cx,
                    |this, _, cx| {
                        this.guide_open = !this.guide_open;
                        cx.notify();
                    },
                ));
            let steps = [
                tr(cx, "Create workspace", "创建工作区"),
                tr(cx, "Connect first integration", "连接首个应用"),
                tr(cx, "Invite a teammate", "邀请团队成员"),
                tr(cx, "Run your first task", "运行首个任务"),
            ];
            let mut list = div()
                .pt(px(6.))
                .border_t_1()
                .border_color(p.line_soft)
                .flex()
                .flex_col()
                .gap(px(4.));
            for (si, step_label) in steps.into_iter().enumerate() {
                let checked = self.guide_checks[si];
                list = list.child(activate(
                    div()
                        .id((self.id.clone(), SharedString::from(format!("gstep-{si}"))))
                        .role(Role::CheckBox)
                        .tab_index(0)
                        .h(px(24.))
                        .px(px(4.))
                        .rounded(px(5.))
                        .hover(move |s| s.bg(p.hover))
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .cursor_pointer()
                        .child(
                            div()
                                .size(px(14.))
                                .rounded(px(4.))
                                .border_1()
                                .border_color(if checked { p.blue } else { p.line_strong })
                                .bg(if checked { p.blue } else { transparent_black() })
                                .flex()
                                .items_center()
                                .justify_center()
                                .when(checked, |d| {
                                    d.child(pro_icon(ProIcon::Check, 10., rgb(0xffffff).into()))
                                }),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(if checked { p.muted } else { p.ink })
                                .when(checked, |d| d.line_through())
                                .child(step_label),
                        ),
                    cx,
                    move |this, _, cx| {
                        this.guide_checks[si] = !this.guide_checks[si];
                        cx.notify();
                    },
                ));
            }
            guide = guide.child(pro_fold(
                (self.id.clone(), "guide-fold"),
                self.guide_open,
                116.,
                5.,
                move || list.into_any_element(),
            ));
            foot = foot.child(guide);
        }

        // User menu popover with spring fold
        if !collapsed {
            let um_card = div()
                .p(px(5.))
                .rounded(px(11.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .px(px(8.))
                        .py(px(6.))
                        .border_b_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_person_avatar(&self.user_name, 28., false, None, cx))
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(12.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .truncate()
                                        .child(self.user_name.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(self.user_email.clone()),
                                ),
                        ),
                )
                .children(
                    [
                        (ProIcon::Sliders, tr(cx, "Preferences", "偏好设置")),
                        (ProIcon::Grid, tr(cx, "Integrations", "集成")),
                        (
                            ProIcon::Settings,
                            tr(cx, "Workspace settings", "工作区设置"),
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(mi, (ic, lbl))| {
                        let target: SharedString = lbl.into();
                        activate(
                            div()
                                .id((self.id.clone(), SharedString::from(format!("um-{mi}"))))
                                .role(Role::MenuItem)
                                .tab_index(0)
                                .h(px(28.))
                                .px(px(8.))
                                .rounded(px(6.))
                                .hover(move |s| s.bg(p.hover))
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .text_size(px(12.5))
                                .text_color(p.text)
                                .cursor_pointer()
                                .child(pro_icon(ic, 14., p.muted))
                                .child(lbl),
                            cx,
                            move |this, _, cx| {
                                this.user_menu_open = false;
                                cx.emit(AppShellEvent::Navigate(target.clone()));
                                cx.notify();
                            },
                        )
                    }),
                );
            foot = foot.child(pro_fold(
                (self.id.clone(), "um-fold"),
                self.user_menu_open,
                146.,
                8.,
                move || um_card.into_any_element(),
            ));
        }

        // User row
        foot = foot.child(activate(
            div()
                .id((self.id.clone(), "user-btn"))
                .role(Role::Button)
                .tab_index(0)
                .h(px(34.))
                .px(px(6.))
                .rounded(px(8.))
                .hover(move |s| s.bg(p.hover))
                .flex()
                .items_center()
                .gap(px(7.))
                .cursor_pointer()
                .child(pro_person_avatar(&self.user_name, 21., false, None, cx))
                .when(!collapsed, |d| {
                    d.child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(self.user_name.clone()),
                    )
                    .child(pro_icon(ProIcon::ChevronDown, 12., p.faint))
                }),
            cx,
            |this, _, cx| {
                this.user_menu_open = !this.user_menu_open;
                this.ws_menu_open = false;
                cx.notify();
            },
        ));

        let is_flat = self.layout == AppShellLayout::Flat;
        div()
            .w(px(side_w))
            .h_full()
            .flex_none()
            .p(px(10.))
            .pb(px(8.))
            .bg(if is_flat { p.side_tint } else { p.page })
            .when(is_flat, |d| d.border_r_1().border_color(p.line))
            .flex()
            .flex_col()
            .min_h_0()
            .when(!(is_flat && self.show_header), |d| d.child(head))
            .when(collapsed, |d| {
                d.child(div().mt(px(4.)).child(pro_icon_btn(
                    (self.id.clone(), "expand-rail"),
                    tr(cx, "Expand sidebar", "展开侧边栏"),
                    ProIcon::SidebarLeft,
                    28.,
                    false,
                    cx,
                    |this, _, cx| {
                        this.sidebar_collapsed = false;
                        cx.emit(AppShellEvent::SidebarCollapsed(false));
                        cx.notify();
                    },
                )))
            })
            .child(search_btn)
            .child(nav_col)
            .child(foot)
    }

    fn render_custom_inspector(
        &mut self,
        title: SharedString,
        icon: ProIcon,
        body: AnyElement,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let is_flat = self.layout == AppShellLayout::Flat;
        let outer = div().w(px(self.panel_width)).h_full().flex_none();
        let body = pro_view_transition((self.id.clone(), "insp-tx"), title.clone(), body);
        if !is_flat {
            outer.py(px(8.)).pr(px(8.)).pl(px(4.)).child(
                div()
                    .size_full()
                    .rounded(px(12.))
                    .bg(p.panel)
                    .border_1()
                    .border_color(p.line)
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(44.))
                            .px(px(12.))
                            .border_b_1()
                            .border_color(p.line_soft)
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .child(
                                div()
                                    .min_w_0()
                                    .max_w(px((self.panel_width - 76.).max(120.)))
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
                                    .child(pro_icon(icon, 13., p.muted))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .text_ellipsis()
                                            .child(title),
                                    ),
                            )
                            .child(pro_icon_btn(
                                (self.id.clone(), "close-custom-panel"),
                                tr(cx, "Fold work area", "收起工作区"),
                                ProIcon::SidebarRight,
                                28.,
                                false,
                                cx,
                                |this, _, cx| {
                                    this.panel_open = false;
                                    cx.emit(AppShellEvent::PanelOpen(false));
                                    cx.notify();
                                },
                            )),
                    )
                    .child(
                        div()
                            .id((self.id.clone(), "custom-panel-scroll"))
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .child(body),
                    ),
            )
        } else {
            outer
                .bg(p.panel)
                .border_l_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .child(
                    div()
                        .id((self.id.clone(), "custom-panel-flat-scroll"))
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .child(body),
                )
        }
    }

    fn render_work_area(
        &mut self,
        pre_custom_panel: Option<AnyElement>,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let is_flat = self.layout == AppShellLayout::Flat;

        if let Some(body) = pre_custom_panel {
            let title = self.panel_title.clone();
            let icon = self.panel_icon;
            return self.render_custom_inspector(title, icon, body, cx);
        }

        // Standard multi-tab work area
        let mut tab_strip = div()
            .h(px(40.))
            .px(px(10.))
            .border_b_1()
            .border_color(p.line_soft)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(4.));

        let wtab_sensor = pro_selection_sensor();
        let active_tab_key = self
            .tabs
            .get(self.active_tab)
            .map(|t| t.id.clone())
            .unwrap_or_else(|| SharedString::from("0"));
        let mut tabs_left = div()
            .relative()
            .flex()
            .items_center()
            .gap(px(3.))
            .min_w_0()
            .overflow_hidden()
            .child(pro_pill_indicator(
                (self.id.clone(), "wtab-pill"),
                active_tab_key.clone(),
                wtab_sensor.clone(),
                7.,
                p.chip,
                None,
                false,
            ));
        for (ti, tab) in self.tabs.clone().into_iter().enumerate() {
            let selected = ti == self.active_tab;
            tabs_left = tabs_left.child(
                div()
                    .relative()
                    .h(px(28.))
                    .pl(px(8.))
                    .pr(px(5.))
                    .rounded(px(7.))
                    .when(selected, |d| d.child(pro_mark_selected(&wtab_sensor)))
                    .when(!selected, |d| d.hover(move |s| s.bg(p.hover)))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(activate(
                        div()
                            .id((self.id.clone(), SharedString::from(format!("wtab-{ti}"))))
                            .role(Role::Tab)
                            .tab_index(0)
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .cursor_pointer()
                            .child(pro_icon(
                                tab.icon,
                                13.,
                                if selected { p.ink } else { p.muted },
                            ))
                            .child(
                                div()
                                    .max_w(px(120.))
                                    .truncate()
                                    .text_size(px(12.5))
                                    .font_weight(if selected {
                                        FontWeight::MEDIUM
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .text_color(if selected { p.ink } else { p.muted })
                                    .child(tab.title),
                            ),
                        cx,
                        move |this, _, cx| {
                            this.active_tab = ti;
                            cx.notify();
                        },
                    ))
                    .child(activate(
                        div()
                            .id((self.id.clone(), SharedString::from(format!("wtab-x-{ti}"))))
                            .role(Role::Button)
                            .aria_label("Close tab")
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
                            this.close_tab(ti, cx);
                        },
                    )),
            );
        }
        tabs_left = tabs_left.child(pro_icon_btn(
            (self.id.clone(), "wtab-plus"),
            tr(cx, "New tab", "新标签页"),
            ProIcon::Plus,
            26.,
            false,
            cx,
            |this, _, cx| this.add_new_tab(cx),
        ));

        tab_strip = tab_strip.child(tabs_left).child(pro_icon_btn(
            (self.id.clone(), "fold-panel"),
            tr(cx, "Close work area", "关闭工作区"),
            ProIcon::SidebarRight,
            28.,
            false,
            cx,
            |this, _, cx| {
                this.panel_open = false;
                cx.emit(AppShellEvent::PanelOpen(false));
                cx.notify();
            },
        ));

        let active_kind = self
            .tabs
            .get(self.active_tab)
            .map(|t| t.kind.clone())
            .unwrap_or(WorkTabKind::NewTab);

        let body = match active_kind {
            WorkTabKind::NewTab => {
                let url_ed = self.url_editor.clone();
                div()
                    .p(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .child(
                        div()
                            .h(px(32.))
                            .px(px(10.))
                            .rounded(px(8.))
                            .bg(p.chip)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .child(pro_icon(ProIcon::Search, 13., p.faint))
                            .when_some(url_ed, |d, ed| d.child(div().flex_1().min_w_0().child(ed))),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                div()
                                    .px(px(4.))
                                    .text_size(px(11.5))
                                    .text_color(p.faint)
                                    .child(tr(cx, "Tools", "工具")),
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
                                            ("terminal", ProIcon::Terminal, "Terminal", "⌃`"),
                                            ("browser", ProIcon::Globe, "Browser", "⌘⇧B"),
                                            ("notes", ProIcon::Note, tr(cx, "Notes", "笔记"), "⌘⇧N"),
                                        ]
                                        .into_iter()
                                        .enumerate()
                                        .map(|(i, (key, ic, name, kbd))| {
                                            activate(
                                                div()
                                                    .id((
                                                        self.id.clone(),
                                                        SharedString::from(format!("wtool-{key}")),
                                                    ))
                                                    .role(Role::Button)
                                                    .tab_index(0)
                                                    .h(px(38.))
                                                    .px(px(10.))
                                                    .when(i > 0, |d| {
                                                        d.border_t_1().border_color(p.line_soft)
                                                    })
                                                    .hover(move |s| s.bg(p.hover))
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(9.))
                                                    .cursor_pointer()
                                                    .child(pro_icon(ic, 15., p.muted))
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .text_size(px(13.))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(p.ink)
                                                            .child(name),
                                                    )
                                                    .child(
                                                        div()
                                                            .px(px(5.))
                                                            .h(px(18.))
                                                            .rounded(px(4.))
                                                            .bg(p.chip)
                                                            .flex()
                                                            .items_center()
                                                            .text_size(px(10.5))
                                                            .text_color(p.faint)
                                                            .child(kbd),
                                                    ),
                                                cx,
                                                move |this, _, cx| this.open_suggested(key, cx),
                                            )
                                        }),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                div()
                                    .px(px(4.))
                                    .text_size(px(11.5))
                                    .text_color(p.faint)
                                    .child(tr(cx, "Suggested", "推荐资料")),
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
                                            (
                                                "geo",
                                                "A",
                                                "GEO: Generative Engine Optimization",
                                                "arxiv.org",
                                            ),
                                            (
                                                "tokens",
                                                "N",
                                                "Design tokens – naming guide",
                                                "notion.so",
                                            ),
                                            (
                                                "checklist",
                                                "L",
                                                "Q3 launch checklist",
                                                "linear.app",
                                            ),
                                            (
                                                "budget",
                                                "S",
                                                "Q3 launch budget",
                                                "sheets.google.com",
                                            ),
                                        ]
                                        .into_iter()
                                        .enumerate()
                                        .map(|(i, (key, badge_ch, title, host))| {
                                            activate(
                                                div()
                                                    .id((
                                                        self.id.clone(),
                                                        SharedString::from(format!("wsug-{key}")),
                                                    ))
                                                    .role(Role::Button)
                                                    .tab_index(0)
                                                    .py(px(8.))
                                                    .px(px(10.))
                                                    .when(i > 0, |d| {
                                                        d.border_t_1().border_color(p.line_soft)
                                                    })
                                                    .hover(move |s| s.bg(p.hover))
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(9.))
                                                    .cursor_pointer()
                                                    .child(
                                                        div()
                                                            .size(px(24.))
                                                            .rounded(px(6.))
                                                            .bg(p.chip)
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .text_size(px(11.))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(p.muted)
                                                            .child(badge_ch),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .min_w_0()
                                                            .flex()
                                                            .flex_col()
                                                            .child(
                                                                div()
                                                                    .text_size(px(12.5))
                                                                    .font_weight(FontWeight::MEDIUM)
                                                                    .text_color(p.ink)
                                                                    .truncate()
                                                                    .child(title),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_size(px(11.5))
                                                                    .text_color(p.muted)
                                                                    .child(host),
                                                            ),
                                                    ),
                                                cx,
                                                move |this, _, cx| this.open_suggested(key, cx),
                                            )
                                        }),
                                    ),
                            ),
                    )
                    .into_any_element()
            }
            WorkTabKind::Terminal => div()
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .font_family("Fira Code")
                .text_size(px(12.))
                .child(
                    div()
                        .rounded(px(9.))
                        .bg(p.chip)
                        .p(px(12.))
                        .flex()
                        .flex_col()
                        .gap(px(5.))
                        .child(div().text_color(p.muted).child("$ cargo test -p kense-kui"))
                        .child(div().text_color(p.green_ink).child("   Compiling kense-kui v0.1.0"))
                        .child(div().text_color(p.ink).child("test result: ok. 394 passed; 0 failed"))
                        .child(div().text_color(p.blue).child("$ kui doctor — ok: gpui =0.3.10")),
                )
                .into_any_element(),
            WorkTabKind::Browser { url } => div()
                .p(px(12.))
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(
                    div()
                        .h(px(28.))
                        .px(px(9.))
                        .rounded(px(7.))
                        .bg(p.chip)
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(pro_icon(ProIcon::Globe, 13., p.faint))
                        .child(url),
                )
                .child(
                    div()
                        .rounded(px(10.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .p(px(14.))
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .child("GEO: Generative Engine Optimization"),
                        )
                        .child(
                            div()
                                .text_size(px(12.5))
                                .line_height(px(19.))
                                .text_color(p.text)
                                .child("Structured citations and direct quantitative claims raise answer inclusion across LLM retrieval pipelines."),
                        ),
                )
                .into_any_element(),
            WorkTabKind::Notes { body } => div()
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.ink)
                        .child(tr(cx, "Scratchpad", "随手记")),
                )
                .child(
                    div()
                        .rounded(px(9.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .p(px(12.))
                        .text_size(px(12.5))
                        .line_height(px(20.))
                        .text_color(p.text)
                        .child(body),
                )
                .into_any_element(),
            WorkTabKind::Document {
                source,
                heading,
                paragraphs,
                checklist,
            } => div()
                .p(px(14.))
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_app_tile(&source, 26., cx))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(heading),
                                )
                                .child(div().text_size(px(11.5)).text_color(p.muted).child(source)),
                        ),
                )
                .children(paragraphs.into_iter().map(|para| {
                    div()
                        .text_size(px(12.5))
                        .line_height(px(19.))
                        .text_color(p.text)
                        .child(para)
                }))
                .children(checklist.into_iter().map(|(line, done)| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(7.))
                        .text_size(px(12.5))
                        .text_color(if done { p.muted } else { p.ink })
                        .child(pro_icon(
                            if done { ProIcon::Check } else { ProIcon::Clock },
                            13.,
                            if done { p.green } else { p.faint },
                        ))
                        .child(line)
                }))
                .into_any_element(),
            WorkTabKind::Sheet {
                source,
                headers,
                rows,
            } => div()
                .p(px(12.))
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(div().text_size(px(11.5)).text_color(p.muted).child(source))
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
                                .bg(p.chip)
                                .px(px(8.))
                                .h(px(28.))
                                .flex()
                                .items_center()
                                .children(headers.into_iter().map(|h| {
                                    div()
                                        .flex_1()
                                        .text_size(px(11.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.muted)
                                        .truncate()
                                        .child(h)
                                })),
                        )
                        .children(rows.into_iter().map(|r| {
                            div()
                                .px(px(8.))
                                .h(px(30.))
                                .border_t_1()
                                .border_color(p.line_soft)
                                .flex()
                                .items_center()
                                .children(r.into_iter().enumerate().map(|(ci, cell)| {
                                    div()
                                        .flex_1()
                                        .text_size(px(12.))
                                        .text_color(if ci == 0 { p.ink } else { p.text })
                                        .truncate()
                                        .child(cell)
                                }))
                        })),
                )
                .into_any_element(),
        };

        let inner = div()
            .size_full()
            .bg(p.panel)
            .when(!is_flat, |d| {
                d.rounded(px(12.)).border_1().border_color(p.line)
            })
            .when(is_flat, |d| d.border_l_1().border_color(p.line))
            .overflow_hidden()
            .flex()
            .flex_col()
            .when(!is_flat, |d| d.child(tab_strip))
            .child(
                div()
                    .id((self.id.clone(), "work-body"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(pro_view_transition(
                        (self.id.clone(), "wtab-body-tx"),
                        active_tab_key,
                        body,
                    )),
            );

        div()
            .w(px(self.panel_width))
            .h_full()
            .flex_none()
            .when(!is_flat, |d| d.py(px(8.)).pr(px(8.)).pl(px(4.)))
            .child(inner)
    }

    fn render_default_placeholder(&self, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let subtitle = if self.layout == AppShellLayout::Flat {
            tr(
                cx,
                "The flat shell brings both sidebars and the scroll container – put any screen inside.",
                "扁平外壳包含两侧边栏与独立滚动容器——可将任意业务页面置入其中。",
            )
        } else {
            tr(
                cx,
                "The shell brings both sidebars and the scroll container – put any screen inside.",
                "内嵌外壳包含两侧边栏与独立滚动容器——可将任意业务页面置入其中。",
            )
        };
        div()
            .size_full()
            .p(px(32.))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(10.))
            .child(
                div()
                    .size(px(38.))
                    .rounded(px(10.))
                    .bg(p.chip)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_icon(ProIcon::PageDoc, 17., p.muted)),
            )
            .child(
                div()
                    .text_size(px(13.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(tr(cx, "Your page goes here", "在此放置你的页面内容")),
            )
            .child(
                div()
                    .max_w(px(280.))
                    .text_size(px(12.5))
                    .line_height(px(18.5))
                    .text_color(p.muted)
                    .text_center()
                    .child(subtitle),
            )
    }
}

fn when_not_collapsed(collapsed: bool, el: AnyElement) -> AnyElement {
    if collapsed {
        div().into_any_element()
    } else {
        el
    }
}

impl AppShell {
    fn render_chrome(
        &mut self,
        w: &mut Window,
        pre_page: Option<AnyElement>,
        resolved_bound_panel: Option<(SharedString, ProIcon, AnyElement)>,
        pre_custom_panel: Option<AnyElement>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let is_flat = self.layout == AppShellLayout::Flat;
        let width = self.measured_width.get();
        let compact = width < 680.;
        let medium = width < 860.;
        let measure_cell = self.measured_width.clone();

        let page_content =
            pre_page.unwrap_or_else(|| self.render_default_placeholder(cx).into_any_element());

        let has_active_panel = if self.bound_panel.is_some() {
            resolved_bound_panel.is_some()
        } else {
            self.has_work_area
        };
        if let Some((ref t, ic, _)) = resolved_bound_panel {
            self.panel_title = t.clone();
            self.panel_icon = ic;
        }

        let target_side_w = if compact || self.sidebar_collapsed {
            48.
        } else {
            self.sidebar_width
        };
        let target_panel_w = if has_active_panel && self.panel_open && !medium {
            self.panel_width
        } else {
            0.
        };
        if !self.slides_ready {
            self.side_w_slide.reset(target_side_w);
            self.panel_w_slide.reset(target_panel_w);
            self.slides_ready = true;
        } else {
            match self.dragging {
                Some((DragEdge::Left, _, _)) => self.side_w_slide.reset(target_side_w),
                _ => self.side_w_slide.duration(target_side_w, 0.25, cx),
            }
            match self.dragging {
                Some((DragEdge::Right, _, _)) => self.panel_w_slide.reset(target_panel_w),
                _ => self.panel_w_slide.duration(target_panel_w, 0.26, cx),
            }
        }
        let anim_side_w = self.side_w_slide.get(w, cx).max(48.);
        let anim_panel_w = self.panel_w_slide.get(w, cx).max(0.);

        let page_scroll = self.page_scroll;
        let main_card = div()
            .id((self.id.clone(), "main-scroll"))
            .size_full()
            .bg(p.panel)
            .when(!is_flat, |d| {
                d.rounded(px(12.)).border_1().border_color(p.line)
            })
            .when(page_scroll, |d| d.overflow_y_scroll())
            .when(!page_scroll, |d| d.overflow_hidden().flex().flex_col())
            .relative()
            .child(page_content)
            .when(compact && !self.mobile_side_open, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(px(10.))
                        .left(px(10.))
                        .child(pro_icon_btn(
                            (self.id.clone(), "mobile-open-side"),
                            tr(cx, "Open menu", "打开菜单"),
                            ProIcon::SidebarLeft,
                            28.,
                            false,
                            cx,
                            |this, _, cx| {
                                this.mobile_side_open = true;
                                cx.notify();
                            },
                        )),
                )
            })
            .when(has_active_panel && !self.panel_open && !is_flat, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(px(10.))
                        .right(px(10.))
                        .child(pro_icon_btn(
                            (self.id.clone(), "reopen-panel"),
                            tr(cx, "Open work area", "打开工作区"),
                            ProIcon::SidebarRight,
                            28.,
                            false,
                            cx,
                            |this, _, cx| {
                                this.panel_open = true;
                                cx.emit(AppShellEvent::PanelOpen(true));
                                cx.notify();
                            },
                        )),
                )
            });

        let main_wrap = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .when(!is_flat, |d| {
                d.py(px(8.))
                    .when(compact, |d| d.px(px(8.)))
                    .when(!compact, |d| {
                        d.pr(px(if has_active_panel && self.panel_open && !medium {
                            4.
                        } else {
                            8.
                        }))
                    })
            })
            .child(main_card);

        // Optional flat top bar across columns
        let flat_top_bar = if is_flat && self.show_header {
            let ws_letter: String = self
                .workspace
                .chars()
                .next()
                .unwrap_or('A')
                .to_uppercase()
                .collect();
            let title_text = self
                .header_title
                .clone()
                .unwrap_or_else(|| self.active.clone());
            Some(
                div()
                    .h(px(48.))
                    .w_full()
                    .flex_none()
                    .border_b_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w(px(if compact { 48. } else { anim_side_w }))
                            .h_full()
                            .overflow_hidden()
                            .px(px(10.))
                            .bg(p.side_tint)
                            .border_r_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_between()
                            .when(!compact && !self.sidebar_collapsed, |d| {
                                d.child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .child(
                                            div()
                                                .size(px(18.))
                                                .rounded(px(5.))
                                                .bg(if p.dark {
                                                    rgb(0x2a2a2e)
                                                } else {
                                                    rgb(0xffffff)
                                                })
                                                .border_1()
                                                .border_color(p.line_strong)
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .text_size(px(10.5))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(p.ink)
                                                .child(ws_letter),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(13.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.ink)
                                                .child(self.workspace.clone()),
                                        )
                                        .child(pro_icon(ProIcon::ChevronDown, 12., p.faint)),
                                )
                            })
                            .child(pro_icon_btn(
                                (self.id.clone(), "flat-side-btn"),
                                tr(cx, "Toggle sidebar", "切换侧边栏"),
                                ProIcon::SidebarLeft,
                                28.,
                                false,
                                cx,
                                move |this, _, cx| {
                                    if compact {
                                        this.mobile_side_open = !this.mobile_side_open;
                                    } else {
                                        this.sidebar_collapsed = !this.sidebar_collapsed;
                                    }
                                    cx.notify();
                                },
                            )),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .px(px(16.))
                            .bg(p.panel)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .child(title_text)
                                    .child(pro_icon(ProIcon::ChevronDown, 12., p.faint)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .child(pro_icon_btn(
                                        (self.id.clone(), "flat-share"),
                                        "Share",
                                        ProIcon::Link,
                                        28.,
                                        false,
                                        cx,
                                        |_, _, _| {},
                                    ))
                                    .child(pro_icon_btn(
                                        (self.id.clone(), "flat-more"),
                                        "More",
                                        ProIcon::MoreHorizontal,
                                        28.,
                                        false,
                                        cx,
                                        |_, _, _| {},
                                    ))
                                    .when(self.has_work_area && !self.panel_open, |d| {
                                        d.child(pro_icon_btn(
                                            (self.id.clone(), "flat-open-panel"),
                                            tr(cx, "Open work area", "打开工作区"),
                                            ProIcon::SidebarRight,
                                            28.,
                                            false,
                                            cx,
                                            |this, _, cx| {
                                                this.panel_open = true;
                                                cx.notify();
                                            },
                                        ))
                                    }),
                            ),
                    )
                    .when(self.has_work_area && anim_panel_w > 1. && !medium, |d| {
                        let tab_title = self
                            .tabs
                            .get(self.active_tab)
                            .map(|t| t.title.clone())
                            .unwrap_or_else(|| self.panel_title.clone());
                        d.child(
                            div().w(px(anim_panel_w)).h_full().overflow_hidden().child(
                                div()
                                    .w(px(self.panel_width))
                                    .h_full()
                                    .px(px(12.))
                                    .bg(p.panel)
                                    .border_l_1()
                                    .border_color(p.line)
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .child(
                                                div()
                                                    .h(px(28.))
                                                    .px(px(8.))
                                                    .rounded(px(7.))
                                                    .bg(p.chip)
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(6.))
                                                    .text_size(px(12.5))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.ink)
                                                    .child(pro_icon(ProIcon::Globe, 13., p.muted))
                                                    .child(tab_title),
                                            )
                                            .child(pro_icon_btn(
                                                (self.id.clone(), "flat-plus-tab"),
                                                tr(cx, "New tab", "新标签页"),
                                                ProIcon::Plus,
                                                26.,
                                                false,
                                                cx,
                                                |this, _, cx| this.add_new_tab(cx),
                                            )),
                                    )
                                    .child(pro_icon_btn(
                                        (self.id.clone(), "flat-close-panel"),
                                        tr(cx, "Close work area", "关闭工作区"),
                                        ProIcon::SidebarRight,
                                        28.,
                                        false,
                                        cx,
                                        |this, _, cx| {
                                            this.panel_open = false;
                                            cx.notify();
                                        },
                                    )),
                            ),
                        )
                    }),
            )
        } else {
            None
        };

        let show_inline_side = !compact;
        let show_inline_panel = has_active_panel && self.panel_open && !medium;

        let side_el = if show_inline_side {
            let show_rail = anim_side_w <= 62.;
            let inner = self.render_sidebar(false, show_rail, cx);
            Some(
                div()
                    .w(px(if show_rail { 48. } else { anim_side_w }))
                    .h_full()
                    .flex_none()
                    .overflow_hidden()
                    .child(inner),
            )
        } else {
            None
        };
        let panel_el = if !medium && anim_panel_w > 1.0 && (has_active_panel || self.has_work_area)
        {
            let inner_panel = if let Some((title, icon, body)) = resolved_bound_panel {
                Some(self.render_custom_inspector(title, icon, body, cx))
            } else if self.has_work_area {
                Some(self.render_work_area(pre_custom_panel, cx))
            } else {
                None
            };
            let fade = (anim_panel_w / self.panel_width.max(1.)).clamp(0., 1.);
            inner_panel.map(|pn| {
                div()
                    .w(px(anim_panel_w))
                    .h_full()
                    .flex_none()
                    .overflow_hidden()
                    .opacity(fade)
                    .child(pn)
            })
        } else {
            None
        };

        let left_resizer = (!compact && !self.sidebar_collapsed).then(|| {
            let x = self.sidebar_width;
            div()
                .id((self.id.clone(), "resize-left"))
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
                            this.sidebar_width = 272.;
                            this.dragging = None;
                        } else {
                            this.dragging = Some((DragEdge::Left, f32::from(ev.position.x), x));
                        }
                        cx.notify();
                    }),
                )
        });

        let right_resizer = show_inline_panel.then(|| {
            let pw = self.panel_width;
            div()
                .id((self.id.clone(), "resize-right"))
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
                            this.panel_width = 360.;
                            this.dragging = None;
                        } else {
                            this.dragging = Some((DragEdge::Right, f32::from(ev.position.x), pw));
                        }
                        cx.notify();
                    }),
                )
        });

        let mut root = div()
            .id(self.id.clone())
            .relative()
            .size_full()
            .min_h(px(560.))
            .bg(p.page)
            .text_color(p.text)
            .text_size(px(13.))
            .overflow_hidden()
            .flex()
            .flex_col()
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, cx| {
                if let Some((edge, start_x, start_w)) = this.dragging {
                    let dx = f32::from(ev.position.x) - start_x;
                    match edge {
                        DragEdge::Left => {
                            this.sidebar_width = (start_w + dx).clamp(220., 380.);
                        }
                        DragEdge::Right => {
                            this.panel_width = (start_w - dx).clamp(280., 520.);
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
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                if ev.keystroke.modifiers.platform && ev.keystroke.key.eq_ignore_ascii_case("k") {
                    this.cmd_open = !this.cmd_open;
                    cx.notify();
                } else if ev.keystroke.key == "escape"
                    && (this.cmd_open
                        || this.ws_menu_open
                        || this.user_menu_open
                        || this.mobile_side_open)
                {
                    this.cmd_open = false;
                    this.ws_menu_open = false;
                    this.user_menu_open = false;
                    this.mobile_side_open = false;
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
            .when_some(flat_top_bar, |d, bar| d.child(bar))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .flex()
                    .when_some(side_el, |d, s| d.child(s))
                    .when_some(left_resizer, |d, r| d.child(r))
                    .child(main_wrap)
                    .when_some(right_resizer, |d, r| d.child(r))
                    .when_some(panel_el, |d, pn| d.child(pn)),
            );

        // Mobile sidebar drawer overlay with slide + scrim fade
        if compact {
            let drawer_side = self.render_sidebar(true, false, cx);
            let mob_id = self.id.clone();
            let scrim_close = cx.listener(|this, _, _, cx| {
                this.mobile_side_open = false;
                cx.notify();
            });
            let side_w = self.sidebar_width;
            root = root.child(pro_presence(
                (self.id.clone(), "mobile-drawer"),
                self.mobile_side_open,
                0.24,
                0.16,
                move |a| {
                    div()
                        .id((mob_id, "mobile-scrim"))
                        .absolute()
                        .inset_0()
                        .bg(hsla(0., 0., 0., 0.45 * a))
                        .flex()
                        .on_mouse_down(MouseButton::Left, scrim_close)
                        .child(
                            div()
                                .relative()
                                .left(px(-side_w * (1. - a)))
                                .h_full()
                                .bg(p.page)
                                .border_r_1()
                                .border_color(p.line_strong)
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .child(drawer_side),
                        )
                        .into_any_element()
                },
            ));
        }

        // Workspace switcher popover with smooth presence
        {
            let ws_cur = self.workspace.clone();
            let ws_base_id = self.id.clone();
            let ws_items = ["Acme", "Fieldwork", "Northwind"]
                .into_iter()
                .map(|ws| {
                    let current = ws_cur.as_ref() == ws;
                    activate(
                        div()
                            .id((
                                ws_base_id.clone(),
                                SharedString::from(format!("ws-opt-{ws}")),
                            ))
                            .role(Role::MenuItem)
                            .tab_index(0)
                            .h(px(30.))
                            .px(px(8.))
                            .rounded(px(7.))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(13.))
                            .text_color(p.ink)
                            .cursor_pointer()
                            .child(ws)
                            .when(current, |d| d.child(pro_icon(ProIcon::Check, 13., p.ink))),
                        cx,
                        move |this, _, cx| {
                            this.workspace = ws.into();
                            this.ws_menu_open = false;
                            cx.notify();
                        },
                    )
                })
                .collect::<Vec<_>>();
            let ws_heading: SharedString = tr(cx, "Workspaces", "工作区").into();
            let pop_id = self.id.clone();
            root = root.child(pro_presence(
                (self.id.clone(), "ws-pres"),
                self.ws_menu_open,
                0.18,
                0.12,
                move |a| {
                    div()
                        .id((pop_id, "ws-popover"))
                        .absolute()
                        .top(px(44. - 5. * (1. - a)))
                        .left(px(12.))
                        .w(px(230.))
                        .p(px(5.))
                        .rounded(px(11.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line_strong)
                        .shadow_md()
                        .opacity(a)
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .px(px(8.))
                                .py(px(4.))
                                .text_size(px(11.5))
                                .text_color(p.faint)
                                .child(ws_heading),
                        )
                        .children(ws_items)
                        .into_any_element()
                },
            ));
        }

        // Command palette modal (⌘K) with smooth presence
        {
            let query_str = self
                .cmd_editor
                .as_ref()
                .map(|e| e.read(cx).value().to_ascii_lowercase())
                .unwrap_or_default();
            let all_items: Vec<ShellNavItem> = self
                .groups
                .iter()
                .flat_map(|g| g.items.iter().cloned())
                .collect();
            let filtered: Vec<ShellNavItem> = all_items
                .into_iter()
                .filter(|it| {
                    query_str.is_empty() || it.label.to_ascii_lowercase().contains(&query_str)
                })
                .take(7)
                .collect();
            let cmd_ed = self.cmd_editor.clone();
            let cmd_base_id = self.id.clone();
            let close_cmd = cx.listener(|this, _, _, cx| {
                this.cmd_open = false;
                cx.notify();
            });
            let cmd_rows = filtered
                .into_iter()
                .map(|it| {
                    let lbl = it.label.clone();
                    let cid = it.id.clone();
                    activate(
                        div()
                            .id((
                                cmd_base_id.clone(),
                                SharedString::from(format!("cmd-it-{cid}")),
                            ))
                            .role(Role::ListBoxOption)
                            .tab_index(0)
                            .h(px(32.))
                            .px(px(10.))
                            .rounded(px(7.))
                            .hover(move |s| s.bg(p.hover))
                            .flex()
                            .items_center()
                            .gap(px(9.))
                            .text_size(px(13.))
                            .text_color(p.ink)
                            .cursor_pointer()
                            .child(pro_icon(it.icon, 15., p.muted))
                            .child(it.label),
                        cx,
                        move |this, _, cx| {
                            this.active = lbl.clone();
                            this.cmd_open = false;
                            cx.emit(AppShellEvent::Navigate(lbl.clone()));
                            cx.notify();
                        },
                    )
                })
                .collect::<Vec<_>>();
            let backdrop_id = self.id.clone();
            let scrim_max = if p.dark { 0.45 } else { 0.16 };
            root = root.child(pro_presence(
                (self.id.clone(), "cmd-pres"),
                self.cmd_open,
                0.22,
                0.14,
                move |a| {
                    div()
                        .id((backdrop_id, "cmd-backdrop"))
                        .absolute()
                        .inset_0()
                        .bg(hsla(0., 0., 0., scrim_max * a))
                        .pt(px(72. + 8. * (1. - a)))
                        .px(px(16.))
                        .flex()
                        .justify_center()
                        .items_start()
                        .on_mouse_down(MouseButton::Left, close_cmd)
                        .child(
                            div()
                                .w_full()
                                .max_w(px(520.))
                                .rounded(px(12.))
                                .bg(p.card)
                                .border_1()
                                .border_color(p.line_strong)
                                .shadow_lg()
                                .opacity(a)
                                .overflow_hidden()
                                .flex()
                                .flex_col()
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
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
                                        .when_some(cmd_ed, |d, ed| {
                                            d.child(div().flex_1().min_w_0().child(ed))
                                        })
                                        .child(
                                            div()
                                                .px(px(5.))
                                                .h(px(18.))
                                                .rounded(px(4.))
                                                .bg(p.chip)
                                                .flex()
                                                .items_center()
                                                .text_size(px(10.5))
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
                                        .children(cmd_rows),
                                ),
                        )
                        .into_any_element()
                },
            ));
        }

        let _ = Theme::global(cx);
        root
    }
}

#[derive(IntoElement)]
struct AppShellDeferred {
    shell: WeakEntity<AppShell>,
}

impl RenderOnce for AppShellDeferred {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let Ok((custom_page, bound_panel, custom_panel)) = self.shell.read_with(cx, |s, _| {
            (
                s.custom_page.clone(),
                s.bound_panel.clone(),
                s.custom_panel.clone(),
            )
        }) else {
            return div().into_any_element();
        };
        let pre_page = custom_page.map(|f| f(w, cx));
        let resolved_bound_panel = bound_panel.and_then(|f| f(w, cx));
        let pre_custom_panel = if resolved_bound_panel.is_none() {
            custom_panel.map(|f| f(w, cx))
        } else {
            None
        };
        self.shell
            .update(cx, |this, cx| {
                this.render_chrome(w, pre_page, resolved_bound_panel, pre_custom_panel, cx)
                    .into_any_element()
            })
            .unwrap_or_else(|_| div().into_any_element())
    }
}

impl Render for AppShell {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editors(w, cx);
        let _ = Theme::global(cx);
        AppShellDeferred {
            shell: cx.entity().downgrade(),
        }
    }
}
