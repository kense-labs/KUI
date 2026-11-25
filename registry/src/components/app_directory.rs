//! SaaS integrations directory with category navigation, status filters, and connect modal.
//!
//! Lets workspace admins browse, search, filter, and connect or disconnect third-party
//! tools across categories with live connected counts and an interactive OAuth / API-key
//! authorization dialog.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_btn, pro_card_shadow, pro_check_draw, pro_eid,
        pro_empty_state, pro_icon_btn, pro_mark_selected, pro_mix_color, pro_modal_appear,
        pro_pill_indicator, pro_popover_presence, pro_selection_sensor, pro_skeleton_bar,
        pro_swap_label, pro_tactile, pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::{UiLanguage, activate},
        editor::{EditEvent, Editor, InputMode},
        family::{Slide, disabled_motion},
        pro_icons::{ProIcon, person_initials, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const VERIFIED_ROSETTE_SVG: &str = r##"<path d="M12 1.2 14.3 3.3 17.4 2.6 18.4 5.6 21.4 6.6 20.7 9.7 22.8 12 20.7 14.3 21.4 17.4 18.4 18.4 17.4 21.4 14.3 20.7 12 22.8 9.7 20.7 6.6 21.4 5.6 18.4 2.6 17.4 3.3 14.3 1.2 12 3.3 9.7 2.6 6.6 5.6 5.6 6.6 2.6 9.7 3.3Z" fill="currentColor"/><path d="m8.2 12.2 2.4 2.4 5.2-5.4" stroke="#ffffff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>"##;

/// Authorization flow style supported inside the connect modal of [`AppDirectoryBlock`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppAuthKind {
    /// One-click workspace OAuth grant with scoped permissions.
    #[default]
    OAuth,
    /// Personal access token or secret API key entry.
    ApiKey,
}

impl AppAuthKind {
    fn key(self) -> &'static str {
        match self {
            Self::OAuth => "oauth",
            Self::ApiKey => "apikey",
        }
    }
}

/// Connection status filter menu selection in [`AppDirectoryBlock`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppStatusFilter {
    /// Show every integration in the current category.
    #[default]
    All,
    /// Show only currently connected integrations.
    Connected,
    /// Show only integrations that have not been connected yet.
    NotConnected,
    /// Show only verified first-party publisher integrations.
    VerifiedOnly,
}

impl AppStatusFilter {
    const ALL: [Self; 4] = [
        Self::All,
        Self::Connected,
        Self::NotConnected,
        Self::VerifiedOnly,
    ];

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::All => tr(cx, "All apps", "全部状态"),
            Self::Connected => tr(cx, "Connected", "已连接"),
            Self::NotConnected => tr(cx, "Not connected", "未连接"),
            Self::VerifiedOnly => tr(cx, "Verified", "官方认证"),
        }
    }
}

/// Sidebar or chip-strip category entry in [`AppDirectoryBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppCategory {
    /// Category slug (for example `"communication"` or `"development"`).
    pub id: SharedString,
    /// Human-readable category label.
    pub label: SharedString,
    /// Leading 24x24 line icon.
    pub icon: ProIcon,
}

impl AppCategory {
    /// Construct a custom [`AppCategory`] with `id`, `label`, and `icon`.
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>, icon: ProIcon) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
        }
    }
}

/// Integration entry rendered inside [`AppDirectoryBlock`].
#[derive(Clone, Debug, PartialEq)]
pub struct DirectoryApp {
    /// Stable slug (for example `"slack"` or `"github"`).
    pub id: SharedString,
    /// Display name (also used to resolve [`pro_app_tile`] artwork).
    pub name: SharedString,
    /// Publisher or vendor subtitle.
    pub vendor: SharedString,
    /// Two-line integration summary shown on the card.
    pub description: SharedString,
    /// Category ids this integration belongs to.
    pub categories: Vec<SharedString>,
    /// Optional custom brand accent for fallback tiles.
    pub tile_color: Option<Hsla>,
    /// Whether this app appears under the built-in `Essentials` view.
    pub essential: bool,
    /// Whether this app is currently connected to the workspace.
    pub connected: bool,
    /// Whether the vendor has a verified publisher badge.
    pub verified: bool,
    /// Default authorization flow selected when opening the connect modal.
    pub default_auth: AppAuthKind,
    /// Scopes summarized in the OAuth authorization dialog.
    pub scopes: Vec<SharedString>,
}

impl DirectoryApp {
    /// Construct a [`DirectoryApp`] with `id`, `name`, `vendor`, and `description`.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        vendor: impl Into<SharedString>,
        description: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            vendor: vendor.into(),
            description: description.into(),
            categories: Vec::new(),
            tile_color: None,
            essential: false,
            connected: false,
            verified: true,
            default_auth: AppAuthKind::OAuth,
            scopes: vec![
                "Read workspace metadata and member directory".into(),
                "Post grounded updates and receive webhook events".into(),
            ],
        }
    }

    /// Assign category slugs for sidebar filtering.
    pub fn with_categories(mut self, categories: Vec<SharedString>) -> Self {
        self.categories = categories;
        self
    }

    /// Set an optional custom tile background color.
    pub fn with_tile_color(mut self, color: Hsla) -> Self {
        self.tile_color = Some(color);
        self
    }

    /// Mark whether this integration belongs to the `Essentials` collection.
    pub fn with_essential(mut self, essential: bool) -> Self {
        self.essential = essential;
        self
    }

    /// Set initial connection state.
    pub fn with_connected(mut self, connected: bool) -> Self {
        self.connected = connected;
        self
    }

    /// Set whether a verified publisher badge is shown next to the app name.
    pub fn with_verified(mut self, verified: bool) -> Self {
        self.verified = verified;
        self
    }

    /// Choose the default authentication tab inside the connect modal.
    pub fn with_default_auth(mut self, auth: AppAuthKind) -> Self {
        self.default_auth = auth;
        self
    }

    /// Override the permission scopes displayed in the OAuth connect dialog.
    pub fn with_scopes(mut self, scopes: Vec<SharedString>) -> Self {
        self.scopes = scopes;
        self
    }

    fn matches_query(&self, query: &str, desc_localized: &str) -> bool {
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            return true;
        }
        self.name.to_ascii_lowercase().contains(&q)
            || self.vendor.to_ascii_lowercase().contains(&q)
            || self.description.to_ascii_lowercase().contains(&q)
            || desc_localized.to_ascii_lowercase().contains(&q)
            || self
                .categories
                .iter()
                .any(|c| c.to_ascii_lowercase().contains(&q))
    }

    fn matches_status(&self, status: AppStatusFilter) -> bool {
        match status {
            AppStatusFilter::All => true,
            AppStatusFilter::Connected => self.connected,
            AppStatusFilter::NotConnected => !self.connected,
            AppStatusFilter::VerifiedOnly => self.verified,
        }
    }
}

/// Typed events emitted by [`AppDirectoryBlock`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppDirectoryEvent {
    /// An integration was connected or disconnected.
    Toggled {
        id: SharedString,
        name: SharedString,
        connected: bool,
    },
    /// Active category selection changed.
    CategorySelected(SharedString),
    /// Status filter selection changed.
    FilterChanged(AppStatusFilter),
    /// User opened the connect modal for an integration.
    ConnectOpened(SharedString),
    /// User clicked the `Learn more` documentation link.
    LearnMoreClicked,
}

fn default_categories() -> Vec<AppCategory> {
    vec![
        AppCategory::new("analytics", "Analytics", ProIcon::UsageChart),
        AppCategory::new("automation", "Automation", ProIcon::Bolt),
        AppCategory::new("billing", "Billing", ProIcon::Coins),
        AppCategory::new("communication", "Communication", ProIcon::Thread),
        AppCategory::new("crm", "CRM", ProIcon::Users),
        AppCategory::new("design", "Design", ProIcon::Sparkles),
        AppCategory::new("development", "Development", ProIcon::Code),
        AppCategory::new("docs", "Docs", ProIcon::PageDoc),
        AppCategory::new("product", "Product", ProIcon::Tasks),
        AppCategory::new("scheduling", "Scheduling", ProIcon::Calendar),
        AppCategory::new("storage", "Storage", ProIcon::Database),
    ]
}

