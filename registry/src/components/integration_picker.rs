//! Two-pane command-menu integrations picker with Workspace and Personal OAuth tiles.
//!
//! Renders a searchable integrations sub-page with a grouped left service list
//! (`Connected` and `Available`), gliding row highlight, and a right detail pane
//! pairing a live Kense + Service connection header with `Workspace` and `Personal`
//! OAuth scope cards.

use crate::{
    components::pro_chrome::{
        ProPalette, pro_app_tile, pro_check_draw, pro_eid, pro_empty_state, pro_mark_selected,
        pro_pill_indicator, pro_selection_sensor, pro_skeleton_bar, pro_spring_f32, pro_swap_label,
        pro_switch, pro_tactile, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        editor::{EditEvent, Editor, InputMode},
        family::disabled_motion,
        pro_icons::{ProIcon, pro_app_artwork, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const ICON_BACK: &str = r#"<path d="M10 3.5 5.5 8l4.5 4.5" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/>"#;
const ICON_SYNC: &str = r#"<path d="M2.5 6h10.5M10.5 3.5 13 6l-2.5 2.5M13.5 10H3M5.5 7.5 3 10l2.5 2.5" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/>"#;
const ICON_CHEVRON_R: &str = r#"<path d="M6.5 4 10.5 8l-4 4" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"/>"#;

/// OAuth connection target scope on [`IntegrationPicker`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConnectionTarget {
    /// Shared across all members of the Kense workspace.
    Workspace,
    /// Scoped privately to the signed-in user's personal account.
    Personal,
}

impl ConnectionTarget {
    /// Localized tile heading.
    pub fn title(self, cx: &App) -> &'static str {
        match self {
            Self::Workspace => tr(cx, "Workspace", "团队工作区"),
            Self::Personal => tr(cx, "Personal", "个人账号"),
        }
    }

    /// Localized visibility caption.
    pub fn subtitle(self, cx: &App) -> &'static str {
        match self {
            Self::Workspace => tr(cx, "Shared with workspace", "全员共享授权"),
            Self::Personal => tr(cx, "Only visible to you", "仅对你本人可见"),
        }
    }
}

/// Service entry displayed inside [`IntegrationPicker`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickerService {
    pub id: SharedString,
    pub name: SharedString,
    pub app_mark: SharedString,
    pub domain: SharedString,
    pub category: SharedString,
    pub description: SharedString,
    pub workspace_scopes: SharedString,
    pub personal_scopes: SharedString,
    pub capabilities: Vec<SharedString>,
    pub workspace_connected: bool,
    pub personal_connected: bool,
}

impl PickerService {
    /// Construct a [`PickerService`] entry (disconnected by default).
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        domain: impl Into<SharedString>,
        category: impl Into<SharedString>,
        description: impl Into<SharedString>,
    ) -> Self {
        let id = id.into();
        Self {
            app_mark: id.clone(),
            id,
            name: name.into(),
            domain: domain.into(),
            category: category.into(),
            description: description.into(),
            workspace_scopes: "read · write · webhooks".into(),
            personal_scopes: "profile:read · notifications".into(),
            capabilities: Vec::new(),
            workspace_connected: false,
            personal_connected: false,
        }
    }

    /// Override the brand key passed to [`pro_app_tile`].
    pub fn with_app_mark(mut self, mark: impl Into<SharedString>) -> Self {
        self.app_mark = mark.into();
        self
    }

    /// Set the OAuth permission summaries for Workspace and Personal tiles.
    pub fn with_scopes(
        mut self,
        workspace_scopes: impl Into<SharedString>,
        personal_scopes: impl Into<SharedString>,
    ) -> Self {
        self.workspace_scopes = workspace_scopes.into();
        self.personal_scopes = personal_scopes.into();
        self
    }

    /// Attach bulleted capabilities listed beneath the two connection tiles.
    pub fn with_capabilities<I, S>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.capabilities = items.into_iter().map(Into::into).collect();
        self
    }

    /// Configure initial Workspace and Personal connection states.
    pub fn with_connections(mut self, workspace: bool, personal: bool) -> Self {
        self.workspace_connected = workspace;
        self.personal_connected = personal;
        self
    }

    /// Return `true` when either Workspace or Personal OAuth is active.
    pub fn is_connected(&self) -> bool {
        self.workspace_connected || self.personal_connected
    }
}

/// Typed events emitted by [`IntegrationPicker`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntegrationPickerEvent {
    /// Service row selected in the left list.
    Selected(SharedString),
    /// Workspace or Personal connection toggled on or off.
    ConnectionToggled {
        id: SharedString,
        target: ConnectionTarget,
        connected: bool,
    },
    /// User clicked `"Explore integration"` on the active service.
    Explore(SharedString),
    /// User clicked the Back button or pressed `←` in an empty search box.
    Back,
    /// User pressed `Esc` in an empty search box.
    Closed,
}

