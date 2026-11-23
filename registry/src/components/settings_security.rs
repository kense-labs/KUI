//! Workspace security settings screen with domain verification, SAML SSO, 2FA, and IP allowlist.
//!
//! Renders inside [`AppShell`](crate::components::app_shell::AppShell) with the `Settings`
//! sidebar preset (`Security` active under `Workspace`), providing email domain verification
//! with DNS TXT hints, automatic domain-join and SAML single sign-on switches, two-step
//! verification, session-length selection, copyable SAML SP endpoints with editable IdP
//! metadata URL, CIDR IP allowlist rules, and a confirmed workspace-wide session sign-out action.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_btn, pro_eid, pro_fold, pro_presence, pro_skeleton_bar,
            pro_spring_bar, pro_spring_f32, pro_switch, pro_toast_banner, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        clipboard,
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        flyout::TriggerBounds,
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const INFO_CIRCLE_SVG: &str = r#"<circle cx="12" cy="12" r="10" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M12 16V12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M12.125 8.25H12M12.25 8.25C12.25 8.11193 12.1381 8 12 8C11.8619 8 11.75 8.11193 11.75 8.25C11.75 8.38807 11.8619 8.5 12 8.5C12.1381 8.5 12.25 8.38807 12.25 8.25Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;

const SIGN_OUT_SVG: &str = r#"<path d="M18 18C18 18.4644 18 18.6965 17.978 18.8918C17.7952 20.5145 16.5145 21.7952 14.8919 21.978C14.6965 22 14.4644 22 14 22H11C7.70017 22 6.05025 22 5.02513 20.9749C4 19.9498 4 18.2998 4 15L4.00001 8.99998C4.00001 5.70016 4.00003 4.05024 5.02514 3.02512C6.05027 2 7.70018 2 11 2H14C14.4644 2 14.6965 2 14.8919 2.02201C16.5145 2.20483 17.7952 3.48547 17.978 5.10816C18 5.30347 18 5.53565 18 6" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M8.07612 11.1183C8 11.3021 8 11.535 8 12.001C8 12.4669 8 12.6999 8.07612 12.8837C8.17761 13.1287 8.37229 13.3234 8.61732 13.4249C8.80109 13.501 9.03406 13.501 9.5 13.501H14.5C14.5002 15.2503 14.511 16.1299 15.0623 16.3858C15.0829 16.3954 15.1037 16.4042 15.1249 16.4124C15.7045 16.6353 16.3999 16.0139 17.7907 14.7711C19.2576 13.4602 19.9912 12.7851 20 11.957C19.9912 11.1289 19.2576 10.4538 17.7907 9.14301C16.3999 7.90018 15.7045 7.27876 15.1249 7.50171C15.1037 7.50985 15.0829 7.51869 15.0623 7.52822C14.5018 7.78844 14.5 8.69318 14.5 10.501H9.5C9.03406 10.501 8.80109 10.501 8.61732 10.5771C8.37229 10.6786 8.17761 10.8733 8.07612 11.1183Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;

const CHECK_SVG: &str = r#"<path d="M5 14L8.5 17.5L19 6.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2.1"/>"#;

/// Email domain registered on a workspace with DNS TXT verification status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecurityDomain {
    /// Lowercase hostname (e.g. `"acme.com"`).
    pub domain: SharedString,
    /// Whether the domain's DNS TXT record has been verified.
    pub verified: bool,
}

impl SecurityDomain {
    /// Construct a [`SecurityDomain`] entry.
    pub fn new(domain: impl Into<SharedString>, verified: bool) -> Self {
        Self {
            domain: domain.into(),
            verified,
        }
    }

    /// Normalize and validate a raw domain string (`"acme.com"`), returning `None` if malformed.
    pub fn normalize_and_validate(raw: &str) -> Option<String> {
        let trimmed = raw
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches('@')
            .trim_end_matches('/')
            .to_ascii_lowercase();
        if trimmed.is_empty() || trimmed.len() > 253 || !trimmed.contains('.') {
            return None;
        }
        let labels: Vec<&str> = trimmed.split('.').collect();
        if labels.len() < 2 {
            return None;
        }
        for label in &labels {
            if label.is_empty() || label.len() > 63 {
                return None;
            }
            let bytes = label.as_bytes();
            if !bytes[0].is_ascii_alphanumeric() || !bytes[bytes.len() - 1].is_ascii_alphanumeric()
            {
                return None;
            }
            if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return None;
            }
        }
        let tld = labels[labels.len() - 1];
        if tld.len() < 2 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
            return None;
        }
        Some(trimmed)
    }
}

/// Idle timeout before members must sign in again.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SessionDuration {
    /// Expire idle sessions after 7 days (`"7 days"`).
    SevenDays,
    /// Expire idle sessions after 14 days (`"14 days"`).
    FourteenDays,
    /// Expire idle sessions after 30 days (`"30 days"`).
    #[default]
    ThirtyDays,
    /// Expire idle sessions after 90 days (`"90 days"`).
    NinetyDays,
}

impl SessionDuration {
    /// All supported session durations in display order.
    pub const ALL: [Self; 4] = [
        Self::SevenDays,
        Self::FourteenDays,
        Self::ThirtyDays,
        Self::NinetyDays,
    ];

    /// Canonical English label.
    pub fn label(self) -> &'static str {
        match self {
            Self::SevenDays => "7 days",
            Self::FourteenDays => "14 days",
            Self::ThirtyDays => "30 days",
            Self::NinetyDays => "90 days",
        }
    }

    /// Localized label matching the active UI language.
    pub fn label_tr(self, cx: &App) -> &'static str {
        match self {
            Self::SevenDays => tr(cx, "7 days", "7 天"),
            Self::FourteenDays => tr(cx, "14 days", "14 天"),
            Self::ThirtyDays => tr(cx, "30 days", "30 天"),
            Self::NinetyDays => tr(cx, "90 days", "90 天"),
        }
    }
}

/// Allowed IPv4/IPv6 CIDR network entry in the workspace IP allowlist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IpAllowRange {
    /// Canonical CIDR or single-host IP range (e.g. `"203.0.113.0/24"`).
    pub cidr: SharedString,
    /// Optional human-readable note (e.g. `"Vienna office"`).
    pub label: Option<SharedString>,
}

impl IpAllowRange {
    /// Construct an [`IpAllowRange`] entry.
    pub fn new(cidr: impl Into<SharedString>) -> Self {
        Self {
            cidr: cidr.into(),
            label: None,
        }
    }

    /// Attach an optional label to this CIDR range.
    pub fn with_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Validate an IPv4 or IPv6 address or CIDR block (`"203.0.113.0/24"`), normalizing bare hosts to `/32` or `/128`.
    pub fn normalize_and_validate(raw: &str) -> Option<String> {
        let s = raw.trim();
        if s.is_empty() {
            return None;
        }
        let (addr_part, prefix_opt) = match s.split_once('/') {
            Some((a, p)) => (a.trim(), Some(p.trim())),
            None => (s, None),
        };
        if addr_part.contains(':') {
            // Basic IPv6 validation
            let prefix: u8 = match prefix_opt {
                Some(p) => p.parse().ok()?,
                None => 128,
            };
            if prefix > 128 || addr_part.len() < 2 {
                return None;
            }
            if !addr_part.chars().all(|c| c.is_ascii_hexdigit() || c == ':') {
                return None;
            }
            Some(format!("{}/{}", addr_part.to_ascii_lowercase(), prefix))
        } else {
            let octets: Vec<&str> = addr_part.split('.').collect();
            if octets.len() != 4 {
                return None;
            }
            let mut parsed = [0u8; 4];
            for (i, part) in octets.iter().enumerate() {
                if part.is_empty() || (part.len() > 1 && part.starts_with('0')) {
                    return None;
                }
                parsed[i] = part.parse::<u8>().ok()?;
            }
            let prefix: u8 = match prefix_opt {
                Some(p) => p.parse().ok()?,
                None => 32,
            };
            if prefix > 32 {
                return None;
            }
            Some(format!(
                "{}.{}.{}.{}/{}",
                parsed[0], parsed[1], parsed[2], parsed[3], prefix
            ))
        }
    }
}

/// Complete workspace security settings managed by [`SettingsSecurityScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecurityValues {
    /// Registered company email domains.
    pub domains: Vec<SecurityDomain>,
    /// DNS TXT token displayed for pending domains (`"workspace-verify=7f3a9c"`).
    pub dns_verify_token: SharedString,
    /// Allow anyone with a verified email address to join without an invite.
    pub join_by_domain: bool,
    /// Require SAML single sign-on for members at a verified domain.
    pub sso_required: bool,
    /// Require TOTP two-step verification for accounts without SSO.
    pub two_factor_required: bool,
    /// Idle session timeout length.
    pub session_length: SessionDuration,
    /// Read-only SAML SP entity ID / Audience URI.
    pub audience_uri: SharedString,
    /// Read-only SAML Assertion Consumer Service URL.
    pub acs_url: SharedString,
    /// Identity provider SAML metadata URL.
    pub metadata_url: SharedString,
    /// Allowed CIDR IP ranges (empty means any network may connect).
    pub ip_ranges: Vec<IpAllowRange>,
}

