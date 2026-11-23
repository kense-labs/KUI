//! Personal profile settings screen with avatar, identity fields, time preferences, context, and password.
//!
//! Renders inside [`AppShell`](crate::components::app_shell::AppShell) with the `Settings`
//! sidebar preset (`Profile` active under `Personal`), providing avatar upload/removal,
//! first/last name and email editing, role/timezone/week-start comboboxes, a 500-character
//! personal context editor, and inline password setup.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_btn, pro_eid, pro_fold, pro_person_avatar, pro_presence,
            pro_skeleton_bar, pro_spring_f32, pro_toast_banner, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        flyout::TriggerBounds,
        pro_icons::{ProIcon, person_initials, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const MAX_PERSONAL_CONTEXT_CHARS: usize = 500;

const INFO_CIRCLE_SVG: &str = r#"<circle cx="12" cy="12" r="10" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M12 16V12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/><path d="M12.125 8.25H12M12.25 8.25C12.25 8.11193 12.1381 8 12 8C11.8619 8 11.75 8.11193 11.75 8.25C11.75 8.38807 11.8619 8.5 12 8.5C12.1381 8.5 12.25 8.38807 12.25 8.25Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"/>"#;
const CHECK_SVG: &str = r#"<path d="M5 14L8.5 17.5L19 6.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2.1"/>"#;

const ROLE_OPTIONS: &[(&str, &str)] = &[
    ("Product & design", "产品与设计"),
    ("Engineering", "研发工程"),
    ("Research & data", "研究与数据"),
    ("Marketing & sales", "市场与商务"),
    ("Operations", "运营支持"),
    ("Finance & legal", "财务与法务"),
    ("Founder & leadership", "创始人与管理"),
    ("Other", "其他"),
];

const TIMEZONE_OPTIONS: &[&str] = &[
    "Europe/Vienna",
    "Europe/London",
    "Europe/Berlin",
    "America/New_York",
    "America/Los_Angeles",
    "Asia/Tokyo",
    "Asia/Singapore",
    "UTC",
];

const AVATAR_PRESETS: &[&str] = &["ocean", "emerald", "amber", "violet"];

/// First day of the calendar week used for reminders, schedules, and reports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WeekStart {
    /// Week begins on Monday.
    #[default]
    Monday,
    /// Week begins on Sunday.
    Sunday,
    /// Week begins on Saturday.
    Saturday,
}

impl WeekStart {
    /// All supported week-start options in display order.
    pub const ALL: [Self; 3] = [Self::Monday, Self::Sunday, Self::Saturday];

    /// Canonical English label for this day.
    pub fn label(self) -> &'static str {
        match self {
            Self::Monday => "Monday",
            Self::Sunday => "Sunday",
            Self::Saturday => "Saturday",
        }
    }

    /// Localized label matching the active UI language.
    pub fn label_tr(self, cx: &App) -> &'static str {
        match self {
            Self::Monday => tr(cx, "Monday", "星期一"),
            Self::Sunday => tr(cx, "Sunday", "星期日"),
            Self::Saturday => tr(cx, "Saturday", "星期六"),
        }
    }
}

/// Editable personal profile values managed by [`SettingsProfileScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileValues {
    /// Given name.
    pub first_name: SharedString,
    /// Family name.
    pub last_name: SharedString,
    /// Account sign-in email address.
    pub email: SharedString,
    /// Optional work role used to tailor assistant suggestions.
    pub role: Option<SharedString>,
    /// IANA timezone identifier (`"Europe/Vienna"` by default).
    pub timezone: SharedString,
    /// First day of the calendar week.
    pub week_start: WeekStart,
    /// Personal assistant context up to 500 characters.
    pub context: SharedString,
    /// Optional uploaded avatar key.
    pub avatar: Option<SharedString>,
    /// Whether an account password has been configured in addition to email sign-in links.
    pub has_password: bool,
}

impl Default for ProfileValues {
    fn default() -> Self {
        Self {
            first_name: "Alex".into(),
            last_name: "Morgan".into(),
            email: "alex@acme.com".into(),
            role: None,
            timezone: "Europe/Vienna".into(),
            week_start: WeekStart::Monday,
            context: "".into(),
            avatar: None,
            has_password: false,
        }
    }
}

impl ProfileValues {
    /// Construct profile values with first name, last name, and email.
    pub fn new(
        first_name: impl Into<SharedString>,
        last_name: impl Into<SharedString>,
        email: impl Into<SharedString>,
    ) -> Self {
        Self {
            first_name: first_name.into(),
            last_name: last_name.into(),
            email: email.into(),
            ..Self::default()
        }
    }

    /// Set the optional work role.
    pub fn with_role(mut self, role: Option<SharedString>) -> Self {
        self.role = role;
        self
    }

    /// Set the IANA timezone identifier.
    pub fn with_timezone(mut self, timezone: impl Into<SharedString>) -> Self {
        self.timezone = timezone.into();
        self
    }

    /// Set the calendar week start day.
    pub fn with_week_start(mut self, week_start: WeekStart) -> Self {
        self.week_start = week_start;
        self
    }

    /// Set the personal assistant context string.
    pub fn with_context(mut self, context: impl Into<SharedString>) -> Self {
        self.context = context.into();
        self
    }

    /// Set the optional uploaded avatar identifier.
    pub fn with_avatar(mut self, avatar: Option<SharedString>) -> Self {
        self.avatar = avatar;
        self
    }

    /// Set whether an account password is already configured.
    pub fn with_has_password(mut self, has_password: bool) -> Self {
        self.has_password = has_password;
        self
    }

    /// Combine `first_name` and `last_name` into a trimmed full display name.
    pub fn full_name(&self) -> String {
        let first = self.first_name.trim();
        let last = self.last_name.trim();
        match (first.is_empty(), last.is_empty()) {
            (true, true) => "Alex Morgan".to_owned(),
            (false, true) => first.to_owned(),
            (true, false) => last.to_owned(),
            (false, false) => format!("{first} {last}"),
        }
    }

    /// Compute 1–2 uppercase initials from `first_name` and `last_name`.
    pub fn initials(&self) -> String {
        let full = self.full_name();
        let init = person_initials(&full);
        if init.is_empty() {
            "AM".to_owned()
        } else {
            init
        }
    }

    /// Return whether required fields are non-empty, email looks valid, and context fits within 500 characters.
    pub fn is_valid(&self) -> bool {
        let email = self.email.trim();
        !self.first_name.trim().is_empty()
            && email.contains('@')
            && !email.starts_with('@')
            && !email.ends_with('@')
            && self.context.chars().count() <= MAX_PERSONAL_CONTEXT_CHARS
    }
}

/// Events emitted by [`SettingsProfileScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsProfileEvent {
    /// Emitted with the committed [`ProfileValues`] whenever profile settings are saved.
    Save(ProfileValues),
    /// Emitted when an account password is added or updated.
    PasswordUpdated,
    /// Emitted when the `"Learn more"` documentation link in the subtitle is clicked.
    LearnMore,
    /// Emitted when a settings sidebar item or `"Back to app"` is activated.
    Navigate(SharedString),
}

