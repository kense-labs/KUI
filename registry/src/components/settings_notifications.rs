//! Personal notification preferences screen with daily digest, event delivery channels, and quiet hours.
//!
//! Renders inside [`AppShell`](crate::components::app_shell::AppShell) with the `Settings`
//! sidebar preset (`Notifications` active under `Personal`), providing a daily digest toggle
//! and send-time picker, weekly usage report switch, per-event delivery channel comboboxes with
//! a bulk `"Notify on"` header menu, and scheduled night-time quiet hours.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProPalette, pro_eid, pro_empty_state, pro_fold, pro_mark_selected, pro_pill_indicator,
            pro_presence, pro_selection_sensor, pro_skeleton_bar, pro_switch, pro_toast_banner,
            pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        flyout::TriggerBounds,
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const SUN_SVG: &str = r#"<path d="M17 12C17 14.7614 14.7614 17 12 17C9.23858 17 7 14.7614 7 12C7 9.23858 9.23858 7 12 7C14.7614 7 17 9.23858 17 12Z" stroke="currentColor" stroke-width="1.9"/><path d="M12 2V3.5M12 20.5V22M19.0708 19.0713L18.0101 18.0106M5.98926 5.98926L4.9286 4.9286M22 12H20.5M3.5 12H2M19.0713 4.92871L18.0106 5.98937M5.98975 18.0107L4.92909 19.0714" stroke="currentColor" stroke-linecap="round" stroke-width="1.9"/>"#;

const MOON_SVG: &str = r#"<path d="M20.5 14.469C19.3635 15.0768 18.0654 15.4199 16.687 15.4199C12.2097 15.4199 8.58014 11.7903 8.58014 7.31299C8.58014 5.9346 8.92324 4.63647 9.53101 3.5C5.76818 4.56898 3 8.03198 3 12.1429C3 17.0345 6.96548 21 11.8571 21C15.968 21 19.431 18.2318 20.5 14.469Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;

const MOBILE_SVG: &str = r#"<path d="M13.5 2H10.5C8.14298 2 6.96447 2 6.23223 2.73223C5.5 3.46447 5.5 4.64298 5.5 7V17C5.5 19.357 5.5 20.5355 6.23223 21.2678C6.96447 22 8.14298 22 10.5 22H13.5C15.857 22 17.0355 22 17.7678 21.2678C18.5 20.5355 18.5 19.357 18.5 17V7C18.5 4.64298 18.5 3.46447 17.7678 2.73223C17.0355 2 15.857 2 13.5 2Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M12.125 19H12M12.25 19C12.25 19.1381 12.1381 19.25 12 19.25C11.8619 19.25 11.75 19.1381 11.75 19C11.75 18.8619 11.8619 18.75 12 18.75C12.1381 18.75 12.25 18.8619 12.25 19Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;

const CHECK_SVG: &str = r#"<path d="M5 14L8.5 17.5L19 6.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2.1"/>"#;

const DIGEST_TIMES: &[&str] = &[
    "07:00", "08:00", "09:00", "10:00", "12:00", "17:00", "18:00",
];
const QUIET_FROM_TIMES: &[&str] = &["20:00", "21:00", "22:00", "23:00", "00:00"];
const QUIET_TO_TIMES: &[&str] = &["06:00", "07:00", "08:00", "09:00", "10:00"];

/// Delivery channel selected for an event category in [`SettingsNotificationsScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NotificationChannel {
    /// Deliver via both email and mobile push notifications (`"Email and mobile"`).
    #[default]
    Both,
    /// Deliver via email (`"Email"`).
    Email,
    /// Deliver via mobile push notifications (`"Mobile"`).
    Mobile,
    /// Keep inside the in-app inbox without external alerts (`"Inbox only"`).
    Inbox,
}

impl NotificationChannel {
    /// All supported delivery channels in menu order.
    pub const ALL: [Self; 4] = [Self::Both, Self::Email, Self::Mobile, Self::Inbox];

    /// Canonical English label for this channel.
    pub fn label(self) -> &'static str {
        match self {
            Self::Both => "Email and mobile",
            Self::Email => "Email",
            Self::Mobile => "Mobile",
            Self::Inbox => "Inbox only",
        }
    }

    /// Localized label matching the active UI language.
    pub fn label_tr(self, cx: &App) -> &'static str {
        match self {
            Self::Both => tr(cx, "Email and mobile", "邮件与移动端"),
            Self::Email => tr(cx, "Email", "邮件"),
            Self::Mobile => tr(cx, "Mobile", "移动端"),
            Self::Inbox => tr(cx, "Inbox only", "仅收件箱"),
        }
    }

    /// Short localized description shown inside bulk channel menus.
    pub fn desc_tr(self, cx: &App) -> &'static str {
        match self {
            Self::Both => tr(cx, "Email and mobile push", "邮件与移动推送"),
            Self::Email => tr(cx, "Email only, plus inbox", "仅发邮件并保留收件箱"),
            Self::Mobile => tr(cx, "Push alert only, plus inbox", "仅移动推送并保留收件箱"),
            Self::Inbox => tr(cx, "Silent in your inbox", "静默保存在收件箱"),
        }
    }

    fn render_icon(self, side: f32, color: Hsla) -> AnyElement {
        match self {
            Self::Both => pro_icon(ProIcon::Bell, side, color),
            Self::Email => pro_icon(ProIcon::Mail, side, color),
            Self::Mobile => pro_svg("0 0 24 24", MOBILE_SVG, side, color),
            Self::Inbox => pro_icon(ProIcon::Inbox, side, color),
        }
    }
}

/// Delivery preference for a single notification category.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationTypePref {
    /// Stable identifier (e.g. `"mentions"`, `"approvals"`, `"failed-runs"`).
    pub id: SharedString,
    /// Primary row title.
    pub title: SharedString,
    /// Secondary explanation shown below the title.
    pub note: SharedString,
    /// Chosen delivery channel.
    pub channel: NotificationChannel,
}

impl NotificationTypePref {
    /// Construct a notification category row.
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        note: impl Into<SharedString>,
        channel: NotificationChannel,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            note: note.into(),
            channel,
        }
    }
}

/// Night-time quiet-hours schedule holding mobile push notifications until morning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuietHours {
    /// Whether mobile pushes are held overnight.
    pub enabled: bool,
    /// Quiet-window start time in 24-hour `"HH:MM"` format (`"22:00"` by default).
    pub from: SharedString,
    /// Quiet-window end time in 24-hour `"HH:MM"` format (`"08:00"` by default).
    pub to: SharedString,
}

impl Default for QuietHours {
    fn default() -> Self {
        Self {
            enabled: false,
            from: "22:00".into(),
            to: "08:00".into(),
        }
    }
}

impl QuietHours {
    /// Construct a [`QuietHours`] configuration.
    pub fn new(enabled: bool, from: impl Into<SharedString>, to: impl Into<SharedString>) -> Self {
        Self {
            enabled,
            from: from.into(),
            to: to.into(),
        }
    }
}

/// Complete personal notification settings managed by [`SettingsNotificationsScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationValues {
    /// Whether the daily email digest is enabled.
    pub digest_enabled: bool,
    /// Time of day at which the daily digest is sent (`"08:00"` by default).
    pub digest_time: SharedString,
    /// IANA timezone displayed beneath `"Send at"` (`"Europe/Vienna"` by default).
    pub timezone: SharedString,
    /// Whether the Monday weekly usage report email is enabled.
    pub weekly_report: bool,
    /// Per-event notification category delivery preferences.
    pub types: Vec<NotificationTypePref>,
    /// Overnight quiet-hours schedule for mobile push notifications.
    pub quiet_hours: QuietHours,
}

impl Default for NotificationValues {
    fn default() -> Self {
        Self {
            digest_enabled: true,
            digest_time: "08:00".into(),
            timezone: "Europe/Vienna".into(),
            weekly_report: false,
            types: Self::default_types(),
            quiet_hours: QuietHours::default(),
        }
    }
}