fn default_apps() -> Vec<DirectoryApp> {
    vec![
        DirectoryApp::new(
            "slack",
            "Slack",
            "Salesforce",
            "Route approvals, incident alerts and daily digests into shared channels.",
        )
        .with_categories(vec!["communication".into(), "automation".into()])
        .with_essential(true)
        .with_connected(true)
        .with_scopes(vec![
            "Post messages and interactive approval cards in channels".into(),
            "Read channel threads when @mentioned by team members".into(),
        ]),
        DirectoryApp::new(
            "notion",
            "Notion",
            "Notion Labs",
            "Search wiki pages, sync meeting notes and draft specs in team databases.",
        )
        .with_categories(vec!["docs".into(), "product".into()])
        .with_essential(true)
        .with_scopes(vec![
            "Read pages and databases shared with the workspace integration".into(),
            "Create and update structured doc drafts".into(),
        ]),
        DirectoryApp::new(
            "linear",
            "Linear",
            "Linear Orbit",
            "Create triaged issues, update cycle status and link customer threads.",
        )
        .with_categories(vec!["product".into(), "development".into()])
        .with_essential(true)
        .with_scopes(vec![
            "Read teams, cycles, projects and issue comments".into(),
            "Create issues and update triage labels".into(),
        ]),
        DirectoryApp::new(
            "figma",
            "Figma",
            "Figma, Inc.",
            "Pull frame previews, inspect design tokens and sync component comments.",
        )
        .with_categories(vec!["design".into(), "product".into()])
        .with_essential(true),
        DirectoryApp::new(
            "github",
            "GitHub",
            "GitHub, Inc.",
            "Link pull requests, inspect CI checks and draft release changelogs.",
        )
        .with_categories(vec!["development".into()])
        .with_essential(true)
        .with_scopes(vec![
            "Read pull requests, commit statuses and release tags".into(),
            "Open review comments and draft release notes".into(),
        ]),
        DirectoryApp::new(
            "stripe",
            "Stripe",
            "Stripe, Inc.",
            "Monitor subscription MRR, invoice status and refund webhooks live.",
        )
        .with_categories(vec!["billing".into(), "analytics".into()])
        .with_essential(true)
        .with_default_auth(AppAuthKind::ApiKey)
        .with_scopes(vec![
            "Read customers, invoices, subscriptions and credit notes".into(),
            "Receive real-time billing webhook events".into(),
        ]),
        DirectoryApp::new(
            "google-calendar",
            "Google Calendar",
            "Google Workspace",
            "Check availability, prep meeting briefs and schedule customer reviews.",
        )
        .with_categories(vec!["scheduling".into()])
        .with_essential(true),
        DirectoryApp::new(
            "hubspot",
            "HubSpot",
            "HubSpot, Inc.",
            "Enrich company records, log call outcomes and track deal stage velocity.",
        )
        .with_categories(vec!["crm".into(), "analytics".into()])
        .with_essential(true),
        DirectoryApp::new(
            "gmail",
            "Gmail",
            "Google Workspace",
            "Draft grounded customer replies and organize shared inbox threads.",
        )
        .with_categories(vec!["communication".into()]),
        DirectoryApp::new(
            "google-drive",
            "Google Drive",
            "Google Workspace",
            "Index shared folders, spreadsheets and signed PDF agreements.",
        )
        .with_categories(vec!["storage".into(), "docs".into()]),
        DirectoryApp::new(
            "supabase",
            "Supabase",
            "Supabase, Inc.",
            "Query read-only Postgres views, auth events and storage buckets.",
        )
        .with_categories(vec!["storage".into(), "development".into()])
        .with_default_auth(AppAuthKind::ApiKey),
        DirectoryApp::new(
            "jira",
            "Jira",
            "Atlassian",
            "Synchronize enterprise engineering backlogs, epics and SLA timers.",
        )
        .with_categories(vec!["product".into(), "development".into()]),
        DirectoryApp::new(
            "vercel",
            "Vercel",
            "Vercel, Inc.",
            "Inspect preview deployments, build logs and edge function latency.",
        )
        .with_categories(vec!["development".into(), "automation".into()]),
        DirectoryApp::new(
            "postgres",
            "Postgres",
            "PostgreSQL Global",
            "Connect a read-replica warehouse for live financial and usage queries.",
        )
        .with_categories(vec!["analytics".into(), "storage".into()])
        .with_verified(false)
        .with_default_auth(AppAuthKind::ApiKey),
        DirectoryApp::new(
            "intercom",
            "Intercom",
            "Intercom, Inc.",
            "Sync messenger conversations, CSAT ratings and VIP account tags.",
        )
        .with_categories(vec!["crm".into(), "communication".into()]),
        DirectoryApp::new(
            "zendesk",
            "Zendesk",
            "Zendesk, Inc.",
            "Triage support queues, attach internal notes and route escalations.",
        )
        .with_categories(vec!["communication".into(), "automation".into()]),
        DirectoryApp::new(
            "zoom",
            "Zoom",
            "Zoom Video",
            "Import cloud call recordings and transcripts into client timelines.",
        )
        .with_categories(vec!["scheduling".into(), "communication".into()]),
        DirectoryApp::new(
            "retool",
            "Retool",
            "Retool, Inc.",
            "Trigger internal admin workflows and back-office credit approvals.",
        )
        .with_categories(vec!["automation".into(), "billing".into()])
        .with_default_auth(AppAuthKind::ApiKey),
    ]
}

fn translate_category_label<'a>(id: &str, fallback: &'a str, cx: &App) -> &'a str {
    if UiLanguage::current(cx) == UiLanguage::English {
        return fallback;
    }
    match id {
        "essentials" => "精选必备",
        "all" => "全部应用",
        "analytics" => "数据分析",
        "automation" => "自动化",
        "billing" => "账单支付",
        "communication" => "沟通协作",
        "crm" => "客户关系",
        "design" => "设计工具",
        "development" => "研发工程",
        "docs" => "知识文档",
        "product" => "产品管理",
        "scheduling" => "日程会议",
        "storage" => "数据存储",
        _ => fallback,
    }
}

fn translate_app_desc<'a>(id: &str, fallback: &'a str, cx: &App) -> &'a str {
    if UiLanguage::current(cx) == UiLanguage::English {
        return fallback;
    }
    match id {
        "slack" => "将审批请求、故障告警与每日摘要推送至团队共享频道。",
        "notion" => "检索知识库页面、同步会议纪要并在团队数据库中起草方案。",
        "linear" => "自动创建分诊工单、更新迭代进度并关联客户反馈线程。",
        "figma" => "拉取画板预览、检视设计令牌并同步组件评审评论。",
        "github" => "关联拉取请求、检查 CI 构建状态并生成版本发布日志。",
        "stripe" => "实时追踪订阅经常性收入、账单状态与退款 Webhook 事件。",
        "google-calendar" => "查询成员空闲时段、准备会前简报并安排客户复盘会议。",
        "hubspot" => "补全企业客户档案、记录通话结果并追踪商机阶段流转。",
        "gmail" => "起草有据可查的客户邮件回复并整理共享收件箱会话。",
        "google-drive" => "索引共享文件夹、财务电子表格与已签署的 PDF 协议。",
        "supabase" => "安全查询只读 Postgres 视图、认证事件与存储桶元数据。",
        "jira" => "双向同步企业研发需求池、史诗任务与 SLA 时效指标。",
        "vercel" => "检视预览环境部署、构建日志与边缘函数响应延迟。",
        "postgres" => "连接只读分析副本以支持实时财务与产品用量查询。",
        "intercom" => "同步在线客服会话、满意度评分与重点客户标签。",
        "zendesk" => "智能分诊客服队列、附加上下文内部备注并转交升级工单。",
        "zoom" => "自动导入云端通话录音与转写文本至客户时间线。",
        "retool" => "触发内部运营管理流与后台账单贷项审批操作。",
        _ => fallback,
    }
}