impl EventEmitter<SettingsProfileEvent> for SettingsProfileScreen {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OpenMenu {
    Role,
    Timezone,
    WeekStart,
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

/// Personal profile settings screen (`settings-profile`).
pub struct SettingsProfileScreen {
    id: ElementId,
    focus: FocusHandle,
    shell: Entity<AppShell>,
    standalone: bool,
    loading: bool,
    workspace: SharedString,
    saved: ProfileValues,
    draft: ProfileValues,
    avatar_cycle: usize,
    open_menu: Option<OpenMenu>,
    last_menu: Option<OpenMenu>,
    menu_highlight: usize,
    role_bounds: TriggerBounds,
    tz_bounds: TriggerBounds,
    week_bounds: TriggerBounds,
    email_editing: bool,
    password_open: bool,
    password_draft: String,
    password_revealed: bool,
    toast: Option<(SharedString, Instant)>,
    started_at: Instant,
    page_width: Rc<Cell<f32>>,
    first_name_editor: Option<Entity<Editor>>,
    last_name_editor: Option<Entity<Editor>>,
    email_editor: Option<Entity<Editor>>,
    context_editor: Option<Entity<Editor>>,
    password_editor: Option<Entity<Editor>>,
}

impl Focusable for SettingsProfileScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl SettingsProfileScreen {
    /// Construct a new [`SettingsProfileScreen`] with sample `Alex Morgan` profile values.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let focus = cx.focus_handle();
        let saved = ProfileValues::default();
        let draft = saved.clone();
        let workspace: SharedString = "Acme".into();
        let full_name: SharedString = saved.full_name().into();
        let email = saved.email.clone();
        let ws_clone = workspace.clone();
        let this = cx.entity();

        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Settings, shell_cx)
                .with_groups(settings_nav_groups(shell_cx))
                .with_workspace(ws_clone)
                .with_user(full_name, email)
                .with_active("Profile")
                .with_work_area(false, false);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s
        });
        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                cx.emit(SettingsProfileEvent::Navigate(label.clone()));
            }
            AppShellEvent::Back => {
                cx.emit(SettingsProfileEvent::Navigate("Back to app".into()));
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
            workspace,
            saved,
            draft,
            avatar_cycle: 0,
            open_menu: None,
            last_menu: None,
            menu_highlight: 0,
            role_bounds: TriggerBounds::default(),
            tz_bounds: TriggerBounds::default(),
            week_bounds: TriggerBounds::default(),
            email_editing: false,
            password_open: false,
            password_draft: String::new(),
            password_revealed: false,
            toast: None,
            started_at: Instant::now(),
            page_width: Rc::new(Cell::new(720.)),
            first_name_editor: None,
            last_name_editor: None,
            email_editor: None,
            context_editor: None,
            password_editor: None,
        }
    }

    /// Override the initial [`ProfileValues`].
    pub fn with_profile(mut self, profile: ProfileValues, cx: &mut Context<Self>) -> Self {
        let full_name: SharedString = profile.full_name().into();
        let email = profile.email.clone();
        self.saved = profile.clone();
        self.draft = profile;
        self.shell.update(cx, |s, s_cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", s_cx)).with_user(full_name, email);
        });
        self
    }

    /// Override the workspace name used by the shell sidebar.
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

    /// Start in skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Render only the page column without the outer [`AppShell`] frame.
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = standalone;
        self
    }

    /// Return the last committed profile values.
    pub fn profile(&self) -> &ProfileValues {
        &self.saved
    }

    /// Return the workspace name shown in the sidebar.
    pub fn workspace(&self) -> &str {
        self.workspace.as_ref()
    }

    /// Return the working draft profile values.
    pub fn draft(&self) -> &ProfileValues {
        &self.draft
    }

    /// Return whether working draft fields differ from the committed profile.
    pub fn is_dirty(&self) -> bool {
        self.draft != self.saved
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

    /// Pick a work role immediately and emit [`SettingsProfileEvent::Save`].
    pub fn select_role(&mut self, role: Option<SharedString>, cx: &mut Context<Self>) {
        self.open_menu = None;
        if self.draft.role != role {
            self.draft.role = role.clone();
            self.saved.role = role;
            self.show_toast(tr(cx, "Role updated", "职位角色已更新"), cx);
            cx.emit(SettingsProfileEvent::Save(self.saved.clone()));
        }
        cx.notify();
    }

    /// Pick a timezone immediately and emit [`SettingsProfileEvent::Save`].
    pub fn select_timezone(&mut self, tz: impl Into<SharedString>, cx: &mut Context<Self>) {
        let tz: SharedString = tz.into();
        self.open_menu = None;
        if self.draft.timezone != tz {
            self.draft.timezone = tz.clone();
            self.saved.timezone = tz;
            self.show_toast(tr(cx, "Timezone updated", "时区已更新"), cx);
            cx.emit(SettingsProfileEvent::Save(self.saved.clone()));
        }
        cx.notify();
    }

    /// Pick the calendar week start day immediately and emit [`SettingsProfileEvent::Save`].
    pub fn select_week_start(&mut self, day: WeekStart, cx: &mut Context<Self>) {
        self.open_menu = None;
        if self.draft.week_start != day {
            self.draft.week_start = day;
            self.saved.week_start = day;
            self.show_toast(tr(cx, "Week start updated", "每周起始日已更新"), cx);
            cx.emit(SettingsProfileEvent::Save(self.saved.clone()));
        }
        cx.notify();
    }

    /// Upload a profile picture immediately and emit [`SettingsProfileEvent::Save`].
    pub fn upload_avatar(&mut self, avatar: impl Into<SharedString>, cx: &mut Context<Self>) {
        let av: SharedString = avatar.into();
        self.draft.avatar = Some(av.clone());
        self.saved.avatar = Some(av);
        self.show_toast(tr(cx, "Profile picture updated", "头像已更新"), cx);
        cx.emit(SettingsProfileEvent::Save(self.saved.clone()));
        cx.notify();
    }

    /// Remove the custom profile picture immediately and emit [`SettingsProfileEvent::Save`].
    pub fn remove_avatar(&mut self, cx: &mut Context<Self>) {
        if self.draft.avatar.is_some() || self.saved.avatar.is_some() {
            self.draft.avatar = None;
            self.saved.avatar = None;
            self.show_toast(tr(cx, "Profile picture removed", "已移除头像"), cx);
            cx.emit(SettingsProfileEvent::Save(self.saved.clone()));
            cx.notify();
        }
    }

    /// Update draft text fields directly (for programmatic updates and unit tests).
    pub fn set_draft_identity(
        &mut self,
        first_name: impl Into<SharedString>,
        last_name: impl Into<SharedString>,
        context: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.draft.first_name = first_name.into();
        self.draft.last_name = last_name.into();
        self.draft.context = context.into();
        cx.notify();
    }

    /// Save a new account password (`>= 8` characters), emitting [`SettingsProfileEvent::PasswordUpdated`] and [`SettingsProfileEvent::Save`].
    pub fn save_password(
        &mut self,
        password: &str,
        w: Option<&Window>,
        cx: &mut Context<Self>,
    ) -> bool {
        if password.chars().count() < 8 {
            return false;
        }
        self.draft.has_password = true;
        self.saved.has_password = true;
        self.password_open = false;
        self.password_draft.clear();
        if let (Some(ed), Some(win)) = (&self.password_editor, w) {
            ed.update(cx, |e, e_cx| e.set_value("", win, e_cx));
        }
        self.show_toast(tr(cx, "Password updated", "密码已更新"), cx);
        cx.emit(SettingsProfileEvent::PasswordUpdated);
        cx.emit(SettingsProfileEvent::Save(self.saved.clone()));
        cx.notify();
        true
    }

    /// Commit working draft text edits if valid, updating sidebar account info and emitting [`SettingsProfileEvent::Save`].
    pub fn commit_save(&mut self, w: Option<&Window>, cx: &mut Context<Self>) -> bool {
        if !self.draft.is_valid() {
            return false;
        }
        self.draft.first_name = self.draft.first_name.trim().to_owned().into();
        self.draft.last_name = self.draft.last_name.trim().to_owned().into();
        self.draft.email = self.draft.email.trim().to_owned().into();
        if self.email_editing {
            self.email_editing = false;
            if let (Some(ed), Some(win)) = (&self.email_editor, w) {
                ed.update(cx, |e, e_cx| e.set_read_only(true, win, e_cx));
            }
        }
        if self.draft == self.saved {
            return false;
        }
        self.saved = self.draft.clone();
        let full_name: SharedString = self.saved.full_name().into();
        let email = self.saved.email.clone();
        self.shell.update(cx, |s, s_cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", s_cx)).with_user(full_name, email);
        });
        self.show_toast(tr(cx, "Profile saved", "个人资料已保存"), cx);
        cx.emit(SettingsProfileEvent::Save(self.saved.clone()));
        cx.notify();
        true
    }

    /// Revert unsaved draft edits back to the last committed [`ProfileValues`].
    pub fn discard_changes(&mut self, w: Option<&Window>, cx: &mut Context<Self>) {
        self.draft = self.saved.clone();
        self.email_editing = false;
        if let Some(win) = w {
            if let Some(ed) = &self.first_name_editor {
                let v = self.saved.first_name.to_string();
                ed.update(cx, |e, e_cx| e.set_value(v, win, e_cx));
            }
            if let Some(ed) = &self.last_name_editor {
                let v = self.saved.last_name.to_string();
                ed.update(cx, |e, e_cx| e.set_value(v, win, e_cx));
            }
            if let Some(ed) = &self.email_editor {
                let v = self.saved.email.to_string();
                ed.update(cx, |e, e_cx| {
                    e.set_value(v, win, e_cx);
                    e.set_read_only(true, win, e_cx);
                });
            }
            if let Some(ed) = &self.context_editor {
                let v = self.saved.context.to_string();
                ed.update(cx, |e, e_cx| e.set_value(v, win, e_cx));
            }
        }
        cx.notify();
    }

    fn show_toast(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast = Some((msg.into(), Instant::now()));
        cx.notify();
    }

    fn toggle_menu(&mut self, which: OpenMenu, cx: &mut Context<Self>) {
        if self.open_menu == Some(which) {
            self.open_menu = None;
        } else {
            self.open_menu = Some(which);
            self.last_menu = Some(which);
            self.menu_highlight = match which {
                OpenMenu::Role => self
                    .draft
                    .role
                    .as_ref()
                    .and_then(|r| ROLE_OPTIONS.iter().position(|(en, _)| *en == r.as_ref()))
                    .unwrap_or(0),
                OpenMenu::Timezone => TIMEZONE_OPTIONS
                    .iter()
                    .position(|tz| *tz == self.draft.timezone.as_ref())
                    .unwrap_or(0),
                OpenMenu::WeekStart => WeekStart::ALL
                    .iter()
                    .position(|d| *d == self.draft.week_start)
                    .unwrap_or(0),
            };
        }
        cx.notify();
    }

    fn menu_len(which: OpenMenu) -> usize {
        match which {
            OpenMenu::Role => ROLE_OPTIONS.len(),
            OpenMenu::Timezone => TIMEZONE_OPTIONS.len(),
            OpenMenu::WeekStart => WeekStart::ALL.len(),
        }
    }

    fn confirm_menu_choice(&mut self, which: OpenMenu, idx: usize, cx: &mut Context<Self>) {
        match which {
            OpenMenu::Role => {
                if let Some((en, _)) = ROLE_OPTIONS.get(idx) {
                    self.select_role(Some((*en).into()), cx);
                }
            }
            OpenMenu::Timezone => {
                if let Some(tz) = TIMEZONE_OPTIONS.get(idx) {
                    self.select_timezone(*tz, cx);
                }
            }
            OpenMenu::WeekStart => {
                if let Some(day) = WeekStart::ALL.get(idx) {
                    self.select_week_start(*day, cx);
                }
            }
        }
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.first_name_editor.is_none() {
            let label: SharedString = tr(cx, "First name", "名字").into();
            let initial = self.draft.first_name.to_string();
            let ed = cx.new(|e_cx| {
                let mut e = Editor::new(label, InputMode::Single, w, e_cx);
                e.set_placeholder("Alex");
                e.font_size = 13.;
                e.set_value(initial, w, e_cx);
                e
            });
            cx.subscribe(&ed, |this, ed_ref, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => {
                    this.draft.first_name = ed_ref.read(cx).value().to_owned().into();
                    cx.notify();
                }
                EditEvent::Submitted => {
                    this.draft.first_name = ed_ref.read(cx).value().to_owned().into();
                    this.commit_save(None, cx);
                }
                _ => {}
            })
            .detach();
            self.first_name_editor = Some(ed);
        }

        if self.last_name_editor.is_none() {
            let label: SharedString = tr(cx, "Last name", "姓").into();
            let initial = self.draft.last_name.to_string();
            let ed = cx.new(|e_cx| {
                let mut e = Editor::new(label, InputMode::Single, w, e_cx);
                e.set_placeholder("Morgan");
                e.font_size = 13.;
                e.set_value(initial, w, e_cx);
                e
            });
            cx.subscribe(&ed, |this, ed_ref, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => {
                    this.draft.last_name = ed_ref.read(cx).value().to_owned().into();
                    cx.notify();
                }
                EditEvent::Submitted => {
                    this.draft.last_name = ed_ref.read(cx).value().to_owned().into();
                    this.commit_save(None, cx);
                }
                _ => {}
            })
            .detach();
            self.last_name_editor = Some(ed);
        }

        if self.email_editor.is_none() {
            let label: SharedString = tr(cx, "Email", "电子邮箱").into();
            let initial = self.draft.email.to_string();
            let read_only = !self.email_editing;
            let ed = cx.new(|e_cx| {
                let mut e = Editor::new(label, InputMode::Single, w, e_cx);
                e.set_placeholder("alex@acme.com");
                e.font_size = 13.;
                e.set_value(initial, w, e_cx);
                e.set_read_only(read_only, w, e_cx);
                e
            });
            cx.subscribe(&ed, |this, ed_ref, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => {
                    this.draft.email = ed_ref.read(cx).value().to_owned().into();
                    cx.notify();
                }
                EditEvent::Submitted => {
                    this.draft.email = ed_ref.read(cx).value().to_owned().into();
                    this.commit_save(None, cx);
                }
                _ => {}
            })
            .detach();
            self.email_editor = Some(ed);
        }

        if self.context_editor.is_none() {
            let label: SharedString = tr(cx, "Personal context", "个人背景").into();
            let ph: SharedString = tr(
                cx,
                "E.g. I lead product at a small team, prefer short answers and write in British English.",
                "例如：我在小型团队负责产品，偏好简短回答，习惯使用地道专业表达。",
            )
            .into();
            let initial = self.draft.context.to_string();
            let ed = cx.new(|e_cx| {
                let mut e = Editor::new(label, InputMode::Multi, w, e_cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                if !initial.is_empty() {
                    e.set_value(initial, w, e_cx);
                }
                e
            });
            cx.subscribe(&ed, |this, ed_ref, ev: &EditEvent, cx| {
                if matches!(ev, EditEvent::Changed) {
                    this.draft.context = ed_ref.read(cx).value().to_owned().into();
                    cx.notify();
                }
            })
            .detach();
            self.context_editor = Some(ed);
        }

        if self.password_editor.is_none() {
            let label: SharedString = tr(cx, "New password", "新密码").into();
            let ph: SharedString = tr(cx, "At least 8 characters", "至少 8 个字符").into();
            let ed = cx.new(|e_cx| {
                let mut e = Editor::new(label, InputMode::Password, w, e_cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |this, ed_ref, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => {
                    this.password_draft = ed_ref.read(cx).value().to_owned();
                    cx.notify();
                }
                EditEvent::Submitted => {
                    let pw = ed_ref.read(cx).value().to_owned();
                    this.save_password(&pw, None, cx);
                }
                EditEvent::Escape => {
                    this.password_open = false;
                    this.password_draft.clear();
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.password_editor = Some(ed);
        }
    }

    fn render_avatar(&self, p: &ProPalette, cx: &mut Context<Self>) -> Div {
        let initials = self.draft.initials();
        let full_name = self.draft.full_name();

        let inner: AnyElement = match self.draft.avatar.as_deref() {
            Some("emerald") => div()
                .size_full()
                .rounded_full()
                .bg(p.green_soft)
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.green_ink)
                .child(initials)
                .into_any_element(),
            Some("amber") => div()
                .size_full()
                .rounded_full()
                .bg(p.amber_soft)
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.amber_ink)
                .child(initials)
                .into_any_element(),
            Some("violet") => div()
                .size_full()
                .rounded_full()
                .bg(p.purple.opacity(0.16))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.purple)
                .child(initials)
                .into_any_element(),
            Some(_) => pro_person_avatar(&full_name, 40., false, None, cx),
            None => div()
                .size_full()
                .rounded_full()
                .bg(p.accent_soft)
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.accent_info)
                .child(initials)
                .into_any_element(),
        };

        let mut wrap = div().relative().size(px(40.)).flex_none().child(inner);

        if self.draft.avatar.is_some() {
            let card_bg = p.card;
            let line = p.line_strong;
            let hover_ink = p.ink;
            let focus = p.focus;
            let muted = p.muted;
            wrap = wrap.child(activate(
                div()
                    .id(pro_eid(&self.id, "remove-avatar"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Remove profile picture", "移除个人头像"))
                    .tab_index(0)
                    .absolute()
                    .top(px(-3.))
                    .right(px(-3.))
                    .size(px(18.))
                    .rounded_full()
                    .bg(card_bg)
                    .border_1()
                    .border_color(line)
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(move |s| s.border_color(hover_ink))
                    .focus_visible(move |s| s.border_1().border_color(focus))
                    .child(pro_icon(ProIcon::Close, 10., muted)),
                cx,
                |this, _, cx| this.remove_avatar(cx),
            ));
        }

        wrap
    }

    #[allow(clippy::too_many_arguments)]
    fn render_select_trigger(
        &self,
        id_suffix: &'static str,
        aria: SharedString,
        lead_icon: Option<ProIcon>,
        value_text: SharedString,
        is_placeholder: bool,
        which: OpenMenu,
        bounds: &TriggerBounds,
        narrow: bool,
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
            .when(narrow, |d| d.w_full())
            .when(!narrow, |d| d.w(px(240.)).flex_none())
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
            .when_some(lead_icon, |d, ic| d.child(pro_icon(ic, 15., p.muted)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(13.))
                    .text_color(if is_placeholder { p.faint } else { p.ink })
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
        let (bounds, menu_key) = match which {
            OpenMenu::Role => (&self.role_bounds, "role"),
            OpenMenu::Timezone => (&self.tz_bounds, "tz"),
            OpenMenu::WeekStart => (&self.week_bounds, "week"),
        };
        let rect = bounds.get();
        let menu_w = f32::from(rect.size.width).max(200.);
        let items: Vec<(SharedString, bool)> = match which {
            OpenMenu::Role => ROLE_OPTIONS
                .iter()
                .map(|(en, zh)| {
                    let selected = self.draft.role.as_deref() == Some(*en);
                    (SharedString::from(tr(cx, en, zh)), selected)
                })
                .collect(),
            OpenMenu::Timezone => TIMEZONE_OPTIONS
                .iter()
                .map(|tz| {
                    let selected = self.draft.timezone.as_ref() == *tz;
                    (SharedString::from(*tz), selected)
                })
                .collect(),
            OpenMenu::WeekStart => WeekStart::ALL
                .iter()
                .map(|day| {
                    let selected = self.draft.week_start == *day;
                    (SharedString::from(day.label_tr(cx)), selected)
                })
                .collect(),
        };

        let est_h = (items.len() as f32 * 32. + 8.).min(280.);
        let (pos, side) = bounds.place(w.viewport_size(), px(est_h), px(5.));
        let anchor_copy = bounds.clone();
        let menu_bg: Hsla = if p.dark {
            rgb(0x1f1f22).into()
        } else {
            rgb(0xffffff).into()
        };

        let mut list = div()
            .id(pro_eid(&self.id, "select-listbox"))
            .role(Role::ListBox)
            .max_h(px(272.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(1.));

        for (idx, (label, selected)) in items.into_iter().enumerate() {
            let highlighted = self.menu_highlight == idx;
            let row_bg = if highlighted {
                p.hover
            } else {
                transparent_black()
            };
            let hover_bg = p.hover;
            list = list.child(
                div()
                    .id(pro_eid(&self.id, format!("opt-{idx}")))
                    .role(Role::ListBoxOption)
                    .aria_label(label.clone())
                    .aria_selected(selected)
                    .min_h(px(32.))
                    .px(px(8.))
                    .py(px(5.))
                    .rounded(px(6.))
                    .bg(row_bg)
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
                            .truncate()
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

        let panel = div()
            .id(pro_eid(&self.id, "select-popover"))
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
                        this.open_menu = None;
                        o_cx.notify();
                    }
                }))
            })
            .child(pro_view_transition(
                pro_eid(&self.id, "select-menu-vtx"),
                menu_key,
                list,
            ))
            .into_any_element();

        pro_presence(
            pro_eid(&self.id, "select-popover-pres"),
            is_open,
            0.18,
            0.14,
            move |amount| {
                deferred(
                    anchored()
                        .anchor(side)
                        .position(pos)
                        .snap_to_window_with_margin(px(10.))
                        .child(
                            div()
                                .relative()
                                .top(px(-4. * (1. - amount)))
                                .opacity(amount)
                                .child(panel),
                        ),
                )
                .with_priority(3)
                .into_any_element()
            },
        )
    }

    fn render_loading_skeleton(&self, narrow: bool, phase: f32, p: &ProPalette, cx: &App) -> Div {
        let field_w = if narrow { 210. } else { 240. };
        let frame_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.03)
        } else {
            hsla(0., 0., 0., 0.02)
        };
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
                    .child(tr(cx, "Profile", "个人资料")),
            )
            .child(
                div()
                    .mt(px(4.))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.text)
                    .child(tr(
                        cx,
                        "Manage your personal details.",
                        "管理你的个人信息。",
                    )),
            )
            .child(
                div()
                    .mt(px(24.))
                    .h(px(40.))
                    .px(px(12.))
                    .rounded(px(10.))
                    .bg(frame_bg)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(pro_skeleton_bar(15., 15., 8., phase, cx))
                    .child(pro_skeleton_bar(260., 12., 4., phase + 0.06, cx)),
            )
            .child(
                div()
                    .mt(px(16.))
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
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(6.))
                                    .child(pro_skeleton_bar(96., 12., 4., phase + 0.1, cx))
                                    .child(pro_skeleton_bar(156., 10., 4., phase + 0.14, cx)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(pro_skeleton_bar(40., 40., 20., phase + 0.18, cx))
                                    .child(pro_skeleton_bar(72., 28., 6., phase + 0.22, cx)),
                            ),
                    )
                    .children((0..4usize).map(|idx| {
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
                            .child(pro_skeleton_bar(
                                78. + (idx as f32 * 14.),
                                12.,
                                4.,
                                phase + 0.24 + idx as f32 * 0.06,
                                cx,
                            ))
                            .child(pro_skeleton_bar(
                                field_w,
                                32.,
                                7.,
                                phase + 0.28 + idx as f32 * 0.06,
                                cx,
                            ))
                    })),
            )
            .child(
                div()
                    .mt(px(36.))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(pro_skeleton_bar(116., 14., 4., phase + 0.5, cx))
                    .child(pro_skeleton_bar(240., 12., 4., phase + 0.54, cx)),
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
                        div()
                            .min_h(px(60.))
                            .py(px(10.))
                            .pl(px(16.))
                            .pr(px(12.))
                            .when(idx > 0, |d| d.border_t_1().border_color(p.line))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(pro_skeleton_bar(
                                88. + idx as f32 * 18.,
                                12.,
                                4.,
                                phase + 0.58 + idx as f32 * 0.06,
                                cx,
                            ))
                            .child(pro_skeleton_bar(
                                field_w,
                                32.,
                                7.,
                                phase + 0.62 + idx as f32 * 0.06,
                                cx,
                            ))
                    })),
            )
            .child(
                div()
                    .mt(px(36.))
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(pro_skeleton_bar(112., 14., 4., phase + 0.72, cx))
                    .child(pro_skeleton_bar(760., 64., 8., phase + 0.76, cx).w_full()),
            )
            .child(
                div()
                    .mt(px(24.))
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
                            .child(pro_skeleton_bar(76., 12., 4., phase + 0.8, cx))
                            .child(pro_skeleton_bar(210., 10., 4., phase + 0.84, cx)),
                    )
                    .child(pro_skeleton_bar(98., 28., 6., phase + 0.88, cx)),
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
        if (self.loading || self.toast.is_some()) && !reduced && w.is_visible() {
            w.request_animation_frame();
        }

        let dirty = !self.loading && self.is_dirty();
        let scroll_pb = pro_spring_f32(
            pro_eid(&self.id, "scroll-pb"),
            if dirty { 88. } else { 56. },
            0.22,
            w,
            cx,
        );

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
        let readonly_surface: Hsla = if p.dark {
            hsla(0., 0., 1., 0.03)
        } else {
            hsla(0., 0., 0., 0.02)
        };
        let input_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.14)
        } else {
            hsla(0., 0., 0., 0.09)
        };
        let frame_bg: Hsla = if p.dark {
            hsla(0., 0., 1., 0.03)
        } else {
            hsla(0., 0., 0., 0.02)
        };

        let body = if self.loading {
            self.render_loading_skeleton(narrow, phase, &p, cx)
        } else {
            let fn_focused = self
                .first_name_editor
                .as_ref()
                .is_some_and(|e| e.read(cx).focus.is_focused(w));
            let ln_focused = self
                .last_name_editor
                .as_ref()
                .is_some_and(|e| e.read(cx).focus.is_focused(w));
            let em_focused = self
                .email_editor
                .as_ref()
                .is_some_and(|e| e.read(cx).focus.is_focused(w));
            let ctx_focused = self
                .context_editor
                .as_ref()
                .is_some_and(|e| e.read(cx).focus.is_focused(w));
            let pw_focused = self
                .password_editor
                .as_ref()
                .is_some_and(|e| e.read(cx).focus.is_focused(w));

            let fn_invalid = self.draft.first_name.trim().is_empty();
            let em_trimmed = self.draft.email.trim();
            let em_invalid = !em_trimmed.contains('@')
                || em_trimmed.starts_with('@')
                || em_trimmed.ends_with('@');
            let ctx_chars = self.draft.context.chars().count();
            let ctx_over = ctx_chars > MAX_PERSONAL_CONTEXT_CHARS;
            let hover_border = p.line_strong;

            // Header + Learn more link
            let link_ink = p.ink;
            let focus = p.focus;
            let learn_more = activate(
                div()
                    .id(pro_eid(&self.id, "learn-more"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Learn more", "了解更多"))
                    .tab_index(0)
                    .border_b_1()
                    .border_color(p.line_strong)
                    .text_size(px(13.))
                    .line_height(px(18.))
                    .text_color(p.text)
                    .cursor_pointer()
                    .hover(move |s| s.text_color(link_ink).border_color(link_ink))
                    .focus_visible(move |s| s.border_1().border_color(focus))
                    .child(tr(cx, "Learn more", "了解更多")),
                cx,
                |this, _, cx| {
                    this.show_toast(
                        tr(
                            cx,
                            "Profile details sync across every workspace you belong to.",
                            "个人资料会自动同步至你加入的所有工作区。",
                        ),
                        cx,
                    );
                    cx.emit(SettingsProfileEvent::LearnMore);
                },
            );

            // Info banner (.ui-sp-note)
            let note_banner = div()
                .mt(px(24.))
                .py(px(10.))
                .px(px(12.))
                .rounded(px(10.))
                .bg(frame_bg)
                .border_1()
                .border_color(p.line)
                .flex()
                .items_center()
                .gap(px(8.))
                .child(pro_svg("0 0 24 24", INFO_CIRCLE_SVG, 15., p.muted))
                .child(
                    div()
                        .min_w_0()
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.text)
                        .child(tr(
                            cx,
                            "Changes to your profile apply to all of your workspaces.",
                            "对个人资料的修改将应用于你所在的全部工作区。",
                        )),
                );

            // Profile card rows
            let row_avatar = div()
                .min_h(px(60.))
                .py(px(if narrow { 12. } else { 10. }))
                .pl(px(if narrow { 14. } else { 16. }))
                .pr(px(if narrow { 14. } else { 12. }))
                .flex()
                .when(narrow, |d| d.flex_col().items_stretch().gap(px(10.)))
                .when(!narrow, |d| d.items_center().justify_between().gap(px(24.)))
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(13.))
                                .line_height(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(tr(cx, "Profile picture", "个人头像")),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(tr(
                                    cx,
                                    "PNG, JPEG or GIF, under 10 MB",
                                    "支持 PNG、JPEG 或 GIF，小于 10 MB",
                                )),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(self.render_avatar(&p, cx))
                        .child(pro_btn(
                            pro_eid(&self.id, "upload-avatar"),
                            tr(cx, "Upload", "上传"),
                            Some(ProIcon::Upload),
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            |this, _, cx| {
                                let preset =
                                    AVATAR_PRESETS[this.avatar_cycle % AVATAR_PRESETS.len()];
                                this.avatar_cycle = this.avatar_cycle.wrapping_add(1);
                                this.upload_avatar(preset, cx);
                            },
                        )),
                );

            let row_first_name = div()
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
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(tr(cx, "First name", "名字")),
                )
                .child(
                    div()
                        .when(narrow, |d| d.w_full())
                        .when(!narrow, |d| d.w(px(240.)).flex_none())
                        .h(px(32.))
                        .px(px(10.))
                        .rounded(px(7.))
                        .bg(input_surface)
                        .border_1()
                        .border_color(if fn_invalid {
                            p.red.opacity(0.65)
                        } else if fn_focused {
                            p.focus
                        } else {
                            input_border
                        })
                        .when(!fn_focused && !fn_invalid, |d| {
                            d.hover(move |s| s.border_color(hover_border))
                        })
                        .flex()
                        .items_center()
                        .when_some(self.first_name_editor.clone(), |d, ed| d.child(ed)),
                );

            let row_last_name = div()
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
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(tr(cx, "Last name", "姓")),
                )
                .child(
                    div()
                        .when(narrow, |d| d.w_full())
                        .when(!narrow, |d| d.w(px(240.)).flex_none())
                        .h(px(32.))
                        .px(px(10.))
                        .rounded(px(7.))
                        .bg(input_surface)
                        .border_1()
                        .border_color(if ln_focused { p.focus } else { input_border })
                        .when(!ln_focused, |d| {
                            d.hover(move |s| s.border_color(hover_border))
                        })
                        .flex()
                        .items_center()
                        .when_some(self.last_name_editor.clone(), |d, ed| d.child(ed)),
                );

            let (role_text, role_is_ph) = if let Some(role) = &self.draft.role {
                let localized = ROLE_OPTIONS
                    .iter()
                    .find(|(en, _)| *en == role.as_ref())
                    .map_or_else(|| role.as_ref(), |(en, zh)| tr(cx, en, zh));
                (SharedString::from(localized.to_owned()), false)
            } else {
                (
                    SharedString::from(tr(cx, "Choose your role", "选择你的角色")),
                    true,
                )
            };
            let role_bounds = self.role_bounds.clone();
            let role_trigger = self.render_select_trigger(
                "role-select",
                tr(cx, "Role", "职位角色").into(),
                None,
                role_text,
                role_is_ph,
                OpenMenu::Role,
                &role_bounds,
                narrow,
                input_surface,
                input_border,
                &p,
                cx,
            );

            let row_role = div()
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
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(13.))
                                .line_height(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(tr(cx, "Role", "职位角色")),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(tr(
                                    cx,
                                    "Helps tailor suggestions to your work",
                                    "帮助根据你的工作内容优化建议",
                                )),
                        ),
                )
                .child(role_trigger);

            let email_bg = if self.email_editing {
                input_surface
            } else {
                readonly_surface
            };
            let email_border = if em_invalid && self.email_editing {
                p.red.opacity(0.65)
            } else if em_focused && self.email_editing {
                p.focus
            } else if self.email_editing {
                input_border
            } else {
                p.line
            };
            let row_email = div()
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
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(tr(cx, "Email", "电子邮箱")),
                )
                .child(
                    div()
                        .when(narrow, |d| d.w_full())
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .when(narrow, |d| d.flex_1().min_w_0())
                                .when(!narrow, |d| d.w(px(200.)).flex_none())
                                .h(px(32.))
                                .px(px(10.))
                                .rounded(px(7.))
                                .bg(email_bg)
                                .border_1()
                                .border_color(email_border)
                                .flex()
                                .items_center()
                                .when_some(self.email_editor.clone(), |d, ed| d.child(ed)),
                        )
                        .child(if self.email_editing {
                            pro_btn(
                                pro_eid(&self.id, "save-email"),
                                tr(cx, "Save", "保存"),
                                None,
                                ProBtnStyle::Primary,
                                true,
                                cx,
                                |this, w, cx| {
                                    if !this.commit_save(Some(w), cx) {
                                        this.email_editing = false;
                                        if let Some(ed) = &this.email_editor {
                                            ed.update(cx, |e, e_cx| {
                                                e.set_read_only(true, w, e_cx);
                                            });
                                        }
                                        cx.notify();
                                    }
                                },
                            )
                        } else {
                            pro_btn(
                                pro_eid(&self.id, "change-email"),
                                tr(cx, "Change", "修改"),
                                None,
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                |this, w, cx| {
                                    this.email_editing = true;
                                    if let Some(ed) = &this.email_editor {
                                        ed.update(cx, |e, e_cx| {
                                            e.set_read_only(false, w, e_cx);
                                            e.focus(w, e_cx);
                                            e.select_all(w, e_cx);
                                        });
                                    }
                                    cx.notify();
                                },
                            )
                        }),
                );

            let profile_card = div()
                .id(pro_eid(&self.id, "profile-card"))
                .role(Role::Region)
                .aria_label(tr(cx, "Profile", "个人资料"))
                .mt(px(16.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .child(row_avatar)
                .child(row_first_name)
                .child(row_last_name)
                .child(row_role)
                .child(row_email);

            // Section 2: Time preferences
            let tz_bounds = self.tz_bounds.clone();
            let tz_trigger = self.render_select_trigger(
                "tz-select",
                tr(cx, "Timezone", "时区").into(),
                Some(ProIcon::Globe),
                self.draft.timezone.clone(),
                false,
                OpenMenu::Timezone,
                &tz_bounds,
                narrow,
                input_surface,
                input_border,
                &p,
                cx,
            );

            let week_bounds = self.week_bounds.clone();
            let week_label: SharedString = self.draft.week_start.label_tr(cx).into();
            let week_trigger = self.render_select_trigger(
                "week-select",
                tr(cx, "Start week on", "每周起始日").into(),
                Some(ProIcon::Calendar),
                week_label,
                false,
                OpenMenu::WeekStart,
                &week_bounds,
                narrow,
                input_surface,
                input_border,
                &p,
                cx,
            );

            let time_section = div()
                .mt(px(36.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(tr(cx, "Time preferences", "时间偏好")),
                )
                .child(
                    div()
                        .mt(px(2.))
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.muted)
                        .child(tr(
                            cx,
                            "Used for reminders, schedules and reports.",
                            "用于提醒、定时任务与统计报表。",
                        )),
                )
                .child(
                    div()
                        .id(pro_eid(&self.id, "time-card"))
                        .role(Role::Region)
                        .aria_label(tr(cx, "Time preferences", "时间偏好"))
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
                                .when(!narrow, |d| d.items_center().justify_between().gap(px(24.)))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(20.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .child(tr(cx, "Timezone", "时区")),
                                )
                                .child(tz_trigger),
                        )
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
                                .when(!narrow, |d| d.items_center().justify_between().gap(px(24.)))
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(20.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .child(tr(cx, "Start week on", "每周起始日")),
                                )
                                .child(week_trigger),
                        ),
                );

            // Section 3: Personal context
            let ctx_border = if ctx_over {
                p.red.opacity(0.65)
            } else if ctx_focused {
                p.focus
            } else {
                input_border
            };
            let personal_ctx_section = div()
                .mt(px(36.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(tr(cx, "Personal context", "个人背景")),
                )
                .child(
                    div()
                        .mt(px(2.))
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.muted)
                        .child(tr(
                            cx,
                            "Describe what you do and how you work, so the assistant can adapt to you.",
                            "描述你的职责与工作习惯，让助手更好地配合你。",
                        )),
                )
                .child(
                    div()
                        .mt(px(12.))
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .w_full()
                                .h(px(64.))
                                .py(px(7.))
                                .px(px(10.))
                                .rounded(px(8.))
                                .bg(input_surface)
                                .border_1()
                                .border_color(ctx_border)
                                .when(!ctx_focused && !ctx_over, |d| {
                                    d.hover(move |s| s.border_color(hover_border))
                                })
                                .when_some(self.context_editor.clone(), |d, ed| d.child(ed)),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(12.))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.red_ink)
                                        .when(ctx_over, |d| {
                                            d.child(tr(
                                                cx,
                                                "Personal context is over the 500-character limit.",
                                                "个人背景已超出 500 字上限。",
                                            ))
                                        }),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(if ctx_over {
                                            p.red
                                        } else if ctx_chars == MAX_PERSONAL_CONTEXT_CHARS {
                                            p.muted
                                        } else {
                                            p.faint
                                        })
                                        .child(format!("{ctx_chars} / 500")),
                                ),
                        ),
                );

            // Card 4: Password (.ui-sp-card--alone)
            let pw_sub = if self.draft.has_password {
                tr(
                    cx,
                    "Password configured · sign in with password or email link",
                    "已设置密码 · 可使用密码或邮件链接登录",
                )
            } else {
                tr(
                    cx,
                    "You sign in with a link sent to your email",
                    "你当前通过发送到邮箱的验证链接登录",
                )
            };
            let pw_btn_label = if self.draft.has_password {
                tr(cx, "Change password", "修改密码")
            } else {
                tr(cx, "Add password", "设置密码")
            };

            let mut password_card = div()
                .id(pro_eid(&self.id, "password-card"))
                .role(Role::Region)
                .aria_label(tr(cx, "Password", "密码"))
                .mt(px(24.))
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
                        .when(!narrow, |d| d.items_center().justify_between().gap(px(24.)))
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(20.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .child(tr(cx, "Password", "密码")),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .child(pw_sub),
                                ),
                        )
                        .child(pro_btn(
                            pro_eid(&self.id, "toggle-password-btn"),
                            pw_btn_label,
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            |this, w, cx| {
                                this.password_open = !this.password_open;
                                if this.password_open
                                    && let Some(ed) = &this.password_editor
                                {
                                    ed.update(cx, |e, e_cx| e.focus(w, e_cx));
                                }
                                cx.notify();
                            },
                        )),
                );

            let pw_ready = self.password_draft.chars().count() >= 8;
            let eye_hover = p.hover;
            let pw_drawer = div()
                .py(px(12.))
                .pl(px(if narrow { 14. } else { 16. }))
                .pr(px(if narrow { 14. } else { 12. }))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .when(narrow, |d| d.flex_col().items_stretch().gap(px(10.)))
                .when(!narrow, |d| d.items_center().justify_between().gap(px(16.)))
                .child(
                    div()
                        .when(narrow, |d| d.w_full())
                        .when(!narrow, |d| d.w(px(260.)))
                        .h(px(32.))
                        .pl(px(10.))
                        .pr(px(4.))
                        .rounded(px(7.))
                        .bg(input_surface)
                        .border_1()
                        .border_color(if pw_focused { p.focus } else { input_border })
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .h_full()
                                .when_some(self.password_editor.clone(), |d, ed| d.child(ed)),
                        )
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, "reveal-password"))
                                .role(Role::Button)
                                .aria_label(tr(cx, "Toggle password visibility", "显示或隐藏密码"))
                                .tab_index(0)
                                .size(px(24.))
                                .rounded(px(5.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .hover(move |s| s.bg(eye_hover))
                                .child(pro_icon(
                                    ProIcon::Eye,
                                    14.,
                                    if self.password_revealed {
                                        p.ink
                                    } else {
                                        p.muted
                                    },
                                )),
                            cx,
                            |this, _, cx| {
                                this.password_revealed = !this.password_revealed;
                                let rev = this.password_revealed;
                                if let Some(ed) = &this.password_editor {
                                    ed.update(cx, |e, e_cx| {
                                        e.revealed = rev;
                                        e_cx.notify();
                                    });
                                }
                                cx.notify();
                            },
                        )),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_btn(
                            pro_eid(&self.id, "cancel-password"),
                            tr(cx, "Cancel", "取消"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            |this, w, cx| {
                                this.password_open = false;
                                this.password_draft.clear();
                                if let Some(ed) = &this.password_editor {
                                    ed.update(cx, |e, e_cx| e.set_value("", w, e_cx));
                                }
                                cx.notify();
                            },
                        ))
                        .child(div().when(!pw_ready, |d| d.opacity(0.45)).child(pro_btn(
                            pro_eid(&self.id, "save-password"),
                            tr(cx, "Save password", "保存密码"),
                            None,
                            ProBtnStyle::Primary,
                            true,
                            cx,
                            |this, w, cx| {
                                let pw = this.password_draft.clone();
                                this.save_password(&pw, Some(w), cx);
                            },
                        ))),
                )
                .into_any_element();

            password_card = password_card.child(pro_fold(
                pro_eid(&self.id, "password-fold"),
                self.password_open,
                58.,
                0.,
                move || pw_drawer,
            ));

            let menu_is_open = self.open_menu.is_some();
            let open_overlay = self
                .open_menu
                .or(self.last_menu)
                .map(|which| self.render_open_dropdown(which, menu_is_open, w, &p, cx));

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
                        .child(tr(cx, "Profile", "个人资料")),
                )
                .child(
                    div()
                        .mt(px(4.))
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(px(5.))
                        .child(
                            div()
                                .text_size(px(13.))
                                .line_height(px(20.))
                                .text_color(p.text)
                                .child(tr(
                                    cx,
                                    "Manage your personal details.",
                                    "管理你的个人信息。",
                                )),
                        )
                        .child(learn_more),
                )
                .child(note_banner)
                .child(profile_card)
                .child(time_section)
                .child(personal_ctx_section)
                .child(password_card)
                .when_some(open_overlay, |d, ov| d.child(ov))
        };

        let can_save = self.draft.is_valid();
        let dirty_id = pro_eid(&self.id, "dirty-bar");
        let dirty_aria: SharedString = tr(cx, "Unsaved changes", "未保存更改").into();
        let dirty_inner = div()
            .h(px(42.))
            .pl(px(14.))
            .pr(px(8.))
            .rounded(px(11.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .shadow_lg()
            .flex()
            .items_center()
            .gap(px(12.))
            .child(
                div()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.ink)
                    .child(tr(cx, "Unsaved changes", "有未保存的更改")),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .child(pro_btn(
                        pro_eid(&self.id, "discard-btn"),
                        tr(cx, "Discard", "放弃"),
                        None,
                        ProBtnStyle::Ghost,
                        true,
                        cx,
                        |this, w, cx| this.discard_changes(Some(w), cx),
                    ))
                    .child(div().when(!can_save, |d| d.opacity(0.45)).child(pro_btn(
                        pro_eid(&self.id, "save-btn"),
                        tr(cx, "Save changes", "保存更改"),
                        None,
                        ProBtnStyle::Primary,
                        true,
                        cx,
                        |this, w, cx| {
                            this.commit_save(Some(w), cx);
                        },
                    ))),
            )
            .into_any_element();

        div()
            .id(pro_eid(&self.id, "page-root"))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, w, cx| {
                let k = ev.keystroke.key.as_str();
                let cmd = ev.keystroke.modifiers.platform || ev.keystroke.modifiers.control;
                if let Some(which) = this.open_menu {
                    let count = Self::menu_len(which);
                    match k {
                        "down" => {
                            this.menu_highlight = (this.menu_highlight + 1) % count.max(1);
                            w.prevent_default();
                            cx.stop_propagation();
                            cx.notify();
                            return;
                        }
                        "up" => {
                            this.menu_highlight =
                                (this.menu_highlight + count.saturating_sub(1)) % count.max(1);
                            w.prevent_default();
                            cx.stop_propagation();
                            cx.notify();
                            return;
                        }
                        "enter" => {
                            let idx = this.menu_highlight;
                            this.confirm_menu_choice(which, idx, cx);
                            w.prevent_default();
                            cx.stop_propagation();
                            return;
                        }
                        "escape" => {
                            this.open_menu = None;
                            w.prevent_default();
                            cx.stop_propagation();
                            cx.notify();
                            return;
                        }
                        _ => {}
                    }
                }
                if cmd && k == "s" {
                    this.commit_save(Some(w), cx);
                    w.prevent_default();
                    cx.stop_propagation();
                } else if k == "escape" && this.password_open {
                    this.password_open = false;
                    this.password_draft.clear();
                    w.prevent_default();
                    cx.stop_propagation();
                    cx.notify();
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
                    .pb(px(scroll_pb))
                    .px(px(pad_x))
                    .child(pro_view_transition(
                        pro_eid(&self.id, "page-body-vtx"),
                        if self.loading { "loading" } else { "ready" },
                        body,
                    )),
            )
            .child(pro_presence(
                pro_eid(&self.id, "dirty-bar-pres"),
                dirty,
                0.22,
                0.18,
                move |amount| {
                    div()
                        .id(dirty_id)
                        .role(Role::Region)
                        .aria_label(dirty_aria)
                        .absolute()
                        .bottom(px(16. - 10. * (1. - amount)))
                        .left_0()
                        .right_0()
                        .opacity(amount)
                        .flex()
                        .justify_center()
                        .px(px(16.))
                        .child(dirty_inner)
                        .into_any_element()
                },
            ))
            .child(pro_toast_banner(self.toast.clone()))
    }
}