impl Default for SecurityValues {
    fn default() -> Self {
        Self {
            domains: Self::default_domains(),
            dns_verify_token: "workspace-verify=7f3a9c".into(),
            join_by_domain: true,
            sso_required: true,
            two_factor_required: false,
            session_length: SessionDuration::ThirtyDays,
            audience_uri: "https://app.workspace.ai/saml/acme".into(),
            acs_url: "https://app.workspace.ai/saml/acme/acs".into(),
            metadata_url: "https://acme.okta.com/app/exk4p2/sso/saml/metadata".into(),
            ip_ranges: Vec::new(),
        }
    }
}

impl SecurityValues {
    /// Return the 2 canonical sample email domains (`acme.com` verified, `acme.io` pending).
    pub fn default_domains() -> Vec<SecurityDomain> {
        vec![
            SecurityDomain::new("acme.com", true),
            SecurityDomain::new("acme.io", false),
        ]
    }

    /// Return whether at least one domain has been verified.
    pub fn has_verified_domain(&self) -> bool {
        self.domains.iter().any(|d| d.verified)
    }

    /// Return whether any domain is still awaiting DNS TXT verification.
    pub fn has_pending_domain(&self) -> bool {
        self.domains.iter().any(|d| !d.verified)
    }

    /// Turn off `join_by_domain` and `sso_required` whenever no verified domain remains.
    pub fn enforce_domain_invariants(&mut self) {
        if !self.has_verified_domain() {
            self.join_by_domain = false;
            self.sso_required = false;
        }
    }
}

/// Account identity displayed at the bottom of the settings sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsSecurityUser {
    /// Full display name of the signed-in user.
    pub name: SharedString,
    /// Email address shown in the account menu.
    pub email: SharedString,
}

impl Default for SettingsSecurityUser {
    fn default() -> Self {
        Self {
            name: "Alex Morgan".into(),
            email: "alex@acme.com".into(),
        }
    }
}

impl SettingsSecurityUser {
    /// Construct a sidebar user summary with `name` and `email`.
    pub fn new(name: impl Into<SharedString>, email: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
        }
    }
}

/// Events emitted by [`SettingsSecurityScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsSecurityEvent {
    /// Emitted with the full [`SecurityValues`] whenever switches, domains, ranges, or metadata URL change.
    Save(SecurityValues),
    /// Emitted when DNS TXT verification is triggered for a domain.
    VerifyDomain(SharedString),
    /// Emitted once signing everyone out of the workspace has been confirmed.
    SignOutAll,
    /// Emitted when a settings sidebar item or `"Back to app"` is activated.
    Navigate(SharedString),
}

