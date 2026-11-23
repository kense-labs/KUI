//! Personal privacy settings screen with data controls, browser cookies, sessions, and shared chats.
//!
//! Renders inside [`AppShell`](crate::components::app_shell::AppShell) with the `Settings`
//! sidebar preset (`Privacy` active under `Personal`), providing ad-measurement, analytics,
//! and model-training capsule switches, an expandable browser-cookie drawer, product email
//! subscription, active device session management, workspace/public shared-chat link revocation,
//! and a danger zone for ZIP data export or deleting all chats with confirmation.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_btn, pro_eid, pro_empty_state, pro_fold,
            pro_mark_selected, pro_pill_indicator, pro_presence, pro_selection_sensor,
            pro_skeleton_bar, pro_spring_bar, pro_switch, pro_toast_banner, pro_view_transition,
            tr,
        },
    },
    foundation::Theme,
    primitives::{
        clipboard,
        control::activate,
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const SIGN_OUT_SVG: &str = r#"<path d="M18 18C18 18.4644 18 18.6965 17.978 18.8918C17.7952 20.5145 16.5145 21.7952 14.8919 21.978C14.6965 22 14.4644 22 14 22H11C7.70017 22 6.05025 22 5.02513 20.9749C4 19.9498 4 18.2998 4 15L4.00001 8.99998C4.00001 5.70016 4.00003 4.05024 5.02514 3.02512C6.05027 2 7.70018 2 11 2H14C14.4644 2 14.6965 2 14.8919 2.02201C16.5145 2.20483 17.7952 3.48547 17.978 5.10816C18 5.30347 18 5.53565 18 6" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M8.07612 11.1183C8 11.3021 8 11.535 8 12.001C8 12.4669 8 12.6999 8.07612 12.8837C8.17761 13.1287 8.37229 13.3234 8.61732 13.4249C8.80109 13.501 9.03406 13.501 9.5 13.501H14.5C14.5002 15.2503 14.511 16.1299 15.0623 16.3858C15.0829 16.3954 15.1037 16.4042 15.1249 16.4124C15.7045 16.6353 16.3999 16.0139 17.7907 14.7711C19.2576 13.4602 19.9912 12.7851 20 11.957C19.9912 11.1289 19.2576 10.4538 17.7907 9.14301C16.3999 7.90018 15.7045 7.27876 15.1249 7.50171C15.1037 7.50985 15.0829 7.51869 15.0623 7.52822C14.5018 7.78844 14.5 8.69318 14.5 10.501H9.5C9.03406 10.501 8.80109 10.501 8.61732 10.5771C8.37229 10.6786 8.17761 10.8733 8.07612 11.1183Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;

const LAPTOP_SVG: &str = r#"<path d="M20.4999 16.5V8.5C20.4999 6.14298 20.4999 4.96447 19.7676 4.23223C19.0354 3.5 17.8569 3.5 15.4999 3.5H8.49988C6.14286 3.5 4.96434 3.5 4.23211 4.23223C3.49988 4.96447 3.49988 6.14298 3.49988 8.5V16.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M21.9841 20.5H2.01567C1.63273 20.5 1.38367 20.1088 1.55493 19.7764L3.49988 16.5H20.4999L22.4448 19.7764C22.6161 20.1088 22.367 20.5 21.9841 20.5Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;

const MOBILE_SVG: &str = r#"<path d="M13.5 2H10.5C8.14298 2 6.96447 2 6.23223 2.73223C5.5 3.46447 5.5 4.64298 5.5 7V17C5.5 19.357 5.5 20.5355 6.23223 21.2678C6.96447 22 8.14298 22 10.5 22H13.5C15.857 22 17.0355 22 17.7678 21.2678C18.5 20.5355 18.5 19.357 18.5 17V7C18.5 4.64298 18.5 3.46447 17.7678 2.73223C17.0355 2 15.857 2 13.5 2Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M12.125 19H12M12.25 19C12.25 19.1381 12.1381 19.25 12 19.25C11.8619 19.25 11.75 19.1381 11.75 19C11.75 18.8619 11.8619 18.75 12 18.75C12.1381 18.75 12.25 18.8619 12.25 19Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;

const DESKTOP_SVG: &str = r#"<path d="M14 21H16M14 21C13.1716 21 12.5 20.3284 12.5 19.5V17L12 17M14 21H10M10 21H8M10 21C10.8284 21 11.5 20.3284 11.5 19.5V17L12 17M12 17V21" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M16 3H8C5.17157 3 3.75736 3 2.87868 3.87868C2 4.75736 2 6.17157 2 9V11C2 13.8284 2 15.2426 2.87868 16.1213C3.75736 17 5.17157 17 8 17H16C18.8284 17 20.2426 17 21.1213 16.1213C22 15.2426 22 13.8284 22 11V9C22 6.17157 22 4.75736 21.1213 3.87868C20.2426 3 18.8284 3 16 3Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;

const CHAT_DOTS_SVG: &str = r#"<path d="M21.5 12C21.5 17.2467 17.2467 21.5 12 21.5C10.3719 21.5 8.8394 21.0904 7.5 20.3687C5.63177 19.362 4.37462 20.2979 3.26592 20.4658C3.09774 20.4913 2.93024 20.4302 2.80997 20.31C2.62741 20.1274 2.59266 19.8451 2.6935 19.6074C3.12865 18.5818 3.5282 16.6382 2.98341 15C2.6698 14.057 2.5 13.0483 2.5 12C2.5 6.75329 6.75329 2.5 12 2.5C17.2467 2.5 21.5 6.75329 21.5 12Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M12.1257 12H12.0007M8.125 12H8M16.125 12H16M12.2507 12C12.2507 12.1381 12.1388 12.25 12.0007 12.25C11.8627 12.25 11.7507 12.1381 11.7507 12C11.7507 11.8619 11.8627 11.75 12.0007 11.75C12.1388 11.75 12.2507 11.8619 12.2507 12ZM8.25 12C8.25 12.1381 8.13807 12.25 8 12.25C7.86193 12.25 7.75 12.1381 7.75 12C7.75 11.8619 7.86193 11.75 8 11.75C8.13807 11.75 8.25 11.8619 8.25 12ZM16.25 12C16.25 12.1381 16.1381 12.25 16 12.25C15.8619 12.25 15.75 12.1381 15.75 12C15.75 11.8619 15.8619 11.75 16 11.75C16.1381 11.75 16.25 11.8619 16.25 12Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;

const CHECK_SVG: &str = r#"<path d="M5 14L8.5 17.5L19 6.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2.1"/>"#;

/// Form factor icon for an active sign-in session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SessionDevice {
    /// Laptop computer (`Chrome on macOS`).
    #[default]
    Laptop,
    /// Smartphone (`Safari on iPhone`).
    Mobile,
    /// Desktop workstation (`Desktop app on Windows`).
    Desktop,
}

impl SessionDevice {
    fn render_icon(self, side: f32, color: Hsla) -> AnyElement {
        match self {
            Self::Laptop => pro_svg("0 0 24 24", LAPTOP_SVG, side, color),
            Self::Mobile => pro_svg("0 0 24 24", MOBILE_SVG, side, color),
            Self::Desktop => pro_svg("0 0 24 24", DESKTOP_SVG, side, color),
        }
    }
}

/// Active account session entry listed under `Active sessions`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivacySession {
    /// Stable session identifier.
    pub id: SharedString,
    /// Device form-factor glyph.
    pub device: SessionDevice,
    /// Browser and OS summary (e.g. `"Chrome on macOS"`).
    pub name: SharedString,
    /// City, auth provider, IP, or last-active metadata.
    pub meta: SharedString,
    /// Whether this session represents the current browser/device (`"This device"`).
    pub current: bool,
}

impl PrivacySession {
    /// Construct a [`PrivacySession`] record.
    pub fn new(
        id: impl Into<SharedString>,
        device: SessionDevice,
        name: impl Into<SharedString>,
        meta: impl Into<SharedString>,
        current: bool,
    ) -> Self {
        Self {
            id: id.into(),
            device,
            name: name.into(),
            meta: meta.into(),
            current,
        }
    }
}

/// Audience scope of a chat shared by link.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SharedChatScope {
    /// Accessible only to signed-in workspace members (`"In the workspace"`).
    #[default]
    Workspace,
    /// Accessible to anyone with the link (`"Public links"`).
    Public,
}