/// Default 10 SaaS services with real brand artwork and Workspace/Personal OAuth scopes.
pub fn default_picker_services(zh: bool) -> Vec<PickerService> {
    if zh {
        vec![
            PickerService::new(
                "linear",
                "Linear",
                "linear.app",
                "项目管理",
                "将对话中的需求直接转为工作项，并双向实时同步状态流转。",
            )
            .with_scopes(
                "issues:write · webhooks:read",
                "assigned:read · comments:write",
            )
            .with_capabilities([
                "从消息一键创建带优先级的 Issue",
                "PR 合并后自动推进状态并回写线程",
            ])
            .with_connections(true, true),
            PickerService::new(
                "slack",
                "Slack",
                "slack.com",
                "团队沟通",
                "在项目频道接收关键事件推送，并直接通过快捷按钮完成审批。",
            )
            .with_scopes(
                "channels:history · chat:write",
                "dm:write · reminders:write",
            )
            .with_capabilities([
                "订阅指定频道并自动提炼每日摘要",
                "个人提及与待办实时私信提醒",
            ])
            .with_connections(true, false),
            PickerService::new(
                "github",
                "GitHub",
                "github.com",
                "代码托管",
                "关联仓库提交、Pull Request 检查结果与部署流水线状态。",
            )
            .with_scopes(
                "repo:status · pull_requests:read",
                "notifications · gist:write",
            )
            .with_capabilities([
                "自动解析 PR 变更影响面与构建日志",
                "行内引用提交记录与分支对比",
            ])
            .with_connections(true, false),
            PickerService::new(
                "notion",
                "Notion",
                "notion.so",
                "知识库与文档",
                "检索团队 Wiki 页面与数据库条目，保持技术方案上下文同步。",
            )
            .with_scopes("pages:read · databases:query", "drafts:write · search")
            .with_capabilities(["实时索引规格说明与会议纪要", "将整理后的结论导出为新页面"]),
            PickerService::new(
                "stripe",
                "Stripe",
                "stripe.com",
                "支付与订阅",
                "在客户上下文旁查看订阅层级、账单状态与最近退款记录。",
            )
            .with_scopes("customers:read · invoices:read", "alerts:read")
            .with_capabilities([
                "自动匹配客户邮箱展示 MRR 与续费日期",
                "付款失败与争议工单即时预警",
            ]),
            PickerService::new(
                "figma",
                "Figma",
                "figma.com",
                "界面设计",
                "展开画板实时缩略图，并在任务旁同步设计评论变更。",
            )
            .with_scopes("files:read · webhooks:manage", "comments:write")
            .with_capabilities([
                "粘贴链接自动生成高清画板预览",
                "设计稿版本更新时通知负责研发",
            ]),
            PickerService::new(
                "hubspot",
                "HubSpot",
                "hubspot.com",
                "客户关系管理",
                "同步商机阶段、联系人画像与最近跟进记录。",
            )
            .with_scopes("crm.deals:read · contacts:write", "tasks:write")
            .with_capabilities(["通话结束后自动补全跟进纪要", "按商机金额筛选高优客户请求"]),
            PickerService::new(
                "google-calendar",
                "Google Calendar",
                "calendar.google.com",
                "日程协作",
                "读取参会人空闲时段并自动附带议程文档链接。",
            )
            .with_app_mark("google calendar")
            .with_scopes(
                "calendars.freebusy · rooms:read",
                "events:write · availability",
            )
            .with_capabilities(["跨时区推荐最佳评审会议窗口", "会前十分钟推送背景资料卡片"]),
            PickerService::new(
                "vercel",
                "Vercel",
                "vercel.com",
                "前端部署",
                "追踪预览环境构建进度、边缘函数日志与线上回滚。",
            )
            .with_scopes("deployments:read · logs:read", "previews:comment")
            .with_capabilities([
                "每个分支自动挂载可点击预览地址",
                "构建报错时直接定位首条异常堆栈",
            ]),
            PickerService::new(
                "supabase",
                "Supabase",
                "supabase.com",
                "数据库与鉴权",
                "安全只读检索表结构、RLS 策略与慢查询指标。",
            )
            .with_scopes("schema:read · metrics:read", "sql_snippets:read")
            .with_capabilities([
                "根据真实表结构生成类型安全查询",
                "监控连接池水位与迁移执行状态",
            ]),
        ]
    } else {
        vec![
            PickerService::new(
                "linear",
                "Linear",
                "linear.app",
                "Project management",
                "Turn requests from chat into issues and keep their status in sync both ways.",
            )
            .with_scopes(
                "issues:write · webhooks:read",
                "assigned:read · comments:write",
            )
            .with_capabilities([
                "Create prioritized issues directly from threads",
                "Sync status transitions and assignees two-way",
            ])
            .with_connections(true, true),
            PickerService::new(
                "slack",
                "Slack",
                "slack.com",
                "Team messaging",
                "Route alerts into project channels and approve agent actions inline.",
            )
            .with_scopes(
                "channels:history · chat:write",
                "dm:write · reminders:write",
            )
            .with_capabilities([
                "Post structured release and incident digests",
                "Deliver personal @mention nudges via direct message",
            ])
            .with_connections(true, false),
            PickerService::new(
                "github",
                "GitHub",
                "github.com",
                "Source control",
                "Link pull requests, CI check runs, and commit diffs next to every task.",
            )
            .with_scopes(
                "repo:status · pull_requests:read",
                "notifications · gist:write",
            )
            .with_capabilities([
                "Unfurl PR checks, reviewers, and branch status",
                "Auto-close linked tasks when pull requests merge",
            ])
            .with_connections(true, false),
            PickerService::new(
                "notion",
                "Notion",
                "notion.so",
                "Docs & wiki",
                "Search workspace specs and databases so answers cite current docs.",
            )
            .with_scopes("pages:read · databases:query", "drafts:write · search")
            .with_capabilities([
                "Index product specs and meeting notes live",
                "Export thread summaries into clean Notion pages",
            ]),
            PickerService::new(
                "stripe",
                "Stripe",
                "stripe.com",
                "Payments & billing",
                "Surface subscription tier, invoice health, and MRR beside customer threads.",
            )
            .with_scopes("customers:read · invoices:read", "alerts:read")
            .with_capabilities([
                "Match accounts by domain for live plan context",
                "Flag failed renewals and disputed charges early",
            ]),
            PickerService::new(
                "figma",
                "Figma",
                "figma.com",
                "Design",
                "Embed live frame previews and keep design review comments synchronized.",
            )
            .with_scopes("files:read · webhooks:manage", "comments:write")
            .with_capabilities([
                "Render crisp frame thumbnails on link paste",
                "Notify engineers when inspected components change",
            ]),
            PickerService::new(
                "hubspot",
                "HubSpot",
                "hubspot.com",
                "CRM",
                "Keep deal stages, account owners, and call notes aligned without tab-switching.",
            )
            .with_scopes("crm.deals:read · contacts:write", "tasks:write")
            .with_capabilities([
                "Log call recaps onto the right deal timeline",
                "Prioritize inbound threads by ARR and stage",
            ]),
            PickerService::new(
                "google-calendar",
                "Google Calendar",
                "calendar.google.com",
                "Scheduling",
                "Check team availability across time zones and attach prep briefs to invites.",
            )
            .with_app_mark("google calendar")
            .with_scopes(
                "calendars.freebusy · rooms:read",
                "events:write · availability",
            )
            .with_capabilities([
                "Propose conflict-free slots in one click",
                "Attach context docs ten minutes before kickoff",
            ]),
            PickerService::new(
                "vercel",
                "Vercel",
                "vercel.com",
                "Deployments",
                "Inspect preview URLs, build durations, and edge logs from the command bar.",
            )
            .with_scopes("deployments:read · logs:read", "previews:comment")
            .with_capabilities([
                "Open branch preview deployments immediately",
                "Surface failing build traces inline",
            ]),
            PickerService::new(
                "supabase",
                "Supabase",
                "supabase.com",
                "Database & auth",
                "Inspect table schemas, row-level security policies, and query health safely.",
            )
            .with_scopes("schema:read · metrics:read", "sql_snippets:read")
            .with_capabilities([
                "Ground SQL helpers in live table definitions",
                "Track migration history and pool saturation",
            ]),
        ]
    }
}