impl EventEmitter<SettingsSecurityEvent> for SettingsSecurityScreen {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CopiedField {
    AudienceUri,
    AcsUrl,
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

/// Workspace security settings screen (`settings-security`).
pub struct SettingsSecurityScreen {
    id: ElementId,
    focus: FocusHandle,
    shell: Entity<AppShell>,
    standalone: bool,
    loading: bool,
    empty: bool,
    workspace: SharedString,
    user: SettingsSecurityUser,
    values: SecurityValues,
    adding_domain: bool,
    domain_draft: String,
    adding_range: bool,
    range_draft: String,
    hovered_domain: Option<SharedString>,
    hovered_range: Option<usize>,
    session_menu_open: bool,
    session_highlight: usize,
    session_bounds: TriggerBounds,
    copied_field: Option<(CopiedField, Instant)>,
    sign_out_confirm_open: bool,
    toast: Option<(SharedString, Instant)>,
    started_at: Instant,
    page_width: Rc<Cell<f32>>,
    domain_editor: Option<Entity<Editor>>,
    range_editor: Option<Entity<Editor>>,
    metadata_editor: Option<Entity<Editor>>,
}

impl Focusable for SettingsSecurityScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl SettingsSecurityScreen {
    /// Construct a new [`SettingsSecurityScreen`] with canonical `Acme` security settings.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let focus = cx.focus_handle();
        let values = SecurityValues::default();
        let workspace: SharedString = "Acme".into();
        let user = SettingsSecurityUser::default();

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
                .with_active("Security")
                .with_work_area(false, false);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s
        });
        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                cx.emit(SettingsSecurityEvent::Navigate(label.clone()));
            }
            AppShellEvent::Back => {
                cx.emit(SettingsSecurityEvent::Navigate("Back to app".into()));
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
            adding_domain: false,
            domain_draft: String::new(),
            adding_range: false,
            range_draft: String::new(),
            hovered_domain: None,
            hovered_range: None,
            session_menu_open: false,
            session_highlight: 2,
            session_bounds: TriggerBounds::default(),
            copied_field: None,
            sign_out_confirm_open: false,
            toast: None,
            started_at: Instant::now(),
            page_width: Rc::new(Cell::new(720.)),
            domain_editor: None,
            range_editor: None,
            metadata_editor: None,
        }
    }

    /// Override the initial [`SecurityValues`].
    pub fn with_security(mut self, mut security: SecurityValues) -> Self {
        security.enforce_domain_invariants();
        self.values = security;
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
    pub fn with_user(mut self, user: SettingsSecurityUser, cx: &mut Context<Self>) -> Self {
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
    pub fn user(&self) -> &SettingsSecurityUser {
        &self.user
    }

    /// Start in skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Start in unconfigured empty state (no domains, no IP ranges, SSO and domain-join off).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.apply_empty(empty);
        self
    }

    /// Render only the page column without the outer [`AppShell`] frame.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Return the current [`SecurityValues`].
    pub fn security(&self) -> &SecurityValues {
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
            self.session_menu_open = false;
            cx.notify();
        }
    }

    /// Return whether the empty state is active.
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    /// Toggle the empty state (`No domains yet` + `No IP ranges`, SSO & domain-join off) at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.apply_empty(empty);
        cx.notify();
    }

    fn apply_empty(&mut self, empty: bool) {
        self.empty = empty;
        if empty {
            self.values.domains.clear();
            self.values.ip_ranges.clear();
            self.values.enforce_domain_invariants();
        } else if self.values.domains.is_empty() {
            self.values.domains = SecurityValues::default_domains();
            self.values.join_by_domain = true;
            self.values.sso_required = true;
        }
    }

    /// Open the inline `"Add domain"` form.
    pub fn open_add_domain(&mut self, w: Option<&mut Window>, cx: &mut Context<Self>) {
        self.adding_domain = true;
        self.domain_draft.clear();
        if let (Some(ed), Some(win)) = (&self.domain_editor, w) {
            ed.update(cx, |e, e_cx| {
                e.set_value("", win, e_cx);
                e.focus(win, e_cx);
            });
        }
        cx.notify();
    }

    /// Cancel the inline `"Add domain"` form.
    pub fn cancel_add_domain(&mut self, w: Option<&Window>, cx: &mut Context<Self>) {
        self.adding_domain = false;
        self.domain_draft.clear();
        if let (Some(ed), Some(win)) = (&self.domain_editor, w) {
            ed.update(cx, |e, e_cx| e.set_value("", win, e_cx));
        }
        cx.notify();
    }

    /// Add a pending email domain if valid and non-duplicate, emitting [`SettingsSecurityEvent::Save`].
    pub fn add_domain(&mut self, raw: &str, w: Option<&Window>, cx: &mut Context<Self>) -> bool {
        let Some(clean) = SecurityDomain::normalize_and_validate(raw) else {
            self.show_toast(
                tr(
                    cx,
                    "Enter a valid domain such as company.com",
                    "请输入有效的企业域名（例如 company.com）",
                ),
                cx,
            );
            return false;
        };
        if self
            .values
            .domains
            .iter()
            .any(|d| d.domain.as_ref() == clean)
        {
            self.show_toast(
                tr(cx, "That domain is already listed", "该域名已在列表中"),
                cx,
            );
            return false;
        }
        self.empty = false;
        self.adding_domain = false;
        self.domain_draft.clear();
        if let (Some(ed), Some(win)) = (&self.domain_editor, w) {
            ed.update(cx, |e, e_cx| e.set_value("", win, e_cx));
        }
        self.values.domains.push(SecurityDomain::new(clean, false));
        self.emit_saved(
            tr(
                cx,
                "Domain added – add the TXT record to verify",
                "已添加域名——请配置 TXT 解析记录后点击验证",
            ),
            cx,
        );
        true
    }

    /// Verify a pending domain's DNS TXT record, emitting [`SettingsSecurityEvent::VerifyDomain`] and [`SettingsSecurityEvent::Save`].
    pub fn verify_domain(&mut self, domain: &str, cx: &mut Context<Self>) -> bool {
        if let Some(entry) = self
            .values
            .domains
            .iter_mut()
            .find(|d| d.domain.as_ref() == domain)
            && !entry.verified
        {
            entry.verified = true;
            let dom_shared = entry.domain.clone();
            cx.emit(SettingsSecurityEvent::VerifyDomain(dom_shared));
            self.emit_saved(
                format!("{} {}", domain, tr(cx, "verified", "已完成验证")),
                cx,
            );
            return true;
        }
        false
    }

    /// Remove an email domain and enforce verified-domain invariants, emitting [`SettingsSecurityEvent::Save`].
    pub fn remove_domain(&mut self, domain: &str, cx: &mut Context<Self>) -> bool {
        if let Some(pos) = self
            .values
            .domains
            .iter()
            .position(|d| d.domain.as_ref() == domain)
        {
            self.values.domains.remove(pos);
            self.values.enforce_domain_invariants();
            self.emit_saved(tr(cx, "Domain removed", "已移除该域名"), cx);
            true
        } else {
            false
        }
    }

    /// Toggle `"Join by domain"` when a verified domain exists; otherwise explain why it is locked.
    pub fn toggle_join_by_domain(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.values.has_verified_domain() {
            self.show_toast(
                tr(
                    cx,
                    "Verify an email domain first to enable joining by domain",
                    "请先验证至少一个企业邮箱域名后再开启",
                ),
                cx,
            );
            return false;
        }
        self.values.join_by_domain = !self.values.join_by_domain;
        let msg = if self.values.join_by_domain {
            tr(cx, "Join by domain enabled", "已允许凭已验证域名自动加入")
        } else {
            tr(cx, "Join by domain disabled", "已关闭凭域名自动加入")
        };
        self.emit_saved(msg, cx);
        true
    }

    /// Toggle `"Require single sign-on (SAML)"` when a verified domain exists; otherwise explain why it is locked.
    pub fn toggle_sso_required(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.values.has_verified_domain() {
            self.show_toast(
                tr(
                    cx,
                    "Verify an email domain first to require single sign-on",
                    "请先验证至少一个企业邮箱域名后再开启单点登录",
                ),
                cx,
            );
            return false;
        }
        self.values.sso_required = !self.values.sso_required;
        let msg = if self.values.sso_required {
            tr(
                cx,
                "SAML single sign-on required",
                "已要求使用 SAML 单点登录",
            )
        } else {
            tr(
                cx,
                "SAML single sign-on optional",
                "已关闭强制 SAML 单点登录",
            )
        };
        self.emit_saved(msg, cx);
        true
    }

    /// Toggle `"Two-step verification"` and emit [`SettingsSecurityEvent::Save`].
    pub fn toggle_two_factor(&mut self, cx: &mut Context<Self>) {
        self.values.two_factor_required = !self.values.two_factor_required;
        let msg = if self.values.two_factor_required {
            tr(cx, "Two-step verification required", "已强制启用两步验证")
        } else {
            tr(cx, "Two-step verification optional", "已关闭强制两步验证")
        };
        self.emit_saved(msg, cx);
    }

    /// Select the workspace idle session timeout and emit [`SettingsSecurityEvent::Save`].
    pub fn set_session_length(&mut self, duration: SessionDuration, cx: &mut Context<Self>) {
        self.session_menu_open = false;
        if self.values.session_length != duration {
            self.values.session_length = duration;
            self.emit_saved(tr(cx, "Session length updated", "会话有效期已更新"), cx);
        } else {
            cx.notify();
        }
    }

    /// Update and commit the IdP SAML metadata URL, emitting [`SettingsSecurityEvent::Save`] when changed.
    pub fn commit_metadata_url(&mut self, url: impl Into<String>, cx: &mut Context<Self>) -> bool {
        let trimmed: SharedString = url.into().trim().to_owned().into();
        if self.values.metadata_url != trimmed {
            self.values.metadata_url = trimmed;
            self.emit_saved(
                tr(
                    cx,
                    "Identity provider metadata saved",
                    "身份提供商元数据地址已保存",
                ),
                cx,
            );
            true
        } else {
            false
        }
    }

    /// Open the inline `"Add range"` CIDR form.
    pub fn open_add_range(&mut self, w: Option<&mut Window>, cx: &mut Context<Self>) {
        self.adding_range = true;
        self.range_draft.clear();
        if let (Some(ed), Some(win)) = (&self.range_editor, w) {
            ed.update(cx, |e, e_cx| {
                e.set_value("", win, e_cx);
                e.focus(win, e_cx);
            });
        }
        cx.notify();
    }

    /// Cancel the inline `"Add range"` CIDR form.
    pub fn cancel_add_range(&mut self, w: Option<&Window>, cx: &mut Context<Self>) {
        self.adding_range = false;
        self.range_draft.clear();
        if let (Some(ed), Some(win)) = (&self.range_editor, w) {
            ed.update(cx, |e, e_cx| e.set_value("", win, e_cx));
        }
        cx.notify();
    }

    /// Validate and append an IPv4/IPv6 CIDR range to the allowlist, emitting [`SettingsSecurityEvent::Save`].
    pub fn add_ip_range(&mut self, raw: &str, w: Option<&Window>, cx: &mut Context<Self>) -> bool {
        let Some(cidr) = IpAllowRange::normalize_and_validate(raw) else {
            self.show_toast(
                tr(
                    cx,
                    "Enter a valid CIDR range such as 203.0.113.0/24",
                    "请输入有效的 CIDR 网段（例如 203.0.113.0/24）",
                ),
                cx,
            );
            return false;
        };
        if self
            .values
            .ip_ranges
            .iter()
            .any(|r| r.cidr.as_ref() == cidr)
        {
            self.show_toast(
                tr(
                    cx,
                    "That IP range is already allowed",
                    "该 IP 网段已在白名单中",
                ),
                cx,
            );
            return false;
        }
        self.empty = false;
        self.adding_range = false;
        self.range_draft.clear();
        if let (Some(ed), Some(win)) = (&self.range_editor, w) {
            ed.update(cx, |e, e_cx| e.set_value("", win, e_cx));
        }
        self.values.ip_ranges.push(IpAllowRange::new(cidr));
        self.emit_saved(tr(cx, "IP range added", "已添加 IP 白名单网段"), cx);
        true
    }

    /// Remove an IP allowlist range by `cidr`, emitting [`SettingsSecurityEvent::Save`].
    pub fn remove_ip_range(&mut self, cidr: &str, cx: &mut Context<Self>) -> bool {
        if let Some(pos) = self
            .values
            .ip_ranges
            .iter()
            .position(|r| r.cidr.as_ref() == cidr)
        {
            self.values.ip_ranges.remove(pos);
            self.emit_saved(tr(cx, "IP range removed", "已移除 IP 白名单网段"), cx);
            true
        } else {
            false
        }
    }

    /// Return whether the workspace-wide sign-out confirmation step is open.
    pub fn is_sign_out_confirm_open(&self) -> bool {
        self.sign_out_confirm_open
    }

    /// Open the confirmation step before signing everyone out of the workspace.
    pub fn open_sign_out_confirm(&mut self, cx: &mut Context<Self>) {
        self.sign_out_confirm_open = true;
        cx.notify();
    }

    /// Cancel signing everyone out.
    pub fn cancel_sign_out_confirm(&mut self, cx: &mut Context<Self>) {
        self.sign_out_confirm_open = false;
        cx.notify();
    }

    /// Confirm signing everyone out of the workspace, emitting [`SettingsSecurityEvent::SignOutAll`].
    pub fn confirm_sign_out_all(&mut self, cx: &mut Context<Self>) {
        self.sign_out_confirm_open = false;
        self.show_toast(
            tr(
                cx,
                "Every workspace session has been signed out",
                "已强制退出工作区内所有活跃会话",
            ),
            cx,
        );
        cx.emit(SettingsSecurityEvent::SignOutAll);
    }

    fn copy_sp_field(&mut self, field: CopiedField, cx: &mut Context<Self>) {
        let text = match field {
            CopiedField::AudienceUri => self.values.audience_uri.as_ref(),
            CopiedField::AcsUrl => self.values.acs_url.as_ref(),
        };
        clipboard::write(text, cx).detach();
        self.copied_field = Some((field, Instant::now()));
        let msg = match field {
            CopiedField::AudienceUri => tr(cx, "Audience URI copied", "已复制 Audience URI"),
            CopiedField::AcsUrl => tr(cx, "ACS URL copied", "已复制 ACS URL"),
        };
        self.show_toast(msg, cx);
    }

    fn show_toast(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast = Some((msg.into(), Instant::now()));
        cx.notify();
    }

    fn emit_saved(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast = Some((msg.into(), Instant::now()));
        cx.emit(SettingsSecurityEvent::Save(self.values.clone()));
        cx.notify();
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.domain_editor.is_none() {
            let label: SharedString = tr(cx, "Domain name", "企业域名").into();
            let ed = cx.new(|e_cx| {
                let mut e = Editor::new(label, InputMode::Single, w, e_cx);
                e.set_placeholder("company.com");
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |this, ed_ref, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => {
                    this.domain_draft = ed_ref.read(cx).value().to_owned();
                    cx.notify();
                }
                EditEvent::Submitted => {
                    let raw = ed_ref.read(cx).value().to_owned();
                    this.add_domain(&raw, None, cx);
                }
                EditEvent::Escape => {
                    this.adding_domain = false;
                    this.domain_draft.clear();
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.domain_editor = Some(ed);
        }

        if self.range_editor.is_none() {
            let label: SharedString = tr(cx, "CIDR IP range", "CIDR 网段").into();
            let ed = cx.new(|e_cx| {
                let mut e = Editor::new(label, InputMode::Single, w, e_cx);
                e.set_placeholder("203.0.113.0/24");
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |this, ed_ref, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => {
                    this.range_draft = ed_ref.read(cx).value().to_owned();
                    cx.notify();
                }
                EditEvent::Submitted => {
                    let raw = ed_ref.read(cx).value().to_owned();
                    this.add_ip_range(&raw, None, cx);
                }
                EditEvent::Escape => {
                    this.adding_range = false;
                    this.range_draft.clear();
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.range_editor = Some(ed);
        }

        if self.metadata_editor.is_none() {
            let label: SharedString = tr(cx, "Metadata URL", "元数据 URL").into();
            let initial = self.values.metadata_url.to_string();
            let ed = cx.new(|e_cx| {
                let mut e = Editor::new(label, InputMode::Single, w, e_cx);
                e.set_placeholder("https://idp.example.com/metadata");
                e.font_size = 13.;
                if !initial.is_empty() {
                    e.set_value(initial, w, e_cx);
                }
                e
            });
            cx.subscribe(&ed, |this, ed_ref, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => {
                    cx.notify();
                }
                EditEvent::Submitted | EditEvent::Blurred => {
                    let val = ed_ref.read(cx).value().to_owned();
                    this.commit_metadata_url(val, cx);
                }
                _ => {}
            })
            .detach();
            self.metadata_editor = Some(ed);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_empty_block(
        &self,
        icon: ProIcon,
        title: &'static str,
        caption: &'static str,
        btn_id_suffix: &'static str,
        btn_label: &'static str,
        frame_bg: Hsla,
        p: &ProPalette,
        cx: &mut Context<Self>,
        on_action: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + Clone + 'static,
    ) -> Div {
        div()
            .mt(px(12.))
            .py(px(48.))
            .px(px(24.))
            .rounded(px(12.))
            .bg(frame_bg)
            .border_1()
            .border_color(p.line)
            .flex()
            .flex_col()
            .items_center()
            .text_center()
            .child(
                div()
                    .size(px(42.))
                    .rounded(px(11.))
                    .bg(p.chip)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_icon(icon, 18., p.muted)),
            )
            .child(
                div()
                    .mt(px(16.))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(title),
            )
            .child(
                div()
                    .mt(px(4.))
                    .max_w(px(430.))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.muted)
                    .child(caption),
            )
            .child(div().mt(px(18.)).child(pro_btn(
                pro_eid(&self.id, btn_id_suffix),
                btn_label,
                Some(ProIcon::Plus),
                ProBtnStyle::Secondary,
                true,
                cx,
                on_action,
            )))
    }

    fn render_session_dropdown(
        &self,
        is_open: bool,
        w: &Window,
        p: &ProPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let rect = self.session_bounds.get();
        let menu_w = f32::from(rect.size.width).max(180.);
        let est_h = SessionDuration::ALL.len() as f32 * 32. + 8.;
        let (pos, side) = self
            .session_bounds
            .place(w.viewport_size(), px(est_h), px(5.));
        let anchor_copy = self.session_bounds.clone();
        let menu_bg: Hsla = if p.dark {
            rgb(0x1f1f22).into()
        } else {
            rgb(0xffffff).into()
        };

        let mut list = div()
            .id(pro_eid(&self.id, "session-listbox"))
            .role(Role::ListBox)
            .flex()
            .flex_col()
            .gap(px(1.));

        for (idx, &dur) in SessionDuration::ALL.iter().enumerate() {
            let selected = self.values.session_length == dur;
            let highlighted = self.session_highlight == idx;
            let label: SharedString = dur.label_tr(cx).into();
            let hover_bg = p.hover;
            list = list.child(
                div()
                    .id(pro_eid(&self.id, format!("sess-opt-{idx}")))
                    .role(Role::ListBoxOption)
                    .aria_label(label.clone())
                    .aria_selected(selected)
                    .min_h(px(32.))
                    .px(px(10.))
                    .py(px(5.))
                    .rounded(px(6.))
                    .bg(if highlighted {
                        p.hover
                    } else {
                        transparent_black()
                    })
                    .hover(move |s| s.bg(hover_bg))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, win, app_cx| {
                        win.prevent_default();
                        app_cx.stop_propagation();
                    })
                    .on_hover(cx.listener(move |this, hovered: &bool, _, h_cx| {
                        if *hovered && this.session_highlight != idx {
                            this.session_highlight = idx;
                            h_cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |this, _, _, c_cx| {
                        this.set_session_length(dur, c_cx);
                        c_cx.stop_propagation();
                    }))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(if selected {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(p.ink)
                            .child(label),
                    )
                    .when(selected, |d| {
                        d.child(pro_svg("0 0 24 24", CHECK_SVG, 14., p.ink))
                    }),
            );
        }

        let popover_inner = div()
            .id(pro_eid(&self.id, "session-popover"))
            .when(is_open, |d| d.occlude())
            .w(px(menu_w))
            .p(px(4.))
            .rounded(px(10.))
            .bg(menu_bg)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .when(is_open, |d| {
                d.on_mouse_down_out(cx.listener(move |this, ev: &MouseDownEvent, _, o_cx| {
                    if !anchor_copy.get().contains(&ev.position) {
                        this.session_menu_open = false;
                        o_cx.notify();
                    }
                }))
            })
            .child(list)
            .into_any_element();

        let pres_wrap_id = pro_eid(&self.id, "session-popover-wrap");
        pro_presence(
            pro_eid(&self.id, "session-popover-pres"),
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
                                .id(pres_wrap_id.clone())
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
        let field_w = if narrow { 200. } else { 240. };
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
                    .child(tr(cx, "Security", "安全")),
            )
            .child(
                div()
                    .mt(px(4.))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.text)
                    .child(tr(
                        cx,
                        "Decide who can sign in to your workspace and how.",
                        "管理谁可以登录你的工作区以及采用何种身份验证方式。",
                    )),
            )
            // Email domains skeleton
            .child(
                div()
                    .mt(px(36.))
                    .flex()
                    .items_end()
                    .justify_between()
                    .gap(px(24.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(pro_skeleton_bar(106., 14., 4., phase, cx))
                            .child(pro_skeleton_bar(280., 12., 4., phase + 0.04, cx)),
                    )
                    .child(pro_skeleton_bar(96., 28., 6., phase + 0.08, cx)),
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
                    .children((0..2usize).map(|idx| {
                        let p_off = phase + 0.12 + idx as f32 * 0.06;
                        div()
                            .min_h(px(52.))
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
                                    .child(pro_skeleton_bar(78., 12., 4., p_off + 0.02, cx))
                                    .child(pro_skeleton_bar(56., 18., 9., p_off + 0.04, cx)),
                            )
                            .child(pro_skeleton_bar(52., 26., 6., p_off + 0.06, cx))
                    })),
            )
            .child(
                div()
                    .mt(px(8.))
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
                            .child(pro_skeleton_bar(96., 12., 4., phase + 0.28, cx))
                            .child(pro_skeleton_bar(236., 10., 4., phase + 0.32, cx)),
                    )
                    .child(pro_skeleton_bar(34., 20., 10., phase + 0.36, cx)),
            )
            // Sign-in skeleton
            .child(
                div()
                    .mt(px(36.))
                    .child(pro_skeleton_bar(68., 14., 4., phase + 0.4, cx)),
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
                        let p_off = phase + 0.44 + idx as f32 * 0.07;
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
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(pro_skeleton_bar(156., 12., 4., p_off, cx))
                                    .child(pro_skeleton_bar(242., 10., 4., p_off + 0.03, cx)),
                            )
                            .child(if idx == 2 {
                                pro_skeleton_bar(160., 32., 7., p_off + 0.05, cx)
                            } else {
                                pro_skeleton_bar(34., 20., 10., p_off + 0.05, cx)
                            })
                    })),
            )
            // Identity provider skeleton
            .child(
                div()
                    .mt(px(36.))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(pro_skeleton_bar(116., 14., 4., phase + 0.68, cx))
                    .child(pro_skeleton_bar(320., 12., 4., phase + 0.72, cx)),
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
                        let p_off = phase + 0.76 + idx as f32 * 0.06;
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
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(pro_skeleton_bar(92., 12., 4., p_off, cx))
                                    .child(pro_skeleton_bar(136., 10., 4., p_off + 0.02, cx)),
                            )
                            .child(pro_skeleton_bar(field_w, 32., 7., p_off + 0.04, cx))
                    })),
            )
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;
        let now = Instant::now();

        if let Some((_, ts)) = &self.toast
            && now.duration_since(*ts).as_secs_f32() > 2.45
        {
            self.toast = None;
        }
        if let Some((_, ts)) = &self.copied_field
            && now.duration_since(*ts).as_secs_f32() > 1.8
        {
            self.copied_field = None;
        }

        if (self.loading || self.toast.is_some() || self.copied_field.is_some())
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
        let readonly_surface: Hsla = if p.dark {
            hsla(0., 0., 1., 0.03)
        } else {
            hsla(0., 0., 0., 0.02)
        };
        let readonly_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.12)
        } else {
            hsla(0., 0., 0., 0.08)
        };

        let body = if self.loading {
            self.render_loading_skeleton(narrow, phase, &p, cx)
        } else {
            let domain_focused = self
                .domain_editor
                .as_ref()
                .is_some_and(|e| e.read(cx).focus.is_focused(w));
            let range_focused = self
                .range_editor
                .as_ref()
                .is_some_and(|e| e.read(cx).focus.is_focused(w));
            let meta_focused = self
                .metadata_editor
                .as_ref()
                .is_some_and(|e| e.read(cx).focus.is_focused(w));

            // 1. Email domains + Joining
            let add_domain_head_btn = pro_btn(
                pro_eid(&self.id, "add-domain-btn"),
                tr(cx, "Add domain", "添加域名"),
                Some(ProIcon::Plus),
                ProBtnStyle::Secondary,
                true,
                cx,
                |this, w, cx| this.open_add_domain(Some(w), cx),
            );

            let is_domains_empty = self.values.domains.is_empty() && !self.adding_domain;
            let domains_inner: AnyElement = if is_domains_empty {
                self.render_empty_block(
                    ProIcon::Globe,
                    tr(cx, "No domains yet", "尚未配置企业域名"),
                    tr(
                        cx,
                        "Verify a company domain to unlock joining by domain and single sign-on.",
                        "验证公司邮箱域名即可解锁凭域名自动加入与单点登录。",
                    ),
                    "empty-add-domain-btn",
                    tr(cx, "Add domain", "添加域名"),
                    frame_bg,
                    &p,
                    cx,
                    |this, w, cx| this.open_add_domain(Some(w), cx),
                )
                .into_any_element()
            } else {
                let mut dom_card = div()
                    .mt(px(12.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col();

                let dom_snapshot = self.values.domains.clone();
                for (idx, dom) in dom_snapshot.iter().enumerate() {
                    let d_verify = dom.domain.clone();
                    let d_remove = dom.domain.clone();
                    let d_name = dom.domain.clone();
                    let verified = dom.verified;
                    let remove_aria = format!("{} {}", tr(cx, "Remove", "移除域名："), d_name);

                    let badge = if verified {
                        div()
                            .px(px(7.))
                            .py(px(1.))
                            .rounded_full()
                            .bg(p.green_soft)
                            .text_size(px(11.5))
                            .line_height(px(18.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.green_ink)
                            .child(tr(cx, "Verified", "已验证"))
                    } else {
                        div()
                            .px(px(7.))
                            .py(px(1.))
                            .rounded_full()
                            .bg(p.chip)
                            .text_size(px(11.5))
                            .line_height(px(18.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted)
                            .child(tr(cx, "Pending", "待验证"))
                    };

                    let verify_btn = if !verified {
                        Some(pro_btn(
                            pro_eid(&self.id, format!("verify-dom-{}", dom.domain)),
                            tr(cx, "Verify", "验证"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            move |this, _, cx| {
                                this.verify_domain(d_verify.as_ref(), cx);
                            },
                        ))
                    } else {
                        None
                    };

                    let d_hover = dom.domain.clone();
                    let row_hovered = self.hovered_domain.as_ref() == Some(&dom.domain);
                    let remove_btn = activate(
                        div()
                            .id(pro_eid(&self.id, format!("remove-dom-{}", dom.domain)))
                            .role(Role::Button)
                            .aria_label(remove_aria)
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
                            this.remove_domain(d_remove.as_ref(), cx);
                        },
                    );

                    dom_card = dom_card.child(
                        div()
                            .id(pro_eid(&self.id, format!("dom-row-{}", dom.domain)))
                            .on_hover(cx.listener(move |this, hovered: &bool, _, h_cx| {
                                if *hovered {
                                    if this.hovered_domain.as_ref() != Some(&d_hover) {
                                        this.hovered_domain = Some(d_hover.clone());
                                        h_cx.notify();
                                    }
                                } else if this.hovered_domain.as_ref() == Some(&d_hover) {
                                    this.hovered_domain = None;
                                    h_cx.notify();
                                }
                            }))
                            .min_h(px(52.))
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
                                            .child(pro_icon(ProIcon::Globe, 15., p.muted)),
                                    )
                                    .child(
                                        div()
                                            .truncate()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .child(d_name),
                                    )
                                    .child(badge),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .when_some(verify_btn, |d, btn| d.child(btn))
                                    .child(remove_btn),
                            ),
                    );
                }

                let draft_val = self.domain_draft.clone();
                let add_dom_row = div()
                    .py(px(10.))
                    .pl(px(if narrow { 14. } else { 16. }))
                    .pr(px(if narrow { 14. } else { 12. }))
                    .when(!dom_snapshot.is_empty(), |d| {
                        d.border_t_1().border_color(p.line)
                    })
                    .flex()
                    .when(narrow, |d| d.flex_col().items_stretch().gap(px(8.)))
                    .when(!narrow, |d| d.items_center().justify_between().gap(px(12.)))
                    .child(
                        div()
                            .flex_1()
                            .when(!narrow, |d| d.max_w(px(320.)))
                            .h(px(32.))
                            .px(px(10.))
                            .rounded(px(7.))
                            .bg(input_surface)
                            .border_1()
                            .border_color(if domain_focused {
                                p.line_strong
                            } else {
                                input_border
                            })
                            .flex()
                            .items_center()
                            .when_some(self.domain_editor.clone(), |d, ed| {
                                d.child(div().w_full().child(ed))
                            }),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(pro_btn(
                                pro_eid(&self.id, "cancel-add-domain"),
                                tr(cx, "Cancel", "取消"),
                                None,
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                |this, w, cx| this.cancel_add_domain(Some(w), cx),
                            ))
                            .child(pro_btn(
                                pro_eid(&self.id, "submit-add-domain"),
                                tr(cx, "Add", "添加"),
                                None,
                                ProBtnStyle::Primary,
                                true,
                                cx,
                                move |this, w, cx| {
                                    this.add_domain(&draft_val, Some(w), cx);
                                },
                            )),
                    )
                    .into_any_element();
                dom_card = dom_card.child(pro_fold(
                    pro_eid(&self.id, "add-domain-fold"),
                    self.adding_domain,
                    54.,
                    0.,
                    move || add_dom_row,
                ));

                let has_pending_domain = self.values.has_pending_domain();
                let token = self.values.dns_verify_token.clone();
                let dns_hint_row = div()
                    .pt(px(10.))
                    .pb(px(12.))
                    .px(px(16.))
                    .border_t_1()
                    .border_color(p.line)
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .child(div().mt(px(2.)).flex_none().child(pro_svg(
                        "0 0 24 24",
                        INFO_CIRCLE_SVG,
                        14.,
                        p.faint,
                    )))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(5.))
                            .text_size(px(12.5))
                            .line_height(px(18.))
                            .text_color(p.muted)
                            .child(tr(cx, "Add a TXT record", "在域名 DNS 中添加 TXT 记录"))
                            .child(
                                div()
                                    .px(px(5.))
                                    .py(px(1.))
                                    .rounded(px(5.))
                                    .bg(p.chip)
                                    .font_family("Menlo")
                                    .text_size(px(11.5))
                                    .text_color(p.ink)
                                    .child(token),
                            )
                            .child(tr(
                                cx,
                                "to the domain's DNS, then press Verify.",
                                "，完成后点击“验证”。",
                            )),
                    )
                    .into_any_element();
                dom_card = dom_card.child(pro_fold(
                    pro_eid(&self.id, "dns-txt-fold"),
                    has_pending_domain,
                    46.,
                    0.,
                    move || dns_hint_row,
                ));

                dom_card.into_any_element()
            };
            let domains_block = pro_view_transition(
                pro_eid(&self.id, "domains-block-vtx"),
                if is_domains_empty { "empty" } else { "card" },
                domains_inner,
            );

            let has_verified = self.values.has_verified_domain();
            let domain_unlock_alpha = pro_spring_f32(
                pro_eid(&self.id, "domain-unlock-opacity"),
                if has_verified { 1.0 } else { 0.5 },
                0.24,
                w,
                cx,
            );
            let join_on = self.values.join_by_domain && has_verified;
            let join_sub = if has_verified {
                tr(
                    cx,
                    "Anyone with a verified address joins as a member without an invite",
                    "拥有已验证域名邮箱的成员无需邀请即可自动加入工作区",
                )
            } else {
                tr(
                    cx,
                    "Verify a domain above to let members join without an invite",
                    "请先在上方验证企业域名，以允许成员免邀请自动加入",
                )
            };

            let joining_card = div()
                .mt(px(8.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .min_h(px(60.))
                .py(px(10.))
                .pl(px(if narrow { 14. } else { 16. }))
                .pr(px(if narrow { 14. } else { 12. }))
                .opacity(domain_unlock_alpha)
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
                                .child(tr(cx, "Join by domain", "凭域名自动加入")),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(join_sub),
                        ),
                )
                .child(pro_switch(
                    pro_eid(&self.id, "join-by-domain-sw"),
                    tr(cx, "Join by domain", "凭域名自动加入"),
                    join_on,
                    cx,
                    |this, _, cx| {
                        this.toggle_join_by_domain(cx);
                    },
                ));

            // 2. Sign-in card
            let sso_on = self.values.sso_required && has_verified;
            let tfa_on = self.values.two_factor_required;
            let sso_sub = if has_verified {
                tr(
                    cx,
                    "Members at a verified domain sign in through your identity provider",
                    "已验证域名下的成员必须通过企业身份提供商单点登录",
                )
            } else {
                tr(
                    cx,
                    "Verify a domain above to require SAML single sign-on",
                    "请先在上方验证企业域名，再开启强制 SAML 单点登录",
                )
            };

            let sess_open = self.session_menu_open;
            let sess_bounds = self.session_bounds.clone();
            let sess_label: SharedString = self.values.session_length.label_tr(cx).into();
            let sess_trigger = activate(
                div()
                    .id(pro_eid(&self.id, "session-length-select"))
                    .role(Role::ComboBox)
                    .aria_label(tr(cx, "Session length", "会话有效期"))
                    .aria_expanded(sess_open)
                    .tab_index(0)
                    .relative()
                    .when(narrow, |d| d.w_full())
                    .when(!narrow, |d| d.w(px(160.)).flex_none())
                    .h(px(32.))
                    .pl(px(10.))
                    .pr(px(8.))
                    .rounded(px(7.))
                    .bg(input_surface)
                    .border_1()
                    .border_color(if sess_open {
                        p.line_strong
                    } else {
                        input_border
                    })
                    .hover(move |s| s.border_color(p.line_strong))
                    .focus_visible(move |s| s.border_1().border_color(p.focus))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .cursor_pointer()
                    .child(pro_icon(ProIcon::Clock, 16., p.muted))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(13.))
                            .text_color(p.ink)
                            .child(sess_label),
                    )
                    .child(pro_icon(
                        if sess_open {
                            ProIcon::ChevronUp
                        } else {
                            ProIcon::ChevronDown
                        },
                        14.,
                        p.muted,
                    ))
                    .child(sess_bounds.measure()),
                cx,
                |this, _, cx| {
                    this.session_menu_open = !this.session_menu_open;
                    if this.session_menu_open {
                        this.session_highlight = SessionDuration::ALL
                            .iter()
                            .position(|&d| d == this.values.session_length)
                            .unwrap_or(2);
                    }
                    cx.notify();
                },
            );

            let tfa_note_row = div()
                .py(px(8.))
                .pl(px(if narrow { 14. } else { 16. }))
                .pr(px(if narrow { 14. } else { 12. }))
                .border_t_1()
                .border_color(p.line)
                .bg(frame_bg)
                .flex()
                .items_center()
                .gap(px(8.))
                .child(pro_icon(ProIcon::Shield, 13., p.green_ink))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(p.muted)
                        .child(tr(
                            cx,
                            "TOTP authenticator codes (6-digit RFC 6238) and recovery keys are enforced on next sign-in.",
                            "下次登录起将强制校验身份验证器 6 位动态口令 (TOTP) 与备用恢复密钥。",
                        )),
                )
                .into_any_element();

            let signin_card = div()
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
                        .opacity(domain_unlock_alpha)
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
                                            "Require single sign-on (SAML)",
                                            "强制要求单点登录 (SAML)",
                                        )),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .child(sso_sub),
                                ),
                        )
                        .child(pro_switch(
                            pro_eid(&self.id, "sso-required-sw"),
                            tr(
                                cx,
                                "Require single sign-on (SAML)",
                                "强制要求单点登录 (SAML)",
                            ),
                            sso_on,
                            cx,
                            |this, _, cx| {
                                this.toggle_sso_required(cx);
                            },
                        )),
                )
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
                                        .child(tr(cx, "Two-step verification", "两步验证")),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .child(tr(
                                            cx,
                                            "Everyone without SSO adds a code from an authenticator app",
                                            "未使用单点登录的成员需输入身份验证器动态验证码",
                                        )),
                                ),
                        )
                        .child(pro_switch(
                            pro_eid(&self.id, "two-step-sw"),
                            tr(cx, "Two-step verification", "两步验证"),
                            tfa_on,
                            cx,
                            |this, _, cx| this.toggle_two_factor(cx),
                        )),
                )
                .child(pro_fold(
                    pro_eid(&self.id, "tfa-note-fold"),
                    tfa_on,
                    36.,
                    0.,
                    move || tfa_note_row,
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
                                        .child(tr(cx, "Session length", "会话有效期")),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .child(tr(
                                            cx,
                                            "How long people stay signed in without activity",
                                            "成员在无操作状态下保持登录的时长",
                                        )),
                                ),
                        )
                        .child(sess_trigger),
                );

            // 3. Identity provider section (smoothly folded when SSO is toggled)
            let aud_copied = self
                .copied_field
                .as_ref()
                .is_some_and(|(f, _)| *f == CopiedField::AudienceUri);
            let acs_copied = self
                .copied_field
                .as_ref()
                .is_some_and(|(f, _)| *f == CopiedField::AcsUrl);

            let render_copy_btn =
                |suffix: &'static str, field: CopiedField, copied: bool, cx: &mut Context<Self>| {
                    activate(
                        div()
                            .id(pro_eid(&self.id, suffix))
                            .role(Role::Button)
                            .aria_label(tr(cx, "Copy value", "复制地址"))
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
                            .text_color(if copied { p.green_ink } else { p.ink })
                            .cursor_pointer()
                            .child(if copied {
                                pro_svg("0 0 24 24", CHECK_SVG, 14., p.green_ink)
                            } else {
                                pro_icon(ProIcon::Copy, 14., p.muted)
                            })
                            .child(if copied {
                                tr(cx, "Copied", "已复制")
                            } else {
                                tr(cx, "Copy", "复制")
                            }),
                        cx,
                        move |this, _, cx| this.copy_sp_field(field, cx),
                    )
                };

            let aud_copy_btn =
                render_copy_btn("copy-audience", CopiedField::AudienceUri, aud_copied, cx);
            let acs_copy_btn = render_copy_btn("copy-acs", CopiedField::AcsUrl, acs_copied, cx);
            let aud_val = self.values.audience_uri.clone();
            let acs_val = self.values.acs_url.clone();

            let idp_inner = div()
                .pt(px(36.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(tr(cx, "Identity provider", "身份提供商 (IdP)")),
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
                            "Copy these into Okta, Entra ID or Google Workspace, then paste the metadata URL they give you.",
                            "将以下服务提供商参数填入 Okta、Entra ID 或 Google Workspace，随后粘贴对方提供的元数据 URL。",
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
                                // Audience URI
                                .child(
                                    div()
                                        .min_h(px(60.))
                                        .py(px(if narrow { 12. } else { 10. }))
                                        .pl(px(if narrow { 14. } else { 16. }))
                                        .pr(px(if narrow { 14. } else { 12. }))
                                        .flex()
                                        .when(narrow, |d| {
                                            d.flex_col().items_stretch().gap(px(10.))
                                        })
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
                                                        .child("Audience URI"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .line_height(px(16.))
                                                        .text_color(p.muted)
                                                        .child("SP entity ID"),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .flex_none()
                                                .flex()
                                                .items_center()
                                                .gap(px(8.))
                                                .child(
                                                    div()
                                                        .when(narrow, |d| d.flex_1().min_w_0())
                                                        .when(!narrow, |d| d.w(px(220.)))
                                                        .h(px(32.))
                                                        .px(px(10.))
                                                        .rounded(px(7.))
                                                        .bg(readonly_surface)
                                                        .border_1()
                                                        .border_color(readonly_border)
                                                        .flex()
                                                        .items_center()
                                                        .child(
                                                            div()
                                                                .w_full()
                                                                .truncate()
                                                                .text_size(px(13.))
                                                                .text_color(p.ink)
                                                                .child(aud_val),
                                                        ),
                                                )
                                                .child(aud_copy_btn),
                                        ),
                                )
                                // ACS URL
                                .child(
                                    div()
                                        .min_h(px(60.))
                                        .py(px(if narrow { 12. } else { 10. }))
                                        .pl(px(if narrow { 14. } else { 16. }))
                                        .pr(px(if narrow { 14. } else { 12. }))
                                        .border_t_1()
                                        .border_color(p.line)
                                        .flex()
                                        .when(narrow, |d| {
                                            d.flex_col().items_stretch().gap(px(10.))
                                        })
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
                                                        .child("ACS URL"),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .line_height(px(16.))
                                                        .text_color(p.muted)
                                                        .child(tr(
                                                            cx,
                                                            "Assertion consumer service",
                                                            "断言消费服务地址",
                                                        )),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .flex_none()
                                                .flex()
                                                .items_center()
                                                .gap(px(8.))
                                                .child(
                                                    div()
                                                        .when(narrow, |d| d.flex_1().min_w_0())
                                                        .when(!narrow, |d| d.w(px(220.)))
                                                        .h(px(32.))
                                                        .px(px(10.))
                                                        .rounded(px(7.))
                                                        .bg(readonly_surface)
                                                        .border_1()
                                                        .border_color(readonly_border)
                                                        .flex()
                                                        .items_center()
                                                        .child(
                                                            div()
                                                                .w_full()
                                                                .truncate()
                                                                .text_size(px(13.))
                                                                .text_color(p.ink)
                                                                .child(acs_val),
                                                        ),
                                                )
                                                .child(acs_copy_btn),
                                        ),
                                )
                                // Metadata URL
                                .child(
                                    div()
                                        .min_h(px(60.))
                                        .py(px(if narrow { 12. } else { 10. }))
                                        .pl(px(if narrow { 14. } else { 16. }))
                                        .pr(px(if narrow { 14. } else { 12. }))
                                        .border_t_1()
                                        .border_color(p.line)
                                        .flex()
                                        .when(narrow, |d| {
                                            d.flex_col().items_stretch().gap(px(10.))
                                        })
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
                                                        .child(tr(cx, "Metadata URL", "元数据 URL")),
                                                )
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .line_height(px(16.))
                                                        .text_color(p.muted)
                                                        .child(tr(
                                                            cx,
                                                            "From your identity provider",
                                                            "由企业身份提供商提供",
                                                        )),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .when(narrow, |d| d.w_full())
                                                .when(!narrow, |d| d.w(px(260.)).flex_none())
                                                .h(px(32.))
                                                .px(px(10.))
                                                .rounded(px(7.))
                                                .bg(input_surface)
                                                .border_1()
                                                .border_color(if meta_focused {
                                                    p.line_strong
                                                } else {
                                                    input_border
                                                })
                                                .flex()
                                                .items_center()
                                                .when_some(
                                                    self.metadata_editor.clone(),
                                                    |d, ed| d.child(div().w_full().child(ed)),
                                                ),
                                        ),
                                ),
                        )
                        .into_any_element();
            let idp_section = pro_fold(
                pro_eid(&self.id, "idp-section-fold"),
                sso_on,
                272.,
                0.,
                move || idp_inner,
            );

            // 4. IP allowlist section
            let add_range_head_btn = pro_btn(
                pro_eid(&self.id, "add-range-btn"),
                tr(cx, "Add range", "添加网段"),
                Some(ProIcon::Plus),
                ProBtnStyle::Secondary,
                true,
                cx,
                |this, w, cx| this.open_add_range(Some(w), cx),
            );

            let is_ranges_empty = self.values.ip_ranges.is_empty() && !self.adding_range;
            let ranges_inner: AnyElement = if is_ranges_empty {
                self.render_empty_block(
                    ProIcon::Shield,
                    tr(cx, "No IP ranges", "暂无 IP 白名单"),
                    tr(
                        cx,
                        "Every network can reach the workspace until you add a range.",
                        "在添加白名单网段之前，所有网络均可访问该工作区。",
                    ),
                    "empty-add-range-btn",
                    tr(cx, "Add range", "添加网段"),
                    frame_bg,
                    &p,
                    cx,
                    |this, w, cx| this.open_add_range(Some(w), cx),
                )
                .into_any_element()
            } else {
                let mut range_card = div()
                    .mt(px(12.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col();

                let ranges_snapshot = self.values.ip_ranges.clone();
                for (idx, item) in ranges_snapshot.iter().enumerate() {
                    let cidr_remove = item.cidr.clone();
                    let cidr_label = item.cidr.clone();
                    let note_opt = item.label.clone();
                    let rem_aria = format!("{} {}", tr(cx, "Remove", "移除网段："), cidr_label);

                    let row_hovered = self.hovered_range == Some(idx);
                    let remove_btn = activate(
                        div()
                            .id(pro_eid(&self.id, format!("remove-range-{idx}")))
                            .role(Role::Button)
                            .aria_label(rem_aria)
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
                            this.remove_ip_range(cidr_remove.as_ref(), cx);
                        },
                    );

                    range_card = range_card.child(
                        div()
                            .id(pro_eid(&self.id, format!("range-row-{idx}")))
                            .on_hover(cx.listener(move |this, hovered: &bool, _, h_cx| {
                                if *hovered {
                                    if this.hovered_range != Some(idx) {
                                        this.hovered_range = Some(idx);
                                        h_cx.notify();
                                    }
                                } else if this.hovered_range == Some(idx) {
                                    this.hovered_range = None;
                                    h_cx.notify();
                                }
                            }))
                            .min_h(px(52.))
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
                                            .child(pro_icon(ProIcon::Shield, 15., p.muted)),
                                    )
                                    .child(
                                        div()
                                            .font_family("Menlo")
                                            .text_size(px(12.5))
                                            .text_color(p.strong)
                                            .child(cidr_label),
                                    )
                                    .when_some(note_opt, |d, lbl| {
                                        d.child(
                                            div()
                                                .px(px(7.))
                                                .py(px(1.))
                                                .rounded_full()
                                                .bg(p.chip)
                                                .text_size(px(11.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(p.muted)
                                                .child(lbl),
                                        )
                                    }),
                            )
                            .child(remove_btn),
                    );
                }

                let range_val = self.range_draft.clone();
                let add_range_row = div()
                    .py(px(10.))
                    .pl(px(if narrow { 14. } else { 16. }))
                    .pr(px(if narrow { 14. } else { 12. }))
                    .when(!ranges_snapshot.is_empty(), |d| {
                        d.border_t_1().border_color(p.line)
                    })
                    .flex()
                    .when(narrow, |d| d.flex_col().items_stretch().gap(px(8.)))
                    .when(!narrow, |d| d.items_center().justify_between().gap(px(12.)))
                    .child(
                        div()
                            .flex_1()
                            .when(!narrow, |d| d.max_w(px(320.)))
                            .h(px(32.))
                            .px(px(10.))
                            .rounded(px(7.))
                            .bg(input_surface)
                            .border_1()
                            .border_color(if range_focused {
                                p.line_strong
                            } else {
                                input_border
                            })
                            .flex()
                            .items_center()
                            .when_some(self.range_editor.clone(), |d, ed| {
                                d.child(div().w_full().child(ed))
                            }),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(pro_btn(
                                pro_eid(&self.id, "cancel-add-range"),
                                tr(cx, "Cancel", "取消"),
                                None,
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                |this, w, cx| this.cancel_add_range(Some(w), cx),
                            ))
                            .child(pro_btn(
                                pro_eid(&self.id, "submit-add-range"),
                                tr(cx, "Add", "添加"),
                                None,
                                ProBtnStyle::Primary,
                                true,
                                cx,
                                move |this, w, cx| {
                                    this.add_ip_range(&range_val, Some(w), cx);
                                },
                            )),
                    )
                    .into_any_element();
                range_card = range_card.child(pro_fold(
                    pro_eid(&self.id, "add-range-fold"),
                    self.adding_range,
                    54.,
                    0.,
                    move || add_range_row,
                ));

                range_card.into_any_element()
            };
            let ranges_block = pro_view_transition(
                pro_eid(&self.id, "ranges-block-vtx"),
                if is_ranges_empty { "empty" } else { "card" },
                ranges_inner,
            );

            // 5. Danger zone section
            let sign_out_trigger = activate(
                div()
                    .id(pro_eid(&self.id, "sign-out-everyone-btn"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Sign out everyone", "强制退出所有人"))
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
                    .child(tr(cx, "Sign out everyone", "退出全部成员"))
                    .child(pro_svg("0 0 24 24", SIGN_OUT_SVG, 15., p.red)),
                cx,
                |this, _, cx| {
                    if this.sign_out_confirm_open {
                        this.cancel_sign_out_confirm(cx);
                    } else {
                        this.open_sign_out_confirm(cx);
                    }
                },
            );

            let confirm_solid = activate(
                div()
                    .id(pro_eid(&self.id, "confirm-sign-out-all-btn"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Confirm sign out everyone", "确认退出全部成员"))
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
                    .child(tr(cx, "Sign out now", "确认退出")),
                cx,
                |this, _, cx| this.confirm_sign_out_all(cx),
            );

            let sign_out_confirm_row = div()
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
                            "Sign everyone out of this workspace right now? Running agents finish, and everyone signs in again.",
                            "确定立即退出工作区内所有成员吗？运行中的智能体将继续完成，所有浏览器与客户端需重新登录。",
                        )),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_btn(
                            pro_eid(&self.id, "cancel-sign-out-all-btn"),
                            tr(cx, "Cancel", "取消"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            |this, _, cx| this.cancel_sign_out_confirm(cx),
                        ))
                        .child(confirm_solid),
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
                                        .child(tr(cx, "Sign out everyone", "退出全部成员")),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .child(tr(
                                            cx,
                                            "Ends every session in the workspace, yours included. People sign in again.",
                                            "立即终止工作区内的全部会话（含你自己），所有成员需重新登录。",
                                        )),
                                ),
                        )
                        .child(sign_out_trigger),
                )
                .child(pro_fold(
                    pro_eid(&self.id, "sign-out-all-fold"),
                    self.sign_out_confirm_open,
                    64.,
                    0.,
                    move || sign_out_confirm_row,
                ));

            let posture_score = usize::from(has_verified)
                + usize::from(sso_on)
                + usize::from(tfa_on)
                + usize::from(!self.values.ip_ranges.is_empty());
            let posture_ratio = posture_score as f32 / 4.0;
            let posture_bar = div()
                .mt(px(16.))
                .py(px(10.))
                .px(px(14.))
                .rounded(px(10.))
                .bg(frame_bg)
                .border_1()
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
                        .items_center()
                        .gap(px(8.))
                        .child(pro_icon(
                            ProIcon::Shield,
                            14.,
                            if posture_score >= 3 {
                                p.green_ink
                            } else {
                                p.muted
                            },
                        ))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .font_features(FontFeatures(std::sync::Arc::new(vec![(
                                    "tnum".into(),
                                    1,
                                )])))
                                .child(format!(
                                    "{}: {posture_score}/4",
                                    tr(cx, "Workspace security posture", "工作区安全防护等级")
                                )),
                        ),
                )
                .child(
                    div()
                        .w(px(if narrow { 84. } else { 128. }))
                        .flex_none()
                        .child(pro_spring_bar(
                            pro_eid(&self.id, "security-posture-bar"),
                            posture_ratio,
                            4.,
                            2.,
                            p.chip,
                            if posture_score >= 3 {
                                p.green_ink
                            } else {
                                p.ink.opacity(0.65)
                            },
                        )),
                );

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
                        .child(tr(cx, "Security", "安全")),
                )
                .child(
                    div()
                        .mt(px(4.))
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.text)
                        .child(tr(
                            cx,
                            "Decide who can sign in to your workspace and how.",
                            "管理谁可以登录你的工作区以及采用何种身份验证方式。",
                        )),
                )
                .child(posture_bar)
                // Email domains + Joining
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
                                                .child(tr(cx, "Email domains", "企业邮箱域名")),
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
                                                    "Verify the domains your team signs in with. Verified domains unlock joining by domain and single sign-on.",
                                                    "验证团队成员登录所用的企业邮箱域名。验证通过后即可开启凭域名自动加入与单点登录。",
                                                )),
                                        ),
                                )
                                .child(add_domain_head_btn),
                        )
                        .child(domains_block)
                        .child(joining_card),
                )
                // Sign-in
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
                                .child(tr(cx, "Sign-in", "登录策略")),
                        )
                        .child(signin_card),
                )
                // Identity provider
                .child(idp_section)
                // IP allowlist
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
                                                .child(tr(cx, "IP allowlist", "IP 白名单")),
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
                                                    "Only let people in from these ranges. With none, every network can reach the workspace.",
                                                    "仅允许来自指定网段的访问。未设置时，所有网络均可连接工作区。",
                                                )),
                                        ),
                                )
                                .child(add_range_head_btn),
                        )
                        .child(ranges_block),
                )
                // Danger zone
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

        let mut outer = div()
            .id(pro_eid(&self.id, "page-root"))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                let k = ev.keystroke.key.as_str();
                if this.session_menu_open {
                    let len = SessionDuration::ALL.len();
                    match k {
                        "escape" => {
                            this.session_menu_open = false;
                            w.prevent_default();
                            cx.stop_propagation();
                            cx.notify();
                        }
                        "down" | "j" => {
                            this.session_highlight = (this.session_highlight + 1) % len;
                            w.prevent_default();
                            cx.stop_propagation();
                            cx.notify();
                        }
                        "up" | "k" => {
                            this.session_highlight = (this.session_highlight + len - 1) % len;
                            w.prevent_default();
                            cx.stop_propagation();
                            cx.notify();
                        }
                        "enter" => {
                            if let Some(&dur) = SessionDuration::ALL.get(this.session_highlight) {
                                this.set_session_length(dur, cx);
                            }
                            w.prevent_default();
                            cx.stop_propagation();
                        }
                        _ => {}
                    }
                } else if k == "escape" {
                    if this.sign_out_confirm_open {
                        this.cancel_sign_out_confirm(cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    } else if this.adding_domain {
                        this.cancel_add_domain(Some(w), cx);
                        w.prevent_default();
                        cx.stop_propagation();
                    } else if this.adding_range {
                        this.cancel_add_range(Some(w), cx);
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
                        pro_eid(&self.id, "security-body-vtx"),
                        if self.loading { "loading" } else { "ready" },
                        body,
                    )),
            )
            .child(pro_toast_banner(self.toast.clone()));

        if !self.loading {
            outer = outer.child(self.render_session_dropdown(self.session_menu_open, w, &p, cx));
        }

        outer
    }
}