impl SharedChatScope {
    /// Canonical English tab label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Workspace => "In the workspace",
            Self::Public => "Public links",
        }
    }

    /// Localized tab label matching the active UI language.
    pub fn label_tr(self, cx: &App) -> &'static str {
        match self {
            Self::Workspace => tr(cx, "In the workspace", "工作区内"),
            Self::Public => tr(cx, "Public links", "公开链接"),
        }
    }
}

/// One chat conversation shared by link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedChatLink {
    /// Stable share identifier.
    pub id: SharedString,
    /// Conversation title.
    pub title: SharedString,
    /// Visibility scope (`Workspace` or `Public`).
    pub scope: SharedChatScope,
    /// Relative share timestamp (e.g. `"Shared 2 days ago"`).
    pub shared: SharedString,
    /// Total link views.
    pub views: u32,
    /// Full share URL copied by `"Copy link"`.
    pub url: SharedString,
}

impl SharedChatLink {
    /// Construct a [`SharedChatLink`] item.
    pub fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        scope: SharedChatScope,
        shared: impl Into<SharedString>,
        views: u32,
        url: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            scope,
            shared: shared.into(),
            views,
            url: url.into(),
        }
    }
}

/// Optional browser cookie categories stored on the current device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserCookiePrefs {
    /// Functional preferences (theme, sidebar width, local layout).
    pub functional: bool,
    /// Crash diagnostics and page-timing cookies.
    pub performance: bool,
}

impl Default for BrowserCookiePrefs {
    fn default() -> Self {
        Self {
            functional: true,
            performance: false,
        }
    }
}

/// Complete personal privacy settings managed by [`SettingsPrivacyScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivacyValues {
    /// Share hashed email and sign-up events with ad measurement partners (`false` by default).
    pub ad_measurement: bool,
    /// First-party cookieless feature usage counts (`true` by default).
    pub product_analytics: bool,
    /// Allow anonymised personal chats to help improve future models (`false` by default).
    pub model_training: bool,
    /// Per-device browser cookie preferences.
    pub cookies: BrowserCookiePrefs,
    /// Subscribed to occasional monthly product update emails (`false` by default).
    pub product_emails: bool,
    /// Active sessions signed in to this account.
    pub sessions: Vec<PrivacySession>,
    /// Conversations shared by workspace or public link.
    pub shared_chats: Vec<SharedChatLink>,
}

impl Default for PrivacyValues {
    fn default() -> Self {
        Self {
            ad_measurement: false,
            product_analytics: true,
            model_training: false,
            cookies: BrowserCookiePrefs::default(),
            product_emails: false,
            sessions: Self::default_sessions(),
            shared_chats: Self::default_shared_chats(),
        }
    }
}

impl PrivacyValues {
    /// Return the 3 canonical sample device sessions.
    pub fn default_sessions() -> Vec<PrivacySession> {
        vec![
            PrivacySession::new(
                "chrome-macos",
                SessionDevice::Laptop,
                "Chrome on macOS",
                "Vienna, Austria · Signed in with Google · 82.214.130.34",
                true,
            ),
            PrivacySession::new(
                "safari-iphone",
                SessionDevice::Mobile,
                "Safari on iPhone",
                "Vienna, Austria · Active 3 hours ago",
                false,
            ),
            PrivacySession::new(
                "desktop-windows",
                SessionDevice::Desktop,
                "Desktop app on Windows",
                "Berlin, Germany · Active 6 days ago",
                false,
            ),
        ]
    }

    /// Return the canonical sample shared chat links (`2` workspace, `1` public).
    pub fn default_shared_chats() -> Vec<SharedChatLink> {
        vec![
            SharedChatLink::new(
                "q4-pricing",
                "Q4 pricing review with Finance",
                SharedChatScope::Workspace,
                "Shared 2 days ago",
                14,
                "https://app.workspace.ai/share/q4-pricing",
            ),
            SharedChatLink::new(
                "onboarding-checklist",
                "Onboarding checklist for new hires",
                SharedChatScope::Workspace,
                "Shared last week",
                31,
                "https://app.workspace.ai/share/onboarding-checklist",
            ),
            SharedChatLink::new(
                "prompt-eval",
                "Prompt evaluation benchmark notes",
                SharedChatScope::Public,
                "Shared 5 days ago",
                89,
                "https://app.workspace.ai/p/prompt-eval",
            ),
        ]
    }

    /// Count active sessions other than the current device.
    pub fn other_session_count(&self) -> usize {
        self.sessions.iter().filter(|s| !s.current).count()
    }

    /// Count shared chats belonging to `scope`.
    pub fn shared_count(&self, scope: SharedChatScope) -> usize {
        self.shared_chats
            .iter()
            .filter(|c| c.scope == scope)
            .count()
    }

    /// Sign out a single non-current session by `id`; returns `false` if not found or if `current`.
    pub fn sign_out_session(&mut self, id: &str) -> bool {
        if let Some(pos) = self
            .sessions
            .iter()
            .position(|s| s.id.as_ref() == id && !s.current)
        {
            self.sessions.remove(pos);
            true
        } else {
            false
        }
    }

    /// Sign out every session except the current device; returns how many sessions were revoked.
    pub fn sign_out_other_sessions(&mut self) -> usize {
        let before = self.sessions.len();
        self.sessions.retain(|s| s.current);
        before.saturating_sub(self.sessions.len())
    }

    /// Stop sharing a chat link by `id`; returns `true` when removed.
    pub fn stop_sharing_chat(&mut self, id: &str) -> bool {
        if let Some(pos) = self.shared_chats.iter().position(|c| c.id.as_ref() == id) {
            self.shared_chats.remove(pos);
            true
        } else {
            false
        }
    }
}

/// Account identity displayed at the bottom of the settings sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsPrivacyUser {
    /// Full display name of the signed-in user.
    pub name: SharedString,
    /// Email address shown in the account menu.
    pub email: SharedString,
}

impl Default for SettingsPrivacyUser {
    fn default() -> Self {
        Self {
            name: "Alex Morgan".into(),
            email: "alex@acme.com".into(),
        }
    }
}

impl SettingsPrivacyUser {
    /// Construct a sidebar user summary with `name` and `email`.
    pub fn new(name: impl Into<SharedString>, email: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
        }
    }
}

/// Events emitted by [`SettingsPrivacyScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsPrivacyEvent {
    /// Emitted with the full [`PrivacyValues`] whenever any switch, session, or shared link changes.
    Save(PrivacyValues),
    /// Emitted when an archive export of chats and files is requested.
    Export,
    /// Emitted once deleting every chat has been confirmed.
    DeleteChats,
    /// Emitted when the inline `"Privacy policy"` link is activated.
    OpenPrivacyPolicy,
    /// Emitted when a settings sidebar item, `"Open chats"`, or `"Back to app"` is activated.
    Navigate(SharedString),
}

impl EventEmitter<SettingsPrivacyEvent> for SettingsPrivacyScreen {}

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

/// Personal privacy settings screen (`settings-privacy`).
pub struct SettingsPrivacyScreen {
    id: ElementId,
    focus: FocusHandle,
    shell: Entity<AppShell>,
    standalone: bool,
    loading: bool,
    empty: bool,
    workspace: SharedString,
    user: SettingsPrivacyUser,
    values: PrivacyValues,
    cookies_open: bool,
    active_scope: SharedChatScope,
    hovered_chat: Option<SharedString>,
    copied_chat: Option<(SharedString, Instant)>,
    export_banner_open: bool,
    delete_confirm_open: bool,
    toast: Option<(SharedString, Instant)>,
    started_at: Instant,
    page_width: Rc<Cell<f32>>,
}