impl NotificationValues {
    /// Return the 8 canonical notification categories from the KUI Pro reference.
    pub fn default_types() -> Vec<NotificationTypePref> {
        vec![
            NotificationTypePref::new(
                "mentions",
                "Mentions",
                "Someone mentions you in a chat or a file",
                NotificationChannel::Both,
            ),
            NotificationTypePref::new(
                "replies",
                "Replies",
                "Answers to your comments",
                NotificationChannel::Both,
            ),
            NotificationTypePref::new(
                "approvals",
                "Approvals",
                "An agent waits for your OK before it acts",
                NotificationChannel::Both,
            ),
            NotificationTypePref::new(
                "finished-runs",
                "Finished runs",
                "An agent you started is done",
                NotificationChannel::Mobile,
            ),
            NotificationTypePref::new(
                "failed-runs",
                "Failed runs",
                "A run stopped with an error or ran out of budget",
                NotificationChannel::Both,
            ),
            NotificationTypePref::new(
                "shared",
                "Shared with you",
                "Chats, folders and agents others share",
                NotificationChannel::Email,
            ),
            NotificationTypePref::new(
                "access-requests",
                "Access requests",
                "People asking to join a folder or agent you own",
                NotificationChannel::Email,
            ),
            NotificationTypePref::new(
                "usage-limits",
                "Usage limits",
                "Your plan reaches 80% and 100% of its credits",
                NotificationChannel::Email,
            ),
        ]
    }

    /// Look up the configured [`NotificationChannel`] for a notification type `id`.
    pub fn channel_for(&self, id: &str) -> Option<NotificationChannel> {
        self.types
            .iter()
            .find(|item| item.id.as_ref() == id)
            .map(|item| item.channel)
    }

    /// Update the [`NotificationChannel`] for a single type `id`; returns `true` when changed.
    pub fn set_type_channel(&mut self, id: &str, channel: NotificationChannel) -> bool {
        if let Some(item) = self.types.iter_mut().find(|t| t.id.as_ref() == id)
            && item.channel != channel
        {
            item.channel = channel;
            return true;
        }
        false
    }

    /// Set every notification type to `channel` at once; returns `true` if any row changed.
    pub fn set_all_channels(&mut self, channel: NotificationChannel) -> bool {
        let mut changed = false;
        for item in &mut self.types {
            if item.channel != channel {
                item.channel = channel;
                changed = true;
            }
        }
        changed
    }
}

/// Account identity displayed at the bottom of the settings sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsNotificationsUser {
    /// Full display name of the signed-in user.
    pub name: SharedString,
    /// Email address shown in the account menu.
    pub email: SharedString,
}

impl Default for SettingsNotificationsUser {
    fn default() -> Self {
        Self {
            name: "Alex Morgan".into(),
            email: "alex@acme.com".into(),
        }
    }
}

impl SettingsNotificationsUser {
    /// Construct a sidebar user summary with `name` and `email`.
    pub fn new(name: impl Into<SharedString>, email: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
        }
    }
}

/// Events emitted by [`SettingsNotificationsScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsNotificationsEvent {
    /// Emitted with the full [`NotificationValues`] whenever any setting changes.
    Save(NotificationValues),
    /// Emitted when a settings sidebar item or `"Back to app"` is activated.
    Navigate(SharedString),
}

impl EventEmitter<SettingsNotificationsEvent> for SettingsNotificationsScreen {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OpenMenu {
    DigestTime,
    NotifyAll,
    TypeChannel(usize),
    QuietFrom,
    QuietTo,
}

impl OpenMenu {
    fn key(self) -> SharedString {
        match self {
            Self::DigestTime => "digest".into(),
            Self::NotifyAll => "all".into(),
            Self::TypeChannel(idx) => format!("type-{idx}").into(),
            Self::QuietFrom => "qfrom".into(),
            Self::QuietTo => "qto".into(),
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum ChannelFilter {
    #[default]
    All,
    Email,
    Mobile,
    Inbox,
}

impl ChannelFilter {
    const ALL: [Self; 4] = [Self::All, Self::Email, Self::Mobile, Self::Inbox];

    fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Email => "email",
            Self::Mobile => "mobile",
            Self::Inbox => "inbox",
        }
    }

    fn label_tr(self, cx: &App) -> &'static str {
        match self {
            Self::All => tr(cx, "All", "全部"),
            Self::Email => tr(cx, "Email", "邮件"),
            Self::Mobile => tr(cx, "Mobile", "移动端"),
            Self::Inbox => tr(cx, "Inbox only", "仅收件箱"),
        }
    }

    fn matches(self, ch: NotificationChannel) -> bool {
        match self {
            Self::All => true,
            Self::Email => matches!(ch, NotificationChannel::Both | NotificationChannel::Email),
            Self::Mobile => matches!(ch, NotificationChannel::Both | NotificationChannel::Mobile),
            Self::Inbox => matches!(ch, NotificationChannel::Inbox),
        }
    }
}

fn settings_nav_groups(cx: &App) -> Vec<ShellNavGroup> {
    vec![
        ShellNavGroup::named(
            tr(cx, "Personal", "个人"),
            vec![
                ShellNavItem::new("profile", tr(cx, "Profile", "个人资料"), ProIcon::User),
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
                ShellNavItem::new("privacy", tr(cx, "Privacy", "隐私"), ProIcon::Shield),
            ],
        ),
        ShellNavGroup::named(
            tr(cx, "Workspace", "工作区"),
            vec![
                ShellNavItem::new("general", tr(cx, "General", "常规"), ProIcon::Settings),
                ShellNavItem::new("members", tr(cx, "Members", "成员与团队"), ProIcon::Users),
                ShellNavItem::new("models", tr(cx, "Models", "模型"), ProIcon::Building),
                ShellNavItem::new("security", tr(cx, "Security", "安全"), ProIcon::KeyLock),
                ShellNavItem::new(
                    "billing",
                    tr(cx, "Billing", "账单与订阅"),
                    ProIcon::CreditCard,
                ),
            ],
        ),
    ]
}

fn time_is_night(hhmm: &str) -> bool {
    let hour = hhmm
        .split(':')
        .next()
        .and_then(|h| h.trim().parse::<u32>().ok())
        .unwrap_or(8);
    !(6..18).contains(&hour)
}

fn render_time_glyph(hhmm: &str, side: f32, color: Hsla) -> AnyElement {
    if time_is_night(hhmm) {
        pro_svg("0 0 24 24", MOON_SVG, side, color)
    } else {
        pro_svg("0 0 24 24", SUN_SVG, side, color)
    }
}

fn localized_type_copy<'a>(item: &'a NotificationTypePref, cx: &App) -> (&'a str, &'a str) {
    match item.id.as_ref() {
        "mentions" => (
            tr(cx, "Mentions", "提及我的"),
            tr(
                cx,
                "Someone mentions you in a chat or a file",
                "有人在对话或文件中提及你",
            ),
        ),
        "replies" => (
            tr(cx, "Replies", "评论回复"),
            tr(cx, "Answers to your comments", "针对你评论的回复"),
        ),
        "approvals" => (
            tr(cx, "Approvals", "待审批操作"),
            tr(
                cx,
                "An agent waits for your OK before it acts",
                "智能体执行敏感动作前等待你的批准",
            ),
        ),
        "finished-runs" => (
            tr(cx, "Finished runs", "运行已完成"),
            tr(
                cx,
                "An agent you started is done",
                "你启动的智能体任务已完成",
            ),
        ),
        "failed-runs" => (
            tr(cx, "Failed runs", "运行失败"),
            tr(
                cx,
                "A run stopped with an error or ran out of budget",
                "任务因错误中止或超出预算上限",
            ),
        ),
        "shared" => (
            tr(cx, "Shared with you", "与我共享"),
            tr(
                cx,
                "Chats, folders and agents others share",
                "他人分享给你的对话、文件夹与智能体",
            ),
        ),
        "access-requests" => (
            tr(cx, "Access requests", "访问权限申请"),
            tr(
                cx,
                "People asking to join a folder or agent you own",
                "成员请求加入你拥有的文件夹或智能体",
            ),
        ),
        "usage-limits" => (
            tr(cx, "Usage limits", "用量阈值提醒"),
            tr(
                cx,
                "Your plan reaches 80% and 100% of its credits",
                "套餐额度达到 80% 与 100% 时提醒",
            ),
        ),
        _ => (item.title.as_ref(), item.note.as_ref()),
    }
}