impl Render for SettingsSecurityScreen {
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
        IpAllowRange, SecurityDomain, SessionDuration, SettingsSecurityEvent,
        SettingsSecurityScreen,
    };
    use gpui::{AppContext as _, TestAppContext};
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn domain_and_cidr_validation_rules() {
        assert_eq!(
            SecurityDomain::normalize_and_validate(" https://Acme.CO.uk/ "),
            Some("acme.co.uk".to_owned())
        );
        assert_eq!(SecurityDomain::normalize_and_validate("localhost"), None);
        assert_eq!(SecurityDomain::normalize_and_validate("-bad.com"), None);

        assert_eq!(
            IpAllowRange::normalize_and_validate("203.0.113.0/24"),
            Some("203.0.113.0/24".to_owned())
        );
        assert_eq!(
            IpAllowRange::normalize_and_validate("10.0.0.5"),
            Some("10.0.0.5/32".to_owned())
        );
        assert_eq!(IpAllowRange::normalize_and_validate("999.0.0.1/24"), None);
        assert_eq!(IpAllowRange::normalize_and_validate("10.0.0.0/33"), None);
    }

    #[test]
    fn domain_verification_sso_lock_cidr_and_sign_out_workflow() {
        let app = TestAppContext::single();
        app.update(crate::init);

        let events: Rc<RefCell<Vec<SettingsSecurityEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let recorded = events.clone();

        let screen = app.update(|cx| cx.new(|cx| SettingsSecurityScreen::new("test-ss", cx)));

        app.update(|cx| {
            cx.subscribe(&screen, move |_, ev: &SettingsSecurityEvent, _| {
                recorded.borrow_mut().push(ev.clone());
            })
            .detach();
            screen.update(cx, |s, s_cx| {
                assert!(s.security().has_verified_domain());
                assert!(s.security().sso_required);

                // Removing the only verified domain disables join-by-domain and SAML SSO
                assert!(s.remove_domain("acme.com", s_cx));
                assert!(!s.security().has_verified_domain());
                assert!(!s.security().join_by_domain);
                assert!(!s.security().sso_required);
                assert!(!s.toggle_sso_required(s_cx));

                // Verifying acme.io unlocks SAML SSO again
                assert!(s.verify_domain("acme.io", s_cx));
                assert!(s.security().has_verified_domain());
                assert!(s.toggle_sso_required(s_cx));
                assert!(s.security().sso_required);

                // Session length, CIDR allowlist, and confirmed workspace sign-out
                s.set_session_length(SessionDuration::FourteenDays, s_cx);
                assert_eq!(s.security().session_length, SessionDuration::FourteenDays);

                assert!(s.add_ip_range("203.0.113.0/24", None, s_cx));
                assert_eq!(s.security().ip_ranges.len(), 1);

                s.open_sign_out_confirm(s_cx);
                assert!(s.is_sign_out_confirm_open());
                s.confirm_sign_out_all(s_cx);
                assert!(!s.is_sign_out_confirm_open());
            });
        });

        let emitted = events.borrow();
        assert!(emitted.iter().any(
            |e| matches!(e, SettingsSecurityEvent::VerifyDomain(d) if d.as_ref() == "acme.io")
        ));
        assert!(
            emitted
                .iter()
                .any(|e| matches!(e, SettingsSecurityEvent::SignOutAll))
        );
    }
}