impl Focusable for SettingsPrivacyScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl SettingsPrivacyScreen {
    /// Construct a new [`SettingsPrivacyScreen`] with canonical sample privacy settings.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let focus = cx.focus_handle();
        let values = PrivacyValues::default();
        let workspace: SharedString = "Acme".into();
        let user = SettingsPrivacyUser::default();

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
                .with_active("Privacy")
                .with_work_area(false, false);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s
        });
        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                cx.emit(SettingsPrivacyEvent::Navigate(label.clone()));
            }
            AppShellEvent::Back => {
                cx.emit(SettingsPrivacyEvent::Navigate("Back to app".into()));
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
            cookies_open: false,
            active_scope: SharedChatScope::Workspace,
            hovered_chat: None,
            copied_chat: None,
            export_banner_open: false,
            delete_confirm_open: false,
            toast: None,
            started_at: Instant::now(),
            page_width: Rc::new(Cell::new(720.)),
        }
    }

    /// Override the initial [`PrivacyValues`].
    pub fn with_privacy(mut self, privacy: PrivacyValues) -> Self {
        self.values = privacy;
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
    pub fn with_user(mut self, user: SettingsPrivacyUser, cx: &mut Context<Self>) -> Self {
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
    pub fn user(&self) -> &SettingsPrivacyUser {
        &self.user
    }

    /// Start in skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Start in empty state (only this device signed in and no shared chats).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.apply_empty(empty);
        self
    }

    /// Render only the page column without the outer [`AppShell`] frame.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Return the current [`PrivacyValues`].
    pub fn privacy(&self) -> &PrivacyValues {
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
            cx.notify();
        }
    }

    /// Return whether the empty state is active.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// Toggle the empty state (only current device signed in, no shared chats) at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.apply_empty(empty);
        cx.notify();
    }

    fn apply_empty(&mut self, empty: bool) {
        self.empty = empty;
        if empty {
            self.values.sessions.retain(|s| s.current);
            self.values.shared_chats.clear();
        } else {
            if self.values.sessions.len() <= 1 {
                self.values.sessions = PrivacyValues::default_sessions();
            }
            if self.values.shared_chats.is_empty() {
                self.values.shared_chats = PrivacyValues::default_shared_chats();
            }
        }
    }

    /// Return the active shared-chat filter tab (`Workspace` or `Public`).
    pub fn active_scope(&self) -> SharedChatScope {
        self.active_scope
    }

    /// Switch the shared-chat filter tab.
    pub fn set_active_scope(&mut self, scope: SharedChatScope, cx: &mut Context<Self>) {
        if self.active_scope != scope {
            self.active_scope = scope;
            cx.notify();
        }
    }

    /// Return whether the browser-cookie choices drawer is expanded.
    pub fn is_cookies_open(&self) -> bool {
        self.cookies_open
    }

    /// Expand or collapse the `"Manage cookies"` drawer.
    pub fn toggle_cookies_open(&mut self, cx: &mut Context<Self>) {
        self.cookies_open = !self.cookies_open;
        cx.notify();
    }

    /// Toggle ad measurement and emit [`SettingsPrivacyEvent::Save`].
    pub fn toggle_ad_measurement(&mut self, cx: &mut Context<Self>) {
        self.values.ad_measurement = !self.values.ad_measurement;
        let msg = if self.values.ad_measurement {
            tr(cx, "Ad measurement enabled", "已开启广告归因测量")
        } else {
            tr(cx, "Ad measurement disabled", "已关闭广告归因测量")
        };
        self.emit_saved(msg, cx);
    }

    /// Toggle cookieless product analytics and emit [`SettingsPrivacyEvent::Save`].
    pub fn toggle_product_analytics(&mut self, cx: &mut Context<Self>) {
        self.values.product_analytics = !self.values.product_analytics;
        let msg = if self.values.product_analytics {
            tr(cx, "Product analytics enabled", "已开启产品使用统计")
        } else {
            tr(cx, "Product analytics disabled", "已关闭产品使用统计")
        };
        self.emit_saved(msg, cx);
    }

    /// Toggle model training on personal chats and emit [`SettingsPrivacyEvent::Save`].
    pub fn toggle_model_training(&mut self, cx: &mut Context<Self>) {
        self.values.model_training = !self.values.model_training;
        let msg = if self.values.model_training {
            tr(cx, "Model training enabled", "已允许脱敏对话用于模型改进")
        } else {
            tr(cx, "Model training disabled", "已禁止使用对话训练模型")
        };
        self.emit_saved(msg, cx);
    }

    /// Toggle functional browser cookies and emit [`SettingsPrivacyEvent::Save`].
    pub fn toggle_functional_cookies(&mut self, cx: &mut Context<Self>) {
        self.values.cookies.functional = !self.values.cookies.functional;
        self.emit_saved(tr(cx, "Cookie preferences saved", "Cookie 偏好已保存"), cx);
    }

    /// Toggle performance browser cookies and emit [`SettingsPrivacyEvent::Save`].
    pub fn toggle_performance_cookies(&mut self, cx: &mut Context<Self>) {
        self.values.cookies.performance = !self.values.cookies.performance;
        self.emit_saved(tr(cx, "Cookie preferences saved", "Cookie 偏好已保存"), cx);
    }

    /// Subscribe or unsubscribe from monthly product update emails and emit [`SettingsPrivacyEvent::Save`].
    pub fn toggle_product_emails(&mut self, cx: &mut Context<Self>) {
        self.values.product_emails = !self.values.product_emails;
        let msg = if self.values.product_emails {
            tr(cx, "Subscribed to product updates", "已订阅产品更新邮件")
        } else {
            tr(
                cx,
                "Unsubscribed from product updates",
                "已退订产品更新邮件",
            )
        };
        self.emit_saved(msg, cx);
    }

    /// Sign out a single non-current session and emit [`SettingsPrivacyEvent::Save`].
    pub fn sign_out_session(&mut self, id: &str, cx: &mut Context<Self>) -> bool {
        if self.values.sign_out_session(id) {
            self.emit_saved(tr(cx, "Session signed out", "已退出该设备登录"), cx);
            true
        } else {
            false
        }
    }

    /// Sign out every device except the current one and emit [`SettingsPrivacyEvent::Save`].
    pub fn sign_out_others(&mut self, cx: &mut Context<Self>) -> usize {
        let count = self.values.sign_out_other_sessions();
        if count > 0 {
            self.emit_saved(tr(cx, "Signed out other devices", "已退出其他所有设备"), cx);
        }
        count
    }

    /// Copy a shared chat link to the clipboard and show brief feedback.
    pub fn copy_shared_link(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(url) = self
            .values
            .shared_chats
            .iter()
            .find(|c| c.id.as_ref() == id)
            .map(|c| c.url.clone())
        {
            clipboard::write(url.as_ref(), cx).detach();
            self.copied_chat = Some((id.to_owned().into(), Instant::now()));
            self.show_toast(tr(cx, "Link copied to clipboard", "链接已复制到剪贴板"), cx);
        }
    }

    /// Revoke a shared chat link immediately and emit [`SettingsPrivacyEvent::Save`].
    pub fn stop_sharing_chat(&mut self, id: &str, cx: &mut Context<Self>) -> bool {
        if self.values.stop_sharing_chat(id) {
            self.emit_saved(tr(cx, "Shared link revoked", "已停止共享并使链接失效"), cx);
            true
        } else {
            false
        }
    }

    /// Request a ZIP archive export of chats and files, emitting [`SettingsPrivacyEvent::Export`].
    pub fn request_export(&mut self, cx: &mut Context<Self>) {
        self.export_banner_open = true;
        self.show_toast(
            tr(
                cx,
                "Preparing export archive – we'll email your download link",
                "正在打包导出归档，完成后将发送下载链接至你的邮箱",
            ),
            cx,
        );
        cx.emit(SettingsPrivacyEvent::Export);
    }

    /// Return whether the delete-all-chats confirmation step is open.
    pub fn is_delete_confirm_open(&self) -> bool {
        self.delete_confirm_open
    }

    /// Open the inline confirmation row before deleting every chat.
    pub fn open_delete_confirm(&mut self, cx: &mut Context<Self>) {
        self.delete_confirm_open = true;
        cx.notify();
    }

    /// Cancel deleting every chat.
    pub fn cancel_delete_confirm(&mut self, cx: &mut Context<Self>) {
        self.delete_confirm_open = false;
        cx.notify();
    }

    /// Confirm deleting every chat (clearing shared links and emitting [`SettingsPrivacyEvent::DeleteChats`]).
    pub fn confirm_delete_chats(&mut self, cx: &mut Context<Self>) {
        self.delete_confirm_open = false;
        self.values.shared_chats.clear();
        self.show_toast(tr(cx, "All chats deleted", "已删除全部对话记录"), cx);
        cx.emit(SettingsPrivacyEvent::DeleteChats);
        cx.emit(SettingsPrivacyEvent::Save(self.values.clone()));
    }

    fn show_toast(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast = Some((msg.into(), Instant::now()));
        cx.notify();
    }

    fn emit_saved(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast = Some((msg.into(), Instant::now()));
        cx.emit(SettingsPrivacyEvent::Save(self.values.clone()));
        cx.notify();
    }

    fn render_loading_skeleton(&self, phase: f32, p: &ProPalette, cx: &App) -> Div {
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
                    .child(tr(cx, "Privacy", "隐私")),
            )
            .child(
                div()
                    .mt(px(4.))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.text)
                    .child(tr(
                        cx,
                        "Decide what you share and where you're signed in.",
                        "管理你共享的数据与已登录的设备。",
                    )),
            )
            // Data and cookies skeleton
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
                            .child(tr(cx, "Data and cookies", "数据与 Cookie")),
                    )
                    .child(
                        div()
                            .mt(px(2.))
                            .text_size(px(13.))
                            .line_height(px(20.))
                            .text_color(p.muted)
                            .child(tr(
                                cx,
                                "Changes apply on all your devices. Your chats and files are never shared with ad partners.",
                                "更改将同步到你的所有设备。你的对话与文件绝不会共享给广告合作方。",
                            )),
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
                            .children((0..4usize).map(|idx| {
                                let p_off = phase + idx as f32 * 0.08;
                                div()
                                    .min_h(px(60.))
                                    .py(px(10.))
                                    .pl(px(16.))
                                    .pr(px(12.))
                                    .when(idx > 0, |d| d.border_t_1().border_color(p.line))
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
                                                124. + idx as f32 * 18.,
                                                12.,
                                                4.,
                                                p_off,
                                                cx,
                                            ))
                                            .child(pro_skeleton_bar(
                                                236. - idx as f32 * 14.,
                                                10.,
                                                4.,
                                                p_off + 0.03,
                                                cx,
                                            )),
                                    )
                                    .child(if idx == 3 {
                                        pro_skeleton_bar(98., 28., 6., p_off + 0.05, cx)
                                    } else {
                                        pro_skeleton_bar(34., 20., 10., p_off + 0.05, cx)
                                    })
                            })),
                    ),
            )
            // Emails skeleton
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
                            .child(tr(cx, "Emails", "邮件订阅")),
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
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(pro_skeleton_bar(112., 12., 4., phase + 0.34, cx))
                                    .child(pro_skeleton_bar(248., 10., 4., phase + 0.38, cx)),
                            )
                            .child(pro_skeleton_bar(76., 28., 6., phase + 0.42, cx)),
                    ),
            )
            // Active sessions skeleton
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
                            .child(tr(cx, "Active sessions", "活跃会话")),
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
                            .children((0..3usize).map(|idx| {
                                let p_off = phase + 0.46 + idx as f32 * 0.07;
                                div()
                                    .min_h(px(60.))
                                    .py(px(10.))
                                    .pl(px(16.))
                                    .pr(px(12.))
                                    .when(idx > 0, |d| d.border_t_1().border_color(p.line))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(10.))
                                            .child(pro_skeleton_bar(32., 32., 9., p_off, cx))
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(6.))
                                                    .child(pro_skeleton_bar(
                                                        132.,
                                                        12.,
                                                        4.,
                                                        p_off + 0.02,
                                                        cx,
                                                    ))
                                                    .child(pro_skeleton_bar(
                                                        206.,
                                                        10.,
                                                        4.,
                                                        p_off + 0.04,
                                                        cx,
                                                    )),
                                            ),
                                    )
                                    .child(if idx == 0 {
                                        pro_skeleton_bar(66., 20., 10., p_off + 0.06, cx)
                                    } else {
                                        pro_skeleton_bar(64., 28., 6., p_off + 0.06, cx)
                                    })
                            })),
                    ),
            )
            // Shared chats skeleton (counts hidden while loading per spec)
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
                            .child(tr(cx, "Shared chats", "已共享对话")),
                    )
                    .child(
                        div()
                            .mt(px(12.))
                            .child(pro_skeleton_bar(214., 28., 8., phase + 0.7, cx)),
                    )
                    .child(
                        div()
                            .mt(px(8.))
                            .rounded(px(12.))
                            .bg(p.card)
                            .border_1()
                            .border_color(p.line)
                            .flex()
                            .flex_col()
                            .children((0..2usize).map(|idx| {
                                let p_off = phase + 0.74 + idx as f32 * 0.08;
                                div()
                                    .min_h(px(60.))
                                    .py(px(10.))
                                    .pl(px(16.))
                                    .pr(px(12.))
                                    .when(idx > 0, |d| d.border_t_1().border_color(p.line))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(10.))
                                            .child(pro_skeleton_bar(28., 28., 8., p_off, cx))
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(6.))
                                                    .child(pro_skeleton_bar(
                                                        176.,
                                                        12.,
                                                        4.,
                                                        p_off + 0.02,
                                                        cx,
                                                    ))
                                                    .child(pro_skeleton_bar(
                                                        132.,
                                                        10.,
                                                        4.,
                                                        p_off + 0.04,
                                                        cx,
                                                    )),
                                            ),
                                    )
                                    .child(pro_skeleton_bar(82., 28., 6., p_off + 0.06, cx))
                            })),
                    ),
            )
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let now = Instant::now();

        if let Some((_, ts)) = &self.toast
            && now.duration_since(*ts).as_secs_f32() > 2.45
        {
            self.toast = None;
        }
        if let Some((_, ts)) = &self.copied_chat
            && now.duration_since(*ts).as_secs_f32() > 1.8
        {
            self.copied_chat = None;
        }

        if (self.loading || self.toast.is_some() || self.copied_chat.is_some())
            && !reduced
            && w.is_visible()
        {
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

        let tile_bg: Hsla = if p.dark {
            rgb(0x2a2a2e).into()
        } else {
            rgb(0xffffff).into()
        };
        let frame_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.03)
        } else {
            hsla(0., 0., 0., 0.02)
        };

        let body = if self.loading {
            self.render_loading_skeleton(phase, &p, cx)
        } else {
            // 1. Data and cookies card
            let ad_on = self.values.ad_measurement;
            let analytics_on = self.values.product_analytics;
            let train_on = self.values.model_training;
            let cookies_open = self.cookies_open;
            let func_cookies = self.values.cookies.functional;
            let perf_cookies = self.values.cookies.performance;

            let policy_link = activate(
                div()
                    .id(pro_eid(&self.id, "privacy-policy-link"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Privacy policy", "隐私政策"))
                    .tab_index(0)
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .border_b_1()
                    .border_color(p.line_strong)
                    .hover(move |s| s.border_color(p.ink))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .text_size(px(12.))
                    .line_height(px(16.))
                    .text_color(p.ink)
                    .cursor_pointer()
                    .child(tr(cx, "Privacy policy", "隐私政策"))
                    .child(pro_icon(ProIcon::ExternalArrow, 11., p.ink)),
                cx,
                |this, _, cx| {
                    this.show_toast(tr(cx, "Opening privacy policy…", "正在打开隐私政策…"), cx);
                    cx.emit(SettingsPrivacyEvent::OpenPrivacyPolicy);
                },
            );

            let manage_cookies_btn = activate(
                div()
                    .id(pro_eid(&self.id, "manage-cookies-btn"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Manage cookies", "管理 Cookie"))
                    .aria_expanded(cookies_open)
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(10.))
                    .when(narrow, |d| d.w_full())
                    .rounded(px(6.))
                    .bg(if p.dark { rgb(0x1c1c1e).into() } else { p.card })
                    .border_1()
                    .border_color(if cookies_open { p.ink } else { p.line_strong })
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .cursor_pointer()
                    .child(tr(cx, "Manage cookies", "管理 Cookie")),
                cx,
                |this, _, cx| this.toggle_cookies_open(cx),
            );

            let mut data_card = div()
                .mt(px(12.))

                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                // Row 1: Ad measurement
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
                                        .child(tr(cx, "Ad measurement", "广告归因测量")),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .items_center()
                                        .gap(px(4.))
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .child(tr(
                                            cx,
                                            "Shares a hashed email and sign-up events with ad partners, so we can tell which ads work.",
                                            "向广告合作方共享哈希处理后的邮箱与注册事件，用于评估广告效果。",
                                        ))
                                        .child(policy_link),
                                ),
                        )
                        .child(pro_switch(
                            pro_eid(&self.id, "ad-measurement-sw"),
                            tr(cx, "Ad measurement", "广告归因测量"),
                            ad_on,
                            cx,
                            |this, _, cx| this.toggle_ad_measurement(cx),
                        )),
                )
                // Row 2: Product analytics
                .child(
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
                                        .child(tr(cx, "Product analytics", "产品使用分析")),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .child(tr(
                                            cx,
                                            "Counts which features you use – never what you type. Works without cookies.",
                                            "仅统计功能使用频次——绝不记录输入内容，无需 Cookie 即可运行。",
                                        )),
                                ),
                        )
                        .child(pro_switch(
                            pro_eid(&self.id, "product-analytics-sw"),
                            tr(cx, "Product analytics", "产品使用分析"),
                            analytics_on,
                            cx,
                            |this, _, cx| this.toggle_product_analytics(cx),
                        )),
                )
                // Row 3: Improve models with my chats
                .child(
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
                                        .child(tr(
                                            cx,
                                            "Improve models with my chats",
                                            "允许使用我的对话改进模型",
                                        )),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .child(tr(
                                            cx,
                                            "Lets anonymised chats help train future models. Off by default; workspace chats are never used.",
                                            "允许脱敏后的个人对话用于训练未来模型。默认关闭；工作区对话绝不会被使用。",
                                        )),
                                ),
                        )
                        .child(pro_switch(
                            pro_eid(&self.id, "model-training-sw"),
                            tr(
                                cx,
                                "Improve models with my chats",
                                "允许使用我的对话改进模型",
                            ),
                            train_on,
                            cx,
                            |this, _, cx| this.toggle_model_training(cx),
                        )),
                )
                // Row 4: Cookies on this browser
                .child(
                    div()
                        .min_h(px(60.))
                        .py(px(if narrow { 12. } else { 10. }))
                        .pl(px(if narrow { 14. } else { 16. }))
                        .pr(px(if narrow { 14. } else { 12. }))
                        .border_t_1()
                        .border_color(p.line)
                        .flex()
                        .when(narrow, |d| d.flex_col().items_stretch().gap(px(10.)))
                        .when(!narrow, |d| {
                            d.items_center().justify_between().gap(px(24.))
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
                                        .line_height(px(20.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .child(tr(
                                            cx,
                                            "Cookies on this browser",
                                            "此浏览器的 Cookie",
                                        )),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .child(tr(
                                            cx,
                                            "Choose which cookies this device may keep",
                                            "选择当前设备允许保留的 Cookie 类别",
                                        )),
                                ),
                        )
                        .child(manage_cookies_btn),
                );

            let cookies_drawer = div()
                .pl(px(if narrow { 14. } else { 16. }))
                .pr(px(if narrow { 14. } else { 12. }))
                .pb(px(12.))
                .child(
                    div()
                        .py(px(4.))
                        .px(px(14.))
                        .rounded(px(10.))
                        .bg(frame_bg)
                        .border_1()
                        .border_color(p.line)
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .min_h(px(52.))
                                .py(px(8.))
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(16.))
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
                                                .text_color(p.strong)
                                                .child(tr(cx, "Essential cookies", "必要 Cookie")),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(p.muted)
                                                .child(tr(
                                                    cx,
                                                    "Sign-in session, security tokens and workspace routing",
                                                    "维持登录状态、安全令牌与工作区路由所必需",
                                                )),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .text_size(px(12.))
                                        .text_color(p.faint)
                                        .child(tr(cx, "Always on", "始终启用")),
                                ),
                        )
                        .child(
                            div()
                                .min_h(px(52.))
                                .py(px(8.))
                                .border_t_1()
                                .border_color(p.line)
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(16.))
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
                                                .text_color(p.strong)
                                                .child(tr(
                                                    cx,
                                                    "Functional preferences",
                                                    "功能偏好 Cookie",
                                                )),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(p.muted)
                                                .child(tr(
                                                    cx,
                                                    "Remembers theme, sidebar width and draft layout",
                                                    "记住主题模式、侧边栏宽度与本地布局状态",
                                                )),
                                        ),
                                )
                                .child(pro_switch(
                                    pro_eid(&self.id, "cookie-func-sw"),
                                    tr(cx, "Functional preferences", "功能偏好 Cookie"),
                                    func_cookies,
                                    cx,
                                    |this, _, cx| this.toggle_functional_cookies(cx),
                                )),
                        )
                        .child(
                            div()
                                .min_h(px(52.))
                                .py(px(8.))
                                .border_t_1()
                                .border_color(p.line)
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(16.))
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
                                                .text_color(p.strong)
                                                .child(tr(
                                                    cx,
                                                    "Performance & crash reports",
                                                    "性能与异常诊断",
                                                )),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .text_color(p.muted)
                                                .child(tr(
                                                    cx,
                                                    "Anonymous page load timings and renderer diagnostics",
                                                    "脱敏页面加载耗时与渲染器异常诊断",
                                                )),
                                        ),
                                )
                                .child(pro_switch(
                                    pro_eid(&self.id, "cookie-perf-sw"),
                                    tr(cx, "Performance & crash reports", "性能与异常诊断"),
                                    perf_cookies,
                                    cx,
                                    |this, _, cx| this.toggle_performance_cookies(cx),
                                )),
                        ),
                )
                .into_any_element();

            let enabled_controls = usize::from(ad_on)
                + usize::from(analytics_on)
                + usize::from(train_on)
                + usize::from(func_cookies)
                + usize::from(perf_cookies);
            let footprint_ratio = enabled_controls as f32 / 5.0;
            let footprint_label = format!(
                "{}: {enabled_controls}/5",
                tr(
                    cx,
                    "Optional telemetry & cookies enabled",
                    "已启用的可选数据与 Cookie"
                )
            );

            data_card = data_card
                .child(pro_fold(
                    pro_eid(&self.id, "cookies-drawer-fold"),
                    cookies_open,
                    182.,
                    0.,
                    move || cookies_drawer,
                ))
                .child(
                    div()
                        .py(px(9.))
                        .pl(px(if narrow { 14. } else { 16. }))
                        .pr(px(if narrow { 14. } else { 12. }))
                        .border_t_1()
                        .border_color(p.line)
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(16.))
                        .child(
                            div()
                                .text_size(px(11.5))
                                .line_height(px(16.))
                                .text_color(p.faint)
                                .font_features(FontFeatures(std::sync::Arc::new(vec![(
                                    "tnum".into(),
                                    1,
                                )])))
                                .child(footprint_label),
                        )
                        .child(
                            div()
                                .w(px(if narrow { 80. } else { 112. }))
                                .flex_none()
                                .child(pro_spring_bar(
                                    pro_eid(&self.id, "privacy-footprint-bar"),
                                    footprint_ratio,
                                    4.,
                                    2.,
                                    p.chip,
                                    p.ink.opacity(0.65),
                                )),
                        ),
                );

            // 2. Emails card
            let subscribed = self.values.product_emails;
            let email_btn = if subscribed {
                pro_btn(
                    pro_eid(&self.id, "product-emails-btn"),
                    tr(cx, "Subscribed", "已订阅"),
                    Some(ProIcon::Check),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    |this, _, cx| this.toggle_product_emails(cx),
                )
            } else {
                pro_btn(
                    pro_eid(&self.id, "product-emails-btn"),
                    tr(cx, "Subscribe", "订阅"),
                    None,
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    |this, _, cx| this.toggle_product_emails(cx),
                )
            };

            let emails_card = div()
                .mt(px(12.))

                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .min_h(px(60.))
                .py(px(if narrow { 12. } else { 10. }))
                .pl(px(if narrow { 14. } else { 16. }))
                .pr(px(if narrow { 14. } else { 12. }))
                .flex()
                .when(narrow, |d| d.flex_col().items_stretch().gap(px(10.)))
                .when(!narrow, |d| {
                    d.items_center().justify_between().gap(px(24.))
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
                                .line_height(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(tr(cx, "Product updates", "产品动态更新")),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(tr(
                                    cx,
                                    "Occasional emails about new features and tips – never more than one a month.",
                                    "不定期发送新功能与使用技巧邮件——每月不超过一封。",
                                )),
                        ),
                )
                .child(email_btn);

            // 3. Active sessions section
            let other_sessions = self.values.other_session_count();
            let sign_out_others_inner = activate(
                div()
                    .id(pro_eid(&self.id, "sign-out-others-btn"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Sign out others", "退出其他设备"))
                    .tab_index(0)
                    .h(px(28.))
                    .pl(px(8.))
                    .pr(px(10.))
                    .rounded(px(6.))
                    .bg(if p.dark { rgb(0x1c1c1e).into() } else { p.card })
                    .border_1()
                    .border_color(p.line_strong)
                    .hover(move |s| s.bg(p.hover))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .cursor_pointer()
                    .child(pro_svg("0 0 24 24", SIGN_OUT_SVG, 14., p.muted))
                    .child(tr(cx, "Sign out others", "退出其他设备")),
                cx,
                |this, _, cx| {
                    this.sign_out_others(cx);
                },
            )
            .into_any_element();
            let sign_out_others_btn: Option<AnyElement> = Some(pro_presence(
                pro_eid(&self.id, "sign-out-others-pres"),
                other_sessions > 0,
                0.2,
                0.16,
                move |amount| {
                    div()
                        .flex_none()
                        .opacity(amount)
                        .child(sign_out_others_inner)
                        .into_any_element()
                },
            ));

            let mut sessions_card = div()
                .mt(px(12.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col();

            let sessions_snapshot = self.values.sessions.clone();
            for (idx, sess) in sessions_snapshot.iter().enumerate() {
                let s_id = sess.id.clone();
                let s_name = sess.name.clone();
                let s_meta = sess.meta.clone();
                let is_curr = sess.current;
                let sign_out_aria = format!("{} {}", tr(cx, "Sign out", "退出登录"), s_name);

                let trailing: Option<AnyElement> = if is_curr {
                    None
                } else {
                    Some(
                        activate(
                            div()
                                .id(pro_eid(&self.id, format!("signout-{}", sess.id)))
                                .role(Role::Button)
                                .aria_label(sign_out_aria)
                                .tab_index(0)
                                .h(px(28.))
                                .px(px(10.))
                                .rounded(px(6.))
                                .bg(if p.dark { rgb(0x1c1c1e).into() } else { p.card })
                                .border_1()
                                .border_color(p.line_strong)
                                .hover(move |s| s.bg(p.hover))
                                .focus_visible(move |s| s.border_1().border_color(p.focus))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(12.5))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.ink)
                                .cursor_pointer()
                                .child(tr(cx, "Sign out", "退出")),
                            cx,
                            move |this, _, cx| {
                                this.sign_out_session(s_id.as_ref(), cx);
                            },
                        )
                        .into_any_element(),
                    )
                };

                sessions_card = sessions_card.child(
                    div()
                        .min_h(px(60.))
                        .py(px(10.))
                        .pl(px(if narrow { 14. } else { 16. }))
                        .pr(px(if narrow { 14. } else { 12. }))
                        .when(idx > 0, |d| d.border_t_1().border_color(p.line))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(16.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .child(
                                    div()
                                        .size(px(32.))
                                        .flex_none()
                                        .rounded(px(9.))
                                        .bg(tile_bg)
                                        .border_1()
                                        .border_color(p.line_strong)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(sess.device.render_icon(16., p.muted)),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .flex()
                                                .flex_wrap()
                                                .items_center()
                                                .gap(px(8.))
                                                .child(
                                                    div()
                                                        .text_size(px(13.))
                                                        .line_height(px(20.))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(p.strong)
                                                        .child(s_name),
                                                )
                                                .when(is_curr, |d| {
                                                    d.child(
                                                        div()
                                                            .px(px(7.))
                                                            .py(px(1.))
                                                            .rounded_full()
                                                            .bg(p.green_soft)
                                                            .text_size(px(11.))
                                                            .line_height(px(16.))
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .text_color(p.green_ink)
                                                            .child(tr(
                                                                cx,
                                                                "This device",
                                                                "当前设备",
                                                            )),
                                                    )
                                                }),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .line_height(px(16.))
                                                .text_color(p.muted)
                                                .child(s_meta),
                                        ),
                                ),
                        )
                        .when_some(trailing, |d, btn| d.child(btn)),
                );
            }

            // 4. Shared chats section
            let ws_count = self.values.shared_count(SharedChatScope::Workspace);
            let pub_count = self.values.shared_count(SharedChatScope::Public);
            let active_scope = self.active_scope;
            let scope_sensor = pro_selection_sensor();
            let active_pill_bg: Hsla = if p.dark { rgb(0x2c2c2f).into() } else { p.card };

            let render_scope_tab =
                |scope: SharedChatScope, count: usize, cx: &mut Context<Self>| {
                    let active = active_scope == scope;
                    let count_bg: Hsla = if active {
                        if p.dark {
                            hsla(0., 0., 1., 0.08)
                        } else {
                            p.chip
                        }
                    } else {
                        p.chip_hover
                    };
                    let suffix = match scope {
                        SharedChatScope::Workspace => "tab-workspace",
                        SharedChatScope::Public => "tab-public",
                    };
                    let sensor = scope_sensor.clone();
                    activate(
                        div()
                            .id(pro_eid(&self.id, suffix))
                            .role(Role::Tab)
                            .aria_label(scope.label_tr(cx))
                            .aria_selected(active)
                            .tab_index(0)
                            .relative()
                            .h(px(24.))
                            .px(px(8.))
                            .rounded(px(6.))
                            .when(!active, |d| d.hover(move |s| s.bg(p.hover)))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if active { p.ink } else { p.text })
                            .cursor_pointer()
                            .child(scope.label_tr(cx))
                            .child(
                                div()
                                    .min_w(px(15.))
                                    .h(px(15.))
                                    .px(px(3.5))
                                    .rounded(px(4.))
                                    .bg(count_bg)
                                    .font_features(FontFeatures(std::sync::Arc::new(vec![(
                                        "tnum".into(),
                                        1,
                                    )])))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(10.5))
                                    .text_color(if active { p.ink } else { p.text })
                                    .child(count.to_string()),
                            )
                            .when(active, |d| d.child(pro_mark_selected(&sensor))),
                        cx,
                        move |this, _, cx| this.set_active_scope(scope, cx),
                    )
                };

            let ws_tab = render_scope_tab(SharedChatScope::Workspace, ws_count, cx);
            let pub_tab = render_scope_tab(SharedChatScope::Public, pub_count, cx);

            let tabs_bar = div().mt(px(12.)).flex().child(
                div()
                    .relative()
                    .p(px(2.))
                    .rounded(px(8.))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .gap(px(2.))
                    .child(pro_pill_indicator(
                        pro_eid(&self.id, "shared-scope-pill"),
                        active_scope.label(),
                        scope_sensor,
                        6.,
                        active_pill_bg,
                        Some(p.line),
                        !p.dark,
                    ))
                    .child(ws_tab)
                    .child(pub_tab),
            );

            let visible_chats: Vec<SharedChatLink> = self
                .values
                .shared_chats
                .iter()
                .filter(|c| c.scope == active_scope)
                .cloned()
                .collect();

            let max_views = self
                .values
                .shared_chats
                .iter()
                .map(|c| c.views)
                .max()
                .unwrap_or(1)
                .max(10);

            let shared_inner: AnyElement = if visible_chats.is_empty() {
                let (empty_title, empty_cap) = match active_scope {
                    SharedChatScope::Workspace => (
                        tr(cx, "No shared chats", "暂无共享对话"),
                        tr(
                            cx,
                            "Chats you share with members in your workspace appear here.",
                            "与工作区成员通过链接共享的对话将显示在这里。",
                        ),
                    ),
                    SharedChatScope::Public => (
                        tr(cx, "No public links", "暂无公开链接"),
                        tr(
                            cx,
                            "Chats shared with anyone who has the link appear here.",
                            "开启公开访问链接的对话将显示在这里。",
                        ),
                    ),
                };
                div()
                    .mt(px(8.))
                    .rounded(px(12.))
                    .bg(frame_bg)
                    .border_1()
                    .border_color(p.line)
                    .child(pro_empty_state(
                        ProIcon::Link,
                        empty_title,
                        empty_cap,
                        Some((
                            pro_eid(&self.id, "open-chats-cta"),
                            tr(cx, "Open chats", "打开对话列表").into(),
                            Box::new(|_, _, cx| {
                                cx.emit(SettingsPrivacyEvent::Navigate("Chats".into()));
                            }),
                        )),
                        cx,
                    ))
                    .into_any_element()
            } else {
                let mut chat_card = div()
                    .mt(px(8.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col();

                for (idx, chat) in visible_chats.iter().enumerate() {
                    let c_id_copy = chat.id.clone();
                    let c_id_stop = chat.id.clone();
                    let c_title = chat.title.clone();
                    let c_shared = chat.shared.clone();
                    let view_ratio = (chat.views as f32 / max_views as f32).clamp(0.06, 1.0);
                    let view_bar_id = pro_eid(&self.id, format!("chat-views-bar-{}", chat.id));
                    let is_copied = self
                        .copied_chat
                        .as_ref()
                        .is_some_and(|(cid, _)| *cid == chat.id);

                    let copy_aria = format!("{} {}", tr(cx, "Copy link to", "复制链接："), c_title);
                    let stop_aria = format!("{} {}", tr(cx, "Stop sharing", "停止共享："), c_title);

                    let copy_btn = activate(
                        div()
                            .id(pro_eid(&self.id, format!("copy-chat-{}", chat.id)))
                            .role(Role::Button)
                            .aria_label(copy_aria)
                            .tab_index(0)
                            .h(px(28.))
                            .when(narrow, |d| d.w(px(30.)).px_0())
                            .when(!narrow, |d| d.pl(px(8.)).pr(px(10.)))
                            .rounded(px(6.))
                            .bg(if p.dark { rgb(0x1c1c1e).into() } else { p.card })
                            .border_1()
                            .border_color(p.line_strong)
                            .hover(move |s| s.bg(p.hover))
                            .focus_visible(move |s| s.border_1().border_color(p.focus))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .gap(px(5.))
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if is_copied { p.green_ink } else { p.ink })
                            .cursor_pointer()
                            .child(if is_copied {
                                pro_svg("0 0 24 24", CHECK_SVG, 14., p.green_ink)
                            } else {
                                pro_icon(ProIcon::Copy, 14., p.muted)
                            })
                            .when(!narrow, |d| {
                                d.child(if is_copied {
                                    tr(cx, "Copied", "已复制")
                                } else {
                                    tr(cx, "Copy link", "复制链接")
                                })
                            }),
                        cx,
                        move |this, _, cx| {
                            this.copy_shared_link(c_id_copy.as_ref(), cx);
                        },
                    );

                    let c_id_hover = chat.id.clone();
                    let row_hovered = self.hovered_chat.as_ref() == Some(&chat.id);
                    let stop_btn = activate(
                        div()
                            .id(pro_eid(&self.id, format!("stop-chat-{}", chat.id)))
                            .role(Role::Button)
                            .aria_label(stop_aria)
                            .tab_index(0)
                            .size(px(28.))
                            .flex_none()
                            .rounded(px(7.))
                            .opacity(if row_hovered { 1. } else { 0. })
                            .hover(move |s| s.opacity(1.).bg(p.hover))
                            .focus_visible(move |s| s.opacity(1.).border_1().border_color(p.focus))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .child(pro_icon(ProIcon::Close, 14., p.muted)),
                        cx,
                        move |this, _, cx| {
                            this.stop_sharing_chat(c_id_stop.as_ref(), cx);
                        },
                    );

                    chat_card = chat_card.child(
                        div()
                            .id(pro_eid(&self.id, format!("chat-row-{}", chat.id)))
                            .on_hover(cx.listener(move |this, hovered: &bool, _, h_cx| {
                                if *hovered {
                                    if this.hovered_chat.as_ref() != Some(&c_id_hover) {
                                        this.hovered_chat = Some(c_id_hover.clone());
                                        h_cx.notify();
                                    }
                                } else if this.hovered_chat.as_ref() == Some(&c_id_hover) {
                                    this.hovered_chat = None;
                                    h_cx.notify();
                                }
                            }))
                            .min_h(px(60.))
                            .py(px(10.))
                            .pl(px(if narrow { 14. } else { 16. }))
                            .pr(px(if narrow { 14. } else { 12. }))
                            .when(idx > 0, |d| d.border_t_1().border_color(p.line))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(16.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .child(
                                        div()
                                            .size(px(28.))
                                            .flex_none()
                                            .rounded(px(8.))
                                            .bg(tile_bg)
                                            .border_1()
                                            .border_color(p.line_strong)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(pro_svg(
                                                "0 0 24 24",
                                                CHAT_DOTS_SVG,
                                                15.,
                                                p.muted,
                                            )),
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
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.strong)
                                                    .child(c_title),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_wrap()
                                                    .items_center()
                                                    .gap(px(5.))
                                                    .text_size(px(12.))
                                                    .line_height(px(16.))
                                                    .text_color(p.muted)
                                                    .child(c_shared)
                                                    .child("·")
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .gap(px(4.))
                                                            .font_features(FontFeatures(
                                                                std::sync::Arc::new(vec![(
                                                                    "tnum".into(),
                                                                    1,
                                                                )]),
                                                            ))
                                                            .child(pro_icon(
                                                                ProIcon::Eye,
                                                                12.,
                                                                p.faint,
                                                            ))
                                                            .child(format!(
                                                                "{} {}",
                                                                chat.views,
                                                                tr(cx, "views", "次浏览")
                                                            ))
                                                            .when(!narrow, |d| {
                                                                d.child(
                                                                    div()
                                                                        .w(px(38.))
                                                                        .ml(px(2.))
                                                                        .flex_none()
                                                                        .child(pro_spring_bar(
                                                                            view_bar_id,
                                                                            view_ratio,
                                                                            3.5,
                                                                            2.,
                                                                            p.chip,
                                                                            p.muted.opacity(0.75),
                                                                        )),
                                                                )
                                                            }),
                                                    ),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(copy_btn)
                                    .child(stop_btn),
                            ),
                    );
                }

                chat_card.into_any_element()
            };
            let shared_content = pro_view_transition(
                pro_eid(&self.id, "shared-chats-vtx"),
                format!("{}-{}", active_scope.label(), visible_chats.len()),
                shared_inner,
            );

            // 5. Danger zone section
            let export_btn = pro_btn(
                pro_eid(&self.id, "export-data-btn"),
                tr(cx, "Export", "导出"),
                Some(ProIcon::Download),
                ProBtnStyle::Secondary,
                true,
                cx,
                |this, _, cx| this.request_export(cx),
            );

            let delete_trigger_btn = activate(
                div()
                    .id(pro_eid(&self.id, "delete-all-chats-btn"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Delete all chats", "删除全部对话"))
                    .tab_index(0)
                    .h(px(30.))
                    .pl(px(12.))
                    .pr(px(10.))
                    .when(narrow, |d| d.w_full())
                    .rounded(px(8.))
                    .border_1()
                    .border_color(p.red.opacity(0.35))
                    .hover(move |s| s.bg(p.red.opacity(0.08)))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.red)
                    .cursor_pointer()
                    .child(tr(cx, "Delete all chats", "删除全部对话"))
                    .child(pro_icon(ProIcon::Trash, 14., p.red)),
                cx,
                |this, _, cx| {
                    if this.delete_confirm_open {
                        this.cancel_delete_confirm(cx);
                    } else {
                        this.open_delete_confirm(cx);
                    }
                },
            );

            let export_email = self.user.email.clone();
            let export_banner_row = div()
                .py(px(10.))
                .pl(px(if narrow { 14. } else { 16. }))
                .pr(px(if narrow { 14. } else { 12. }))
                .border_t_1()
                .border_color(p.line)
                .bg(frame_bg)
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_icon(ProIcon::Check, 14., p.green_ink))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(12.))
                                .line_height(px(18.))
                                .text_color(p.text)
                                .child(format!(
                                    "{} {export_email}",
                                    tr(
                                        cx,
                                        "Archive requested — a ZIP download link will be sent to",
                                        "已提交导出请求——ZIP 下载链接将发送至",
                                    )
                                )),
                        ),
                )
                .child(activate(
                    div()
                        .id(pro_eid(&self.id, "dismiss-export-banner-btn"))
                        .role(Role::Button)
                        .tab_index(0)
                        .h(px(24.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .hover(move |s| s.bg(p.hover))
                        .focus_visible(move |s| s.border_1().border_color(p.focus))
                        .flex_none()
                        .flex()
                        .items_center()
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .cursor_pointer()
                        .child(tr(cx, "Dismiss", "关闭")),
                    cx,
                    |this, _, cx| {
                        this.export_banner_open = false;
                        cx.notify();
                    },
                ))
                .into_any_element();

            let confirm_solid_btn = activate(
                div()
                    .id(pro_eid(&self.id, "confirm-delete-chats-btn"))
                    .role(Role::Button)
                    .aria_label(tr(
                        cx,
                        "Confirm delete all chats",
                        "确认不可恢复删除全部对话",
                    ))
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(12.))
                    .rounded(px(7.))
                    .bg(p.red)
                    .hover(move |s| s.bg(p.red.opacity(0.88)))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(0xffffff))
                    .cursor_pointer()
                    .child(tr(cx, "Delete forever", "确认删除")),
                cx,
                |this, _, cx| this.confirm_delete_chats(cx),
            );

            let delete_confirm_row = div()
                .pt(px(12.))
                .pb(px(14.))
                .pl(px(if narrow { 14. } else { 16. }))
                .pr(px(if narrow { 14. } else { 12. }))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .when(narrow, |d| d.flex_col().items_stretch().gap(px(10.)))
                .when(!narrow, |d| {
                    d.items_center().justify_between().gap(px(16.))
                })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(12.5))
                        .line_height(px(18.))
                        .text_color(p.ink)
                        .child(tr(
                            cx,
                            "Delete every chat you started? Shared links break immediately and cannot be undone.",
                            "确定删除你发起的所有对话吗？所有共享链接将立即失效且无法撤销。",
                        )),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_btn(
                            pro_eid(&self.id, "cancel-delete-chats-btn"),
                            tr(cx, "Cancel", "取消"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            |this, _, cx| this.cancel_delete_confirm(cx),
                        ))
                        .child(confirm_solid_btn),
                )
                .into_any_element();

            let danger_card = div()
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
                        .py(px(if narrow { 12. } else { 10. }))
                        .pl(px(if narrow { 14. } else { 16. }))
                        .pr(px(if narrow { 14. } else { 12. }))
                        .flex()
                        .when(narrow, |d| d.flex_col().items_stretch().gap(px(10.)))
                        .when(!narrow, |d| {
                            d.items_center().justify_between().gap(px(24.))
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
                                        .line_height(px(20.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .child(tr(cx, "Export my data", "导出我的数据")),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .child(tr(
                                            cx,
                                            "Every chat and file you own, as a ZIP of Markdown and JSON",
                                            "将你拥有的全部对话与文件打包为 Markdown 与 JSON 格式的 ZIP 归档",
                                        )),
                                ),
                        )
                        .child(export_btn),
                )
                .child(pro_fold(
                    pro_eid(&self.id, "export-banner-fold"),
                    self.export_banner_open,
                    44.,
                    0.,
                    move || export_banner_row,
                ))
                .child(
                    div()
                        .min_h(px(60.))
                        .py(px(if narrow { 12. } else { 10. }))
                        .pl(px(if narrow { 14. } else { 16. }))
                        .pr(px(if narrow { 14. } else { 12. }))
                        .border_t_1()
                        .border_color(p.line)
                        .flex()
                        .when(narrow, |d| d.flex_col().items_stretch().gap(px(10.)))
                        .when(!narrow, |d| {
                            d.items_center().justify_between().gap(px(24.))
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
                                        .line_height(px(20.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .child(tr(cx, "Delete all chats", "删除全部对话")),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .child(tr(
                                            cx,
                                            "Removes every chat you started, shared ones included. Files and agents stay.",
                                            "清空移除你发起的全部对话（含共享链接），文件与智能体不受影响。",
                                        )),
                                ),
                        )
                        .child(delete_trigger_btn),
                )
                .child(pro_fold(
                    pro_eid(&self.id, "delete-chats-fold"),
                    self.delete_confirm_open,
                    62.,
                    0.,
                    move || delete_confirm_row,
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
                        .child(tr(cx, "Privacy", "隐私")),
                )
                .child(
                    div()
                        .mt(px(4.))
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.text)
                        .child(tr(
                            cx,
                            "Decide what you share and where you're signed in.",
                            "管理你共享的数据与已登录的设备。",
                        )),
                )
                // Section 1
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
                                .child(tr(cx, "Data and cookies", "数据与 Cookie")),
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
                                    "Changes apply on all your devices. Your chats and files are never shared with ad partners.",
                                    "更改将同步到你的所有设备。你的对话与文件绝不会共享给广告合作方。",
                                )),
                        )
                        .child(data_card),
                )
                // Section 2
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
                                .child(tr(cx, "Emails", "邮件订阅")),
                        )
                        .child(emails_card),
                )
                // Section 3
                .child(
                    div()
                        .mt(px(36.))
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .flex()
                                .when(narrow, |d| d.flex_col().items_start().gap(px(10.)))
                                .when(!narrow, |d| {
                                    d.items_end().justify_between().gap(px(24.))
                                })
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(px(14.))
                                                .line_height(px(20.))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.strong)
                                                .child(tr(cx, "Active sessions", "活跃会话")),
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
                                                    "Devices signed in to your account. Sign out any you don't recognise.",
                                                    "当前登录你账号的设备列表。如发现未知设备可立即退出。",
                                                )),
                                        ),
                                )
                                .when_some(sign_out_others_btn, |d, btn| d.child(btn)),
                        )
                        .child(pro_view_transition(
                            pro_eid(&self.id, "sessions-vtx"),
                            format!("sessions-{}", sessions_snapshot.len()),
                            sessions_card,
                        )),
                )
                // Section 4
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
                                .child(tr(cx, "Shared chats", "已共享对话")),
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
                                    "Every chat you've shared by link. Stopping a share breaks the link at once.",
                                    "你通过链接共享的所有对话。停止共享后链接将立即失效。",
                                )),
                        )
                        .child(tabs_bar)
                        .child(shared_content),
                )
                // Section 5
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
                                .child(tr(cx, "Danger zone", "危险操作")),
                        )
                        .child(danger_card),
                )
        };

        div()
            .id(pro_eid(&self.id, "page-root"))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                let k = ev.keystroke.key.as_str();
                if k == "escape" {
                    if this.delete_confirm_open {
                        this.cancel_delete_confirm(cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    } else if this.export_banner_open {
                        this.export_banner_open = false;
                        w.prevent_default();
                        cx.stop_propagation();
                        cx.notify();
                    } else if this.cookies_open {
                        this.toggle_cookies_open(cx);
                        w.prevent_default();
                        cx.stop_propagation();
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
                        pro_eid(&self.id, "privacy-body-vtx"),
                        if self.loading { "loading" } else { "ready" },
                        body,
                    )),
            )
            .child(pro_toast_banner(self.toast.clone()))
    }
}