fn render_directory_tile(app: &str, side: f32, custom_bg: Option<Hsla>, cx: &App) -> AnyElement {
    let p = ProPalette::get(cx);
    let radius = if side <= 24. {
        6.
    } else if side <= 34. {
        8.
    } else {
        9.
    };
    if let Some(bg) = custom_bg {
        return div()
            .size(px(side))
            .rounded(px(radius))
            .bg(bg)
            .border_1()
            .border_color(if p.dark {
                hsla(0., 0., 1., 0.09)
            } else {
                hsla(0., 0., 0., 0.08)
            })
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px((side * 0.40).round()))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(0xffffff))
            .child(
                app.chars()
                    .next()
                    .map(|c| c.to_uppercase().collect::<String>())
                    .unwrap_or_else(|| "A".to_owned()),
            )
            .into_any_element();
    }
    if app.trim().eq_ignore_ascii_case("stripe")
        && let Some(mark) = pro_app_artwork("stripe", side, cx)
    {
        return div()
            .size(px(side))
            .flex_none()
            .rounded(px(radius))
            .overflow_hidden()
            .child(mark)
            .into_any_element();
    }
    let mark_side = (side * 0.58).round();
    let content = if let Some(mark) = pro_app_artwork(app, mark_side, cx) {
        mark
    } else {
        let initial = person_initials(app);
        div()
            .text_size(px((side * 0.40).max(9.)))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.ink)
            .child(if initial.is_empty() {
                "A".to_owned()
            } else {
                initial.chars().take(1).collect()
            })
            .into_any_element()
    };
    let tile_bg: Hsla = if p.dark {
        rgb(0x28282c).into()
    } else {
        rgb(0xffffff).into()
    };
    let bdr: Hsla = if p.dark {
        hsla(0., 0., 1., 0.09)
    } else {
        hsla(0., 0., 0., 0.065)
    };
    let mut tile = div()
        .size(px(side))
        .flex_none()
        .rounded(px(radius))
        .bg(tile_bg)
        .border_1()
        .border_color(bdr)
        .flex()
        .items_center()
        .justify_center()
        .child(content);
    if !p.dark && side >= 28. {
        tile = tile.shadow(vec![
            BoxShadow::new(px(0.), px(1.), black().opacity(0.05)).blur_radius(px(2.)),
        ]);
    }
    tile.into_any_element()
}

struct AppCardAnim {
    hover: Slide,
}

#[derive(IntoElement)]
struct AppCardSurface {
    id: ElementId,
    content: AnyElement,
}

impl RenderOnce for AppCardSurface {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let st = w.use_keyed_state(pro_eid(&self.id, "card-surf"), cx, |_, _| AppCardAnim {
            hover: Slide::snappy(0.),
        });
        let hov = st.update(cx, |s, cx| s.hover.get(w, cx).clamp(0., 1.));

        let border = pro_mix_color(p.line, p.line_strong, hov);
        let bg = pro_mix_color(p.card, pro_mix_color(p.card, p.ink, 0.018), hov);

