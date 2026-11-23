//! Workspace members, seat allocation, pending invitations, role access, and teams settings.
//!
//! Renders inside [`AppShellPreset::Settings`] with seat and admin counters, member search
//! and role filtering, inline email invitations, bulk role/plan actions, 2FA indicators,
//! verified domain auto-join switch, and team group management.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset},
        pro_chrome::{
            ProBtnStyle, ProPalette, ProTone, pro_badge, pro_btn, pro_eid, pro_empty_state,
            pro_fold, pro_icon_btn, pro_mark_selected, pro_person_avatar, pro_pill_indicator,
            pro_selection_sensor, pro_skeleton_bar, pro_spring_bar, pro_switch, pro_toast_banner,
            pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        pro_icons::{ProIcon, person_initials, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::BTreeSet, rc::Rc};
use web_time::Instant;

const SVG_INFO_CIRCLE: &str = r#"<circle cx="12" cy="12" r="10" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></circle><path d="M12 16V12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M12.125 8.25H12M12.25 8.25C12.25 8.11193 12.1381 8 12 8C11.8619 8 11.75 8.11193 11.75 8.25C11.75 8.38807 11.8619 8.5 12 8.5C12.1381 8.5 12.25 8.38807 12.25 8.25Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_FUNNEL: &str = r#"<path d="M8.85746 12.5061C6.36901 10.6456 4.59564 8.59915 3.62734 7.44867C3.3276 7.09253 3.22938 6.8319 3.17033 6.3728C2.96811 4.8008 2.86701 4.0148 3.32795 3.5074C3.7889 3 4.60404 3 6.23433 3H17.7657C19.396 3 20.2111 3 20.672 3.5074C21.133 4.0148 21.0319 4.8008 20.8297 6.37281C20.7706 6.83191 20.6724 7.09254 20.3726 7.44867C19.403 8.60062 17.6261 10.6507 15.1326 12.5135C14.907 12.6821 14.7583 12.9567 14.7307 13.2614C14.4837 15.992 14.2559 17.4876 14.1141 18.2442C13.8853 19.4657 12.1532 20.2006 11.226 20.8563C10.6741 21.2466 10.0043 20.782 9.93278 20.1778C9.79643 19.0261 9.53961 16.6864 9.25927 13.2614C9.23409 12.9539 9.08486 12.6761 8.85746 12.5061Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path>"#;

const SVG_USER_PLUS: &str = r#"<path d="M3 20.5002C3.28417 16.8058 6.3 13.7193 10.0008 13.5379C10.3134 13.5226 10.6446 13.5097 11 13.5L11.995 13.5663C12.6939 13.6129 13.3665 13.7543 14 13.9777" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><path d="M18 15.5V21.5M21 18.5L15 18.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></path><circle cx="11" cy="6.5" r="4" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.9"></circle>"#;

/// Active top-level tab on [`SettingsMembersScreen`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MembersTab {
    /// Full workspace roster (active members + pending invites).
    #[default]
    Members,
    /// Filtered view of pending email invitations with resend/revoke controls.
    PendingInvites,
    /// Workspace functional teams (`Product`, `Engineering`, `Design`).
    Teams,
}

impl MembersTab {
    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Members => tr(cx, "Members", "成员"),
            Self::PendingInvites => tr(cx, "Pending invites", "待接受邀请"),
            Self::Teams => tr(cx, "Teams", "团队"),
        }
    }
}

/// Workspace permission role assigned to a member or pending invite.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MemberRole {
    /// Full organization ownership, billing, and deletion rights.
    Owner,
    /// Can invite, remove members, and manage workspace billing.
    Admin,
    /// Standard workspace collaborator.
    #[default]
    Member,
    /// Read-only viewer seat.
    Viewer,
    /// Single-project external guest collaborator.
    Guest,
}

impl MemberRole {
    /// All selectable workspace roles in permission order.
    pub const ALL: [Self; 5] = [
        Self::Owner,
        Self::Admin,
        Self::Member,
        Self::Viewer,
        Self::Guest,
    ];

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Owner => tr(cx, "Owner", "所有者"),
            Self::Admin => tr(cx, "Admin", "管理员"),
            Self::Member => tr(cx, "Member", "成员"),
            Self::Viewer => tr(cx, "Viewer", "只读成员"),
            Self::Guest => tr(cx, "Guest", "访客"),
        }
    }

    fn icon(self) -> ProIcon {
        match self {
            Self::Owner | Self::Admin => ProIcon::Shield,
            Self::Member => ProIcon::User,
            Self::Viewer | Self::Guest => ProIcon::Eye,
        }
    }

    /// Return whether this role carries administrative privileges.
    pub fn is_admin(self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }
}

/// Seat plan tier assigned to an individual member.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MemberPlan {
    /// Unlimited meetings, governance, and 4x workflow runs.
    Business,
    /// Full AI model and agent seat.
    #[default]
    Pro,
    /// Non-billable viewer/free seat.
    Free,
}

impl MemberPlan {
    /// All selectable member seat plans.
    pub const ALL: [Self; 3] = [Self::Business, Self::Pro, Self::Free];

    fn label(self, cx: &App) -> &'static str {
        match self {
            Self::Business => tr(cx, "Business", "商业版"),
            Self::Pro => tr(cx, "Pro", "专业版"),
            Self::Free => tr(cx, "Free", "免费版"),
        }
    }
}

/// Workspace member or invited email entry.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkspaceMember {
    pub id: SharedString,
    pub name: Option<SharedString>,
    pub email: SharedString,
    pub role: MemberRole,
    pub plan: MemberPlan,
    /// Credit/seat quota utilization in `0.0..=1.0`.
    pub usage: f32,
    pub invited: bool,
    pub two_factor: bool,
}

impl WorkspaceMember {
    /// Construct an active workspace member row.
    pub fn active(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
        role: MemberRole,
        plan: MemberPlan,
        usage: f32,
        two_factor: bool,
    ) -> Self {
        Self {
            id: id.into(),
            name: Some(name.into()),
            email: email.into(),
            role,
            plan,
            usage: usage.clamp(0., 1.),
            invited: false,
            two_factor,
        }
    }

    /// Construct a pending email invitation row.
    pub fn invited(
        id: impl Into<SharedString>,
        email: impl Into<SharedString>,
        role: MemberRole,
        plan: MemberPlan,
    ) -> Self {
        Self {
            id: id.into(),
            name: None,
            email: email.into(),
            role,
            plan,
            usage: 0.,
            invited: true,
            two_factor: false,
        }
    }

    /// Display label (`name` when joined, otherwise `email`).
    pub fn display_name(&self) -> &SharedString {
        self.name.as_ref().unwrap_or(&self.email)
    }
}

/// Workspace team grouping member ids.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceTeam {
    pub id: SharedString,
    pub name: SharedString,
    pub description: SharedString,
    pub member_ids: Vec<SharedString>,
}

impl WorkspaceTeam {
    /// Construct a workspace team with a description and member id list.
    pub fn new(
        id: impl Into<SharedString>,
        name: impl Into<SharedString>,
        description: impl Into<SharedString>,
        member_ids: Vec<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            member_ids,
        }
    }
}

/// Computed overview statistics for the seat summary header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeatOverview {
    pub total_members: usize,
    pub active_count: usize,
    pub invited_count: usize,
    pub seat_limit: usize,
    pub admin_count: usize,
}

/// Compute seat and role totals from the current roster.
pub fn compute_seat_overview(members: &[WorkspaceMember], seat_limit: usize) -> SeatOverview {
    let total_members = members.len();
    let invited_count = members.iter().filter(|m| m.invited).count();
    let active_count = total_members.saturating_sub(invited_count);
    let admin_count = members.iter().filter(|m| m.role.is_admin()).count();
    SeatOverview {
        total_members,
        active_count,
        invited_count,
        seat_limit: seat_limit.max(total_members),
        admin_count,
    }
}

/// Filter workspace members by tab, role filter, and case-insensitive name/email search.
pub fn filter_members(
    members: &[WorkspaceMember],
    tab: MembersTab,
    role_filter: Option<MemberRole>,
    query: &str,
) -> Vec<usize> {
    let q = query.trim().to_ascii_lowercase();
    let mut out = Vec::new();
    for (idx, m) in members.iter().enumerate() {
        if tab == MembersTab::PendingInvites && !m.invited {
            continue;
        }
        if let Some(rf) = role_filter
            && m.role != rf
        {
            continue;
        }
        let text_ok = q.is_empty()
            || m.email.to_ascii_lowercase().contains(&q)
            || m.name
                .as_ref()
                .is_some_and(|n| n.to_ascii_lowercase().contains(&q));
        if text_ok {
            out.push(idx);
        }
    }
    out
}