/// Personal notification settings screen (`settings-notifications`).
pub struct SettingsNotificationsScreen {
    id: ElementId,
    focus: FocusHandle,
    shell: Entity<AppShell>,
    standalone: bool,
    loading: bool,
    empty: bool,
    workspace: SharedString,
    user: SettingsNotificationsUser,
    values: NotificationValues,
    open_menu: Option<OpenMenu>,
    last_menu: Option<OpenMenu>,
    menu_highlight: usize,
    channel_filter: ChannelFilter,
    digest_bounds: TriggerBounds,
    all_bounds: TriggerBounds,
    type_bounds: Vec<TriggerBounds>,
    quiet_from_bounds: TriggerBounds,
    quiet_to_bounds: TriggerBounds,
    toast: Option<(SharedString, Instant)>,
    started_at: Instant,
    page_width: Rc<Cell<f32>>,
}

impl Focusable for SettingsNotificationsScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl SettingsNotificationsScreen {
    /// Construct a new [`SettingsNotificationsScreen`] with canonical sample preferences.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let focus = cx.focus_handle();
        let values = NotificationValues::default();
        let workspace: SharedString = "Acme".into();
        let user = SettingsNotificationsUser::default();
        let type_bounds = (0..values.types.len())
            .map(|_| TriggerBounds::default())
            .collect();