/// Ordered indices split into `(connected_indices, available_indices)` matching `query`.
pub(crate) fn partition_services(
    services: &[PickerService],
    query: &str,
) -> (Vec<usize>, Vec<usize>) {
    let q = query.trim().to_lowercase();
    let mut connected = Vec::new();
    let mut available = Vec::new();
    for (idx, svc) in services.iter().enumerate() {
        let matched = q.is_empty()
            || svc.name.to_lowercase().contains(&q)
            || svc.category.to_lowercase().contains(&q)
            || svc.domain.to_lowercase().contains(&q)
            || svc.description.to_lowercase().contains(&q);
        if matched {
            if svc.is_connected() {
                connected.push(idx);
            } else {
                available.push(idx);
            }
        }
    }
    (connected, available)
}

fn render_kbd(label: &str, p: &ProPalette) -> Div {
    div()
        .min_w(px(20.))
        .h(px(20.))
        .px(px(5.))
        .rounded(px(5.))
        .border_1()
        .border_color(p.line_strong)
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(11.))
        .line_height(px(16.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(p.muted)
        .child(label.to_string())
}

/// Two-pane integrations command-menu page with Workspace and Personal OAuth scope cards.
///
/// # Examples
///
/// ```no_run
/// use gpui::prelude::*;
/// use kui::components::integration_picker::{IntegrationPicker, IntegrationPickerEvent};
///
/// # fn doc(cx: &mut gpui::App) {
/// let picker = cx.new(|cx| {
///     IntegrationPicker::new("cmd-integrations", cx)
///         .with_app_name("Kense")
///         .with_selected("linear")
/// });
/// cx.subscribe(&picker, |_, ev: &IntegrationPickerEvent, _| {
///     let _ = ev;
/// })
/// .detach();
/// # }
/// ```
pub struct IntegrationPicker {
    id: ElementId,
    app_name: SharedString,
    services: Vec<PickerService>,
    custom_services: bool,
    services_lang_zh: bool,
    selected_id: SharedString,
    show_back: bool,
    search_editor: Option<Entity<Editor>>,
    editor_lang_zh: Option<bool>,
    measured_width: Rc<Cell<f32>>,
    started_at: Instant,
    loading: bool,
    empty: bool,
}

impl EventEmitter<IntegrationPickerEvent> for IntegrationPicker {}

impl IntegrationPicker {
    /// Construct an [`IntegrationPicker`] with 10 SaaS connectors and `"Kense"` product branding.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);
        let services = default_picker_services(zh);
        let selected_id = services
            .first()
            .map(|s| s.id.clone())
            .unwrap_or_else(|| "linear".into());
        Self {
            id: id.into(),
            app_name: "Kense".into(),
            services,
            custom_services: false,
            services_lang_zh: zh,
            selected_id,
            show_back: true,
            search_editor: None,
            editor_lang_zh: None,
            measured_width: Rc::new(Cell::new(660.)),
            started_at: Instant::now(),
            loading: false,
            empty: false,
        }
    }

    /// Override the host product name shown in the detail title (e.g. `"Kense + Linear"`).
    pub fn with_app_name(mut self, app_name: impl Into<SharedString>) -> Self {
        self.app_name = app_name.into();
        self
    }

    /// Replace the list of selectable services.
    pub fn with_services(mut self, services: Vec<PickerService>) -> Self {
        if let Some(first) = services.first() {
            self.selected_id = first.id.clone();
        }
        self.services = services;
        self.custom_services = true;
        self
    }

    /// Select the initially highlighted service by `id`.
    pub fn with_selected(mut self, id: impl Into<SharedString>) -> Self {
        self.selected_id = id.into();
        self
    }

    /// Show or hide the leading Back button in the search bar.
    pub fn with_back_button(mut self, show: bool) -> Self {
        self.show_back = show;
        self
    }

    /// Enable or disable the loading skeleton state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Enable or disable the empty state override.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return all configured services and their live connection states.
    pub fn services(&self) -> &[PickerService] {
        &self.services
    }

    /// Return the currently selected service `id`.
    pub fn selected_id(&self) -> &SharedString {
        &self.selected_id
    }

    /// Return the count of services with at least one active connection.
    pub fn connected_count(&self) -> usize {
        self.services.iter().filter(|s| s.is_connected()).count()
    }

    fn query(&self, cx: &App) -> String {
        self.search_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default()
    }

    fn sync_language_defaults(&mut self, cx: &App) {
        if self.custom_services {
            return;
        }
        let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);
        if self.services_lang_zh != zh {
            let states: Vec<(SharedString, bool, bool)> = self
                .services
                .iter()
                .map(|s| (s.id.clone(), s.workspace_connected, s.personal_connected))
                .collect();
            let mut next = default_picker_services(zh);
            for svc in &mut next {
                if let Some((_, ws, ps)) = states.iter().find(|(id, _, _)| id == &svc.id) {
                    svc.workspace_connected = *ws;
                    svc.personal_connected = *ps;
                }
            }
            self.services = next;
            self.services_lang_zh = zh;
        }
    }

    fn visible_order(&self, cx: &App) -> Vec<usize> {
        let (mut conn, avail) = partition_services(&self.services, &self.query(cx));
        conn.extend(avail);
        conn
    }

    fn move_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let order = self.visible_order(cx);
        if order.is_empty() {
            return;
        }
        let cur_pos = order
            .iter()
            .position(|&idx| self.services[idx].id == self.selected_id)
            .unwrap_or(0) as isize;
        let next_pos = (cur_pos + delta).rem_euclid(order.len() as isize) as usize;
        let next_id = self.services[order[next_pos]].id.clone();
        self.selected_id = next_id.clone();
        cx.emit(IntegrationPickerEvent::Selected(next_id));
        cx.notify();
    }

    /// Toggle `Workspace` or `Personal` connection state on service `id`.
    pub fn toggle_target(
        &mut self,
        id: &SharedString,
        target: ConnectionTarget,
        cx: &mut Context<Self>,
    ) {
        if let Some(svc) = self.services.iter_mut().find(|s| &s.id == id) {
            let next = match target {
                ConnectionTarget::Workspace => {
                    svc.workspace_connected = !svc.workspace_connected;
                    svc.workspace_connected
                }
                ConnectionTarget::Personal => {
                    svc.personal_connected = !svc.personal_connected;
                    svc.personal_connected
                }
            };
            let svc_id = svc.id.clone();
            cx.emit(IntegrationPickerEvent::ConnectionToggled {
                id: svc_id,
                target,
                connected: next,
            });
            cx.notify();
        }
    }

    /// Toggle the primary (`Workspace`) connection of the currently selected service.
    pub fn toggle_selected_primary(&mut self, cx: &mut Context<Self>) {
        let id = self.selected_id.clone();
        if let Some(svc) = self.services.iter_mut().find(|s| s.id == id) {
            if svc.is_connected() {
                svc.workspace_connected = false;
                svc.personal_connected = false;
                cx.emit(IntegrationPickerEvent::ConnectionToggled {
                    id,
                    target: ConnectionTarget::Workspace,
                    connected: false,
                });
            } else {
                svc.workspace_connected = true;
                cx.emit(IntegrationPickerEvent::ConnectionToggled {
                    id,
                    target: ConnectionTarget::Workspace,
                    connected: true,
                });
            }
            cx.notify();
        }
    }

    fn ensure_editor(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let zh = matches!(UiLanguage::current(cx), UiLanguage::Chinese);
        let ph: SharedString = tr(cx, "Search integrations…", "搜索集成服务…").into();
        if let Some(ed) = &self.search_editor {
            if self.editor_lang_zh != Some(zh) {
                self.editor_lang_zh = Some(zh);
                ed.update(cx, |e, cx| {
                    e.set_placeholder(ph);
                    cx.notify();
                });
            }
            return;
        }
        self.editor_lang_zh = Some(zh);
        let ed = cx.new(|cx| {
            let mut e = Editor::new(
                tr(cx, "Search integrations", "搜索集成服务"),
                InputMode::Combobox,
                w,
                cx,
            );
            e.set_placeholder(ph);
            e.font_size = 13.;
            e
        });
        cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
            EditEvent::Changed => {
                let order = this.visible_order(cx);
                if !order.is_empty()
                    && !order
                        .iter()
                        .any(|&idx| this.services[idx].id == this.selected_id)
                {
                    this.selected_id = this.services[order[0]].id.clone();
                }
                cx.notify();
            }
            EditEvent::Navigate(down) => {
                this.move_selection(if *down { 1 } else { -1 }, cx);
            }
            EditEvent::Submitted => {
                this.toggle_selected_primary(cx);
            }
            _ => {}
        })
        .detach();
        self.search_editor = Some(ed);
    }
}