/// Sample workspace members matching the reference `Settings / Teams` DOM.
pub fn default_workspace_members() -> Vec<WorkspaceMember> {
    vec![
        WorkspaceMember::active(
            "alex",
            "Alex Morgan",
            "alex@acme.com",
            MemberRole::Admin,
            MemberPlan::Business,
            0.68,
            true,
        ),
        WorkspaceMember::active(
            "jonas",
            "Jonas Weber",
            "jonas@acme.com",
            MemberRole::Admin,
            MemberPlan::Business,
            0.42,
            true,
        ),
        WorkspaceMember::active(
            "mara",
            "Mara Klein",
            "mara@acme.com",
            MemberRole::Member,
            MemberPlan::Pro,
            0.81,
            true,
        ),
        WorkspaceMember::active(
            "lea",
            "Lea Braun",
            "lea@acme.com",
            MemberRole::Member,
            MemberPlan::Pro,
            0.23,
            false,
        ),
        WorkspaceMember::active(
            "tom",
            "Tom Fischer",
            "tom@acme.com",
            MemberRole::Viewer,
            MemberPlan::Free,
            0.05,
            false,
        ),
        WorkspaceMember::invited("sam", "sam@acme.com", MemberRole::Member, MemberPlan::Pro),
    ]
}

/// Sample workspace teams for the `Teams` tab.
pub fn default_workspace_teams() -> Vec<WorkspaceTeam> {
    vec![
        WorkspaceTeam::new(
            "product",
            "Product & Leadership",
            "Roadmap, launches, and workspace administration",
            vec!["alex".into(), "jonas".into(), "mara".into()],
        ),
        WorkspaceTeam::new(
            "engineering",
            "Engineering",
            "Core platform, agents, and deployment pipelines",
            vec!["jonas".into(), "mara".into(), "lea".into()],
        ),
        WorkspaceTeam::new(
            "design",
            "Design & Research",
            "Brand systems, user research, and QA review",
            vec!["alex".into(), "lea".into(), "tom".into()],
        ),
    ]
}

/// Events emitted by [`SettingsMembersScreen`].
#[derive(Clone, Debug, PartialEq)]
pub enum SettingsMembersEvent {
    /// Sidebar or banner link navigation (`"Billing"`, `"Back to app"`, etc.).
    Navigate(SharedString),
    /// New email invitation sent.
    Invited {
        email: SharedString,
        role: MemberRole,
    },
    /// Pending invitation resent.
    InviteResent(SharedString),
    /// Member role updated.
    RoleChanged {
        member_id: SharedString,
        role: MemberRole,
    },
    /// Member plan tier updated.
    PlanChanged {
        member_id: SharedString,
        plan: MemberPlan,
    },
    /// Member or pending invite removed from the workspace.
    Removed(SharedString),
    /// Verified email domain auto-join toggled.
    DomainAutoJoinChanged(bool),
    /// Member roster exported as CSV.
    ExportedCsv(usize),
    /// New team created.
    TeamCreated(SharedString),
}

impl EventEmitter<SettingsMembersEvent> for SettingsMembersScreen {}

#[derive(Clone, PartialEq, Eq)]
enum OpenPopover {
    RoleFilter,
    RowRole(usize),
    RowPlan(usize),
    RowMore(usize),
    BulkRole,
}

/// KUI Pro `Settings / Members & Teams` screen (`settings-members`).
pub struct SettingsMembersScreen {
    id: ElementId,
    shell: Entity<AppShell>,
    focus: FocusHandle,
    standalone: bool,
    loading: bool,
    empty: bool,
    me: SharedString,
    seat_limit: usize,
    tab: MembersTab,
    members: Vec<WorkspaceMember>,
    teams: Vec<WorkspaceTeam>,
    selected_ids: BTreeSet<SharedString>,
    role_filter: Option<MemberRole>,
    invite_open: bool,
    invite_role: MemberRole,
    team_form_open: bool,
    domain_auto_join: bool,
    verified_domain: SharedString,
    popover: Option<OpenPopover>,
    last_popover: Option<OpenPopover>,
    toast: Option<(SharedString, Instant)>,
    search_editor: Option<Entity<Editor>>,
    invite_editor: Option<Entity<Editor>>,
    team_name_editor: Option<Entity<Editor>>,
    measured_width: Rc<Cell<f32>>,
    mounted_at: Instant,
}