impl Render for SettingsPrivacyScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{PrivacyValues, SettingsPrivacyEvent, SettingsPrivacyScreen, SharedChatScope};
    use gpui::{AppContext as _, TestAppContext};
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn session_protection_and_shared_chat_scope_counts() {
        let mut vals = PrivacyValues::default();
        assert!(!vals.ad_measurement);
        assert!(vals.product_analytics);
        assert!(!vals.model_training);
        assert!(!vals.product_emails);
        assert_eq!(vals.other_session_count(), 2);
        assert_eq!(vals.shared_count(SharedChatScope::Workspace), 2);
        assert_eq!(vals.shared_count(SharedChatScope::Public), 1);

        // Current device cannot be signed out from Privacy
        assert!(!vals.sign_out_session("chrome-macos"));
        assert!(vals.sign_out_session("safari-iphone"));
        assert_eq!(vals.other_session_count(), 1);

        assert_eq!(vals.sign_out_other_sessions(), 1);
        assert_eq!(vals.other_session_count(), 0);
        assert_eq!(vals.sessions.len(), 1);
        assert!(vals.sessions[0].current);
    }

    #[test]
    fn privacy_switches_share_revocation_export_and_delete_confirmation() {
        let app = TestAppContext::single();
        app.update(crate::init);

        let events: Rc<RefCell<Vec<SettingsPrivacyEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let recorded = events.clone();

        let screen = app.update(|cx| cx.new(|cx| SettingsPrivacyScreen::new("test-spv", cx)));

        app.update(|cx| {
            cx.subscribe(&screen, move |_, ev: &SettingsPrivacyEvent, _| {
                recorded.borrow_mut().push(ev.clone());
            })
            .detach();
            screen.update(cx, |s, s_cx| {
                s.toggle_model_training(s_cx);
                assert!(s.privacy().model_training);

                s.toggle_product_emails(s_cx);
                assert!(s.privacy().product_emails);

                assert!(s.stop_sharing_chat("q4-pricing", s_cx));
                assert_eq!(s.privacy().shared_count(SharedChatScope::Workspace), 1);

                s.request_export(s_cx);

                s.open_delete_confirm(s_cx);
                assert!(s.is_delete_confirm_open());
                s.confirm_delete_chats(s_cx);
                assert!(!s.is_delete_confirm_open());
                assert_eq!(s.privacy().shared_chats.len(), 0);
            });
        });

        let emitted = events.borrow();
        assert!(
            emitted
                .iter()
                .any(|e| matches!(e, SettingsPrivacyEvent::Export))
        );
        assert!(
            emitted
                .iter()
                .any(|e| matches!(e, SettingsPrivacyEvent::DeleteChats))
        );
    }
}