impl Render for IntegrationPicker {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_language_defaults(cx);
        self.ensure_editor(w, cx);

        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let width = self.measured_width.get();
        let stacked = width < 560.;

        if self.loading {
            if !reduced {
                w.request_animation_frame();
            }
            let phase = (self.started_at.elapsed().as_secs_f32() / 1.4).fract();
            let card_bg: Hsla = if p.dark {
                rgb(0x1c1c1e).into()
            } else {
                rgb(0xffffff).into()
            };
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(660.))
                        .rounded(px(10.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(p.line)
                        .shadow_lg()
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .h(px(48.))
                                .px(px(12.))
                                .border_b_1()
                                .border_color(p.line)
                                .flex()
                                .items_center()
                                .gap(px(8.))
                                .child(pro_skeleton_bar(78., 20., 5., phase, cx))
                                .child(pro_skeleton_bar(200., 14., 4., phase + 0.1, cx)),
                        )
                        .child(
                            div()
                                .h(px(368.))
                                .p(px(12.))
                                .flex()
                                .gap(px(12.))
                                .child(pro_skeleton_bar(240., 340., 8., phase + 0.18, cx))
                                .child(pro_skeleton_bar(380., 340., 8., phase + 0.28, cx)),
                        ),
                )
                .into_any_element();
        }

        if self.empty {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(660.))
                .rounded(px(10.))
                .bg(if p.dark { rgb(0x1c1c1e) } else { rgb(0xffffff) })
                .border_1()
                .border_color(p.line)
                .child(pro_empty_state(
                    ProIcon::Link,
                    tr(cx, "No integrations available", "暂无可用集成"),
                    tr(
                        cx,
                        "Enable workspace connectors to link SaaS tools from the command menu.",
                        "启用工作区连接器后即可在命令面板中绑定第三方服务。",
                    ),
                    None,
                    cx,
                ))
                .into_any_element();
        }

        let card_bg: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let card_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.07)
        };
        let pill_fill: Hsla = if p.dark {
            hsla(0., 0., 1., 0.08)
        } else {
            hsla(0., 0., 0., 0.06)
        };
        let soft_surface: Hsla = if p.dark {
            hsla(0., 0., 1., 0.025)
        } else {
            hsla(0., 0., 0., 0.018)
        };

        let query_str = self.query(cx);
        let (connected_idxs, available_idxs) = partition_services(&self.services, &query_str);
        let total_connected = self.connected_count();
        let total_services = self.services.len();

        // Top search bar
        let editor_el = self
            .search_editor
            .as_ref()
            .map(|ed| ed.clone().into_any_element())
            .unwrap_or_else(|| div().into_any_element());

        let back_aria: SharedString = tr(cx, "Back", "返回上级").into();
        let top_bar = div()
            .h(px(48.))
            .pl(px(10.))
            .pr(px(12.))
            .border_b_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .gap(px(6.))
            .when(self.show_back, |d| {
                d.child(
                    pro_tactile(
                        pro_eid(&self.id, "back-btn"),
                        back_aria,
                        6.,
                        transparent_black(),
                        p.hover,
                        transparent_black(),
                        transparent_black(),
                        false,
                        div()
                            .size(px(28.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(pro_svg("0 0 16 16", ICON_BACK, 16., p.muted)),
                        cx,
                        |_, _, cx| {
                            cx.emit(IntegrationPickerEvent::Back);
                        },
                    )
                    .flex_none(),
                )
            })
            .child(
                div()
                    .id(pro_eid(&self.id, "search-hit"))
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .on_click(cx.listener(|v, _, w, cx| {
                        if let Some(ed) = &v.search_editor {
                            ed.clone().update(cx, |e, cx| e.focus(w, cx));
                        }
                    }))
                    .child(
                        div()
                            .px(px(6.))
                            .py(px(2.))
                            .flex_none()
                            .rounded(px(5.))
                            .bg(pill_fill)
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.text)
                            .child(tr(cx, "Integrations", "集成")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h(px(22.))
                            .flex()
                            .items_center()
                            .child(editor_el),
                    )
                    .when(!stacked, |d| d.child(render_kbd("⌘/", &p))),
            );

        // Left scrollable list with Connected & Available groups + gliding pill
        let sensor = pro_selection_sensor();
        let selected_key = self.selected_id.clone();

        let mut list_inner = div()
            .relative()
            .w_full()
            .flex()
            .flex_col()
            .child(pro_pill_indicator(
                pro_eid(&self.id, "svc-pill"),
                selected_key.clone(),
                sensor.clone(),
                6.,
                pill_fill,
                None,
                false,
            ));

        let groups_to_render = [
            (tr(cx, "Connected", "已连接"), &connected_idxs),
            (tr(cx, "Available", "可连接"), &available_idxs),
        ];

        let has_any_visible = !connected_idxs.is_empty() || !available_idxs.is_empty();
        if !has_any_visible {
            list_inner = list_inner.child(
                div()
                    .px(px(10.))
                    .py(px(18.))
                    .text_size(px(13.))
                    .text_color(p.muted)
                    .child(format!(
                        "{} “{query_str}”",
                        tr(cx, "No services matching", "未找到匹配服务")
                    )),
            );
        } else {
            for (heading, indices) in groups_to_render {
                if indices.is_empty() {
                    continue;
                }
                list_inner = list_inner.child(
                    div()
                        .pt(px(10.))
                        .pb(px(4.))
                        .px(px(8.))
                        .text_size(px(10.))
                        .line_height(px(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.muted)
                        .child(heading.to_uppercase()),
                );
                for &idx in indices {
                    let svc = &self.services[idx];
                    let svc_id = svc.id.clone();
                    let is_sel = svc_id == selected_key;
                    let is_conn = svc.is_connected();
                    let both_conn = svc.workspace_connected && svc.personal_connected;
                    let row_id = pro_eid(&self.id, format!("svc-row-{svc_id}"));

                    list_inner = list_inner.child(
                        div()
                            .id(row_id)
                            .role(Role::ListItem)
                            .aria_label(svc.name.clone())
                            .relative()
                            .h(px(44.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .cursor_pointer()
                            .when(is_sel, |d| d.child(pro_mark_selected(&sensor)))
                            .when(!is_sel, move |d| {
                                d.hover(move |s| s.bg(p.hover.opacity(0.45)))
                            })
                            .active(move |s| s.top(px(1.)))
                            .on_click(cx.listener(move |v, _, _, cx| {
                                v.selected_id = svc_id.clone();
                                cx.emit(IntegrationPickerEvent::Selected(svc_id.clone()));
                                cx.notify();
                            }))
                            .child(pro_app_tile(svc.app_mark.as_ref(), 24., cx))
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
                                            .text_color(p.ink)
                                            .truncate()
                                            .child(svc.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.5))
                                            .line_height(px(15.))
                                            .text_color(p.muted)
                                            .truncate()
                                            .child(svc.category.clone()),
                                    ),
                            )
                            .when(is_conn, |d| {
                                d.child(
                                    div()
                                        .flex_none()
                                        .flex()
                                        .items_center()
                                        .gap(px(3.))
                                        .child(div().size(px(6.)).rounded_full().bg(p.green))
                                        .when(both_conn, |r| {
                                            r.child(div().size(px(6.)).rounded_full().bg(p.blue))
                                        }),
                                )
                            }),
                    );
                }
            }
        }

        let left_pane = div()
            .id(pro_eid(&self.id, "svc-list-scroll"))
            .when_else(
                stacked,
                |d| d.w_full().max_h(px(180.)).border_b_1().border_color(p.line),
                |d| {
                    d.w(px(262.))
                        .flex_none()
                        .h_full()
                        .border_r_1()
                        .border_color(p.line)
                },
            )
            .px(px(4.))
            .pb(px(4.))
            .overflow_y_scroll()
            .child(list_inner);

        // Right detail pane for selected service
        let active_svc = self
            .services
            .iter()
            .find(|s| s.id == self.selected_id)
            .or_else(|| self.services.first())
            .cloned();

        let right_pane = if let Some(svc) = active_svc {
            let is_conn = svc.is_connected();
            let svc_id = svc.id.clone();
            let started = self.started_at;
            let ring_stroke: Hsla = if p.dark {
                hsla(0., 0., 1., 0.06)
            } else {
                hsla(0., 0., 0., 0.06)
            };
            let accent_col: Hsla = if p.dark {
                rgb(0x4fb0fe).into()
            } else {
                rgba(0x0060e2c4).into()
            };

            if is_conn && !reduced {
                w.request_animation_frame();
            }

            let rings_canvas = canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    let center = bounds.center();
                    for &diam in &[90.0f32, 146.0, 202.0] {
                        let r = diam * 0.5;
                        let mut pb = PathBuilder::stroke(px(1.));
                        let steps = 40usize;
                        for i in 0..=steps {
                            let a = (i as f32 / steps as f32) * std::f32::consts::TAU;
                            let pt = center + point(px(r * a.cos()), px(r * a.sin()));
                            if i == 0 {
                                pb.move_to(pt);
                            } else {
                                pb.line_to(pt);
                            }
                        }
                        if let Ok(path) = pb.build() {
                            window.paint_path(path, ring_stroke);
                        }
                    }
                    if is_conn && !reduced {
                        let u = (started.elapsed().as_secs_f32() / 2.8).fract();
                        let scale = 0.9 + 1.7 * (1. - (1. - u) * (1. - u));
                        let alpha = 0.48 * (1. - u);
                        let r = 45.0 * scale;
                        let mut pb = PathBuilder::stroke(px(1.2));
                        let steps = 44usize;
                        for i in 0..=steps {
                            let a = (i as f32 / steps as f32) * std::f32::consts::TAU;
                            let pt = center + point(px(r * a.cos()), px(r * a.sin()));
                            if i == 0 {
                                pb.move_to(pt);
                            } else {
                                pb.line_to(pt);
                            }
                        }
                        if let Ok(path) = pb.build() {
                            window.paint_path(path, accent_col.opacity(alpha));
                        }
                    }
                },
            )
            .absolute()
            .inset_0();

            let kense_glyph = pro_app_artwork("kense", 20., cx)
                .unwrap_or_else(|| pro_icon(ProIcon::Sparkles, 18., rgb(0xffffff).into()));

            let art_banner = div()
                .relative()
                .h(px(88.))
                .mx(px(8.))
                .mt(px(8.))
                .flex_none()
                .rounded(px(8.))
                .bg(soft_surface)
                .border_1()
                .border_color(p.line)
                .overflow_hidden()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(16.))
                .child(rings_canvas)
                .child(
                    div()
                        .relative()
                        .size(px(42.))
                        .rounded(px(10.))
                        .bg(rgb(0x171717))
                        .border_1()
                        .border_color(hsla(0., 0., 1., 0.12))
                        .shadow_sm()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(kense_glyph),
                )
                .child(
                    div()
                        .relative()
                        .size(px(28.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(pro_svg(
                            "0 0 16 16",
                            ICON_SYNC,
                            16.,
                            if is_conn { accent_col } else { p.faint },
                        )),
                )
                .child(div().relative().shadow_sm().child(pro_app_tile(
                    svc.app_mark.as_ref(),
                    42.,
                    cx,
                )));

            // Build the two connection tiles: Workspace vs Personal
            let ws_amt = pro_spring_f32(
                pro_eid(&self.id, format!("tile-sp-{svc_id}-ws")),
                if svc.workspace_connected { 1.0 } else { 0.0 },
                0.22,
                w,
                cx,
            );
            let ps_amt = pro_spring_f32(
                pro_eid(&self.id, format!("tile-sp-{svc_id}-ps")),
                if svc.personal_connected { 1.0 } else { 0.0 },
                0.22,
                w,
                cx,
            );

            let build_scope_tile = |target: ConnectionTarget,
                                    connected: bool,
                                    conn_amt: f32,
                                    scopes: SharedString,
                                    lead_ic: ProIcon| {
                let id_for_sw = svc_id.clone();
                let id_for_btn = svc_id.clone();
                let tile_key = match target {
                    ConnectionTarget::Workspace => "ws",
                    ConnectionTarget::Personal => "ps",
                };
                let status_lbl = if connected {
                    tr(cx, "Connected", "已连接")
                } else {
                    tr(cx, "Not connected", "未连接")
                };
                let btn_lbl: SharedString = if connected {
                    tr(cx, "Disconnect", "断开连接").into()
                } else {
                    tr(cx, "Connect", "立即连接").into()
                };
                let active_bg = if p.dark {
                    hsla(210. / 360., 0.85, 0.60, 0.025 + 0.055 * conn_amt)
                } else {
                    hsla(212. / 360., 0.90, 0.48, 0.018 + 0.032 * conn_amt)
                };
                let tile_bg = if conn_amt > 0.01 {
                    active_bg
                } else {
                    soft_surface
                };
                let tile_border = if conn_amt > 0.01 {
                    accent_col.opacity(0.10 + 0.24 * conn_amt)
                } else {
                    p.line
                };
                let btn_rest_bg: Hsla = if connected { p.card } else { p.primary };
                let btn_hover_bg: Hsla = if connected {
                    p.hover
                } else if p.dark {
                    hsla(0., 0., 1., 0.88)
                } else {
                    hsla(0., 0., 0., 0.84)
                };
                let btn_rest_bdr: Hsla = if connected {
                    p.line_strong
                } else {
                    transparent_black()
                };
                let btn_fg: Hsla = if connected { p.text } else { p.on_primary };

                div()
                    .flex_1()
                    .min_w(px(152.))
                    .p(px(10.))
                    .rounded(px(8.))
                    .bg(tile_bg)
                    .border_1()
                    .border_color(tile_border)
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(6.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(5.))
                                    .min_w_0()
                                    .child(pro_icon(
                                        lead_ic,
                                        13.5,
                                        if connected { accent_col } else { p.muted },
                                    ))
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .truncate()
                                            .child(target.title(cx)),
                                    ),
                            )
                            .child(pro_switch(
                                pro_eid(&self.id, format!("sw-{svc_id}-{tile_key}")),
                                format!("{} {}", target.title(cx), svc.name),
                                connected,
                                cx,
                                move |v, _, cx| {
                                    v.toggle_target(&id_for_sw, target, cx);
                                },
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .line_height(px(15.))
                            .text_color(p.muted)
                            .truncate()
                            .child(target.subtitle(cx)),
                    )
                    .child(
                        div()
                            .px(px(6.))
                            .py(px(3.))
                            .rounded(px(5.))
                            .bg(p.chip)
                            .text_size(px(10.5))
                            .line_height(px(14.))
                            .text_color(p.text)
                            .truncate()
                            .child(scopes),
                    )
                    .child(
                        div()
                            .pt(px(2.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(6.))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .child(pro_check_draw(
                                        pro_eid(&self.id, format!("chk-{svc_id}-{tile_key}")),
                                        connected,
                                        12.,
                                        false,
                                        p.green,
                                    ))
                                    .child(pro_swap_label(
                                        pro_eid(&self.id, format!("st-{svc_id}-{tile_key}")),
                                        status_lbl,
                                        11.,
                                        FontWeight::NORMAL,
                                        if connected { p.green_ink } else { p.muted },
                                    )),
                            )
                            .child(
                                pro_tactile(
                                    pro_eid(&self.id, format!("act-{svc_id}-{tile_key}")),
                                    btn_lbl.clone(),
                                    5.,
                                    btn_rest_bg,
                                    btn_hover_bg,
                                    btn_rest_bdr,
                                    p.line_strong,
                                    false,
                                    div()
                                        .h(px(22.))
                                        .px(px(8.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .whitespace_nowrap()
                                        .child(pro_swap_label(
                                            pro_eid(
                                                &self.id,
                                                format!("act-lbl-{svc_id}-{tile_key}"),
                                            ),
                                            btn_lbl,
                                            11.,
                                            FontWeight::MEDIUM,
                                            btn_fg,
                                        )),
                                    cx,
                                    move |v, _, cx| {
                                        v.toggle_target(&id_for_btn, target, cx);
                                    },
                                )
                                .flex_none(),
                            ),
                    )
            };

            let ws_tile = build_scope_tile(
                ConnectionTarget::Workspace,
                svc.workspace_connected,
                ws_amt,
                svc.workspace_scopes.clone(),
                ProIcon::Building,
            );
            let ps_tile = build_scope_tile(
                ConnectionTarget::Personal,
                svc.personal_connected,
                ps_amt,
                svc.personal_scopes.clone(),
                ProIcon::User,
            );

            let mut caps_col = div().flex().flex_col().gap(px(3.));
            for cap in &svc.capabilities {
                caps_col = caps_col.child(
                    div()
                        .flex()
                        .items_start()
                        .gap(px(7.))
                        .text_size(px(12.))
                        .line_height(px(18.))
                        .text_color(p.text)
                        .child(div().mt(px(3.)).flex_none().child(pro_icon(
                            ProIcon::Check,
                            13.,
                            p.muted,
                        )))
                        .child(div().min_w_0().child(cap.clone())),
                );
            }

            let explore_lbl: SharedString = tr(cx, "Explore integration", "查看集成详情").into();
            let explore_id = svc.id.clone();

            let detail_content = div()
                .size_full()
                .min_h_0()
                .flex()
                .flex_col()
                .when(!stacked, |d| d.child(art_banner))
                .child(
                    div()
                        .id(pro_eid(&self.id, format!("detail-scroll-{}", svc.id)))
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .px(px(14.))
                        .pt(px(10.))
                        .pb(px(12.))
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex()
                                .items_start()
                                .justify_between()
                                .gap(px(10.))
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(px(14.5))
                                                .line_height(px(22.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.strong)
                                                .child(format!("{} + {}", self.app_name, svc.name)),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(6.))
                                                .text_size(px(12.))
                                                .line_height(px(16.))
                                                .text_color(p.muted)
                                                .child(svc.domain.clone())
                                                .when(is_conn, |d| {
                                                    d.child(
                                                        div()
                                                            .px(px(5.))
                                                            .rounded(px(4.))
                                                            .bg(p.green_soft)
                                                            .text_size(px(11.))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(p.green_ink)
                                                            .child(tr(cx, "Active", "已启用")),
                                                    )
                                                }),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(12.5))
                                .line_height(px(18.5))
                                .text_color(p.text)
                                .child(svc.description.clone()),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap(px(8.))
                                .child(ws_tile)
                                .child(ps_tile),
                        )
                        .when(!stacked && !svc.capabilities.is_empty(), |d| {
                            d.child(caps_col)
                        })
                        .child(
                            div().pt(px(2.)).flex().items_center().child(
                                pro_tactile(
                                    pro_eid(&self.id, format!("explore-{}", svc.id)),
                                    explore_lbl.clone(),
                                    6.,
                                    p.card,
                                    p.hover,
                                    p.line_strong,
                                    p.line_strong,
                                    false,
                                    div()
                                        .h(px(27.))
                                        .pl(px(10.))
                                        .pr(px(6.))
                                        .flex()
                                        .items_center()
                                        .gap(px(4.))
                                        .text_size(px(12.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .whitespace_nowrap()
                                        .child(explore_lbl)
                                        .child(pro_svg("0 0 16 16", ICON_CHEVRON_R, 14., p.muted)),
                                    cx,
                                    move |_, _, cx| {
                                        cx.emit(IntegrationPickerEvent::Explore(
                                            explore_id.clone(),
                                        ));
                                    },
                                )
                                .flex_none(),
                            ),
                        ),
                );

            div()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .h_full()
                .overflow_hidden()
                .child(pro_view_transition(
                    pro_eid(&self.id, "detail-vtx"),
                    svc.id,
                    detail_content,
                ))
                .into_any_element()
        } else {
            div().flex_1().into_any_element()
        };

        let body = div()
            .when_else(
                stacked,
                |d| d.flex().flex_col().h(px(520.)),
                |d| d.flex().h(px(396.)),
            )
            .min_h_0()
            .child(left_pane)
            .child(right_pane);

        let count_text = if matches!(UiLanguage::current(cx), UiLanguage::Chinese) {
            format!("已连接 {total_connected} / {total_services} 项")
        } else {
            format!("{total_connected} of {total_services} connected")
        };

        let footer = div()
            .h(px(40.))
            .px(px(12.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .text_size(px(12.))
            .line_height(px(16.))
            .text_color(p.muted)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(div().size(px(6.)).rounded(px(2.)).bg(p.green))
                    .child(pro_swap_label(
                        pro_eid(&self.id, "foot-cnt"),
                        count_text,
                        12.,
                        FontWeight::NORMAL,
                        p.muted,
                    )),
            )
            .when(!stacked, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(render_kbd("↑", &p))
                        .child(render_kbd("↓", &p))
                        .child(tr(cx, "to move", "切换"))
                        .child(render_kbd("↵", &p))
                        .child(tr(cx, "to switch", "启停")),
                )
            });

        let width_cell = self.measured_width.clone();
        div()
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(b) = bounds.first() {
                    let w_px = f32::from(b.size.width);
                    if w_px > 10. {
                        width_cell.set(w_px);
                    }
                }
            })
            .id(self.id.clone())
            .w_full()
            .px(px(14.))
            .py(px(16.))
            .flex()
            .justify_center()
            .capture_key_down(cx.listener(|v, ev: &KeyDownEvent, w, cx| {
                let m = ev.keystroke.modifiers;
                let cmd = m.platform || m.control;
                let k = ev.keystroke.key.as_str();
                if cmd && k == "/" {
                    if let Some(ed) = &v.search_editor {
                        ed.clone().update(cx, |e, cx| e.focus(w, cx));
                    }
                    w.prevent_default();
                    cx.stop_propagation();
                    return;
                }
                match k {
                    "left" if v.show_back && v.query(cx).is_empty() => {
                        cx.emit(IntegrationPickerEvent::Back);
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                    "escape" => {
                        if !v.query(cx).is_empty() {
                            if let Some(ed) = v.search_editor.clone() {
                                ed.update(cx, |e, cx| e.set_value("", w, cx));
                            }
                            cx.notify();
                        } else {
                            cx.emit(IntegrationPickerEvent::Closed);
                        }
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .on_key_down(cx.listener(
                |v, ev: &KeyDownEvent, w, cx| match ev.keystroke.key.as_str() {
                    "up" => {
                        v.move_selection(-1, cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                    "down" => {
                        v.move_selection(1, cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                    "enter" => {
                        v.toggle_selected_primary(cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    }
                    _ => {}
                },
            ))
            .child(
                div()
                    .w_full()
                    .max_w(px(660.))
                    .rounded(px(10.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .shadow_lg()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(top_bar)
                    .child(body)
                    .child(footer),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use gpui::AppContext;

    use super::{ConnectionTarget, IntegrationPicker, default_picker_services, partition_services};
    use gpui::TestAppContext;

    #[test]
    fn partitions_connected_and_available_services_cleanly() {
        let services = default_picker_services(false);
        assert_eq!(services.len(), 10);
        let (conn, avail) = partition_services(&services, "");
        assert_eq!(conn.len(), 3);
        assert_eq!(avail.len(), 7);

        let (conn_q, avail_q) = partition_services(&services, "payments");
        assert!(conn_q.is_empty());
        assert_eq!(avail_q.len(), 1);
        assert_eq!(services[avail_q[0]].id.as_ref(), "stripe");
    }

    #[gpui::test]
    fn toggles_workspace_and_personal_scopes_independently(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let picker = cx.new(|cx| IntegrationPicker::new("ip-test", cx));
        picker.update(cx, |ip, cx| {
            assert_eq!(ip.connected_count(), 3);
            let notion_id = "notion".into();
            ip.toggle_target(&notion_id, ConnectionTarget::Personal, cx);
            assert_eq!(ip.connected_count(), 4);
            let notion = ip
                .services()
                .iter()
                .find(|s| s.id.as_ref() == "notion")
                .expect("notion exists");
            assert!(!notion.workspace_connected);
            assert!(notion.personal_connected);
            assert!(notion.is_connected());
        });
    }
}