        let ws_clone = workspace.clone();
        let u_name = user.name.clone();
        let u_email = user.email.clone();
        let this = cx.entity();

        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Settings, shell_cx)
                .with_groups(settings_nav_groups(shell_cx))
                .with_workspace(ws_clone)
                .with_user(u_name, u_email)
                .with_active("Notifications")
                .with_work_area(false, false);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s
        });
        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                cx.emit(SettingsNotificationsEvent::Navigate(label.clone()));
            }
            AppShellEvent::Back => {
                cx.emit(SettingsNotificationsEvent::Navigate("Back to app".into()));
            }
            _ => {}
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            focus,
            shell,
            standalone: false,
            loading: false,
            empty: false,
            workspace,
            user,
            values,
            open_menu: None,
            last_menu: None,
            menu_highlight: 0,
            channel_filter: ChannelFilter::All,
            digest_bounds: TriggerBounds::default(),
            all_bounds: TriggerBounds::default(),
            type_bounds,
            quiet_from_bounds: TriggerBounds::default(),
            quiet_to_bounds: TriggerBounds::default(),
            toast: None,
            started_at: Instant::now(),
            page_width: Rc::new(Cell::new(720.)),
        }
    }

    /// Override the initial [`NotificationValues`].
    pub fn with_notifications(mut self, notifications: NotificationValues) -> Self {
        self.type_bounds = (0..notifications.types.len())
            .map(|_| TriggerBounds::default())
            .collect();
        self.values = notifications;
        self
    }

    /// Override the workspace name shown in the shell sidebar.
    pub fn with_workspace(
        mut self,
        workspace: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let ws: SharedString = workspace.into();
        self.workspace = ws.clone();
        self.shell.update(cx, |s, s_cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", s_cx)).with_workspace(ws);
        });
        self
    }

    /// Override the account displayed at the bottom of the settings sidebar.
    pub fn with_user(mut self, user: SettingsNotificationsUser, cx: &mut Context<Self>) -> Self {
        let u_name = user.name.clone();
        let u_email = user.email.clone();
        self.user = user;
        self.shell.update(cx, |s, s_cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", s_cx)).with_user(u_name, u_email);
        });
        self
    }

    /// Return the workspace name displayed in the shell sidebar.
    pub fn workspace(&self) -> &str {
        self.workspace.as_ref()
    }

    /// Return the sidebar account summary.
    pub fn user(&self) -> &SettingsNotificationsUser {
        &self.user
    }

    /// Start in skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Start in empty notification-types state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        if empty {
            self.values.types.clear();
            self.type_bounds.clear();
        }
        self
    }

    /// Render only the page column without the outer [`AppShell`] frame.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Return the current [`NotificationValues`].
    pub fn notifications(&self) -> &NotificationValues {
        &self.values
    }

    /// Return whether skeleton loading placeholders are active.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    /// Toggle skeleton loading placeholders at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        if self.loading != loading {
            self.loading = loading;
            self.open_menu = None;
            cx.notify();
        }
    }

    /// Return whether the empty state is active.
    pub fn is_empty(&self) -> bool {
        self.empty || self.values.types.is_empty()
    }

    /// Toggle the empty notification-types state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        self.open_menu = None;
        if empty {
            self.values.types.clear();
            self.type_bounds.clear();
        } else if self.values.types.is_empty() {
            self.values.types = NotificationValues::default_types();
            self.type_bounds = (0..self.values.types.len())
                .map(|_| TriggerBounds::default())
                .collect();
        }
        cx.notify();
    }

    /// Restore the 8 default notification categories and emit [`SettingsNotificationsEvent::Save`].
    pub fn reset_defaults(&mut self, cx: &mut Context<Self>) {
        self.empty = false;
        self.values = NotificationValues::default();
        self.type_bounds = (0..self.values.types.len())
            .map(|_| TriggerBounds::default())
            .collect();
        self.emit_saved(
            tr(cx, "Notification defaults restored", "已恢复默认通知设置"),
            cx,
        );
    }

    /// Toggle the daily digest switch and emit [`SettingsNotificationsEvent::Save`].
    pub fn toggle_digest(&mut self, cx: &mut Context<Self>) {
        self.values.digest_enabled = !self.values.digest_enabled;
        self.open_menu = None;
        let msg = if self.values.digest_enabled {
            tr(cx, "Daily digest turned on", "已开启每日摘要")
        } else {
            tr(cx, "Daily digest turned off", "已关闭每日摘要")
        };
        self.emit_saved(msg, cx);
    }

    /// Update the daily digest delivery time and emit [`SettingsNotificationsEvent::Save`].
    pub fn set_digest_time(&mut self, time: impl Into<SharedString>, cx: &mut Context<Self>) {
        let time: SharedString = time.into();
        self.open_menu = None;
        if self.values.digest_time != time {
            self.values.digest_time = time;
            self.emit_saved(tr(cx, "Digest time updated", "摘要发送时间已更新"), cx);
        } else {
            cx.notify();
        }
    }

    /// Toggle the Monday weekly usage report switch and emit [`SettingsNotificationsEvent::Save`].
    pub fn toggle_weekly_report(&mut self, cx: &mut Context<Self>) {
        self.values.weekly_report = !self.values.weekly_report;
        let msg = if self.values.weekly_report {
            tr(cx, "Weekly usage report enabled", "已开启每周用量报告")
        } else {
            tr(cx, "Weekly usage report disabled", "已关闭每周用量报告")
        };
        self.emit_saved(msg, cx);
    }

    /// Set the delivery channel for a single notification category `id` and emit [`SettingsNotificationsEvent::Save`].
    pub fn set_channel(
        &mut self,
        id: &str,
        channel: NotificationChannel,
        cx: &mut Context<Self>,
    ) -> bool {
        self.open_menu = None;
        if self.values.set_type_channel(id, channel) {
            self.emit_saved(tr(cx, "Notification channel updated", "通知渠道已更新"), cx);
            true
        } else {
            cx.notify();
            false
        }
    }

    /// Set every notification category to `channel` at once and emit [`SettingsNotificationsEvent::Save`].
    pub fn set_all_channels(&mut self, channel: NotificationChannel, cx: &mut Context<Self>) {
        self.open_menu = None;
        if self.values.set_all_channels(channel) {
            let msg = format!(
                "{}: {}",
                tr(cx, "All types set to", "全部通知已设为"),
                channel.label_tr(cx)
            );
            self.emit_saved(msg, cx);
        } else {
            cx.notify();
        }
    }

    /// Toggle overnight quiet hours and emit [`SettingsNotificationsEvent::Save`].
    pub fn toggle_quiet_hours(&mut self, cx: &mut Context<Self>) {
        self.values.quiet_hours.enabled = !self.values.quiet_hours.enabled;
        self.open_menu = None;
        let msg = if self.values.quiet_hours.enabled {
            tr(cx, "Quiet hours enabled", "夜间免打扰已开启")
        } else {
            tr(cx, "Quiet hours disabled", "夜间免打扰已关闭")
        };
        self.emit_saved(msg, cx);
    }

    /// Update the quiet-hours window (`from` and `to`) and emit [`SettingsNotificationsEvent::Save`].
    pub fn set_quiet_window(
        &mut self,
        from: impl Into<SharedString>,
        to: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let from: SharedString = from.into();
        let to: SharedString = to.into();
        self.open_menu = None;
        if self.values.quiet_hours.from != from || self.values.quiet_hours.to != to {
            self.values.quiet_hours.from = from;
            self.values.quiet_hours.to = to;
            self.emit_saved(tr(cx, "Quiet hours updated", "免打扰时段已更新"), cx);
        } else {
            cx.notify();
        }
    }

    fn emit_saved(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast = Some((msg.into(), Instant::now()));
        cx.emit(SettingsNotificationsEvent::Save(self.values.clone()));
        cx.notify();
    }

    fn ensure_type_bounds(&mut self) {
        if self.type_bounds.len() != self.values.types.len() {
            self.type_bounds = (0..self.values.types.len())
                .map(|_| TriggerBounds::default())
                .collect();
        }
    }

    fn toggle_menu(&mut self, which: OpenMenu, cx: &mut Context<Self>) {
        if self.open_menu == Some(which) {
            self.open_menu = None;
        } else {
            self.open_menu = Some(which);
            self.last_menu = Some(which);
            self.menu_highlight = match which {
                OpenMenu::DigestTime => DIGEST_TIMES
                    .iter()
                    .position(|t| *t == self.values.digest_time.as_ref())
                    .unwrap_or(1),
                OpenMenu::NotifyAll => 0,
                OpenMenu::TypeChannel(idx) => self
                    .values
                    .types
                    .get(idx)
                    .and_then(|item| {
                        NotificationChannel::ALL
                            .iter()
                            .position(|c| *c == item.channel)
                    })
                    .unwrap_or(0),
                OpenMenu::QuietFrom => QUIET_FROM_TIMES
                    .iter()
                    .position(|t| *t == self.values.quiet_hours.from.as_ref())
                    .unwrap_or(2),
                OpenMenu::QuietTo => QUIET_TO_TIMES
                    .iter()
                    .position(|t| *t == self.values.quiet_hours.to.as_ref())
                    .unwrap_or(2),
            };
        }
        cx.notify();
    }

    fn menu_len(which: OpenMenu) -> usize {
        match which {
            OpenMenu::DigestTime => DIGEST_TIMES.len(),
            OpenMenu::NotifyAll | OpenMenu::TypeChannel(_) => NotificationChannel::ALL.len(),
            OpenMenu::QuietFrom => QUIET_FROM_TIMES.len(),
            OpenMenu::QuietTo => QUIET_TO_TIMES.len(),
        }
    }

    fn confirm_menu_choice(&mut self, which: OpenMenu, idx: usize, cx: &mut Context<Self>) {
        match which {
            OpenMenu::DigestTime => {
                if let Some(&t) = DIGEST_TIMES.get(idx) {
                    self.set_digest_time(t, cx);
                }
            }
            OpenMenu::NotifyAll => {
                if let Some(&ch) = NotificationChannel::ALL.get(idx) {
                    self.set_all_channels(ch, cx);
                }
            }
            OpenMenu::TypeChannel(row_idx) => {
                if let (Some(id), Some(&ch)) = (
                    self.values.types.get(row_idx).map(|t| t.id.clone()),
                    NotificationChannel::ALL.get(idx),
                ) {
                    self.set_channel(id.as_ref(), ch, cx);
                }
            }
            OpenMenu::QuietFrom => {
                if let Some(&from) = QUIET_FROM_TIMES.get(idx) {
                    let to = self.values.quiet_hours.to.clone();
                    self.set_quiet_window(from, to, cx);
                }
            }
            OpenMenu::QuietTo => {
                if let Some(&to) = QUIET_TO_TIMES.get(idx) {
                    let from = self.values.quiet_hours.from.clone();
                    self.set_quiet_window(from, to, cx);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_combobox_trigger(
        &self,
        id_suffix: impl Into<SharedString>,
        aria: SharedString,
        lead_icon: AnyElement,
        value_text: SharedString,
        width: f32,
        tabular: bool,
        which: OpenMenu,
        bounds: &TriggerBounds,
        narrow_full: bool,
        input_surface: Hsla,
        input_border: Hsla,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let is_open = self.open_menu == Some(which);
        let border_c = if is_open { p.line_strong } else { input_border };
        let hover_border = p.line_strong;
        let focus = p.focus;
        let chevron = if is_open {
            ProIcon::ChevronUp
        } else {
            ProIcon::ChevronDown
        };
        let el = div()
            .id(pro_eid(&self.id, id_suffix))
            .role(Role::ComboBox)
            .aria_label(aria)
            .aria_expanded(is_open)
            .tab_index(0)
            .relative()
            .when(narrow_full, |d| d.w_full())
            .when(!narrow_full, |d| d.w(px(width)).flex_none())
            .h(px(32.))
            .pl(px(10.))
            .pr(px(8.))
            .rounded(px(7.))
            .bg(input_surface)
            .border_1()
            .border_color(border_c)
            .hover(move |s| s.border_color(hover_border))
            .focus_visible(move |s| s.border_1().border_color(focus))
            .flex()
            .items_center()
            .gap(px(8.))
            .cursor_pointer()
            .when(tabular, |d| {
                d.font_features(FontFeatures(std::sync::Arc::new(vec![("tnum".into(), 1)])))
            })
            .child(lead_icon)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(13.))
                    .text_color(p.ink)
                    .child(value_text),
            )
            .child(pro_icon(chevron, 14., p.muted))
            .child(bounds.measure());

        activate(el, cx, move |this, _, cx| this.toggle_menu(which, cx))
    }

    fn render_open_dropdown(
        &self,
        which: OpenMenu,
        is_open: bool,
        w: &Window,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let bounds = match which {
            OpenMenu::DigestTime => &self.digest_bounds,
            OpenMenu::NotifyAll => &self.all_bounds,
            OpenMenu::TypeChannel(idx) => match self.type_bounds.get(idx) {
                Some(b) => b,
                None => return div().into_any_element(),
            },
            OpenMenu::QuietFrom => &self.quiet_from_bounds,
            OpenMenu::QuietTo => &self.quiet_to_bounds,
        };

        let rect = bounds.get();
        let min_w = match which {
            OpenMenu::DigestTime | OpenMenu::QuietFrom | OpenMenu::QuietTo => 136.,
            OpenMenu::NotifyAll => 214.,
            OpenMenu::TypeChannel(_) => 190.,
        };
        let menu_w = f32::from(rect.size.width).max(min_w);

        struct MenuOption {
            icon: AnyElement,
            label: SharedString,
            desc: Option<SharedString>,
            selected: bool,
        }

        let options: Vec<MenuOption> = match which {
            OpenMenu::DigestTime => DIGEST_TIMES
                .iter()
                .map(|&t| MenuOption {
                    icon: render_time_glyph(t, 15., p.muted),
                    label: t.into(),
                    desc: None,
                    selected: self.values.digest_time.as_ref() == t,
                })
                .collect(),
            OpenMenu::QuietFrom => QUIET_FROM_TIMES
                .iter()
                .map(|&t| MenuOption {
                    icon: render_time_glyph(t, 15., p.muted),
                    label: t.into(),
                    desc: None,
                    selected: self.values.quiet_hours.from.as_ref() == t,
                })
                .collect(),
            OpenMenu::QuietTo => QUIET_TO_TIMES
                .iter()
                .map(|&t| MenuOption {
                    icon: render_time_glyph(t, 15., p.muted),
                    label: t.into(),
                    desc: None,
                    selected: self.values.quiet_hours.to.as_ref() == t,
                })
                .collect(),
            OpenMenu::NotifyAll => NotificationChannel::ALL
                .iter()
                .map(|&ch| {
                    let all_match = !self.values.types.is_empty()
                        && self.values.types.iter().all(|t| t.channel == ch);
                    MenuOption {
                        icon: ch.render_icon(15., p.muted),
                        label: ch.label_tr(cx).into(),
                        desc: Some(ch.desc_tr(cx).into()),
                        selected: all_match,
                    }
                })
                .collect(),
            OpenMenu::TypeChannel(row_idx) => {
                let current = self
                    .values
                    .types
                    .get(row_idx)
                    .map(|t| t.channel)
                    .unwrap_or_default();
                NotificationChannel::ALL
                    .iter()
                    .map(|&ch| MenuOption {
                        icon: ch.render_icon(15., p.muted),
                        label: ch.label_tr(cx).into(),
                        desc: None,
                        selected: current == ch,
                    })
                    .collect()
            }
        };

        let rich = matches!(which, OpenMenu::NotifyAll);
        let row_h = if rich { 44. } else { 32. };
        let header_h = if rich { 26. } else { 0. };
        let est_h = (options.len() as f32 * row_h + header_h + 8.).min(300.);
        let (pos, side) = bounds.place(w.viewport_size(), px(est_h), px(5.));
        let anchor_copy = bounds.clone();
        let menu_bg: Hsla = if p.dark {
            rgb(0x1f1f22).into()
        } else {
            rgb(0xffffff).into()
        };

        let mut list = div()
            .id(pro_eid(&self.id, "select-listbox"))
            .role(if rich { Role::Menu } else { Role::ListBox })
            .max_h(px(288.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(1.));

        if rich {
            list = list.child(
                div()
                    .px(px(10.))
                    .pt(px(6.))
                    .pb(px(4.))
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.faint)
                    .child(tr(cx, "Set every type to", "将全部通知设为")),
            );
        }

        for (idx, opt) in options.into_iter().enumerate() {
            let highlighted = self.menu_highlight == idx;
            let row_bg = if highlighted {
                p.hover
            } else {
                transparent_black()
            };
            let hover_bg = p.hover;
            let selected = opt.selected;
            let label = opt.label;
            list = list.child(
                div()
                    .id(pro_eid(&self.id, format!("menu-opt-{idx}")))
                    .role(if rich {
                        Role::MenuItem
                    } else {
                        Role::ListBoxOption
                    })
                    .aria_label(label.clone())
                    .when(!rich, |d| d.aria_selected(selected))
                    .min_h(px(row_h))
                    .px(px(10.))
                    .py(px(if rich { 6. } else { 5. }))
                    .rounded(px(7.))
                    .bg(row_bg)
                    .hover(move |s| s.bg(hover_bg))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(9.))
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, win, app_cx| {
                        win.prevent_default();
                        app_cx.stop_propagation();
                    })
                    .on_hover(cx.listener(move |this, hovered: &bool, _, h_cx| {
                        if *hovered && this.menu_highlight != idx {
                            this.menu_highlight = idx;
                            h_cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |this, _, _, c_cx| {
                        this.confirm_menu_choice(which, idx, c_cx);
                        c_cx.stop_propagation();
                    }))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(9.))
                            .child(opt.icon)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(1.))
                                    .child(
                                        div()
                                            .truncate()
                                            .text_size(px(13.))
                                            .font_weight(if selected {
                                                FontWeight::MEDIUM
                                            } else {
                                                FontWeight::NORMAL
                                            })
                                            .text_color(if selected { p.ink } else { p.text })
                                            .child(label),
                                    )
                                    .when_some(opt.desc, |d, desc| {
                                        d.child(
                                            div()
                                                .truncate()
                                                .text_size(px(12.))
                                                .line_height(px(16.))
                                                .text_color(p.faint)
                                                .child(desc),
                                        )
                                    }),
                            ),
                    )
                    .when(selected, |d| {
                        d.child(pro_svg("0 0 24 24", CHECK_SVG, 14., p.ink))
                    }),
            );
        }

        let list_vtx = pro_view_transition(pro_eid(&self.id, "select-menu-vtx"), which.key(), list);

        let popover_inner = div()
            .id(pro_eid(&self.id, "select-popover"))
            .when(is_open, |d| d.occlude())
            .w(px(menu_w))
            .p(px(4.))
            .rounded(px(12.))
            .bg(menu_bg)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .when(is_open, |d| {
                d.on_mouse_down_out(cx.listener(move |this, ev: &MouseDownEvent, _, o_cx| {
                    if !anchor_copy.get().contains(&ev.position) {
                        this.open_menu = None;
                        o_cx.notify();
                    }
                }))
            })
            .child(list_vtx)
            .into_any_element();

        let pres_id = pro_eid(&self.id, "select-popover-wrap");
        pro_presence(
            pro_eid(&self.id, "select-popover-pres"),
            is_open,
            0.18,
            0.14,
            move |amount| {
                let dy = -4. * (1. - amount);
                deferred(
                    anchored()
                        .anchor(side)
                        .position(pos)
                        .snap_to_window_with_margin(px(10.))
                        .child(
                            div()
                                .id(pres_id.clone())
                                .relative()
                                .top(px(dy))
                                .opacity(amount)
                                .child(popover_inner),
                        ),
                )
                .with_priority(3)
                .into_any_element()
            },
        )
    }

    fn render_loading_skeleton(&self, narrow: bool, phase: f32, p: &ProPalette, cx: &App) -> Div {
        let ch_w = if narrow { 150. } else { 176. };
        div()
            .w_full()
            .max_w(px(760.))
            .mx_auto()
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(24.))
                    .line_height(px(32.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(tr(cx, "Notifications", "通知")),
            )
            .child(
                div()
                    .mt(px(4.))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.text)
                    .child(tr(
                        cx,
                        "Choose what you hear about and where. Everything still lands in your inbox.",
                        "选择你想接收的通知内容与投递方式。所有消息仍会保留在收件箱中。",
                    )),
            )
            // Digest skeleton
            .child(
                div()
                    .mt(px(36.))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(tr(cx, "Digest", "邮件摘要")),
                    )
                    .child(
                        div()
                            .mt(px(2.))
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(p.muted)
                            .child(tr(
                                cx,
                                "One email instead of many – what happened while you were away.",
                                "将多封通知合并为一封邮件——汇总你离开期间的动态。",
                            )),
                    ),
            )
            .child(
                div()
                    .mt(px(12.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .min_h(px(60.))
                            .py(px(10.))
                            .pl(px(16.))
                            .pr(px(12.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(24.))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(pro_skeleton_bar(92., 12., 4., phase, cx))
                                    .child(pro_skeleton_bar(228., 10., 4., phase + 0.05, cx)),
                            )
                            .child(pro_skeleton_bar(34., 20., 10., phase + 0.1, cx)),
                    )
                    .child(
                        div()
                            .min_h(px(60.))
                            .py(px(10.))
                            .pl(px(16.))
                            .pr(px(12.))
                            .border_t_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(24.))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(pro_skeleton_bar(68., 12., 4., phase + 0.14, cx))
                                    .child(pro_skeleton_bar(156., 10., 4., phase + 0.18, cx)),
                            )
                            .child(pro_skeleton_bar(112., 32., 7., phase + 0.22, cx)),
                    )
                    .child(
                        div()
                            .min_h(px(60.))
                            .py(px(10.))
                            .pl(px(16.))
                            .pr(px(12.))
                            .border_t_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(24.))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(pro_skeleton_bar(134., 12., 4., phase + 0.26, cx))
                                    .child(pro_skeleton_bar(248., 10., 4., phase + 0.3, cx)),
                            )
                            .child(pro_skeleton_bar(34., 20., 10., phase + 0.34, cx)),
                    ),
            )
            // Notification types skeleton
            .child(
                div()
                    .mt(px(36.))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(tr(cx, "Notification types", "通知类型")),
                    )
                    .child(
                        div()
                            .mt(px(2.))
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(p.muted)
                            .child(tr(
                                cx,
                                "Choose where each one is delivered.",
                                "为每种事件选择投递渠道。",
                            )),
                    ),
            )
            .child(
                div()
                    .mt(px(12.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(40.))
                            .pl(px(16.))
                            .pr(px(12.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(pro_skeleton_bar(92., 11., 4., phase + 0.36, cx))
                            .child(pro_skeleton_bar(72., 14., 4., phase + 0.38, cx)),
                    )
                    .children((0..8usize).map(|idx| {
                        let p_off = phase + 0.4 + idx as f32 * 0.05;
                        div()
                            .min_h(px(56.))
                            .py(px(10.))
                            .pl(px(16.))
                            .pr(px(12.))
                            .border_t_1()
                            .border_color(p.line)
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(24.))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(pro_skeleton_bar(
                                        84. + (idx % 3) as f32 * 24.,
                                        12.,
                                        4.,
                                        p_off,
                                        cx,
                                    ))
                                    .child(pro_skeleton_bar(
                                        176. + (idx % 4) as f32 * 22.,
                                        10.,
                                        4.,
                                        p_off + 0.02,
                                        cx,
                                    )),
                            )
                            .child(pro_skeleton_bar(ch_w, 32., 7., p_off + 0.04, cx))
                    })),
            )
            // Quiet hours skeleton
            .child(
                div()
                    .mt(px(36.))
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(tr(cx, "Quiet hours", "免打扰时段")),
            )
            .child(
                div()
                    .mt(px(12.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .min_h(px(60.))
                    .py(px(10.))
                    .pl(px(16.))
                    .pr(px(12.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(24.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(pro_skeleton_bar(124., 12., 4., phase + 0.8, cx))
                            .child(pro_skeleton_bar(236., 10., 4., phase + 0.84, cx)),
                    )
                    .child(pro_skeleton_bar(34., 20., 10., phase + 0.88, cx)),
            )
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_type_bounds();
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let now = Instant::now();

        if let Some((_, ts)) = &self.toast
            && now.duration_since(*ts).as_secs_f32() > 2.45
        {
            self.toast = None;
        }

        if (self.loading || self.toast.is_some()) && !reduced && w.is_visible() {
            w.request_animation_frame();
        }

        let phase = if reduced {
            0.25
        } else {
            (now.duration_since(self.started_at).as_secs_f32() * 0.75).fract()
        };

        let measured_w = self.page_width.get();
        let narrow = measured_w < 600.;
        let measure_cell = self.page_width.clone();
        let pad_x = if narrow {
            16.
        } else if measured_w < 860. {
            24.
        } else {
            32.
        };

        let input_surface: Hsla = if p.dark {
            rgb(0x1c1c1e).into()
        } else {
            rgb(0xffffff).into()
        };
        let input_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.14)
        } else {
            hsla(0., 0., 0., 0.09)
        };

        let body = if self.loading {
            self.render_loading_skeleton(narrow, phase, &p, cx)
        } else {
            // 1. Digest section
            let digest_on = self.values.digest_enabled;
            let weekly_on = self.values.weekly_report;
            let digest_time = self.values.digest_time.clone();
            let tz_sub = format!(
                "{}, {}",
                tr(cx, "In your time zone", "按你的时区发送"),
                self.values.timezone
            );

            let mut digest_card = div()
                .mt(px(12.))

                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .child(
                    div()
                        .min_h(px(60.))
                        .py(px(10.))
                        .pl(px(if narrow { 14. } else { 16. }))
                        .pr(px(if narrow { 14. } else { 12. }))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(24.))
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
                                        .child(tr(cx, "Daily digest", "每日摘要")),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .child(tr(
                                            cx,
                                            "Approvals waiting, runs that finished or failed, and mentions",
                                            "汇总待审批请求、已完成或失败的运行任务及提及",
                                        )),
                                ),
                        )
                        .child(pro_switch(
                            pro_eid(&self.id, "digest-sw"),
                            tr(cx, "Daily digest", "每日摘要"),
                            digest_on,
                            cx,
                            |this, _, cx| this.toggle_digest(cx),
                        )),
                );

            let digest_bounds = self.digest_bounds.clone();
            let time_trigger = self.render_combobox_trigger(
                "digest-time",
                tr(cx, "Digest time", "摘要发送时间").into(),
                render_time_glyph(digest_time.as_ref(), 16., p.muted),
                digest_time,
                112.,
                true,
                OpenMenu::DigestTime,
                &digest_bounds,
                narrow,
                input_surface,
                input_border,
                &p,
                cx,
            );
            let digest_send_row = div()
                .min_h(px(60.))
                .py(px(if narrow { 12. } else { 10. }))
                .pl(px(if narrow { 14. } else { 16. }))
                .pr(px(if narrow { 14. } else { 12. }))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .when(narrow, |d| d.flex_col().items_stretch().gap(px(10.)))
                .when(!narrow, |d| d.items_center().justify_between().gap(px(24.)))
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
                                .child(tr(cx, "Send at", "发送时间")),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(tz_sub),
                        ),
                )
                .child(time_trigger)
                .into_any_element();
            digest_card = digest_card.child(pro_fold(
                pro_eid(&self.id, "digest-time-fold"),
                digest_on,
                60.,
                0.,
                move || digest_send_row,
            ));

            digest_card = digest_card.child(
                div()
                    .min_h(px(60.))
                    .py(px(10.))
                    .pl(px(if narrow { 14. } else { 16. }))
                    .pr(px(if narrow { 14. } else { 12. }))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(24.))
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
                                    .child(tr(cx, "Weekly usage report", "每周用量报告")),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(16.))
                                    .text_color(p.muted)
                                    .child(tr(
                                        cx,
                                        "Credits spent, the busiest agents and the models used, every Monday",
                                        "每周一发送积分消耗、最活跃智能体及所用模型统计",
                                    )),
                            ),
                    )
                    .child(pro_switch(
                        pro_eid(&self.id, "weekly-report-sw"),
                        tr(cx, "Weekly usage report", "每周用量报告"),
                        weekly_on,
                        cx,
                        |this, _, cx| this.toggle_weekly_report(cx),
                    )),
            );

            // 2. Notification types section
            let is_types_empty = self.is_empty();
            let types_inner: AnyElement = if is_types_empty {
                div()
                    .mt(px(12.))
                    .rounded(px(12.))
                    .bg(if p.dark {
                        hsla(0., 0., 1., 0.03)
                    } else {
                        hsla(0., 0., 0., 0.02)
                    })
                    .border_1()
                    .border_color(p.line)
                    .child(pro_empty_state(
                        ProIcon::Bell,
                        tr(cx, "No notification types", "暂无通知类型"),
                        tr(
                            cx,
                            "Restore the default event list to choose where each notification is delivered.",
                            "恢复默认事件列表以配置每项通知的投递渠道。",
                        ),
                        Some((
                            pro_eid(&self.id, "restore-defaults-btn"),
                            tr(cx, "Restore defaults", "恢复默认").into(),
                            Box::new(|this: &mut Self, _, cx| this.reset_defaults(cx)),
                        )),
                        cx,
                    ))
                    .into_any_element()
            } else {
                let active_filter = self.channel_filter;
                let filter_sensor = pro_selection_sensor();
                let seg_track_bg = if p.dark {
                    hsla(0., 0., 1., 0.04)
                } else {
                    hsla(0., 0., 0., 0.035)
                };
                let seg_pill_bg: Hsla = if p.dark {
                    hsla(0., 0., 1., 0.11)
                } else {
                    rgb(0xffffff).into()
                };
                let focus_ring = p.focus;
                let hover_ink = p.ink;

                let mut filter_bar = div()
                    .id(pro_eid(&self.id, "notif-ch-filter-bar"))
                    .role(Role::TabList)
                    .relative()
                    .h(px(32.))
                    .p(px(3.))
                    .rounded(px(8.))
                    .bg(seg_track_bg)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .child(pro_pill_indicator(
                        pro_eid(&self.id, "notif-ch-filter-pill"),
                        active_filter.key(),
                        filter_sensor.clone(),
                        6.,
                        seg_pill_bg,
                        Some(p.line),
                        !p.dark,
                    ));

                for filt in ChannelFilter::ALL {
                    let selected = active_filter == filt;
                    let count = self
                        .values
                        .types
                        .iter()
                        .filter(|t| filt.matches(t.channel))
                        .count();
                    let label = format!("{} ({count})", filt.label_tr(cx));
                    filter_bar = filter_bar.child(activate(
                        div()
                            .id(pro_eid(&self.id, format!("notif-filt-{}", filt.key())))
                            .role(Role::Tab)
                            .aria_selected(selected)
                            .tab_index(0)
                            .relative()
                            .h_full()
                            .px(px(if narrow { 8. } else { 10. }))
                            .rounded(px(6.))
                            .flex()
                            .items_center()
                            .text_size(px(12.))
                            .font_weight(if selected {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(if selected { p.ink } else { p.muted })
                            .hover(move |s| s.text_color(hover_ink))
                            .focus_visible(move |s| s.border_1().border_color(focus_ring))
                            .cursor_pointer()
                            .child(label)
                            .when(selected, |d| d.child(pro_mark_selected(&filter_sensor))),
                        cx,
                        move |this, _, cx| {
                            if this.channel_filter != filt {
                                this.channel_filter = filt;
                                this.open_menu = None;
                                cx.notify();
                            }
                        },
                    ));
                }

                let all_open = self.open_menu == Some(OpenMenu::NotifyAll);
                let all_bounds = self.all_bounds.clone();
                let hover_bg = p.hover;
                let focus = p.focus;

                let all_btn = activate(
                    div()
                        .id(pro_eid(&self.id, "notify-all-btn"))
                        .role(Role::Button)
                        .aria_label(tr(cx, "Notify on – for every type", "统一设置全部通知渠道"))
                        .aria_expanded(all_open)
                        .tab_index(0)
                        .relative()
                        .when(!narrow, |d| d.w(px(176.)))
                        .h(px(28.))
                        .px(px(6.))
                        .mr(px(-6.))
                        .rounded(px(6.))
                        .bg(if all_open {
                            p.hover
                        } else {
                            transparent_black()
                        })
                        .text_size(px(12.))
                        .text_color(if all_open { p.ink } else { p.muted })
                        .hover(move |s| s.bg(hover_bg).text_color(hover_ink))
                        .focus_visible(move |s| s.border_1().border_color(focus))
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .cursor_pointer()
                        .child(tr(cx, "Notify on", "通知渠道"))
                        .child(pro_icon(
                            if all_open {
                                ProIcon::ChevronUp
                            } else {
                                ProIcon::ChevronDown
                            },
                            13.,
                            if all_open { p.ink } else { p.muted },
                        ))
                        .child(all_bounds.measure()),
                    cx,
                    |this, _, cx| this.toggle_menu(OpenMenu::NotifyAll, cx),
                );

                let thead = div()
                    .h(px(40.))
                    .pl(px(if narrow { 14. } else { 16. }))
                    .pr(px(if narrow { 14. } else { 12. }))
                    .flex()
                    .items_center()
                    .when(narrow, |d| d.justify_end())
                    .when(!narrow, |d| d.justify_between().gap(px(24.)))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .when(!narrow, |d| {
                        d.child(div().child(tr(cx, "Notify me about", "通知事件")))
                    })
                    .child(all_btn);

                let rows_snapshot = self.values.types.clone();
                let mut rows_box = div().w_full().flex().flex_col();
                let mut matched_count = 0usize;

                for (idx, item) in rows_snapshot.iter().enumerate() {
                    if !active_filter.matches(item.channel) {
                        continue;
                    }
                    matched_count += 1;
                    let (title_txt, note_txt) = localized_type_copy(item, cx);
                    let title_owned = title_txt.to_owned();
                    let note_owned = note_txt.to_owned();
                    let aria = format!("{} {}", tr(cx, "Where to send", "投递方式："), title_owned);
                    let ch = item.channel;
                    let bounds_clone = self.type_bounds[idx].clone();
                    let select_el = self.render_combobox_trigger(
                        format!("type-ch-{}", item.id),
                        aria.into(),
                        ch.render_icon(16., p.muted),
                        ch.label_tr(cx).into(),
                        176.,
                        false,
                        OpenMenu::TypeChannel(idx),
                        &bounds_clone,
                        narrow,
                        input_surface,
                        input_border,
                        &p,
                        cx,
                    );

                    rows_box = rows_box.child(
                        div()
                            .min_h(px(56.))
                            .py(px(if narrow { 12. } else { 10. }))
                            .pl(px(if narrow { 14. } else { 16. }))
                            .pr(px(if narrow { 14. } else { 12. }))
                            .border_t_1()
                            .border_color(p.line)
                            .flex()
                            .when(narrow, |d| d.flex_col().items_stretch().gap(px(10.)))
                            .when(!narrow, |d| d.items_center().justify_between().gap(px(24.)))
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
                                            .child(title_owned),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.))
                                            .line_height(px(16.))
                                            .text_color(p.muted)
                                            .child(note_owned),
                                    ),
                            )
                            .child(select_el),
                    );
                }

                if matched_count == 0 {
                    rows_box = rows_box.child(
                        div()
                            .py(px(28.))
                            .px(px(16.))
                            .border_t_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(6.))
                            .child(div().text_size(px(13.)).text_color(p.muted).child(tr(
                                cx,
                                "No notification types use this channel.",
                                "当前没有使用此渠道的通知类型。",
                            )))
                            .child(activate(
                                div()
                                    .id(pro_eid(&self.id, "notif-filter-reset"))
                                    .role(Role::Button)
                                    .tab_index(0)
                                    .px(px(10.))
                                    .h(px(26.))
                                    .rounded(px(6.))
                                    .bg(p.chip)
                                    .hover(move |s| s.bg(hover_bg))
                                    .focus_visible(move |s| s.border_1().border_color(focus))
                                    .flex()
                                    .items_center()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.ink)
                                    .cursor_pointer()
                                    .child(tr(cx, "Show all", "显示全部")),
                                cx,
                                |this, _, cx| {
                                    this.channel_filter = ChannelFilter::All;
                                    cx.notify();
                                },
                            )),
                    );
                }

                let types_card = div()
                    .mt(px(10.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .child(thead)
                    .child(pro_view_transition(
                        pro_eid(&self.id, "notif-channel-vtx"),
                        format!("{}-{}", active_filter.key(), matched_count == 0),
                        rows_box,
                    ));

                div()
                    .mt(px(12.))
                    .flex()
                    .flex_col()
                    .child(div().flex().items_center().child(filter_bar))
                    .child(types_card)
                    .into_any_element()
            };
            let types_content = pro_view_transition(
                pro_eid(&self.id, "notif-types-vtx"),
                if is_types_empty { "empty" } else { "populated" },
                types_inner,
            );

            // 3. Quiet hours section
            let quiet_on = self.values.quiet_hours.enabled;
            let quiet_from = self.values.quiet_hours.from.clone();
            let quiet_to = self.values.quiet_hours.to.clone();
            let quiet_tz_sub = format!(
                "{}, {}",
                tr(cx, "In your time zone", "按你的时区执行"),
                self.values.timezone
            );

            let mut quiet_card = div()
                .mt(px(12.))

                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .child(
                    div()
                        .min_h(px(60.))
                        .py(px(10.))
                        .pl(px(if narrow { 14. } else { 16. }))
                        .pr(px(if narrow { 14. } else { 12. }))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(24.))
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
                                        .child(tr(cx, "Hold pushes at night", "夜间暂缓推送")),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .child(tr(
                                            cx,
                                            "Mobile pushes wait until morning; emails are sent as usual",
                                            "移动端推送将延后至清晨发送，邮件照常投递",
                                        )),
                                ),
                        )
                        .child(pro_switch(
                            pro_eid(&self.id, "quiet-hours-sw"),
                            tr(cx, "Hold pushes at night", "夜间暂缓推送"),
                            quiet_on,
                            cx,
                            |this, _, cx| this.toggle_quiet_hours(cx),
                        )),
                );

            let from_bounds = self.quiet_from_bounds.clone();
            let to_bounds = self.quiet_to_bounds.clone();
            let from_trigger = self.render_combobox_trigger(
                "quiet-from",
                tr(cx, "Quiet hours start", "免打扰开始时间").into(),
                render_time_glyph(quiet_from.as_ref(), 16., p.muted),
                quiet_from,
                112.,
                true,
                OpenMenu::QuietFrom,
                &from_bounds,
                narrow,
                input_surface,
                input_border,
                &p,
                cx,
            );
            let to_trigger = self.render_combobox_trigger(
                "quiet-to",
                tr(cx, "Quiet hours end", "免打扰结束时间").into(),
                render_time_glyph(quiet_to.as_ref(), 16., p.muted),
                quiet_to,
                112.,
                true,
                OpenMenu::QuietTo,
                &to_bounds,
                narrow,
                input_surface,
                input_border,
                &p,
                cx,
            );

            let quiet_schedule_row = div()
                .min_h(px(60.))
                .py(px(if narrow { 12. } else { 10. }))
                .pl(px(if narrow { 14. } else { 16. }))
                .pr(px(if narrow { 14. } else { 12. }))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .when(narrow, |d| d.flex_col().items_stretch().gap(px(10.)))
                .when(!narrow, |d| d.items_center().justify_between().gap(px(24.)))
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
                                .child(tr(cx, "Schedule", "免打扰窗口")),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(quiet_tz_sub),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(div().when(narrow, |d| d.flex_1()).child(from_trigger))
                        .child(div().text_size(px(13.)).text_color(p.faint).child("–"))
                        .child(div().when(narrow, |d| d.flex_1()).child(to_trigger)),
                )
                .into_any_element();
            quiet_card = quiet_card.child(pro_fold(
                pro_eid(&self.id, "quiet-schedule-fold"),
                quiet_on,
                60.,
                0.,
                move || quiet_schedule_row,
            ));

            div()
                .w_full()
                .max_w(px(760.))
                .mx_auto()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(24.))
                        .line_height(px(32.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(tr(cx, "Notifications", "通知")),
                )
                .child(
                    div()
                        .mt(px(4.))
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.text)
                        .child(tr(
                            cx,
                            "Choose what you hear about and where. Everything still lands in your inbox.",
                            "选择你想接收的通知内容与投递方式。所有消息仍会保留在收件箱中。",
                        )),
                )
                .child(
                    div()
                        .mt(px(36.))
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(14.))
                                .line_height(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(tr(cx, "Digest", "邮件摘要")),
                        )
                        .child(
                            div()
                                .mt(px(2.))
                                .max_w(px(620.))
                                .text_size(px(13.))
                                .line_height(px(20.))
                                .text_color(p.muted)
                                .child(tr(
                                    cx,
                                    "One email instead of many – what happened while you were away.",
                                    "将多封通知合并为一封邮件——汇总你离开期间的动态。",
                                )),
                        )
                        .child(digest_card),
                )
                .child(
                    div()
                        .mt(px(36.))
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(14.))
                                .line_height(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(tr(cx, "Notification types", "通知类型")),
                        )
                        .child(
                            div()
                                .mt(px(2.))
                                .max_w(px(620.))
                                .text_size(px(13.))
                                .line_height(px(20.))
                                .text_color(p.muted)
                                .child(tr(
                                    cx,
                                    "Choose where each one is delivered.",
                                    "为每种事件选择投递渠道。",
                                )),
                        )
                        .child(types_content),
                )
                .child(
                    div()
                        .mt(px(36.))
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(14.))
                                .line_height(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(tr(cx, "Quiet hours", "免打扰时段")),
                        )
                        .child(quiet_card),
                )
        };

        let mut outer = div()
            .id(pro_eid(&self.id, "page-root"))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                let k = ev.keystroke.key.as_str();
                if let Some(which) = this.open_menu {
                    let len = Self::menu_len(which);
                    match k {
                        "escape" => {
                            this.open_menu = None;
                            w.prevent_default();
                            cx.stop_propagation();
                            cx.notify();
                        }
                        "down" | "j" if len > 0 => {
                            this.menu_highlight = (this.menu_highlight + 1) % len;
                            w.prevent_default();
                            cx.stop_propagation();
                            cx.notify();
                        }
                        "up" | "k" if len > 0 => {
                            this.menu_highlight = (this.menu_highlight + len - 1) % len;
                            w.prevent_default();
                            cx.stop_propagation();
                            cx.notify();
                        }
                        "enter" if len > 0 => {
                            let idx = this.menu_highlight;
                            this.confirm_menu_choice(which, idx, cx);
                            w.prevent_default();
                            cx.stop_propagation();
                        }
                        _ => {}
                    }
                }
            }))
            .relative()
            .size_full()
            .bg(p.panel)
            .overflow_hidden()
            .child(
                canvas(
                    move |bounds, _, _| {
                        let w_px = f32::from(bounds.size.width);
                        if w_px > 1. {
                            measure_cell.set(w_px);
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .size_full(),
            )
            .child(
                div()
                    .id(pro_eid(&self.id, "scroll"))
                    .size_full()
                    .overflow_y_scroll()
                    .pt(px(46.))
                    .pb(px(56.))
                    .px(px(pad_x))
                    .child(pro_view_transition(
                        pro_eid(&self.id, "notif-body-vtx"),
                        if self.loading { "loading" } else { "ready" },
                        body,
                    )),
            )
            .child(pro_toast_banner(self.toast.clone()));

        if let Some(which) = self.open_menu.or(self.last_menu)
            && !self.loading
        {
            outer =
                outer.child(self.render_open_dropdown(which, self.open_menu.is_some(), w, &p, cx));
        }

        outer
    }
}

impl Render for SettingsNotificationsScreen {
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
        NotificationChannel, NotificationValues, SettingsNotificationsEvent,
        SettingsNotificationsScreen,
    };
    use gpui::{AppContext as _, TestAppContext};
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn default_notification_matrix_and_bulk_override() {
        let mut vals = NotificationValues::default();
        assert!(vals.digest_enabled);
        assert_eq!(vals.digest_time.as_ref(), "08:00");
        assert!(!vals.weekly_report);
        assert!(!vals.quiet_hours.enabled);
        assert_eq!(vals.types.len(), 8);
        assert_eq!(
            vals.channel_for("mentions"),
            Some(NotificationChannel::Both)
        );
        assert_eq!(
            vals.channel_for("finished-runs"),
            Some(NotificationChannel::Mobile)
        );
        assert_eq!(vals.channel_for("shared"), Some(NotificationChannel::Email));

        assert!(vals.set_type_channel("finished-runs", NotificationChannel::Inbox));
        assert!(!vals.set_type_channel("finished-runs", NotificationChannel::Inbox));
        assert_eq!(
            vals.channel_for("finished-runs"),
            Some(NotificationChannel::Inbox)
        );

        assert!(vals.set_all_channels(NotificationChannel::Email));
        assert!(
            vals.types
                .iter()
                .all(|t| t.channel == NotificationChannel::Email)
        );
        assert!(!vals.set_all_channels(NotificationChannel::Email));
    }

    #[test]
    fn screen_switches_channels_and_quiet_window_emit_save() {
        let app = TestAppContext::single();
        app.update(crate::init);

        let events: Rc<RefCell<Vec<SettingsNotificationsEvent>>> =
            Rc::new(RefCell::new(Vec::new()));
        let recorded = events.clone();

        let screen = app.update(|cx| cx.new(|cx| SettingsNotificationsScreen::new("test-sn", cx)));

        app.update(|cx| {
            cx.subscribe(&screen, move |_, ev: &SettingsNotificationsEvent, _| {
                recorded.borrow_mut().push(ev.clone());
            })
            .detach();
            screen.update(cx, |s, s_cx| {
                s.set_digest_time("09:00", s_cx);
                assert_eq!(s.notifications().digest_time.as_ref(), "09:00");

                s.toggle_weekly_report(s_cx);
                assert!(s.notifications().weekly_report);

                assert!(s.set_channel("approvals", NotificationChannel::Mobile, s_cx));
                assert_eq!(
                    s.notifications().channel_for("approvals"),
                    Some(NotificationChannel::Mobile)
                );

                s.toggle_quiet_hours(s_cx);
                assert!(s.notifications().quiet_hours.enabled);
                s.set_quiet_window("23:00", "07:00", s_cx);
                assert_eq!(s.notifications().quiet_hours.from.as_ref(), "23:00");
                assert_eq!(s.notifications().quiet_hours.to.as_ref(), "07:00");

                s.set_all_channels(NotificationChannel::Inbox, s_cx);
                assert!(
                    s.notifications()
                        .types
                        .iter()
                        .all(|t| t.channel == NotificationChannel::Inbox)
                );
            });
        });

        assert_eq!(events.borrow().len(), 6);
    }
}