        let paint_face = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                if bounds.size.width <= px(2.) || bounds.size.height <= px(2.) {
                    return;
                }
                let r = px(12.);
                window.paint_quad(quad(
                    bounds,
                    Corners::all(r),
                    bg,
                    Edges::all(px(1.)),
                    border,
                    BorderStyle::Solid,
                ));
            },
        )
        .absolute()
        .inset_0();

        div()
            .id(self.id)
            .flex_1()
            .relative()
            .rounded(px(12.))
            .shadow(pro_card_shadow(p.dark))
            .p(px(16.))
            .flex()
            .flex_col()
            .justify_between()
            .on_hover({
                let st = st.clone();
                move |inside, w, cx| {
                    st.update(cx, |s, cx| {
                        s.hover.set(if *inside { 1. } else { 0. }, cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .child(paint_face)
            .child(
                div()
                    .relative()
                    .top(px(-0.6 * hov))
                    .flex_1()
                    .flex()
                    .flex_col()
                    .justify_between()
                    .child(self.content),
            )
    }
}

struct AppConnectBtnAnim {
    hover: Slide,
    press: Slide,
    conn_val: bool,
    conn_slide: Slide,
}

type AppClickCallback = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(IntoElement)]
struct AppConnectBtn {
    id: ElementId,
    app_name: SharedString,
    connected: bool,
    on_click: AppClickCallback,
}

impl RenderOnce for AppConnectBtn {
    fn render(self, w: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let init_conn = self.connected;
        let st = w.use_keyed_state(pro_eid(&self.id, "cbtn-st"), cx, |_, _| AppConnectBtnAnim {
            hover: Slide::snappy(0.),
            press: Slide::snappy(0.),
            conn_val: init_conn,
            conn_slide: Slide::snappy(f32::from(init_conn)),
        });
        let (hov, down, conn_t) = st.update(cx, |s, cx| {
            if s.conn_val != self.connected {
                s.conn_val = self.connected;
                s.conn_slide.duration(f32::from(self.connected), 0.24, cx);
            }
            (
                s.hover.get(w, cx).clamp(0., 1.),
                s.press.get(w, cx).clamp(0., 1.),
                s.conn_slide.get(w, cx).clamp(0., 1.),
            )
        });

        let unconn_bg = pro_mix_color(p.card, p.chip_hover, hov);
        let conn_bg = pro_mix_color(p.green_soft, p.red_soft, hov);
        let face_bg = pro_mix_color(unconn_bg, conn_bg, conn_t);

        let unconn_border = p.line_strong;
        let conn_border = pro_mix_color(p.green.opacity(0.24), p.red.opacity(0.28), hov);
        let border = pro_mix_color(unconn_border, conn_border, conn_t);

        let conn_fg = pro_mix_color(p.green_ink, p.red_ink, hov);
        let fg = pro_mix_color(p.ink, conn_fg, conn_t);

        let disconnect_intent = self.connected && hov > 0.45;
        let label_txt = if !self.connected {
            tr(cx, "Connect", "连接")
        } else if disconnect_intent {
            tr(cx, "Disconnect", "断开连接")
        } else {
            tr(cx, "Connected", "已连接")
        };
        let aria_txt = if self.connected {
            format!("{} {}", tr(cx, "Disconnect", "断开连接"), self.app_name)
        } else {
            format!("{} {}", tr(cx, "Connect", "连接"), self.app_name)
        };

        let dark = p.dark;
        let paint_face = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let bw = f32::from(bounds.size.width);
                let bh = f32::from(bounds.size.height);
                if bw <= 1. || bh <= 1. {
                    return;
                }
                let ix = (bw * 0.022 * down).clamp(0., 2.0);
                let iy = (bh * 0.042 * down).clamp(0., 1.3);
                let rect = Bounds::new(
                    point(bounds.origin.x + px(ix), bounds.origin.y + px(iy)),
                    size(px((bw - 2. * ix).max(1.)), px((bh - 2. * iy).max(1.))),
                );
                let r = px(6.).min(rect.size.width / 2.).min(rect.size.height / 2.);
                if !dark && hov > 0.02 && down < 0.7 {
                    window.paint_quad(
                        fill(
                            Bounds::new(point(rect.origin.x, rect.origin.y + px(1.)), rect.size),
                            black().opacity(0.04 * hov * (1. - down)),
                        )
                        .corner_radii(r),
                    );
                }
                window.paint_quad(quad(
                    rect,
                    Corners::all(r),
                    face_bg,
                    Edges::all(px(1.)),
                    border,
                    BorderStyle::Solid,
                ));
            },
        )
        .absolute()
        .inset_0();

        let show_icon = self.connected || conn_t > 0.03;
        let chk_alpha = ((1. - hov * 1.45).clamp(0., 1.) * conn_t).clamp(0., 1.);
        let close_alpha = (((hov - 0.30) / 0.50).clamp(0., 1.) * conn_t).clamp(0., 1.);

        let on_click = self.on_click;
        div()
            .id(self.id.clone())
            .role(Role::Button)
            .aria_label(aria_txt)
            .tab_index(0)
            .self_start()
            .h(px(28.))
            .px(px(10.))
            .relative()
            .rounded(px(6.))
            .flex()
            .items_center()
            .cursor_pointer()
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .on_hover({
                let st = st.clone();
                move |inside, w, cx| {
                    st.update(cx, |s, cx| {
                        s.hover.set(if *inside { 1. } else { 0. }, cx);
                        if !*inside {
                            s.press.set(0., cx);
                        }
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_down(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    st.update(cx, |s, cx| {
                        s.press.set(1., cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_up(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    st.update(cx, |s, cx| {
                        s.press.set(0., cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_mouse_up_out(MouseButton::Left, {
                let st = st.clone();
                move |_, w, cx| {
                    st.update(cx, |s, cx| {
                        s.press.set(0., cx);
                        cx.notify();
                    });
                    w.refresh();
                }
            })
            .on_key_down({
                let st = st.clone();
                move |ev, w, cx| {
                    if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                        st.update(cx, |s, cx| {
                            s.press.set(1., cx);
                            cx.notify();
                        });
                        w.refresh();
                    }
                }
            })
            .on_key_up({
                let st = st.clone();
                move |ev, w, cx| {
                    if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                        st.update(cx, |s, cx| {
                            s.press.set(0., cx);
                            cx.notify();
                        });
                        w.refresh();
                    }
                }
            })
            .on_click(move |ev, w, cx| (on_click)(ev, w, cx))
            .child(paint_face)
            .child(
                div()
                    .relative()
                    .top(px(0.5 * down))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .when(show_icon, |d| {
                        d.child(
                            div()
                                .relative()
                                .size(px(14.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .opacity(chk_alpha)
                                        .child(pro_icon(ProIcon::Link, 12., fg)),
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .inset_0()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .opacity(close_alpha)
                                        .child(pro_icon(ProIcon::Close, 12.5, fg)),
                                ),
                        )
                    })
                    .child(pro_swap_label(
                        pro_eid(&self.id, "lbl"),
                        label_txt,
                        12.5,
                        FontWeight::MEDIUM,
                        fg,
                    )),
            )
    }
}

#[derive(Clone)]
struct ConnectDialogState {
    app_id: SharedString,
    auth_mode: AppAuthKind,
}

/// SaaS integrations marketplace block with sidebar/chip categories, search, filter menu, and OAuth/API-key connect modal.
pub struct AppDirectoryBlock {
    id: ElementId,
    focus: FocusHandle,
    title: Option<SharedString>,
    subtitle: Option<SharedString>,
    apps: Vec<DirectoryApp>,
    custom_apps: bool,
    categories: Vec<AppCategory>,
    custom_categories: bool,
    active_category: SharedString,
    status_filter: AppStatusFilter,
    filter_menu_open: bool,
    connect_modal: Option<ConnectDialogState>,
    search_editor: Option<Entity<Editor>>,
    api_key_editor: Option<Entity<Editor>>,
    loading: bool,
    empty: bool,
    toast: Option<(SharedString, Instant)>,
    clock: Instant,
    measured_width: Rc<Cell<f32>>,
}

impl EventEmitter<AppDirectoryEvent> for AppDirectoryBlock {}

impl AppDirectoryBlock {
    /// Construct an [`AppDirectoryBlock`] with 18 third-party integrations and `Essentials` selected.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        Self {
            id: id.into(),
            focus: cx.focus_handle().tab_stop(true),
            title: None,
            subtitle: None,
            apps: default_apps(),
            custom_apps: false,
            categories: default_categories(),
            custom_categories: false,
            active_category: "essentials".into(),
            status_filter: AppStatusFilter::All,
            filter_menu_open: false,
            connect_modal: None,
            search_editor: None,
            api_key_editor: None,
            loading: false,
            empty: false,
            toast: None,
            clock: Instant::now(),
            measured_width: Rc::new(Cell::new(894.)),
        }
    }

    /// Override the integration catalog.
    pub fn with_apps(mut self, apps: Vec<DirectoryApp>) -> Self {
        self.custom_apps = true;
        self.apps = apps;
        self
    }

    /// Override the custom category list (`Essentials` and `All apps` remain built-in at the top).
    pub fn with_categories(mut self, categories: Vec<AppCategory>) -> Self {
        self.custom_categories = true;
        self.categories = categories;
        self
    }

    /// Override the directory heading (`"Integrations"` by default).
    pub fn with_title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Override the directory subtitle beneath the heading.
    pub fn with_subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Set the initial selected category slug (`"essentials"`, `"all"`, or a category id).
    pub fn with_category(mut self, category_id: impl Into<SharedString>) -> Self {
        self.active_category = category_id.into();
        self
    }

    /// Set the initial connection status filter.
    pub fn with_status_filter(mut self, filter: AppStatusFilter) -> Self {
        self.status_filter = filter;
        self
    }

    /// Configure the skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Configure the zero-apps empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the number of currently connected integrations.
    pub fn connected_count(&self) -> usize {
        self.apps.iter().filter(|a| a.connected).count()
    }

    /// Return the configured applications slice.
    pub fn apps(&self) -> &[DirectoryApp] {
        &self.apps
    }

    /// Programmatically toggle connection state for `id`, returning the new state if found.
    pub fn toggle_app(&mut self, id: &str, cx: &mut Context<Self>) -> Option<bool> {
        let app = self.apps.iter_mut().find(|a| a.id.as_ref() == id)?;
        app.connected = !app.connected;
        let next = app.connected;
        let app_id = app.id.clone();
        let app_name = app.name.clone();
        cx.emit(AppDirectoryEvent::Toggled {
            id: app_id,
            name: app_name,
            connected: next,
        });
        cx.notify();
        Some(next)
    }

    /// Toggle skeleton loading state at runtime.
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

    /// Toggle empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        if self.empty != empty {
            self.empty = empty;
            cx.notify();
        }
    }

    /// Return whether empty state is forced on.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let search_ph: SharedString = tr(cx, "Search apps...", "搜索集成应用...").into();
        if let Some(ed) = &self.search_editor {
            ed.update(cx, |e, _| e.set_placeholder(search_ph));
        } else {
            let ed = cx.new(|cx| {
                let mut e = Editor::new(search_ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(search_ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |this, ed_ent, ev: &EditEvent, w, cx| match ev {
                EditEvent::Changed | EditEvent::Submitted => {
                    cx.notify();
                }
                EditEvent::Escape => {
                    if !ed_ent.read(cx).value().is_empty() {
                        ed_ent.update(cx, |e, cx| e.set_value("", w, cx));
                    } else {
                        this.focus.focus(w, cx);
                    }
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.search_editor = Some(ed);
        }

        let key_ph: SharedString = tr(
            cx,
            "sk_live_... or personal access token",
            "输入 API 密钥或个人访问令牌...",
        )
        .into();
        if let Some(ed) = &self.api_key_editor {
            ed.update(cx, |e, _| e.set_placeholder(key_ph));
        } else {
            let ed = cx.new(|cx| {
                let mut e = Editor::new(key_ph.clone(), InputMode::Password, w, cx);
                e.set_placeholder(key_ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe_in(&ed, w, |this, _, ev: &EditEvent, w, cx| match ev {
                EditEvent::Submitted => {
                    this.confirm_modal_connect(w, cx);
                }
                EditEvent::Escape => {
                    this.connect_modal = None;
                    this.focus.focus(w, cx);
                    cx.notify();
                }
                EditEvent::Changed => {
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.api_key_editor = Some(ed);
        }
    }

    fn current_query(&self, cx: &App) -> String {
        self.search_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_owned())
            .unwrap_or_default()
    }

    fn category_matches(app: &DirectoryApp, category_id: &str) -> bool {
        match category_id {
            "essentials" => app.essential,
            "all" => true,
            other => app.categories.iter().any(|c| c.as_ref() == other),
        }
    }

    fn category_count(&self, category_id: &str) -> usize {
        self.apps
            .iter()
            .filter(|a| Self::category_matches(a, category_id))
            .count()
    }

    fn visible_app_indices(&self, query: &str, cx: &App) -> Vec<usize> {
        let q_non_empty = !query.trim().is_empty();
        self.apps
            .iter()
            .enumerate()
            .filter(|(_, app)| {
                let cat_ok = if q_non_empty {
                    true
                } else {
                    Self::category_matches(app, &self.active_category)
                };
                let desc = if self.custom_apps {
                    app.description.as_ref()
                } else {
                    translate_app_desc(&app.id, &app.description, cx)
                };
                cat_ok && app.matches_status(self.status_filter) && app.matches_query(query, desc)
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn open_connect_modal(&mut self, app_id: SharedString, w: &mut Window, cx: &mut Context<Self>) {
        let default_auth = self
            .apps
            .iter()
            .find(|a| a.id == app_id)
            .map(|a| a.default_auth)
            .unwrap_or(AppAuthKind::OAuth);
        if let Some(ed) = &self.api_key_editor {
            ed.update(cx, |e, cx| e.set_value("", w, cx));
        }
        self.filter_menu_open = false;
        self.connect_modal = Some(ConnectDialogState {
            app_id: app_id.clone(),
            auth_mode: default_auth,
        });
        cx.emit(AppDirectoryEvent::ConnectOpened(app_id));
        cx.notify();
    }

    fn confirm_modal_connect(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(modal) = self.connect_modal.clone() else {
            return;
        };
        if let Some(app) = self.apps.iter_mut().find(|a| a.id == modal.app_id) {
            app.connected = true;
            let id = app.id.clone();
            let name = app.name.clone();
            let msg: SharedString = if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("已连接 {name}").into()
            } else {
                format!("Connected {name}").into()
            };
            self.connect_modal = None;
            if let Some(ed) = &self.api_key_editor {
                ed.update(cx, |e, cx| e.set_value("", w, cx));
            }
            self.toast = Some((msg, Instant::now()));
            self.focus.focus(w, cx);
            cx.emit(AppDirectoryEvent::Toggled {
                id,
                name,
                connected: true,
            });
            cx.notify();
        }
    }

    fn disconnect_app(&mut self, app_id: &str, cx: &mut Context<Self>) {
        if let Some(app) = self.apps.iter_mut().find(|a| a.id.as_ref() == app_id) {
            app.connected = false;
            let id = app.id.clone();
            let name = app.name.clone();
            let msg: SharedString = if UiLanguage::current(cx) == UiLanguage::Chinese {
                format!("已断开 {name}").into()
            } else {
                format!("Disconnected {name}").into()
            };
            self.toast = Some((msg, Instant::now()));
            cx.emit(AppDirectoryEvent::Toggled {
                id,
                name,
                connected: false,
            });
            cx.notify();
        }
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        match ev.keystroke.key.as_str() {
            "escape" => {
                if self.connect_modal.is_some() {
                    self.connect_modal = None;
                    self.focus.focus(w, cx);
                    cx.stop_propagation();
                    cx.notify();
                } else if self.filter_menu_open {
                    self.filter_menu_open = false;
                    cx.stop_propagation();
                    cx.notify();
                }
            }
            "/" => {
                if self.connect_modal.is_none()
                    && let Some(ed) = &self.search_editor
                    && !ed.read(cx).focus.is_focused(w)
                {
                    ed.read(cx).focus.clone().focus(w, cx);
                    cx.stop_propagation();
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    fn render_category_nav(&self, sidebar_mode: bool, cx: &mut Context<Self>) -> AnyElement {
        let p = ProPalette::get(cx);
        let nav_sensor = pro_selection_sensor();

        let mut entries: Vec<(SharedString, SharedString, ProIcon)> = vec![
            (
                "essentials".into(),
                translate_category_label("essentials", "Essentials", cx).into(),
                ProIcon::Star,
            ),
            (
                "all".into(),
                translate_category_label("all", "All apps", cx).into(),
                ProIcon::Grid,
            ),
        ];
        for cat in &self.categories {
            let lbl: SharedString = if self.custom_categories {
                cat.label.clone()
            } else {
                translate_category_label(&cat.id, &cat.label, cx).into()
            };
            entries.push((cat.id.clone(), lbl, cat.icon));
        }

        let mut container = if sidebar_mode {
            div()
                .id(pro_eid(&self.id, "cat-sidebar"))
                .role(Role::Navigation)
                .aria_label(tr(cx, "Categories", "应用分类"))
                .w(px(184.))
                .flex_none()
                .relative()
                .flex()
                .flex_col()
                .gap(px(2.))
        } else {
            div()
                .id(pro_eid(&self.id, "cat-chips"))
                .role(Role::Navigation)
                .aria_label(tr(cx, "Categories", "应用分类"))
                .w_full()
                .overflow_x_scroll()
                .pb(px(2.))
                .relative()
                .flex()
                .items_center()
                .gap(px(4.))
        };

        container = container.child(pro_pill_indicator(
            pro_eid(&self.id, "cat-pill"),
            self.active_category.clone(),
            nav_sensor.clone(),
            6.,
            p.chip,
            None,
            false,
        ));

        for (cat_id, label, icon) in entries {
            let active = self.active_category == cat_id;
            let count = self.category_count(&cat_id);
            let cid = cat_id.clone();

            let mut inner = div()
                .h(px(28.))
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(pro_icon(icon, 13.5, if active { p.ink } else { p.muted }))
                .child(
                    div()
                        .when(sidebar_mode, |d| d.flex_1().min_w_0().truncate())
                        .when(!sidebar_mode, |d| d.whitespace_nowrap())
                        .text_size(px(13.))
                        .font_weight(if active {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(if active { p.ink } else { p.text })
                        .child(label.clone()),
                );

            if sidebar_mode {
                inner = inner.child(
                    div()
                        .flex_none()
                        .text_size(px(12.))
                        .text_color(p.faint)
                        .child(format!("{count}")),
                );
            }

            let row = pro_tactile(
                pro_eid(&self.id, format!("cat-{cat_id}")),
                format!("{label} ({count})"),
                6.,
                transparent_black(),
                if active { transparent_black() } else { p.hover },
                transparent_black(),
                transparent_black(),
                active,
                inner,
                cx,
                move |this, _, cx| {
                    if this.active_category != cid {
                        this.active_category = cid.clone();
                        this.filter_menu_open = false;
                        cx.emit(AppDirectoryEvent::CategorySelected(cid.clone()));
                        cx.notify();
                    }
                },
            )
            .relative()
            .when(sidebar_mode, |d| d.w_full())
            .when(!sidebar_mode, |d| d.flex_none())
            .when(active, |d| d.child(pro_mark_selected(&nav_sensor)));

            container = container.child(row);
        }

        container.into_any_element()
    }

    fn render_app_card(&self, app: &DirectoryApp, cx: &mut Context<Self>) -> AnyElement {
        let p = ProPalette::get(cx);
        let desc_str: SharedString = if self.custom_apps {
            app.description.clone()
        } else {
            translate_app_desc(&app.id, &app.description, cx).into()
        };

        let tile = render_directory_tile(&app.name, 34., app.tile_color, cx);

        let app_id = app.id.clone();
        let click_id = app.id.clone();
        let is_connected = app.connected;
        let on_click: AppClickCallback =
            Rc::new(cx.listener(move |this, _: &ClickEvent, w, cx| {
                if is_connected {
                    this.disconnect_app(&click_id, cx);
                } else {
                    this.open_connect_modal(click_id.clone(), w, cx);
                }
            }));

        let action_btn = AppConnectBtn {
            id: pro_eid(&self.id, format!("conn-btn-{app_id}")),
            app_name: app.name.clone(),
            connected: app.connected,
            on_click,
        }
        .into_any_element();

        let body = div()
            .flex_1()
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .min_w_0()
                            .child(tile)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .min_w_0()
                                            .child(
                                                div()
                                                    .text_size(px(13.))
                                                    .line_height(px(20.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.strong)
                                                    .truncate()
                                                    .child(app.name.clone()),
                                            )
                                            .when(app.verified, |d| {
                                                d.child(pro_svg(
                                                    "0 0 24 24",
                                                    VERIFIED_ROSETTE_SVG,
                                                    13.5,
                                                    p.accent_info,
                                                ))
                                            }),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .line_height(px(16.))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(app.vendor.clone()),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .mt(px(12.))
                            .mb(px(16.))
                            .min_h(px(40.))
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(p.text)
                            .child(desc_str),
                    ),
            )
            .child(action_btn);

        AppCardSurface {
            id: pro_eid(&self.id, format!("card-{}", app.id)),
            content: body.into_any_element(),
        }
        .into_any_element()
    }

    fn render_discover_card(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = ProPalette::get(cx);
        let extra_count = self
            .apps
            .len()
            .saturating_sub(self.category_count("essentials"));
        let stack_apps = ["Gmail", "Google Drive", "Supabase"];

        let mut stack = div().flex().items_center();
        for (idx, name) in stack_apps.iter().enumerate() {
            stack = stack.child(
                div()
                    .when(idx > 0, |d| d.ml(px(-6.)))
                    .rounded(px(7.))
                    .bg(p.card)
                    .p(px(1.5))
                    .child(render_directory_tile(name, 24., None, cx)),
            );
        }

        let caption: SharedString = if UiLanguage::current(cx) == UiLanguage::Chinese {
            format!("探索全部 {} 款集成应用", self.apps.len()).into()
        } else {
            format!("Explore {} more integrations", extra_count.max(1)).into()
        };

        let browse_btn = pro_tactile(
            pro_eid(&self.id, "discover-browse-all"),
            tr(cx, "Browse all apps", "浏览全部应用"),
            6.,
            p.card,
            p.chip_hover,
            p.line_strong,
            p.line_strong,
            false,
            div()
                .h(px(28.))
                .pl(px(10.))
                .pr(px(8.))
                .flex()
                .items_center()
                .gap(px(4.))
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.ink)
                .whitespace_nowrap()
                .child(tr(cx, "Browse all", "查看全部"))
                .child(pro_icon(ProIcon::ChevronRight, 13., p.muted)),
            cx,
            |this, _, cx| {
                this.active_category = "all".into();
                cx.emit(AppDirectoryEvent::CategorySelected("all".into()));
                cx.notify();
            },
        )
        .mt(px(16.))
        .self_start()
        .flex_none();

        div()
            .id(pro_eid(&self.id, "discover-card"))
            .flex_1()
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .shadow(pro_card_shadow(p.dark))
            .p(px(16.))
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
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
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(tr(cx, "More in directory", "更多目录集成")),
                            )
                            .child(stack),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(p.text)
                            .child(caption),
                    ),
            )
            .child(browse_btn)
            .into_any_element()
    }

    fn render_connect_modal(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(modal) = self.connect_modal.clone() else {
            return div().into_any_element();
        };
        let Some(app) = self.apps.iter().find(|a| a.id == modal.app_id).cloned() else {
            return div().into_any_element();
        };
        let p = ProPalette::get(cx);
        let auth_sensor = pro_selection_sensor();
        let thumb_bg: Hsla = if p.dark {
            rgb(0x2c2c2e).into()
        } else {
            rgb(0xffffff).into()
        };

        let mut seg = div()
            .mt(px(14.))
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.chip)
            .relative()
            .flex()
            .items_center()
            .child(pro_pill_indicator(
                pro_eid(&self.id, "modal-auth-pill"),
                modal.auth_mode.key(),
                auth_sensor.clone(),
                6.,
                thumb_bg,
                Some(p.line),
                true,
            ));

        for (mode, lbl) in [
            (AppAuthKind::OAuth, tr(cx, "OAuth 2.0", "OAuth 授权")),
            (AppAuthKind::ApiKey, tr(cx, "API Key", "API 密钥")),
        ] {
            let active = modal.auth_mode == mode;
            seg = seg.child(
                pro_tactile(
                    pro_eid(&self.id, format!("modal-tab-{}", mode.key())),
                    lbl,
                    6.,
                    transparent_black(),
                    if active { transparent_black() } else { p.hover },
                    transparent_black(),
                    transparent_black(),
                    active,
                    div()
                        .h(px(26.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if active { p.ink } else { p.text })
                        .child(lbl),
                    cx,
                    move |this, _, cx| {
                        if let Some(m) = &mut this.connect_modal {
                            m.auth_mode = mode;
                            cx.notify();
                        }
                    },
                )
                .flex_1()
                .relative()
                .when(active, |d| d.child(pro_mark_selected(&auth_sensor))),
            );
        }

        let body = match modal.auth_mode {
            AppAuthKind::OAuth => {
                let mut scopes_list = div().flex().flex_col().gap(px(6.));
                for (s_idx, scope) in app.scopes.iter().enumerate() {
                    scopes_list = scopes_list.child(
                        div()
                            .flex()
                            .items_start()
                            .gap(px(8.))
                            .text_size(px(12.5))
                            .line_height(px(18.))
                            .text_color(p.text)
                            .child(div().mt(px(2.)).child(pro_check_draw(
                                pro_eid(&self.id, format!("scope-chk-{}-{s_idx}", app.id)),
                                true,
                                14.,
                                false,
                                p.green,
                            )))
                            .child(div().flex_1().min_w_0().child(scope.clone())),
                    );
                }
                div()
                    .mt(px(14.))
                    .rounded(px(9.))
                    .bg(p.subtle)
                    .border_1()
                    .border_color(p.line)
                    .p(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Requested permissions", "请求的工作区权限")),
                    )
                    .child(scopes_list)
                    .into_any_element()
            }
            AppAuthKind::ApiKey => {
                let ed_click = self.api_key_editor.clone();
                div()
                    .mt(px(14.))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(tr(cx, "Secret token or API key", "安全密钥或访问令牌")),
                    )
                    .child(
                        div()
                            .h(px(34.))
                            .px(px(10.))
                            .rounded(px(8.))
                            .bg(p.subtle)
                            .border_1()
                            .border_color(p.line_strong)
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .cursor_text()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |_, _, w, cx| {
                                    if let Some(ed) = &ed_click {
                                        ed.read(cx).focus.clone().focus(w, cx);
                                    }
                                }),
                            )
                            .child(pro_icon(ProIcon::KeyLock, 14., p.muted))
                            .when_some(self.api_key_editor.clone(), |d, ed| {
                                d.child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .h(px(22.))
                                        .overflow_hidden()
                                        .child(ed),
                                )
                            }),
                    )
                    .child(div().text_size(px(11.5)).text_color(p.muted).child(tr(
                        cx,
                        "Encrypted at rest with KMS and scoped to this workspace.",
                        "密钥经 KMS 静态加密存储，仅限当前工作区调用。",
                    )))
                    .into_any_element()
            }
        };

        let confirm_label: SharedString = match modal.auth_mode {
            AppAuthKind::OAuth => {
                if UiLanguage::current(cx) == UiLanguage::Chinese {
                    format!("授权连接 {}", app.name).into()
                } else {
                    format!("Authorize {}", app.name).into()
                }
            }
            AppAuthKind::ApiKey => tr(cx, "Save & connect", "保存并连接").into(),
        };

        let dialog = div()
            .id(pro_eid(&self.id, "connect-dialog"))
            .role(Role::Dialog)
            .aria_label(format!("Connect {}", app.name))
            .w(px(400.))
            .max_w_full()
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_xl()
            .p(px(20.))
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .min_w_0()
                            .child(render_directory_tile(&app.name, 38., app.tile_color, cx))
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(15.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .truncate()
                                            .child(
                                                if UiLanguage::current(cx) == UiLanguage::Chinese {
                                                    format!("连接 {}", app.name)
                                                } else {
                                                    format!("Connect {}", app.name)
                                                },
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(app.vendor.clone()),
                                    ),
                            ),
                    )
                    .child(pro_icon_btn(
                        pro_eid(&self.id, "modal-close"),
                        tr(cx, "Close dialog", "关闭弹窗"),
                        ProIcon::Close,
                        28.,
                        false,
                        cx,
                        |this, w, cx| {
                            this.connect_modal = None;
                            this.focus.focus(w, cx);
                            cx.notify();
                        },
                    )),
            )
            .child(seg)
            .child(pro_view_transition(
                pro_eid(&self.id, "modal-auth-vtx"),
                modal.auth_mode.key(),
                body,
            ))
            .child(
                div()
                    .mt(px(18.))
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(8.))
                    .child(pro_btn(
                        pro_eid(&self.id, "modal-cancel"),
                        tr(cx, "Cancel", "取消"),
                        None,
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        |this, w, cx| {
                            this.connect_modal = None;
                            this.focus.focus(w, cx);
                            cx.notify();
                        },
                    ))
                    .child(pro_btn(
                        pro_eid(&self.id, "modal-confirm"),
                        confirm_label,
                        Some(ProIcon::Check),
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        |this, w, cx| {
                            this.confirm_modal_connect(w, cx);
                        },
                    )),
            );

        div()
            .id(pro_eid(&self.id, "modal-backdrop"))
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, w, cx| {
                    this.connect_modal = None;
                    this.focus.focus(w, cx);
                    cx.notify();
                }),
            )
            .child(pro_modal_appear(
                pro_eid(&self.id, "connect-modal-appear"),
                modal.app_id.clone(),
                if p.dark { 0.56 } else { 0.32 },
                dialog,
            ))
            .into_any_element()
    }

    fn render_loading(
        &self,
        sidebar_mode: bool,
        cols: usize,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        if !disabled_motion(cx) {
            w.request_animation_frame();
        }
        let phase = (self.clock.elapsed().as_secs_f32() * 0.75).fract();
        let mut rows = div().flex_1().min_w_0().flex().flex_col().gap(px(12.));
        for r in 0..2usize {
            let mut row = div().flex().gap(px(12.));
            for c in 0..cols {
                let off = (r * cols + c) as f32 * 0.08;
                row = row.child(
                    div()
                        .flex_1()
                        .rounded(px(12.))
                        .bg(p.card)
                        .border_1()
                        .border_color(p.line)
                        .p(px(16.))
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .child(pro_skeleton_bar(32., 32., 8., phase + off, cx))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(5.))
                                        .child(pro_skeleton_bar(88., 13., 4., phase + off, cx))
                                        .child(pro_skeleton_bar(64., 10., 4., phase + off, cx)),
                                ),
                        )
                        .child(pro_skeleton_bar(150., 12., 4., phase + off + 0.1, cx))
                        .child(pro_skeleton_bar(72., 26., 6., phase + off + 0.2, cx)),
                );
            }
            rows = rows.child(row);
        }

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(20.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(pro_skeleton_bar(160., 22., 6., phase, cx))
                    .child(pro_skeleton_bar(80., 14., 4., phase + 0.1, cx)),
            )
            .child(pro_skeleton_bar(320., 32., 8., phase + 0.15, cx))
            .child(
                div()
                    .flex()
                    .when(!sidebar_mode, |d| d.flex_col())
                    .gap(px(24.))
                    .when(sidebar_mode, |d| {
                        let mut side = div().w(px(184.)).flex_none().flex().flex_col().gap(px(6.));
                        for i in 0..7usize {
                            side = side.child(pro_skeleton_bar(
                                156.,
                                24.,
                                6.,
                                phase + i as f32 * 0.05,
                                cx,
                            ));
                        }
                        d.child(side)
                    })
                    .child(rows),
            )
    }
}

impl Render for AppDirectoryBlock {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let measure_cell = self.measured_width.clone();
        let container_w = measure_cell.get().max(320.);
        let sidebar_mode = container_w > 960.;
        let single_col = container_w <= 560.;
        let cols = if sidebar_mode {
            3usize
        } else if single_col {
            1usize
        } else {
            2usize
        };

        let sensor = canvas(
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
        .h(px(0.));

        if self.loading {
            return div()
                .id(self.id.clone())
                .w_full()
                .py(px(8.))
                .relative()
                .child(sensor)
                .child(self.render_loading(sidebar_mode, cols, w, cx))
                .into_any_element();
        }

        if self.empty || self.apps.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .relative()
                .rounded(px(12.))
                .bg(p.panel)
                .border_1()
                .border_color(p.line)
                .child(sensor)
                .child(pro_empty_state(
                    ProIcon::Grid,
                    tr(cx, "No integrations available", "暂无可用集成应用"),
                    tr(
                        cx,
                        "Add apps to the directory so workspace members can connect their tools.",
                        "向应用目录添加集成项后，团队成员即可在此授权连接。",
                    ),
                    Some((
                        pro_eid(&self.id, "restore-apps"),
                        tr(cx, "Load default directory", "载入默认集成目录").into(),
                        Box::new(|this: &mut Self, _, cx| {
                            this.empty = false;
                            if this.apps.is_empty() {
                                this.apps = default_apps();
                                this.custom_apps = false;
                            }
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element();
        }

        let heading_text: SharedString = self
            .title
            .clone()
            .unwrap_or_else(|| tr(cx, "Integrations", "集成目录").into());
        let subtitle_text: SharedString = self.subtitle.clone().unwrap_or_else(|| {
            tr(
                cx,
                "Connect the tools your team already works in.",
                "连接团队日常办公所使用的第三方工具与数据源。",
            )
            .into()
        });

        let connected_n = self.connected_count();
        let connected_suffix = tr(cx, " connected", " 已连接");

        let intro = div()
            .flex()
            .when(single_col, |d| d.flex_col().gap(px(6.)))
            .when(!single_col, |d| {
                d.items_start().justify_between().gap(px(24.))
            })
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(24.))
                            .line_height(px(32.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(heading_text.clone()),
                    )
                    .child(
                        div()
                            .mt(px(4.))
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_x(px(6.))
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(p.text)
                            .child(subtitle_text)
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, "learn-more"))
                                    .role(Role::Link)
                                    .aria_label(tr(
                                        cx,
                                        "Learn more about integrations",
                                        "了解更多集成说明",
                                    ))
                                    .tab_index(0)
                                    .rounded(px(4.))
                                    .flex()
                                    .items_center()
                                    .gap(px(2.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.accent_info)
                                    .cursor_pointer()
                                    .hover(|s| s.opacity(0.82))
                                    .child(tr(cx, "Learn more", "了解更多"))
                                    .child(pro_icon(ProIcon::ExternalArrow, 12., p.accent_info)),
                                cx,
                                |this, _, cx| {
                                    this.toast = Some((
                                        tr(
                                            cx,
                                            "All tokens are encrypted with workspace KMS",
                                            "所有集成凭证均由工作区 KMS 加密托管",
                                        )
                                        .into(),
                                        Instant::now(),
                                    ));
                                    cx.emit(AppDirectoryEvent::LearnMoreClicked);
                                    cx.notify();
                                },
                            )),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .when(!single_col, |d| d.mt(px(6.)))
                    .flex()
                    .items_center()
                    .child(pro_swap_label(
                        pro_eid(&self.id, "connected-roll"),
                        format!("{connected_n}{connected_suffix}"),
                        12.5,
                        FontWeight::NORMAL,
                        p.muted,
                    )),
            );

        let query = self.current_query(cx);
        let has_query = !query.is_empty();
        let ed_click = self.search_editor.clone();
        let ed_clear = self.search_editor.clone();

        let search_bar = div()
            .flex_1()
            .min_w_0()
            .h(px(32.))
            .px(px(10.))
            .rounded(px(8.))
            .bg(p.chip)
            .border_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(8.))
            .cursor_text()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |_, _, w, cx| {
                    if let Some(ed) = &ed_click {
                        ed.read(cx).focus.clone().focus(w, cx);
                    }
                }),
            )
            .child(pro_icon(ProIcon::Search, 14., p.muted))
            .when_some(self.search_editor.clone(), |d, ed| {
                d.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h(px(22.))
                        .overflow_hidden()
                        .child(ed),
                )
            })
            .when(has_query, |d| {
                d.child(pro_icon_btn(
                    pro_eid(&self.id, "search-clear"),
                    tr(cx, "Clear search", "清空搜索"),
                    ProIcon::Close,
                    20.,
                    false,
                    cx,
                    move |this, w, cx| {
                        if let Some(ed) = &ed_clear {
                            ed.update(cx, |e, cx| e.set_value("", w, cx));
                        }
                        this.focus.focus(w, cx);
                        cx.notify();
                    },
                ))
            });

        let filter_active = self.status_filter != AppStatusFilter::All;
        let filter_btn_label = if filter_active {
            self.status_filter.label(cx)
        } else {
            tr(cx, "Filter", "筛选")
        };

        let filter_fg = if filter_active { p.accent_info } else { p.ink };
        let filter_btn = pro_tactile(
            pro_eid(&self.id, "filter-trigger"),
            tr(cx, "Filter apps by status", "按连接状态筛选"),
            8.,
            if filter_active { p.accent_soft } else { p.card },
            if filter_active {
                p.accent_info.opacity(0.20)
            } else {
                p.chip_hover
            },
            if filter_active {
                p.accent_info.opacity(0.28)
            } else {
                p.line_strong
            },
            if filter_active {
                p.accent_info.opacity(0.36)
            } else {
                p.line_strong
            },
            self.filter_menu_open,
            div()
                .h(px(32.))
                .px(px(12.))
                .flex()
                .items_center()
                .gap(px(6.))
                .whitespace_nowrap()
                .child(pro_icon(
                    ProIcon::Filter,
                    13.5,
                    if filter_active {
                        p.accent_info
                    } else {
                        p.muted
                    },
                ))
                .child(pro_swap_label(
                    pro_eid(&self.id, "filter-btn-lbl"),
                    filter_btn_label,
                    13.,
                    FontWeight::MEDIUM,
                    filter_fg,
                )),
            cx,
            |this, _, cx| {
                this.filter_menu_open = !this.filter_menu_open;
                cx.notify();
            },
        )
        .flex_none();

        let cur_status = self.status_filter;
        let menu_items: Vec<(AppStatusFilter, &'static str)> = AppStatusFilter::ALL
            .into_iter()
            .map(|st| (st, st.label(cx)))
            .collect();
        let menu_el = {
            let mut menu_box = div()
                .id(pro_eid(&self.id, "filter-menu"))
                .role(Role::Menu)
                .w(px(188.))
                .p(px(4.))
                .rounded(px(10.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .shadow_lg()
                .flex()
                .flex_col()
                .gap(px(1.));
            for (st, lbl) in menu_items {
                let checked = cur_status == st;
                menu_box = menu_box.child(
                    pro_tactile(
                        pro_eid(&self.id, format!("fmenu-{st:?}")),
                        lbl,
                        6.,
                        transparent_black(),
                        p.hover,
                        transparent_black(),
                        transparent_black(),
                        checked,
                        div()
                            .h(px(28.))
                            .px(px(8.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.))
                            .text_size(px(13.))
                            .font_weight(if checked {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(p.ink)
                            .child(lbl)
                            .child(pro_check_draw(
                                pro_eid(&self.id, format!("fmenu-chk-{st:?}")),
                                checked,
                                14.,
                                false,
                                p.accent_info,
                            )),
                        cx,
                        move |this, _, cx| {
                            this.status_filter = st;
                            this.filter_menu_open = false;
                            cx.emit(AppDirectoryEvent::FilterChanged(st));
                            cx.notify();
                        },
                    )
                    .w_full(),
                );
            }
            menu_box
        };

        let filter_pop = div()
            .absolute()
            .top(px(38.))
            .right_0()
            .child(pro_popover_presence(
                pro_eid(&self.id, "filter-menu-pres"),
                self.filter_menu_open,
                move || menu_el.into_any_element(),
            ));

        let tools_row = div()
            .mt(px(22.))
            .flex()
            .items_center()
            .gap(px(8.))
            .child(search_bar)
            .child(
                div()
                    .relative()
                    .flex_none()
                    .child(filter_btn)
                    .child(filter_pop),
            );

        let visible_indices = self.visible_app_indices(&query, cx);
        let section_heading: SharedString = if !query.trim().is_empty() {
            tr(cx, "Search results", "搜索结果").into()
        } else {
            let fallback = self
                .categories
                .iter()
                .find(|c| c.id == self.active_category)
                .map(|c| c.label.as_ref())
                .unwrap_or(match self.active_category.as_ref() {
                    "essentials" => "Essentials",
                    "all" => "All apps",
                    other => other,
                });
            if self.custom_categories
                && !matches!(self.active_category.as_ref(), "essentials" | "all")
            {
                fallback.to_owned().into()
            } else {
                translate_category_label(&self.active_category, fallback, cx).into()
            }
        };

        let mut cards: Vec<AnyElement> = visible_indices
            .iter()
            .map(|&idx| {
                let app = self.apps[idx].clone();
                self.render_app_card(&app, cx)
            })
            .collect();

        let show_discover = query.trim().is_empty()
            && self.active_category.as_ref() == "essentials"
            && self.status_filter == AppStatusFilter::All
            && !cards.is_empty();
        if show_discover {
            cards.push(self.render_discover_card(cx));
        }

        let grid_body = if cards.is_empty() {
            div()
                .rounded(px(12.))
                .bg(p.subtle)
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::Search,
                    tr(cx, "No matching apps", "没有匹配的集成应用"),
                    tr(
                        cx,
                        "Try clearing the search query or switching to All apps.",
                        "请尝试清空搜索关键词或切换至“全部应用”查看。",
                    ),
                    Some((
                        pro_eid(&self.id, "reset-filters"),
                        tr(cx, "Reset filters", "重置筛选").into(),
                        Box::new(|this: &mut Self, w, cx| {
                            if let Some(ed) = &this.search_editor {
                                ed.update(cx, |e, cx| e.set_value("", w, cx));
                            }
                            this.status_filter = AppStatusFilter::All;
                            this.active_category = "all".into();
                            cx.notify();
                        }),
                    )),
                    cx,
                ))
                .into_any_element()
        } else {
            let mut rows = div().flex().flex_col().gap(px(12.));
            let mut iter = cards.into_iter();
            loop {
                let mut chunk = Vec::with_capacity(cols);
                for _ in 0..cols {
                    if let Some(el) = iter.next() {
                        chunk.push(el);
                    }
                }
                if chunk.is_empty() {
                    break;
                }
                let missing = cols - chunk.len();
                let mut row = div().flex().items_stretch().gap(px(12.));
                for card_el in chunk {
                    row = row.child(div().flex_1().min_w_0().flex().flex_col().child(card_el));
                }
                for _ in 0..missing {
                    row = row.child(div().flex_1().min_w_0());
                }
                rows = rows.child(row);
            }
            rows.into_any_element()
        };

        let result_count = visible_indices.len();
        let main_col = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .id(pro_eid(&self.id, "results-heading"))
                    .role(Role::Status)
                    .mb(px(12.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_swap_label(
                        pro_eid(&self.id, "sec-head-lbl"),
                        section_heading.clone(),
                        13.,
                        FontWeight::MEDIUM,
                        p.strong,
                    ))
                    .child(pro_swap_label(
                        pro_eid(&self.id, "sec-cnt-lbl"),
                        format!("{result_count}"),
                        12.,
                        FontWeight::NORMAL,
                        p.muted,
                    )),
            )
            .child(pro_view_transition(
                pro_eid(&self.id, "grid-vtx"),
                format!("{}-{:?}", self.active_category, self.status_filter),
                grid_body,
            ));

        let nav_el = self.render_category_nav(sidebar_mode, cx);
        let body_layout = if sidebar_mode {
            div()
                .mt(px(24.))
                .flex()
                .items_start()
                .gap(px(32.))
                .child(nav_el)
                .child(main_col)
        } else {
            div()
                .mt(px(18.))
                .flex()
                .flex_col()
                .gap(px(16.))
                .child(nav_el)
                .child(main_col)
        };

        let modal_open = self.connect_modal.is_some();
        let modal_rendered = if modal_open {
            Some(self.render_connect_modal(cx))
        } else {
            None
        };

        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .role(Role::Region)
            .aria_label(heading_text)
            .w_full()
            .px(px(if single_col { 16. } else { 36. }))
            .py(px(if single_col { 20. } else { 32. }))
            .relative()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                this.handle_key_down(ev, w, cx);
            }))
            .child(sensor)
            .child(intro)
            .child(tools_row)
            .child(body_layout)
            .when_some(modal_rendered, |d, m| d.child(m))
            .child(pro_toast_banner(self.toast.clone()))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{AppDirectoryBlock, AppStatusFilter, DirectoryApp, default_apps};
    use gpui::TestAppContext;

    #[test]
    fn default_catalog_has_eight_essentials_and_one_initial_connection() {
        let apps = default_apps();
        assert_eq!(apps.len(), 18);
        let essentials = apps.iter().filter(|a| a.essential).count();
        let connected = apps.iter().filter(|a| a.connected).count();
        assert_eq!(essentials, 8);
        assert_eq!(connected, 1);
        assert_eq!(apps[0].id.as_ref(), "slack");
        assert!(apps[0].connected);
    }

    #[test]
    fn toggling_apps_and_filtering_updates_counts_and_visibility() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (block, w) = app.add_window_view(|_, cx| {
            AppDirectoryBlock::new("test-app-dir", cx)
                .with_category("development")
                .with_status_filter(AppStatusFilter::NotConnected)
        });

        w.update(|_, cx| {
            block.update(cx, |dir, cx| {
                assert_eq!(dir.connected_count(), 1);
                let dev_unconnected = dir.visible_app_indices("", cx).len();
                assert!(dev_unconnected >= 4);

                let new_state = dir.toggle_app("github", cx);
                assert_eq!(new_state, Some(true));
                assert_eq!(dir.connected_count(), 2);
                assert_eq!(dir.visible_app_indices("", cx).len(), dev_unconnected - 1);

                let custom = DirectoryApp::new("custom", "Custom", "Acme", "Custom webhook bridge")
                    .with_essential(true)
                    .with_connected(true);
                assert!(custom.matches_status(AppStatusFilter::Connected));
                assert!(!custom.matches_status(AppStatusFilter::NotConnected));
            });
        });
    }
}