impl SettingsMembersScreen {
    /// Construct a `Settings / Members` screen hosted inside [`AppShellPreset::Settings`].
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let this = cx.entity();
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Settings, shell_cx)
                .with_active("Members")
                .with_work_area(false, false);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s
        });
        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                cx.emit(SettingsMembersEvent::Navigate(label.clone()));
            }
            AppShellEvent::Back => {
                cx.emit(SettingsMembersEvent::Navigate("Back to app".into()));
            }
            _ => {}
        })
        .detach();

        Self {
            id: ElementId::Name(id_str),
            shell,
            focus: cx.focus_handle(),
            standalone: false,
            loading: false,
            empty: false,
            me: "alex".into(),
            seat_limit: 10,
            tab: MembersTab::Members,
            members: default_workspace_members(),
            teams: default_workspace_teams(),
            selected_ids: BTreeSet::new(),
            role_filter: None,
            invite_open: false,
            invite_role: MemberRole::Member,
            team_form_open: false,
            domain_auto_join: true,
            verified_domain: "acme.com".into(),
            popover: None,
            last_popover: None,
            toast: None,
            search_editor: None,
            invite_editor: None,
            team_name_editor: None,
            measured_width: Rc::new(Cell::new(960.)),
            mounted_at: Instant::now(),
        }
    }

    /// Render only the inner settings page without the outer [`AppShell`] frame.
    pub fn standalone(mut self) -> Self {
        self.standalone = true;
        self
    }

    /// Override the workspace name displayed in the settings sidebar.
    pub fn with_workspace(
        self,
        workspace: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let ws = workspace.into();
        self.shell.update(cx, |s, cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", cx)).with_workspace(ws);
        });
        self
    }

    /// Override the signed-in account footer in the settings sidebar.
    pub fn with_user(
        self,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        let (n, e) = (name.into(), email.into());
        self.shell.update(cx, |s, cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", cx)).with_user(n, e);
        });
        self
    }

    /// Set the signed-in viewer's member id (`"alex"` by default, marked `(You)` and protected from removal).
    pub fn with_me(mut self, me: impl Into<SharedString>) -> Self {
        self.me = me.into();
        self
    }

    /// Set the initially active tab (`Members`, `PendingInvites`, or `Teams`).
    pub fn with_tab(mut self, tab: MembersTab) -> Self {
        self.tab = tab;
        self
    }

    /// Override the workspace member roster.
    pub fn with_members(mut self, members: Vec<WorkspaceMember>) -> Self {
        self.members = members;
        self
    }

    /// Override the workspace team list.
    pub fn with_teams(mut self, teams: Vec<WorkspaceTeam>) -> Self {
        self.teams = teams;
        self
    }

    /// Set the total purchased seat capacity shown in the summary bar.
    pub fn with_seat_limit(mut self, limit: usize) -> Self {
        self.seat_limit = limit.max(1);
        self
    }

    /// Toggle skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Mutate skeleton loading state at runtime.
    pub fn set_loading(&mut self, loading: bool, cx: &mut Context<Self>) {
        self.loading = loading;
        cx.notify();
    }

    /// Toggle empty state (`No members yet` / `No teams yet`).
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Mutate empty state at runtime.
    pub fn set_empty(&mut self, empty: bool, cx: &mut Context<Self>) {
        self.empty = empty;
        cx.notify();
    }

    fn show_toast(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast = Some((msg.into(), Instant::now()));
        cx.notify();
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.search_editor.is_none() {
            let ph: SharedString = tr(cx, "Search name or email", "搜索姓名或邮箱").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
                EditEvent::Changed | EditEvent::Submitted => cx.notify(),
                EditEvent::Escape => {
                    this.popover = None;
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.search_editor = Some(ed);
        }

        if self.invite_editor.is_none() {
            let ph: SharedString = "colleague@acme.com".into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new("Invite email", InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
                EditEvent::Submitted => this.submit_invite_no_window(cx),
                EditEvent::Escape => {
                    this.invite_open = false;
                    cx.notify();
                }
                EditEvent::Changed => cx.notify(),
                _ => {}
            })
            .detach();
            self.invite_editor = Some(ed);
        }

        if self.team_name_editor.is_none() {
            let ph: SharedString = tr(cx, "Team name (e.g. Security)", "团队名称").into();
            let ed = cx.new(|cx| {
                let mut e = Editor::new(ph.clone(), InputMode::Single, w, cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |this, _, ev: &EditEvent, cx| match ev {
                EditEvent::Submitted => this.submit_team_no_window(cx),
                EditEvent::Escape => {
                    this.team_form_open = false;
                    cx.notify();
                }
                EditEvent::Changed => cx.notify(),
                _ => {}
            })
            .detach();
            self.team_name_editor = Some(ed);
        }
    }

    fn submit_invite_no_window(&mut self, cx: &mut Context<Self>) {
        let raw = self
            .invite_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_string())
            .unwrap_or_default();
        if raw.is_empty() || !raw.contains('@') {
            return;
        }
        let email: SharedString = raw.to_ascii_lowercase().into();
        let role = self.invite_role;
        if !self.members.iter().any(|m| m.email == email) {
            let id = format!("inv-{}", self.members.len() + 1);
            self.members.push(WorkspaceMember::invited(
                id,
                email.clone(),
                role,
                MemberPlan::Pro,
            ));
        }
        self.empty = false;
        self.invite_open = false;
        cx.emit(SettingsMembersEvent::Invited {
            email: email.clone(),
            role,
        });
        self.show_toast(format!("Invitation sent to {email}"), cx);
    }

    fn submit_invite(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.submit_invite_no_window(cx);
        if !self.invite_open
            && let Some(ed) = &self.invite_editor
        {
            ed.update(cx, |e, cx| e.set_value("", w, cx));
        }
    }

    fn submit_team_no_window(&mut self, cx: &mut Context<Self>) {
        let name = self
            .team_name_editor
            .as_ref()
            .map(|e| e.read(cx).value().trim().to_string())
            .unwrap_or_default();
        if name.is_empty() {
            return;
        }
        let id = format!("team-{}", self.teams.len() + 1);
        let shared_name: SharedString = name.into();
        self.teams.push(WorkspaceTeam::new(
            id,
            shared_name.clone(),
            "Shared workspace agents and project permissions",
            vec![self.me.clone()],
        ));
        self.empty = false;
        self.team_form_open = false;
        cx.emit(SettingsMembersEvent::TeamCreated(shared_name.clone()));
        self.show_toast(format!("Created team {shared_name}"), cx);
    }

    fn submit_team(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.submit_team_no_window(cx);
        if !self.team_form_open
            && let Some(ed) = &self.team_name_editor
        {
            ed.update(cx, |e, cx| e.set_value("", w, cx));
        }
    }

    fn handle_key(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        match ev.keystroke.key.as_str() {
            "escape" => {
                if self.popover.take().is_some()
                    || self.invite_open
                    || self.team_form_open
                    || !self.selected_ids.is_empty()
                {
                    self.invite_open = false;
                    self.team_form_open = false;
                    self.selected_ids.clear();
                    cx.notify();
                }
            }
            "/" if !ev.keystroke.modifiers.platform && !ev.keystroke.modifiers.control => {
                if let Some(ed) = &self.search_editor {
                    let f = ed.read(cx).focus.clone();
                    if !f.is_focused(w) {
                        w.focus(&f, cx);
                        cx.stop_propagation();
                    }
                }
            }
            _ => {}
        }
    }

    fn render_checkbox(
        id: ElementId,
        aria: SharedString,
        checked: bool,
        indeterminate: bool,
        disabled: bool,
        cx: &Context<Self>,
        on_toggle: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + Clone + 'static,
    ) -> AnyElement {
        let p = ProPalette::get(cx);
        let filled = checked || indeterminate;
        let box_el = div()
            .id(id)
            .role(Role::CheckBox)
            .aria_label(aria)
            .when(!disabled, |d| d.tab_index(0).cursor_pointer())
            .when(disabled, |d| d.opacity(0.38).cursor_default())
            .size(px(16.))
            .rounded(px(4.5))
            .bg(if filled { p.primary } else { p.card })
            .when(!filled, |d| d.border_1().border_color(p.line_strong))
            .flex()
            .items_center()
            .justify_center()
            .focus_visible(move |s| s.border_1().border_color(p.focus))
            .when(indeterminate, |d| {
                d.child(pro_icon(ProIcon::Minus, 11., p.on_primary))
            })
            .when(checked && !indeterminate, |d| {
                d.child(pro_icon(ProIcon::Check, 11., p.on_primary))
            });
        if disabled {
            box_el.into_any_element()
        } else {
            activate(box_el, cx, on_toggle).into_any_element()
        }
    }

    fn render_overview_stats(
        &self,
        overview: SeatOverview,
        narrow: bool,
        cx: &Context<Self>,
    ) -> Div {
        let p = ProPalette::get(cx);
        let seat_pct = if overview.seat_limit == 0 {
            0.0
        } else {
            (overview.total_members as f32 / overview.seat_limit as f32).clamp(0., 1.)
        };

        let cell_1 = div()
            .flex_1()
            .p(px(15.))
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                cx,
                "Members",
                "成员总数",
            )))
            .child(
                div()
                    .text_size(px(20.))
                    .line_height(px(28.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(overview.total_members.to_string()),
            )
            .child(div().text_size(px(12.)).text_color(p.muted).child(format!(
                "{} {} · {} {}",
                overview.active_count,
                tr(cx, "active", "已激活"),
                overview.invited_count,
                tr(cx, "invited", "待接受")
            )));

        let cell_2 = div()
            .flex_1()
            .p(px(15.))
            .when(narrow, |d| d.border_t_1().border_color(p.line))
            .when(!narrow, |d| d.border_l_1().border_color(p.line))
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(tr(cx, "Seats", "席位")),
            )
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(20.))
                            .line_height(px(28.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.strong)
                            .child(overview.total_members.to_string()),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(p.muted)
                            .child(format!("of {}", overview.seat_limit)),
                    ),
            )
            .child(div().mt(px(6.)).w_full().child(pro_spring_bar(
                pro_eid(&self.id, "seats-overview-bar"),
                seat_pct,
                4.,
                2.,
                p.chip,
                p.blue,
            )));

        let cell_3 = div()
            .flex_1()
            .p(px(15.))
            .when(narrow, |d| d.border_t_1().border_color(p.line))
            .when(!narrow, |d| d.border_l_1().border_color(p.line))
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(tr(cx, "Admins", "管理员")),
            )
            .child(
                div()
                    .text_size(px(20.))
                    .line_height(px(28.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(overview.admin_count.to_string()),
            )
            .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                cx,
                "Can invite, remove and bill",
                "可邀请成员、移除席位与管理账单",
            )));

        div()
            .mt(px(20.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .flex()
            .when(narrow, |d| d.flex_col())
            .child(cell_1)
            .child(cell_2)
            .child(cell_3)
    }

    fn render_invite_form(&self, narrow: bool, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let roles = [
            MemberRole::Owner,
            MemberRole::Admin,
            MemberRole::Member,
            MemberRole::Guest,
        ];
        let inv_role_key = match self.invite_role {
            MemberRole::Owner => "owner",
            MemberRole::Admin => "admin",
            MemberRole::Member => "member",
            MemberRole::Viewer => "viewer",
            MemberRole::Guest => "guest",
        };
        let role_sensor = pro_selection_sensor();
        let mut role_seg = div()
            .relative()
            .p(px(2.))
            .rounded(px(8.))
            .bg(p.chip)
            .flex()
            .items_center()
            .gap(px(2.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "inv-role-pill"),
                inv_role_key,
                role_sensor.clone(),
                6.,
                p.card,
                Some(p.line),
                true,
            ));
        for (ri, r) in roles.into_iter().enumerate() {
            let active = self.invite_role == r;
            role_seg = role_seg.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("inv-role-{ri}")))
                    .role(Role::RadioButton)
                    .tab_index(0)
                    .relative()
                    .h(px(26.))
                    .px(px(10.))
                    .rounded(px(6.))
                    .when(active, |d| d.child(pro_mark_selected(&role_sensor)))
                    .flex()
                    .items_center()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if active { p.ink } else { p.muted })
                    .cursor_pointer()
                    .child(r.label(cx)),
                cx,
                move |this, _, cx| {
                    this.invite_role = r;
                    cx.notify();
                },
            ));
        }

        div().pt(px(12.)).child(
            div()
                .p(px(14.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.strong)
                                .child(tr(cx, "Invite people to Acme", "邀请新成员加入工作区")),
                        )
                        .child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                            cx,
                            "Enter sends · Esc closes",
                            "回车发送 · Esc 关闭",
                        ))),
                )
                .child(
                    div()
                        .flex()
                        .when(narrow, |d| d.flex_col().items_start())
                        .when(!narrow, |d| d.items_center())
                        .gap(px(8.))
                        .child(
                            div()
                                .flex_1()
                                .w_full()
                                .h(px(32.))
                                .px(px(10.))
                                .rounded(px(7.))
                                .bg(p.subtle)
                                .border_1()
                                .border_color(p.line_strong)
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(pro_icon(ProIcon::Mail, 14., p.muted))
                                .when_some(self.invite_editor.clone(), |d, ed| {
                                    d.child(div().flex_1().min_w_0().child(ed))
                                }),
                        )
                        .child(role_seg)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.))
                                .child(pro_btn(
                                    pro_eid(&self.id, "inv-cancel"),
                                    tr(cx, "Cancel", "取消"),
                                    None,
                                    ProBtnStyle::Ghost,
                                    true,
                                    cx,
                                    |this, _, cx| {
                                        this.invite_open = false;
                                        cx.notify();
                                    },
                                ))
                                .child(pro_btn(
                                    pro_eid(&self.id, "inv-send"),
                                    tr(cx, "Send invite", "发送邀请"),
                                    None,
                                    ProBtnStyle::Primary,
                                    true,
                                    cx,
                                    |this, w, cx| this.submit_invite(w, cx),
                                )),
                        ),
                ),
        )
    }

    fn render_bulk_bar(&self, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let count = self.selected_ids.len();
        let bulk_role_open = self.popover == Some(OpenPopover::BulkRole);
        let frame_tint: Hsla = if p.dark {
            hsla(0., 0., 1., 0.04)
        } else {
            hsla(0., 0., 0., 0.025)
        };

        let mut wrap = div().flex().flex_col().gap(px(6.));
        wrap = wrap.child(
            div()
                .px(px(12.))
                .py(px(6.))
                .rounded(px(10.))
                .bg(frame_tint)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(6.))
                .child(
                    div()
                        .mr(px(6.))
                        .text_size(px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(format!("{count} {}", tr(cx, "selected", "项已选"))),
                )
                .child(pro_btn(
                    pro_eid(&self.id, "bulk-role-btn"),
                    tr(cx, "Change role", "批量改角色"),
                    Some(ProIcon::Shield),
                    ProBtnStyle::Secondary,
                    true,
                    cx,
                    |this, _, cx| {
                        this.popover = if this.popover == Some(OpenPopover::BulkRole) {
                            None
                        } else {
                            Some(OpenPopover::BulkRole)
                        };
                        cx.notify();
                    },
                ))
                .child(pro_btn(
                    pro_eid(&self.id, "bulk-remove"),
                    tr(cx, "Remove", "移除"),
                    Some(ProIcon::Trash),
                    ProBtnStyle::Danger,
                    true,
                    cx,
                    |this, _, cx| {
                        let picked = std::mem::take(&mut this.selected_ids);
                        let me_id = this.me.clone();
                        let mut removed_n = 0usize;
                        this.members.retain(|m| {
                            if m.id != me_id && picked.contains(&m.id) {
                                removed_n += 1;
                                cx.emit(SettingsMembersEvent::Removed(m.id.clone()));
                                false
                            } else {
                                true
                            }
                        });
                        this.popover = None;
                        this.show_toast(format!("Removed {removed_n} member(s)"), cx);
                    },
                ))
                .child(div().ml_auto().child(pro_btn(
                    pro_eid(&self.id, "bulk-clear"),
                    tr(cx, "Clear", "取消选择"),
                    None,
                    ProBtnStyle::Ghost,
                    true,
                    cx,
                    |this, _, cx| {
                        this.selected_ids.clear();
                        this.popover = None;
                        cx.notify();
                    },
                ))),
        );

        let mut menu = div()
            .p(px(4.))
            .rounded(px(8.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(4.));
        for (ri, r) in MemberRole::ALL.into_iter().enumerate() {
            menu = menu.child(pro_btn(
                pro_eid(&self.id, format!("bulk-role-opt-{ri}")),
                r.label(cx),
                Some(r.icon()),
                ProBtnStyle::Ghost,
                true,
                cx,
                move |this, _, cx| {
                    let mut n = 0usize;
                    for m in &mut this.members {
                        if this.selected_ids.contains(&m.id) {
                            m.role = r;
                            n += 1;
                        }
                    }
                    this.popover = None;
                    this.show_toast(format!("Updated role for {n} member(s)"), cx);
                },
            ));
        }
        let menu_el = menu.into_any_element();
        wrap = wrap.child(pro_fold(
            pro_eid(&self.id, "bulk-role-fold"),
            bulk_role_open,
            40.,
            6.,
            move || menu_el,
        ));
        div().pt(px(12.)).child(wrap)
    }

    fn render_members_table(&self, visible: &[usize], narrow: bool, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let selectable_ids: Vec<SharedString> = visible
            .iter()
            .map(|&i| &self.members[i])
            .filter(|m| m.id != self.me)
            .map(|m| m.id.clone())
            .collect();
        let selected_visible = selectable_ids
            .iter()
            .filter(|id| self.selected_ids.contains(*id))
            .count();
        let all_checked = !selectable_ids.is_empty() && selected_visible == selectable_ids.len();
        let some_checked = selected_visible > 0 && !all_checked;

        let head_check = Self::render_checkbox(
            pro_eid(&self.id, "chk-all"),
            tr(cx, "Select all members", "全选成员").into(),
            all_checked,
            some_checked,
            selectable_ids.is_empty(),
            cx,
            move |this, _, cx| {
                let all_now = selectable_ids
                    .iter()
                    .all(|id| this.selected_ids.contains(id));
                if all_now {
                    for id in &selectable_ids {
                        this.selected_ids.remove(id);
                    }
                } else {
                    for id in &selectable_ids {
                        this.selected_ids.insert(id.clone());
                    }
                }
                cx.notify();
            },
        );

        let mut table = div()
            .mt(px(12.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .min_h(px(40.))
                    .pl(px(14.))
                    .pr(px(10.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(div().w(px(28.)).flex_none().child(head_check))
                    .child(div().flex_1().min_w_0().child(tr(cx, "Name", "姓名")))
                    .child(
                        div()
                            .w(px(if narrow { 100. } else { 122. }))
                            .flex_none()
                            .child(tr(cx, "Role", "角色")),
                    )
                    .when(!narrow, |d| {
                        d.child(div().w(px(106.)).flex_none().child(tr(cx, "Plan", "套餐")))
                            .child(div().w(px(116.)).flex_none().child(tr(cx, "Usage", "用量")))
                    })
                    .child(div().w(px(36.)).flex_none()),
            );

        for &mi in visible {
            let member = &self.members[mi];
            let is_me = member.id == self.me;
            let picked = self.selected_ids.contains(&member.id);
            let m_id = member.id.clone();
            let display = member.display_name().clone();
            let role_open = self.popover == Some(OpenPopover::RowRole(mi));
            let plan_open = self.popover == Some(OpenPopover::RowPlan(mi));
            let more_open = self.popover == Some(OpenPopover::RowMore(mi));

            let row_check = Self::render_checkbox(
                pro_eid(&self.id, format!("chk-{}", member.id)),
                format!("Select {display}").into(),
                picked,
                false,
                is_me,
                cx,
                move |this, _, cx| {
                    if !this.selected_ids.remove(&m_id) {
                        this.selected_ids.insert(m_id.clone());
                    }
                    cx.notify();
                },
            );

            let avatar_el: AnyElement = if let Some(name) = &member.name {
                pro_person_avatar(name.as_ref(), 28., false, None, cx)
            } else {
                let rose_bg: Hsla = if p.dark {
                    rgb(0x492739).into()
                } else {
                    rgb(0xfce7f3).into()
                };
                div()
                    .size(px(28.))
                    .flex_none()
                    .rounded_full()
                    .bg(rose_bg)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(pro_icon(ProIcon::Mail, 13., p.pink))
                    .into_any_element()
            };

            let role_btn = activate(
                div()
                    .id(pro_eid(&self.id, format!("role-btn-{}", member.id)))
                    .role(Role::Button)
                    .aria_label(format!("Role of {display}"))
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(8.))
                    .ml(px(-8.))
                    .rounded(px(6.))
                    .when(role_open, |d| d.bg(p.chip))
                    .hover(move |s| s.bg(p.chip))
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .cursor_pointer()
                    .child(pro_icon(member.role.icon(), 13.5, p.muted))
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(p.text)
                            .child(member.role.label(cx)),
                    )
                    .child(pro_icon(ProIcon::ChevronDown, 11., p.muted)),
                cx,
                move |this, _, cx| {
                    this.popover = if this.popover == Some(OpenPopover::RowRole(mi)) {
                        None
                    } else {
                        Some(OpenPopover::RowRole(mi))
                    };
                    cx.notify();
                },
            );

            let is_business = member.plan == MemberPlan::Business;
            let plan_btn = activate(
                div()
                    .id(pro_eid(&self.id, format!("plan-btn-{}", member.id)))
                    .role(Role::Button)
                    .aria_label(format!("Plan of {display}"))
                    .tab_index(0)
                    .h(px(24.))
                    .pl(px(8.))
                    .pr(px(6.))
                    .rounded(px(6.))
                    .bg(if is_business { p.accent_soft } else { p.chip })
                    .hover(move |s| s.opacity(0.88))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .cursor_pointer()
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if is_business { p.blue } else { p.text })
                            .child(member.plan.label(cx)),
                    )
                    .child(pro_icon(
                        ProIcon::ChevronDown,
                        11.,
                        if is_business { p.blue } else { p.muted },
                    )),
                cx,
                move |this, _, cx| {
                    this.popover = if this.popover == Some(OpenPopover::RowPlan(mi)) {
                        None
                    } else {
                        Some(OpenPopover::RowPlan(mi))
                    };
                    cx.notify();
                },
            );

            let pct = (member.usage * 100.).round() as u32;
            let high_usage = pct >= 80;
            let bar_fill: Hsla = if high_usage {
                rgb(0xd97506).into()
            } else {
                p.faint
            };

            let usage_cell = if member.invited {
                div()
                    .w(px(116.))
                    .flex_none()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child("—")
            } else {
                div()
                    .w(px(116.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().w(px(56.)).flex_none().child(pro_spring_bar(
                        pro_eid(&self.id, format!("member-usage-bar-{}", member.id)),
                        member.usage.clamp(0.03, 1.0),
                        4.,
                        2.,
                        p.chip,
                        bar_fill,
                    )))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(p.muted)
                            .child(format!("{pct}%")),
                    )
            };

            table = table.child(
                div()
                    .min_h(px(56.))
                    .pl(px(14.))
                    .pr(px(10.))
                    .border_t_1()
                    .border_color(p.line)
                    .when(picked, |d| d.bg(p.hover))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(div().w(px(28.)).flex_none().child(row_check))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(avatar_el)
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
                                            .gap(px(5.))
                                            .min_w_0()
                                            .child(
                                                div()
                                                    .truncate()
                                                    .text_size(px(13.))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(p.strong)
                                                    .child(display.clone()),
                                            )
                                            .when(is_me, |d| {
                                                d.child(
                                                    div()
                                                        .text_size(px(12.5))
                                                        .text_color(p.muted)
                                                        .child(tr(cx, "(You)", "(你)")),
                                                )
                                            })
                                            .when(member.invited, |d| {
                                                d.child(
                                                    div()
                                                        .px(px(6.))
                                                        .h(px(18.))
                                                        .rounded(px(4.))
                                                        .bg(p.chip)
                                                        .flex()
                                                        .items_center()
                                                        .text_size(px(11.))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(p.muted)
                                                        .child(tr(cx, "Invited", "待接受")),
                                                )
                                            })
                                            .when(member.two_factor, |d| {
                                                d.child(pro_badge("2FA", ProTone::Green, false, cx))
                                            }),
                                    )
                                    .when(member.name.is_some(), |d| {
                                        d.child(
                                            div()
                                                .truncate()
                                                .text_size(px(12.))
                                                .text_color(p.muted)
                                                .child(member.email.clone()),
                                        )
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .w(px(if narrow { 100. } else { 122. }))
                            .flex_none()
                            .flex()
                            .items_center()
                            .child(role_btn),
                    )
                    .when(!narrow, |d| {
                        d.child(
                            div()
                                .w(px(106.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .child(plan_btn),
                        )
                        .child(usage_cell)
                    })
                    .child(
                        div()
                            .w(px(36.))
                            .flex_none()
                            .flex()
                            .justify_end()
                            .child(pro_icon_btn(
                                pro_eid(&self.id, format!("more-{}", member.id)),
                                format!("More for {display}"),
                                ProIcon::MoreHorizontal,
                                28.,
                                more_open,
                                cx,
                                move |this, _, cx| {
                                    this.popover = if this.popover == Some(OpenPopover::RowMore(mi))
                                    {
                                        None
                                    } else {
                                        Some(OpenPopover::RowMore(mi))
                                    };
                                    cx.notify();
                                },
                            )),
                    ),
            );

            let row_open = role_open || plan_open || more_open;
            let active_pop = self.popover.as_ref().or(self.last_popover.as_ref());
            let show_role =
                role_open || matches!(active_pop, Some(OpenPopover::RowRole(i)) if *i == mi);
            let show_plan = !show_role
                && (plan_open || matches!(active_pop, Some(OpenPopover::RowPlan(i)) if *i == mi));

            let mut strip = div()
                .px(px(14.))
                .py(px(8.))
                .bg(p.subtle)
                .border_t_1()
                .border_color(p.line)
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(6.));

            if show_role {
                for (ri, r) in MemberRole::ALL.into_iter().enumerate() {
                    let chosen = member.role == r;
                    let person = display.clone();
                    strip = strip.child(pro_btn(
                        pro_eid(&self.id, format!("row-role-opt-{mi}-{ri}")),
                        r.label(cx),
                        Some(r.icon()),
                        if chosen {
                            ProBtnStyle::Primary
                        } else {
                            ProBtnStyle::Secondary
                        },
                        true,
                        cx,
                        move |this, _, cx| {
                            if let Some(m) = this.members.get_mut(mi) {
                                m.role = r;
                                let mid = m.id.clone();
                                this.popover = None;
                                cx.emit(SettingsMembersEvent::RoleChanged {
                                    member_id: mid,
                                    role: r,
                                });
                                this.show_toast(format!("Updated role for {person}"), cx);
                            }
                        },
                    ));
                }
            } else if show_plan {
                for (pi, pl) in MemberPlan::ALL.into_iter().enumerate() {
                    let chosen = member.plan == pl;
                    let person = display.clone();
                    strip = strip.child(pro_btn(
                        pro_eid(&self.id, format!("row-plan-opt-{mi}-{pi}")),
                        pl.label(cx),
                        None,
                        if chosen {
                            ProBtnStyle::Primary
                        } else {
                            ProBtnStyle::Secondary
                        },
                        true,
                        cx,
                        move |this, _, cx| {
                            if let Some(m) = this.members.get_mut(mi) {
                                m.plan = pl;
                                let mid = m.id.clone();
                                this.popover = None;
                                cx.emit(SettingsMembersEvent::PlanChanged {
                                    member_id: mid,
                                    plan: pl,
                                });
                                this.show_toast(format!("Updated plan for {person}"), cx);
                            }
                        },
                    ));
                }
            } else {
                if member.invited {
                    let email_target = member.email.clone();
                    strip = strip.child(pro_btn(
                        pro_eid(&self.id, format!("row-resend-{mi}")),
                        tr(cx, "Resend invite", "重发邀请"),
                        Some(ProIcon::Mail),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        move |this, _, cx| {
                            this.popover = None;
                            cx.emit(SettingsMembersEvent::InviteResent(email_target.clone()));
                            this.show_toast(format!("Resent invitation to {email_target}"), cx);
                        },
                    ));
                } else {
                    let person = display.clone();
                    strip = strip.child(pro_btn(
                        pro_eid(&self.id, format!("row-2fa-{mi}")),
                        if member.two_factor {
                            tr(cx, "2FA verified", "已开启双重验证")
                        } else {
                            tr(cx, "Require 2FA", "要求开启 2FA")
                        },
                        Some(ProIcon::KeyLock),
                        ProBtnStyle::Secondary,
                        true,
                        cx,
                        move |this, _, cx| {
                            if let Some(m) = this.members.get_mut(mi) {
                                m.two_factor = !m.two_factor;
                                this.popover = None;
                                this.show_toast(format!("Updated 2FA policy for {person}"), cx);
                            }
                        },
                    ));
                }

                if !is_me {
                    let person = display.clone();
                    strip = strip.child(pro_btn(
                        pro_eid(&self.id, format!("row-remove-{mi}")),
                        if member.invited {
                            tr(cx, "Revoke invite", "撤回邀请")
                        } else {
                            tr(cx, "Remove member", "移除成员")
                        },
                        Some(ProIcon::Trash),
                        ProBtnStyle::Danger,
                        true,
                        cx,
                        move |this, _, cx| {
                            if mi < this.members.len() {
                                let removed = this.members.remove(mi);
                                this.selected_ids.remove(&removed.id);
                                this.popover = None;
                                cx.emit(SettingsMembersEvent::Removed(removed.id));
                                this.show_toast(format!("Removed {person}"), cx);
                            }
                        },
                    ));
                } else {
                    strip = strip.child(div().text_size(px(12.)).text_color(p.muted).child(tr(
                        cx,
                        "Your own admin seat cannot be removed here.",
                        "无法在此移除您自己的管理员席位。",
                    )));
                }
            }
            let strip_el = strip.into_any_element();
            table = table.child(pro_fold(
                pro_eid(&self.id, format!("row-strip-fold-{}", member.id)),
                row_open,
                46.,
                0.,
                move || strip_el,
            ));
        }
        table
    }

    fn render_teams_panel(&self, narrow: bool, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let mut col = div().mt(px(20.)).flex().flex_col().gap(px(12.)).child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_size(px(13.)).text_color(p.muted).child(format!(
                    "{} {}",
                    self.teams.len(),
                    tr(cx, "workspace teams", "个工作区团队")
                )))
                .child(pro_btn(
                    pro_eid(&self.id, "btn-new-team"),
                    tr(cx, "New team", "新建团队"),
                    Some(ProIcon::Plus),
                    ProBtnStyle::Primary,
                    true,
                    cx,
                    |this, _, cx| {
                        this.team_form_open = !this.team_form_open;
                        cx.notify();
                    },
                )),
        );

        let team_form_el = div()
            .p(px(14.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line_strong)
            .flex()
            .when(narrow, |d| d.flex_col().items_start())
            .when(!narrow, |d| d.items_center())
            .gap(px(8.))
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .h(px(32.))
                    .px(px(10.))
                    .rounded(px(7.))
                    .bg(p.subtle)
                    .border_1()
                    .border_color(p.line_strong)
                    .flex()
                    .items_center()
                    .when_some(self.team_name_editor.clone(), |d, ed| {
                        d.child(div().w_full().child(ed))
                    }),
            )
            .child(pro_btn(
                pro_eid(&self.id, "team-cancel"),
                tr(cx, "Cancel", "取消"),
                None,
                ProBtnStyle::Ghost,
                true,
                cx,
                |this, _, cx| {
                    this.team_form_open = false;
                    cx.notify();
                },
            ))
            .child(pro_btn(
                pro_eid(&self.id, "team-create"),
                tr(cx, "Create team", "创建团队"),
                None,
                ProBtnStyle::Primary,
                true,
                cx,
                |this, w, cx| this.submit_team(w, cx),
            ))
            .into_any_element();
        col = col.child(pro_fold(
            pro_eid(&self.id, "team-form-fold"),
            self.team_form_open,
            62.,
            12.,
            move || team_form_el,
        ));

        if self.empty || self.teams.is_empty() {
            return col.child(
                div()
                    .rounded(px(12.))
                    .border_1()
                    .border_color(p.line)
                    .child(pro_empty_state(
                        ProIcon::Users,
                        tr(cx, "No teams yet", "暂无团队"),
                        tr(
                            cx,
                            "Group members into teams to share agents, projects, and folder permissions.",
                            "将成员按职能分组，便于统一共享智能体与项目文件夹权限。",
                        ),
                        Some((
                            pro_eid(&self.id, "empty-new-team"),
                            tr(cx, "New team", "新建团队").into(),
                            Box::new(|this: &mut Self, _, cx| {
                                this.team_form_open = true;
                                cx.notify();
                            }),
                        )),
                        cx,
                    )),
            );
        }

        let mut card = div()
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .overflow_hidden()
            .flex()
            .flex_col();

        for (ti, team) in self.teams.iter().enumerate() {
            let mark_letters = person_initials(team.name.as_ref());
            let mut faces = div().flex().items_center();
            for (fi, mid) in team.member_ids.iter().take(4).enumerate() {
                let label = self
                    .members
                    .iter()
                    .find(|m| &m.id == mid)
                    .map(|m| m.display_name().clone())
                    .unwrap_or_else(|| mid.clone());
                faces = faces.child(
                    div()
                        .when(fi > 0, |d| d.ml(px(-6.)))
                        .rounded_full()
                        .border_1()
                        .border_color(p.card)
                        .child(pro_person_avatar(label.as_ref(), 26., false, None, cx)),
                );
            }
            if team.member_ids.len() > 4 {
                faces = faces.child(
                    div()
                        .ml(px(6.))
                        .text_size(px(12.))
                        .text_color(p.muted)
                        .child(format!("+{}", team.member_ids.len() - 4)),
                );
            }

            card = card.child(
                div()
                    .min_h(px(60.))
                    .pl(px(14.))
                    .pr(px(12.))
                    .py(px(10.))
                    .when(ti > 0, |d| d.border_t_1().border_color(p.line))
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(
                        div()
                            .size(px(30.))
                            .flex_none()
                            .rounded(px(8.))
                            .bg(p.chip)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(11.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.text)
                            .child(mark_letters),
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
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(p.strong)
                                    .child(team.name.clone()),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(format!(
                                        "{} {} · {}",
                                        team.member_ids.len(),
                                        tr(cx, "members", "位成员"),
                                        team.description
                                    )),
                            ),
                    )
                    .child(faces),
            );
        }
        col.child(card)
    }

    fn render_domain_card(&self, narrow: bool, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        let tile_bg: Hsla = if p.dark {
            rgb(0x2a2a2e).into()
        } else {
            rgb(0xffffff).into()
        };
        div()
            .mt(px(24.))
            .rounded(px(12.))
            .bg(p.card)
            .border_1()
            .border_color(p.line)
            .min_h(px(62.))
            .pl(px(16.))
            .pr(px(14.))
            .py(px(12.))
            .flex()
            .when(narrow, |d| d.flex_col().items_start().gap(px(10.)))
            .when(!narrow, |d| d.items_center().justify_between().gap(px(20.)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .min_w_0()
                    .child(
                        div()
                            .size(px(30.))
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
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .min_w_0()
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .items_center()
                                    .gap(px(6.))
                                    .child(
                                        div()
                                            .text_size(px(13.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(p.strong)
                                            .child(format!(
                                                "{} @{}",
                                                tr(
                                                    cx,
                                                    "Verified domain auto-join for",
                                                    "已验证域名自动加入："
                                                ),
                                                self.verified_domain
                                            )),
                                    )
                                    .child(pro_badge(
                                        tr(cx, "Verified", "已验证"),
                                        ProTone::Green,
                                        false,
                                        cx,
                                    )),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.muted)
                                    .child(tr(
                                        cx,
                                        "Anyone signing in with a verified @acme.com email joins automatically as Member.",
                                        "使用已验证 @acme.com 企业邮箱登录的同事将自动以成员身份加入。",
                                    )),
                            ),
                    ),
            )
            .child(pro_switch(
                pro_eid(&self.id, "sw-domain-auto-join"),
                tr(cx, "Allow verified domain auto-join", "开启企业域名自动加入"),
                self.domain_auto_join,
                cx,
                |this, _, cx| {
                    this.domain_auto_join = !this.domain_auto_join;
                    let on = this.domain_auto_join;
                    cx.emit(SettingsMembersEvent::DomainAutoJoinChanged(on));
                    this.show_toast(
                        if on {
                            "Enabled @acme.com auto-join"
                        } else {
                            "Disabled @acme.com auto-join"
                        },
                        cx,
                    );
                },
            ))
    }

    fn render_loading_skeleton(&self, phase: f32, cx: &Context<Self>) -> Div {
        let p = ProPalette::get(cx);
        div()
            .mt(px(20.))
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(
                div()
                    .p(px(16.))
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(20.))
                    .children((0..3).map(|_| {
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(pro_skeleton_bar(64., 12., 4., phase, cx))
                            .child(pro_skeleton_bar(48., 22., 5., phase, cx))
                            .child(pro_skeleton_bar(110., 10., 4., phase, cx))
                    })),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(pro_skeleton_bar(220., 28., 7., phase, cx))
                    .child(pro_skeleton_bar(160., 28., 7., phase, cx)),
            )
            .child(
                div()
                    .rounded(px(12.))
                    .bg(p.card)
                    .border_1()
                    .border_color(p.line)
                    .flex()
                    .flex_col()
                    .children((0..5).map(|i| {
                        div()
                            .h(px(56.))
                            .px(px(16.))
                            .when(i > 0, |d| d.border_t_1().border_color(p.line))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(10.))
                                    .child(pro_skeleton_bar(28., 28., 14., phase, cx))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(6.))
                                            .child(pro_skeleton_bar(120., 13., 4., phase, cx))
                                            .child(pro_skeleton_bar(150., 11., 4., phase, cx)),
                                    ),
                            )
                            .child(pro_skeleton_bar(84., 22., 6., phase, cx))
                    })),
            )
    }

    fn render_page(&mut self, w: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_editors(w, cx);
        let p = ProPalette::get(cx);
        let reduced = Theme::global(cx).reduced_motion;

        if self.popover.is_some() {
            self.last_popover = self.popover.clone();
        }
        if let Some((_, ts)) = &self.toast
            && ts.elapsed().as_secs_f32() > 2.45
        {
            self.toast = None;
        } else if self.toast.is_some() && !reduced {
            w.request_animation_frame();
        }
        let skel_phase = if self.loading && !reduced {
            w.request_animation_frame();
            (self.mounted_at.elapsed().as_secs_f32() * 0.9).fract()
        } else {
            0.25
        };

        let width = self.measured_width.get();
        let narrow = width < 720.;
        let query = self
            .search_editor
            .as_ref()
            .map(|e| e.read(cx).value().to_string())
            .unwrap_or_default();

        let effective_members: &[WorkspaceMember] = if self.empty { &[] } else { &self.members };
        let overview = compute_seat_overview(effective_members, self.seat_limit);

        let tabs = [
            (MembersTab::Members, None),
            (
                MembersTab::PendingInvites,
                (!self.loading && overview.invited_count > 0).then_some(overview.invited_count),
            ),
            (MembersTab::Teams, None),
        ];
        let tab_key = match self.tab {
            MembersTab::Members => "members",
            MembersTab::PendingInvites => "pending",
            MembersTab::Teams => "teams",
        };
        let tab_sensor = pro_selection_sensor();
        let mut tab_strip = div()
            .mt(px(24.))
            .relative()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.))
            .child(pro_pill_indicator(
                pro_eid(&self.id, "members-tab-pill"),
                tab_key,
                tab_sensor.clone(),
                6.,
                p.select,
                Some(p.line_strong),
                false,
            ));
        for (ti, (t, badge_n)) in tabs.into_iter().enumerate() {
            let selected = self.tab == t;
            tab_strip = tab_strip.child(activate(
                div()
                    .id(pro_eid(&self.id, format!("tab-{ti}")))
                    .role(Role::Tab)
                    .tab_index(0)
                    .relative()
                    .h(px(28.))
                    .px(px(11.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(if selected {
                        transparent_black()
                    } else {
                        p.line
                    })
                    .when(selected, |d| d.child(pro_mark_selected(&tab_sensor)))
                    .hover(move |s| s.text_color(p.ink))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .font_weight(if selected {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(if selected { p.ink } else { p.text })
                    .cursor_pointer()
                    .child(t.label(cx))
                    .when_some(badge_n, |d, cnt| {
                        d.child(
                            div()
                                .px(px(5.))
                                .h(px(16.))
                                .rounded_full()
                                .bg(p.chip)
                                .flex()
                                .items_center()
                                .text_size(px(11.))
                                .text_color(p.muted)
                                .child(cnt.to_string()),
                        )
                    }),
                cx,
                move |this, _, cx| {
                    this.tab = t;
                    this.popover = None;
                    cx.notify();
                },
            ));
        }

        let mut content = div()
            .w_full()
            .max_w(px(880.))
            .mx_auto()
            .pt(px(if narrow { 32. } else { 46. }))
            .pb(px(56.))
            .px(px(if narrow { 20. } else { 32. }))
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(24.))
                    .line_height(px(32.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.strong)
                    .child(tr(cx, "Members & teams", "成员与团队")),
            )
            .child(
                div()
                    .mt(px(4.))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(5.))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.text)
                    .child(tr(
                        cx,
                        "Invite people, set what they can do and group them into teams.",
                        "邀请团队成员、分配席位角色并按职能组建协作团队。",
                    ))
                    .child(activate(
                        div()
                            .id(pro_eid(&self.id, "learn-more"))
                            .role(Role::Button)
                            .tab_index(0)
                            .underline()
                            .text_color(p.text)
                            .hover(move |s| s.text_color(p.ink))
                            .cursor_pointer()
                            .child(tr(cx, "Learn more", "了解更多")),
                        cx,
                        |this, _, cx| {
                            this.show_toast("Roles: Owner · Admin · Member · Viewer · Guest", cx);
                        },
                    )),
            )
            .child(tab_strip);

        let (tab_vtx_key, tab_body): (SharedString, AnyElement) = if self.loading {
            (
                "loading".into(),
                self.render_loading_skeleton(skel_phase, cx)
                    .into_any_element(),
            )
        } else if self.tab == MembersTab::Teams {
            (
                "teams".into(),
                self.render_teams_panel(narrow, cx).into_any_element(),
            )
        } else {
            let role_filter_open = self.popover == Some(OpenPopover::RoleFilter);

            let mut panel = div()
                .flex()
                .flex_col()
                .child(self.render_overview_stats(overview, narrow, cx))
                .child(
                    div()
                        .mt(px(16.))
                        .px(px(12.))
                        .py(px(10.))
                        .rounded(px(10.))
                        .bg(p.accent_soft)
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(13.))
                        .text_color(p.blue)
                        .child(pro_svg("0 0 24 24", SVG_INFO_CIRCLE, 15., p.blue))
                        .child(tr(
                            cx,
                            "Extra usage and spending limits are set on",
                            "超额用量与支出上限可在以下页面配置：",
                        ))
                        .child(activate(
                            div()
                                .id(pro_eid(&self.id, "link-usage-limits"))
                                .role(Role::Button)
                                .tab_index(0)
                                .underline()
                                .font_weight(FontWeight::MEDIUM)
                                .cursor_pointer()
                                .child(tr(cx, "Usage & limits", "用量与限额")),
                            cx,
                            |_, _, cx| {
                                cx.emit(SettingsMembersEvent::Navigate("Billing".into()));
                            },
                        )),
                );

            // Toolbar: Search + Role filter + Export + Invite members
            let search_box = div()
                .w(px(if narrow { 200. } else { 232. }))
                .when(narrow, |d| d.flex_1())
                .h(px(28.))
                .px(px(9.))
                .rounded(px(6.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .items_center()
                .gap(px(6.))
                .child(pro_icon(ProIcon::Search, 14., p.muted))
                .when_some(self.search_editor.clone(), |d, ed| {
                    d.child(div().flex_1().min_w_0().child(ed))
                })
                .when(!query.is_empty(), |d| {
                    d.child(pro_icon_btn(
                        pro_eid(&self.id, "clear-member-search"),
                        tr(cx, "Clear search", "清空搜索"),
                        ProIcon::Close,
                        20.,
                        false,
                        cx,
                        |this, w, cx| {
                            if let Some(ed) = &this.search_editor {
                                ed.update(cx, |e, cx| e.set_value("", w, cx));
                            }
                            cx.notify();
                        },
                    ))
                });

            let role_filter_btn = activate(
                div()
                    .id(pro_eid(&self.id, "btn-role-filter"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(10.))
                    .rounded(px(6.))
                    .bg(p.card)
                    .border_1()
                    .border_color(if self.role_filter.is_some() || role_filter_open {
                        p.ink
                    } else {
                        p.line_strong
                    })
                    .hover(move |s| s.bg(p.hover))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .cursor_pointer()
                    .child(pro_svg("0 0 24 24", SVG_FUNNEL, 14., p.muted))
                    .child(div().text_size(px(13.)).text_color(p.ink).child(
                        if let Some(rf) = self.role_filter {
                            rf.label(cx)
                        } else {
                            tr(cx, "Role", "角色")
                        },
                    ))
                    .child(pro_icon(ProIcon::ChevronDown, 12., p.muted)),
                cx,
                |this, _, cx| {
                    this.popover = if this.popover == Some(OpenPopover::RoleFilter) {
                        None
                    } else {
                        Some(OpenPopover::RoleFilter)
                    };
                    cx.notify();
                },
            );

            let invite_btn = activate(
                div()
                    .id(pro_eid(&self.id, "btn-invite-members"))
                    .role(Role::Button)
                    .tab_index(0)
                    .h(px(28.))
                    .px(px(10.))
                    .rounded(px(6.))
                    .bg(p.primary)
                    .hover(move |s| s.bg(p.primary.opacity(0.9)))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .cursor_pointer()
                    .child(pro_svg("0 0 24 24", SVG_USER_PLUS, 14., p.on_primary))
                    .child(
                        div()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.on_primary)
                            .child(tr(cx, "Invite members", "邀请成员")),
                    ),
                cx,
                |this, _, cx| {
                    this.invite_open = !this.invite_open;
                    cx.notify();
                },
            );

            panel = panel.child(
                div()
                    .mt(px(28.))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.))
                    .child(search_box)
                    .child(role_filter_btn)
                    .child(
                        div()
                            .ml_auto()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(pro_btn(
                                pro_eid(&self.id, "btn-export"),
                                tr(cx, "Export", "导出"),
                                Some(ProIcon::Download),
                                ProBtnStyle::Ghost,
                                true,
                                cx,
                                |this, _, cx| {
                                    let count = this.members.len();
                                    cx.emit(SettingsMembersEvent::ExportedCsv(count));
                                    this.show_toast(format!("Exported {count} members as CSV"), cx);
                                },
                            ))
                            .child(invite_btn),
                    ),
            );

            let mut rf_bar = div()
                .p(px(4.))
                .rounded(px(8.))
                .bg(p.card)
                .border_1()
                .border_color(p.line_strong)
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(4.))
                .child(pro_btn(
                    pro_eid(&self.id, "rf-all"),
                    tr(cx, "All roles", "全部角色"),
                    None,
                    if self.role_filter.is_none() {
                        ProBtnStyle::Primary
                    } else {
                        ProBtnStyle::Ghost
                    },
                    true,
                    cx,
                    |this, _, cx| {
                        this.role_filter = None;
                        this.popover = None;
                        cx.notify();
                    },
                ));
            for (ri, r) in MemberRole::ALL.into_iter().enumerate() {
                rf_bar = rf_bar.child(pro_btn(
                    pro_eid(&self.id, format!("rf-opt-{ri}")),
                    r.label(cx),
                    Some(r.icon()),
                    if self.role_filter == Some(r) {
                        ProBtnStyle::Primary
                    } else {
                        ProBtnStyle::Ghost
                    },
                    true,
                    cx,
                    move |this, _, cx| {
                        this.role_filter = Some(r);
                        this.popover = None;
                        cx.notify();
                    },
                ));
            }
            let rf_el = div().pt(px(8.)).child(rf_bar).into_any_element();
            panel = panel.child(pro_fold(
                pro_eid(&self.id, "role-filter-fold"),
                role_filter_open,
                46.,
                0.,
                move || rf_el,
            ));

            let invite_el = self.render_invite_form(narrow, cx).into_any_element();
            panel = panel.child(pro_fold(
                pro_eid(&self.id, "invite-form-fold"),
                self.invite_open,
                96.,
                0.,
                move || invite_el,
            ));

            let bulk_el = self.render_bulk_bar(cx).into_any_element();
            panel = panel.child(pro_fold(
                pro_eid(&self.id, "bulk-bar-fold"),
                !self.selected_ids.is_empty(),
                52.,
                0.,
                move || bulk_el,
            ));

            let rf_tag = match self.role_filter {
                None => "all",
                Some(MemberRole::Owner) => "owner",
                Some(MemberRole::Admin) => "admin",
                Some(MemberRole::Member) => "member",
                Some(MemberRole::Viewer) => "viewer",
                Some(MemberRole::Guest) => "guest",
            };
            let roster_view: AnyElement = if self.empty {
                div()
                    .mt(px(12.))
                    .rounded(px(12.))
                    .border_1()
                    .border_color(p.line)
                    .child(pro_empty_state(
                        ProIcon::Users,
                        tr(cx, "No members yet", "暂无成员"),
                        tr(
                            cx,
                            "Invite teammates by email to share workspace projects, models, and billing.",
                            "通过邮箱邀请同事共享工作区项目、模型配额与团队账单。",
                        ),
                        Some((
                            pro_eid(&self.id, "empty-invite-cta"),
                            tr(cx, "Invite members", "邀请成员").into(),
                            Box::new(|this: &mut Self, _, cx| {
                                this.invite_open = true;
                                cx.notify();
                            }),
                        )),
                        cx,
                    ))
                    .into_any_element()
            } else {
                let visible = filter_members(&self.members, self.tab, self.role_filter, &query);
                if visible.is_empty() {
                    div()
                        .mt(px(12.))
                        .rounded(px(12.))
                        .border_1()
                        .border_color(p.line)
                        .child(pro_empty_state(
                            ProIcon::Search,
                            if self.tab == MembersTab::PendingInvites {
                                tr(cx, "No pending invites", "暂无待接受邀请")
                            } else {
                                tr(cx, "No matching members", "未找到匹配的成员")
                            },
                            tr(
                                cx,
                                "Try clearing your search or role filter, or send a new email invite.",
                                "请可改清空搜索词或角色筛选，或发送新的邮箱邀请。",
                            ),
                            Some((
                                pro_eid(&self.id, "clear-filters-cta"),
                                tr(cx, "Clear filters", "清除筛选").into(),
                                Box::new(|this: &mut Self, w, cx| {
                                    this.role_filter = None;
                                    this.tab = MembersTab::Members;
                                    if let Some(ed) = &this.search_editor {
                                        ed.update(cx, |e, cx| e.set_value("", w, cx));
                                    }
                                    cx.notify();
                                }),
                            )),
                            cx,
                        ))
                        .into_any_element()
                } else {
                    self.render_members_table(&visible, narrow, cx)
                        .into_any_element()
                }
            };
            panel = panel.child(pro_view_transition(
                pro_eid(&self.id, "members-roster-vtx"),
                format!("{tab_key}-{rf_tag}-{}", self.empty),
                roster_view,
            ));

            panel = panel.child(self.render_domain_card(narrow, cx));
            (tab_key.into(), panel.into_any_element())
        };
        content = content.child(pro_view_transition(
            pro_eid(&self.id, "members-tab-vtx"),
            tab_vtx_key,
            tab_body,
        ));

        let measure_cell = self.measured_width.clone();
        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .min_h(px(520.))
            .bg(p.panel)
            .overflow_hidden()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(Self::handle_key))
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
            .child(
                div()
                    .id(pro_eid(&self.id, "scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(content),
            )
            .child(pro_toast_banner(self.toast.clone()))
    }
}

impl Render for SettingsMembersScreen {
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
        MemberRole, MembersTab, compute_seat_overview, default_workspace_members, filter_members,
    };

    #[test]
    fn seat_overview_counts_active_invited_and_admins_accurately() {
        let roster = default_workspace_members();
        let stats = compute_seat_overview(&roster, 10);
        assert_eq!(stats.total_members, 6);
        assert_eq!(stats.active_count, 5);
        assert_eq!(stats.invited_count, 1);
        assert_eq!(stats.seat_limit, 10);
        assert_eq!(stats.admin_count, 2);
    }

    #[test]
    fn member_filtering_respects_pending_tab_role_and_search() {
        let roster = default_workspace_members();

        let pending = filter_members(&roster, MembersTab::PendingInvites, None, "");
        assert_eq!(pending, vec![5]);
        assert!(roster[pending[0]].invited);

        let admins = filter_members(&roster, MembersTab::Members, Some(MemberRole::Admin), "");
        assert_eq!(admins, vec![0, 1]);

        let mara = filter_members(&roster, MembersTab::Members, None, "mara@acme");
        assert_eq!(mara, vec![2]);
    }
}