impl Render for SettingsProfileScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{ProfileValues, SettingsProfileEvent, SettingsProfileScreen, WeekStart};
    use gpui::{AppContext as _, TestAppContext};
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn profile_initials_full_name_and_validation() {
        let p = ProfileValues::default();
        assert_eq!(p.full_name(), "Alex Morgan");
        assert_eq!(p.initials(), "AM");
        assert!(p.is_valid());

        let single = ProfileValues::new("Elena", "", "elena@acme.com");
        assert_eq!(single.full_name(), "Elena");
        assert_eq!(single.initials(), "E");
        assert!(single.is_valid());

        let bad_email = ProfileValues::new("Alex", "Morgan", "invalid-email");
        assert!(!bad_email.is_valid());

        let over_ctx =
            ProfileValues::new("Alex", "Morgan", "alex@acme.com").with_context("x".repeat(501));
        assert!(!over_ctx.is_valid());
    }

    #[test]
    fn immediate_choices_password_and_dirty_text_workflow() {
        struct EventSink;

        let app = TestAppContext::single();
        app.update(crate::init);

        let events: Rc<RefCell<Vec<SettingsProfileEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let recorded = events.clone();

        let (screen, _sink) = app.update(|cx| {
            let screen = cx.new(|cx| SettingsProfileScreen::new("test-sp", cx));
            let sink = cx.new(|sub_cx| {
                sub_cx
                    .subscribe(
                        &screen,
                        move |_: &mut EventSink, _, ev: &SettingsProfileEvent, _| {
                            recorded.borrow_mut().push(ev.clone());
                        },
                    )
                    .detach();
                EventSink
            });
            (screen, sink)
        });

        app.update(|cx| {
            screen.update(cx, |s, s_cx| {
                // Immediate choice updates emit Save without leaving dirty text
                s.select_role(Some("Engineering".into()), s_cx);
                s.select_timezone("Asia/Tokyo", s_cx);
                s.select_week_start(WeekStart::Sunday, s_cx);
                assert!(!s.is_dirty());
                assert_eq!(s.profile().role.as_deref(), Some("Engineering"));
                assert_eq!(s.profile().timezone.as_ref(), "Asia/Tokyo");
                assert_eq!(s.profile().week_start, WeekStart::Sunday);

                // Password requires >= 8 chars
                assert!(!s.save_password("short", None, s_cx));
                assert!(!s.profile().has_password);
                assert!(s.save_password("strong-pass-2026", None, s_cx));
                assert!(s.profile().has_password);

                // Text draft dirty/discard/save cycle
                s.set_draft_identity("Jordan", "Lee", "Prefers concise diffs.", s_cx);
                assert!(s.is_dirty());
                s.discard_changes(None, s_cx);
                assert!(!s.is_dirty());
                assert_eq!(s.profile().first_name.as_ref(), "Alex");

                s.set_draft_identity("Jordan", "Lee", "Prefers concise diffs.", s_cx);
                assert!(s.commit_save(None, s_cx));
                assert_eq!(s.profile().full_name(), "Jordan Lee");
            });
        });

        let emitted = events.borrow();
        assert_eq!(emitted.len(), 6);
        assert!(
            emitted
                .iter()
                .any(|e| matches!(e, SettingsProfileEvent::PasswordUpdated))
        );
    }
}
