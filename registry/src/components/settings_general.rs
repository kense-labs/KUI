//! Workspace general settings screen with icon upload, URL slug, company context, and deletion guard.
//!
//! Renders inside [`AppShell`](crate::components::app_shell::AppShell) with the `Settings`
//! sidebar preset (`General` active under `Workspace`), providing live URL slug editing,
//! an expandable 2,000-character company context editor, dirty-state save/discard actions,
//! and a name-verified workspace deletion confirmation drawer.

use crate::{
    components::{
        app_shell::{AppShell, AppShellEvent, AppShellPreset, ShellNavGroup, ShellNavItem},
        pro_chrome::{
            ProBtnStyle, ProPalette, pro_btn, pro_eid, pro_fold, pro_presence, pro_skeleton_bar,
            pro_spring_f32, pro_toast_banner, pro_view_transition, tr,
        },
    },
    foundation::Theme,
    primitives::{
        control::activate,
        editor::{EditEvent, Editor, InputMode},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const MAX_CONTEXT_CHARS: usize = 2_000;

const EXPAND_SVG: &str = r#"<path d="M16.5 3h4.5v4.5M21 3l-6 6M7.5 21H3v-4.5M3 21l6-6" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.7"/>"#;
const COLLAPSE_SVG: &str = r#"<path d="M15 9h6M15 9V3M15 9l6-6M9 15H3M9 15v6M9 15l-6 6" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.7"/>"#;

const ICON_PRESETS: [&str; 4] = ["indigo", "emerald", "amber", "violet"];

/// Editable workspace configuration values managed by [`SettingsGeneralScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceValues {
    /// Display name shown in the sidebar header and member invitations.
    pub name: SharedString,
    /// URL path slug appended after `app.workspace.ai/`.
    pub slug: SharedString,
    /// Workspace-wide company context automatically provided to the assistant.
    pub context: SharedString,
    /// Optional uploaded workspace icon identifier or preset key.
    pub icon: Option<SharedString>,
}

impl Default for WorkspaceValues {
    fn default() -> Self {
        Self {
            name: "Acme".into(),
            slug: "acme".into(),
            context: "".into(),
            icon: None,
        }
    }
}

impl WorkspaceValues {
    /// Construct workspace values with the given `name` and URL `slug`.
    pub fn new(name: impl Into<SharedString>, slug: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            slug: slug.into(),
            context: "".into(),
            icon: None,
        }
    }

    /// Set the initial assistant company context text.
    pub fn with_context(mut self, context: impl Into<SharedString>) -> Self {
        self.context = context.into();
        self
    }

    /// Set the optional workspace icon preset or key.
    pub fn with_icon(mut self, icon: Option<SharedString>) -> Self {
        self.icon = icon;
        self
    }

    /// Normalize a raw string into a lowercase URL-safe workspace slug.
    pub fn sanitize_slug(raw: &str) -> String {
        let mut out = String::with_capacity(raw.len());
        let mut prev_dash = false;
        for ch in raw.trim().chars() {
            if ch.is_ascii_alphanumeric() {
                out.push(ch.to_ascii_lowercase());
                prev_dash = false;
            } else if (ch == '-' || ch == '_' || ch.is_whitespace())
                && !out.is_empty()
                && !prev_dash
            {
                out.push('-');
                prev_dash = true;
            }
        }
        while out.ends_with('-') {
            out.pop();
        }
        out
    }

    /// Return whether the workspace has a non-empty name, valid slug, and context within limits.
    pub fn is_valid(&self) -> bool {
        let clean_slug = Self::sanitize_slug(&self.slug);
        !self.name.trim().is_empty()
            && !clean_slug.is_empty()
            && self.context.chars().count() <= MAX_CONTEXT_CHARS
    }
}

/// Account identity displayed at the bottom of the settings sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsGeneralUser {
    /// Full display name of the signed-in user.
    pub name: SharedString,
    /// Email address shown in the account menu.
    pub email: SharedString,
}

impl Default for SettingsGeneralUser {
    fn default() -> Self {
        Self {
            name: "Alex Morgan".into(),
            email: "alex@acme.com".into(),
        }
    }
}

impl SettingsGeneralUser {
    /// Construct a sidebar user summary with `name` and `email`.
    pub fn new(name: impl Into<SharedString>, email: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
        }
    }
}

/// Events emitted by [`SettingsGeneralScreen`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsGeneralEvent {
    /// Emitted with the committed [`WorkspaceValues`] whenever workspace settings are saved.
    Save(WorkspaceValues),
    /// Emitted once workspace deletion is confirmed by typing the exact workspace name.
    Delete,
    /// Emitted when a settings sidebar item or `"Back to app"` is activated.
    Navigate(SharedString),
}

impl EventEmitter<SettingsGeneralEvent> for SettingsGeneralScreen {}

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

fn format_char_count(n: usize) -> String {
    if n < 1_000 {
        n.to_string()
    } else {
        format!("{},{:03}", n / 1_000, n % 1_000)
    }
}

/// Workspace general settings screen (`settings-general`).
pub struct SettingsGeneralScreen {
    id: ElementId,
    focus: FocusHandle,
    shell: Entity<AppShell>,
    standalone: bool,
    loading: bool,
    saved: WorkspaceValues,
    draft: WorkspaceValues,
    user: SettingsGeneralUser,
    icon_cycle: usize,
    context_expanded: bool,
    delete_open: bool,
    delete_confirm_text: String,
    toast: Option<(SharedString, Instant)>,
    started_at: Instant,
    page_width: Rc<Cell<f32>>,
    name_editor: Option<Entity<Editor>>,
    slug_editor: Option<Entity<Editor>>,
    context_editor: Option<Entity<Editor>>,
    delete_editor: Option<Entity<Editor>>,
}

impl Focusable for SettingsGeneralScreen {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl SettingsGeneralScreen {
    /// Construct a new [`SettingsGeneralScreen`] with sample `Acme` workspace values.
    pub fn new(id: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<crate::foundation::Theme>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::primitives::control::UiLanguage>(|_, cx| cx.notify())
            .detach();
        let id_str: SharedString = id.into();
        let focus = cx.focus_handle();
        let saved = WorkspaceValues::default();
        let draft = saved.clone();
        let user = SettingsGeneralUser::default();
        let this = cx.entity();
        let shell_user_name = user.name.clone();
        let shell_user_email = user.email.clone();
        let shell_ws = saved.name.clone();
        let shell = cx.new(|shell_cx| {
            let mut s = AppShell::new(format!("{id_str}-shell"), shell_cx)
                .with_preset(AppShellPreset::Settings, shell_cx)
                .with_groups(settings_nav_groups(shell_cx))
                .with_workspace(shell_ws)
                .with_user(shell_user_name, shell_user_email)
                .with_active("General")
                .with_work_area(false, false);
            s.bind_page(&this, shell_cx, |screen, w, cx| {
                screen.render_page(w, cx).into_any_element()
            });
            s
        });
        cx.subscribe(&shell, |_, _, ev: &AppShellEvent, cx| match ev {
            AppShellEvent::Navigate(label) => {
                cx.emit(SettingsGeneralEvent::Navigate(label.clone()));
            }
            AppShellEvent::Back => {
                cx.emit(SettingsGeneralEvent::Navigate("Back to app".into()));
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
            saved,
            draft,
            user,
            icon_cycle: 0,
            context_expanded: false,
            delete_open: false,
            delete_confirm_text: String::new(),
            toast: None,
            started_at: Instant::now(),
            page_width: Rc::new(Cell::new(720.)),
            name_editor: None,
            slug_editor: None,
            context_editor: None,
            delete_editor: None,
        }
    }

    /// Override the initial workspace values.
    pub fn with_workspace(mut self, workspace: WorkspaceValues, cx: &mut Context<Self>) -> Self {
        let ws_name = workspace.name.clone();
        self.saved = workspace.clone();
        self.draft = workspace;
        self.shell.update(cx, |s, s_cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", s_cx)).with_workspace(ws_name);
        });
        self
    }

    /// Override the account displayed at the bottom of the settings sidebar.
    pub fn with_user(mut self, user: SettingsGeneralUser, cx: &mut Context<Self>) -> Self {
        let name = user.name.clone();
        let email = user.email.clone();
        self.user = user;
        self.shell.update(cx, |s, s_cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", s_cx)).with_user(name, email);
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

    /// Return the last committed workspace configuration.
    pub fn workspace(&self) -> &WorkspaceValues {
        &self.saved
    }

    /// Return the sidebar account summary.
    pub fn user(&self) -> &SettingsGeneralUser {
        &self.user
    }

    /// Return the working draft values including unsaved edits.
    pub fn draft(&self) -> &WorkspaceValues {
        &self.draft
    }

    /// Return whether working draft fields differ from the last committed workspace.
    pub fn is_dirty(&self) -> bool {
        self.draft != self.saved
    }

    /// Return whether skeleton placeholders are active.
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

    /// Return whether the inline workspace deletion confirmation drawer is open.
    pub fn is_delete_open(&self) -> bool {
        self.delete_open
    }

    /// Return whether the typed confirmation text matches the committed workspace name.
    pub fn can_confirm_delete(&self) -> bool {
        !self.saved.name.trim().is_empty()
            && self.delete_confirm_text.trim() == self.saved.name.as_ref()
    }

    /// Open the inline workspace deletion confirmation drawer.
    pub fn open_delete_confirm(&mut self, cx: &mut Context<Self>) {
        self.delete_open = true;
        self.delete_confirm_text.clear();
        cx.notify();
    }

    /// Cancel and close the workspace deletion confirmation drawer.
    pub fn cancel_delete_confirm(&mut self, w: Option<&Window>, cx: &mut Context<Self>) {
        self.delete_open = false;
        self.delete_confirm_text.clear();
        if let (Some(ed), Some(win)) = (&self.delete_editor, w) {
            ed.update(cx, |e, e_cx| e.set_value("", win, e_cx));
        }
        cx.notify();
    }

    /// Set the confirmation input string programmatically.
    pub fn set_delete_confirmation(&mut self, input: impl Into<String>, cx: &mut Context<Self>) {
        self.delete_confirm_text = input.into();
        cx.notify();
    }

    /// Confirm workspace deletion if the typed text matches; returns `true` when emitted.
    pub fn confirm_delete(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.can_confirm_delete() {
            return false;
        }
        self.delete_open = false;
        self.delete_confirm_text.clear();
        self.show_toast(
            tr(cx, "Workspace scheduled for deletion", "已确认删除工作区"),
            cx,
        );
        cx.emit(SettingsGeneralEvent::Delete);
        cx.notify();
        true
    }

    /// Upload or cycle the workspace icon immediately and emit [`SettingsGeneralEvent::Save`].
    pub fn upload_icon(&mut self, icon_key: impl Into<SharedString>, cx: &mut Context<Self>) {
        let icon: SharedString = icon_key.into();
        self.draft.icon = Some(icon.clone());
        self.saved.icon = Some(icon);
        self.show_toast(tr(cx, "Workspace icon updated", "工作区图标已更新"), cx);
        cx.emit(SettingsGeneralEvent::Save(self.saved.clone()));
        cx.notify();
    }

    /// Remove the custom workspace icon immediately and emit [`SettingsGeneralEvent::Save`].
    pub fn remove_icon(&mut self, cx: &mut Context<Self>) {
        if self.draft.icon.is_some() || self.saved.icon.is_some() {
            self.draft.icon = None;
            self.saved.icon = None;
            self.show_toast(tr(cx, "Workspace icon removed", "已移除工作区图标"), cx);
            cx.emit(SettingsGeneralEvent::Save(self.saved.clone()));
            cx.notify();
        }
    }

    /// Toggle expanded height on the company context field.
    pub fn toggle_context_expanded(&mut self, cx: &mut Context<Self>) {
        self.context_expanded = !self.context_expanded;
        cx.notify();
    }

    /// Update draft text fields directly (useful for headless tests and programmatic updates).
    pub fn set_draft_fields(
        &mut self,
        name: impl Into<SharedString>,
        slug: impl Into<SharedString>,
        context: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.draft.name = name.into();
        self.draft.slug = slug.into();
        self.draft.context = context.into();
        cx.notify();
    }

    /// Commit working draft changes if valid, emitting [`SettingsGeneralEvent::Save`].
    pub fn commit_save(&mut self, cx: &mut Context<Self>) -> bool {
        let clean_slug = WorkspaceValues::sanitize_slug(&self.draft.slug);
        let trimmed_name = self.draft.name.trim().to_owned();
        if trimmed_name.is_empty()
            || clean_slug.is_empty()
            || self.draft.context.chars().count() > MAX_CONTEXT_CHARS
        {
            return false;
        }
        self.draft.name = trimmed_name.into();
        self.draft.slug = clean_slug.into();
        if self.draft == self.saved {
            return false;
        }
        self.saved = self.draft.clone();
        let ws_name = self.saved.name.clone();
        self.shell.update(cx, |s, s_cx| {
            *s = std::mem::replace(s, AppShell::new("tmp", s_cx)).with_workspace(ws_name);
        });
        self.show_toast(tr(cx, "Changes saved", "更改已保存"), cx);
        cx.emit(SettingsGeneralEvent::Save(self.saved.clone()));
        cx.notify();
        true
    }

    /// Revert unsaved draft edits back to the last committed [`WorkspaceValues`].
    pub fn discard_changes(&mut self, w: Option<&Window>, cx: &mut Context<Self>) {
        self.draft = self.saved.clone();
        if let Some(win) = w {
            if let Some(ed) = &self.name_editor {
                let val = self.saved.name.to_string();
                ed.update(cx, |e, e_cx| e.set_value(val, win, e_cx));
            }
            if let Some(ed) = &self.slug_editor {
                let val = self.saved.slug.to_string();
                ed.update(cx, |e, e_cx| e.set_value(val, win, e_cx));
            }
            if let Some(ed) = &self.context_editor {
                let val = self.saved.context.to_string();
                ed.update(cx, |e, e_cx| e.set_value(val, win, e_cx));
            }
        }
        cx.notify();
    }

    fn show_toast(&mut self, msg: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast = Some((msg.into(), Instant::now()));
        cx.notify();
    }

    fn ensure_editors(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        if self.name_editor.is_none() {
            let label: SharedString = tr(cx, "Workspace name", "工作区名称").into();
            let initial = self.draft.name.to_string();
            let ed = cx.new(|e_cx| {
                let mut e = Editor::new(label, InputMode::Single, w, e_cx);
                e.set_placeholder(tr(e_cx, "Acme", "Acme"));
                e.font_size = 13.;
                e.set_value(initial, w, e_cx);
                e
            });
            cx.subscribe(&ed, |this, ed_ref, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => {
                    this.draft.name = ed_ref.read(cx).value().to_owned().into();
                    cx.notify();
                }
                EditEvent::Submitted => {
                    this.draft.name = ed_ref.read(cx).value().to_owned().into();
                    this.commit_save(cx);
                }
                _ => {}
            })
            .detach();
            self.name_editor = Some(ed);
        }

        if self.slug_editor.is_none() {
            let label: SharedString = tr(cx, "Workspace URL", "工作区地址").into();
            let initial = self.draft.slug.to_string();
            let ed = cx.new(|e_cx| {
                let mut e = Editor::new(label, InputMode::Single, w, e_cx);
                e.set_placeholder("acme");
                e.font_size = 13.;
                e.set_value(initial, w, e_cx);
                e
            });
            cx.subscribe(&ed, |this, ed_ref, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => {
                    this.draft.slug = ed_ref.read(cx).value().to_owned().into();
                    cx.notify();
                }
                EditEvent::Submitted => {
                    this.draft.slug = ed_ref.read(cx).value().to_owned().into();
                    this.commit_save(cx);
                }
                _ => {}
            })
            .detach();
            self.slug_editor = Some(ed);
        }

        if self.context_editor.is_none() {
            let label: SharedString = tr(cx, "Company context", "公司背景").into();
            let ph: SharedString = tr(
                cx,
                "E.g. We have three business units – Sales, Operations and Finance. Common terms: CRM (customer relationship management), SLA (service level agreement). Regional leads escalate to the COO.",
                "例如：公司下设商务、运营与财务三个业务部门。常用术语：CRM（客户关系管理）、SLA（服务等级协议）。区域负责人向 COO 汇报。",
            )
            .into();
            let initial = self.draft.context.to_string();
            let ed = cx.new(|e_cx| {
                let mut e = Editor::new(label, InputMode::Multi, w, e_cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e.set_command_submit(true);
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

        if self.delete_editor.is_none() {
            let label: SharedString = tr(cx, "Confirm workspace name", "输入工作区名称确认").into();
            let ph = self.saved.name.clone();
            let ed = cx.new(|e_cx| {
                let mut e = Editor::new(label, InputMode::Single, w, e_cx);
                e.set_placeholder(ph);
                e.font_size = 13.;
                e
            });
            cx.subscribe(&ed, |this, ed_ref, ev: &EditEvent, cx| match ev {
                EditEvent::Changed => {
                    this.delete_confirm_text = ed_ref.read(cx).value().to_owned();
                    cx.notify();
                }
                EditEvent::Submitted => {
                    this.delete_confirm_text = ed_ref.read(cx).value().to_owned();
                    this.confirm_delete(cx);
                }
                EditEvent::Escape => {
                    this.delete_open = false;
                    this.delete_confirm_text.clear();
                    cx.notify();
                }
                _ => {}
            })
            .detach();
            self.delete_editor = Some(ed);
        }
    }

    fn render_workspace_icon(&self, p: &ProPalette, cx: &mut Context<Self>) -> Div {
        let initial: String = self
            .draft
            .name
            .trim()
            .chars()
            .next()
            .unwrap_or('A')
            .to_uppercase()
            .collect();

        let (tile_bg, tile_fg, tile_border) = match self.draft.icon.as_deref() {
            Some("indigo") => (
                p.blue.opacity(if p.dark { 0.24 } else { 0.14 }),
                p.blue,
                p.blue.opacity(0.36),
            ),
            Some("emerald") => (
                p.green.opacity(if p.dark { 0.22 } else { 0.15 }),
                p.green_ink,
                p.green.opacity(0.36),
            ),
            Some("amber") => (
                p.amber.opacity(if p.dark { 0.22 } else { 0.16 }),
                p.amber_ink,
                p.amber.opacity(0.36),
            ),
            Some(_) => (
                p.purple.opacity(if p.dark { 0.24 } else { 0.14 }),
                p.purple,
                p.purple.opacity(0.36),
            ),
            None => (
                if p.dark {
                    rgb(0x2a2a2e).into()
                } else {
                    rgb(0xffffff).into()
                },
                p.ink,
                p.line_strong,
            ),
        };

        let mut wrap = div()
            .relative()
            .size(px(36.))
            .flex_none()
            .rounded(px(10.))
            .bg(tile_bg)
            .border_1()
            .border_color(tile_border)
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(14.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(tile_fg)
            .child(initial);

        if self.draft.icon.is_some() {
            let hover_ink = p.ink;
            let focus = p.focus;
            let card_bg = p.card;
            let line = p.line_strong;
            let muted = p.muted;
            wrap = wrap.child(activate(
                div()
                    .id(pro_eid(&self.id, "remove-icon"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Remove workspace icon", "移除工作区图标"))
                    .tab_index(0)
                    .absolute()
                    .top(px(-4.))
                    .right(px(-4.))
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
                |this, _, cx| this.remove_icon(cx),
            ));
        }

        wrap
    }

    fn render_loading_skeleton(&self, narrow: bool, phase: f32, p: &ProPalette, cx: &App) -> Div {
        let field_w = if narrow { 220. } else { 260. };
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
                    .child(tr(cx, "General", "常规")),
            )
            .child(
                div()
                    .mt(px(4.))
                    .text_size(px(13.))
                    .line_height(px(20.))
                    .text_color(p.text)
                    .child(tr(
                        cx,
                        "Set up the basics of your workspace.",
                        "设置工作区的基础信息。",
                    )),
            )
            .child(
                div()
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
                                    .child(pro_skeleton_bar(104., 12., 4., phase, cx))
                                    .child(pro_skeleton_bar(172., 10., 4., phase + 0.08, cx)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .child(pro_skeleton_bar(36., 36., 10., phase + 0.12, cx))
                                    .child(pro_skeleton_bar(72., 28., 6., phase + 0.16, cx)),
                            ),
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
                                    .child(pro_skeleton_bar(112., 12., 4., phase + 0.2, cx))
                                    .child(pro_skeleton_bar(168., 10., 4., phase + 0.24, cx)),
                            )
                            .child(pro_skeleton_bar(field_w, 32., 7., phase + 0.28, cx)),
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
                                    .child(pro_skeleton_bar(102., 12., 4., phase + 0.32, cx))
                                    .child(pro_skeleton_bar(196., 10., 4., phase + 0.36, cx)),
                            )
                            .child(pro_skeleton_bar(field_w, 32., 7., phase + 0.4, cx)),
                    ),
            )
            .child(
                div()
                    .mt(px(36.))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(pro_skeleton_bar(124., 14., 4., phase + 0.44, cx))
                    .child(pro_skeleton_bar(360., 12., 4., phase + 0.48, cx)),
            )
            .child(
                div()
                    .mt(px(12.))
                    .w_full()
                    .child(pro_skeleton_bar(760., 84., 8., phase + 0.52, cx).w_full()),
            )
            .child(
                div()
                    .mt(px(36.))
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(pro_skeleton_bar(92., 14., 4., phase + 0.56, cx))
                    .child(
                        div()
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
                                    .child(pro_skeleton_bar(116., 12., 4., phase + 0.6, cx))
                                    .child(pro_skeleton_bar(240., 10., 4., phase + 0.64, cx)),
                            )
                            .child(pro_skeleton_bar(134., 30., 8., phase + 0.68, cx)),
                    ),
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

        let ctx_box_h = pro_spring_f32(
            pro_eid(&self.id, "ctx-box-h"),
            if self.context_expanded { 184. } else { 84. },
            0.24,
            w,
            cx,
        );
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
        let input_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.14)
        } else {
            hsla(0., 0., 0., 0.09)
        };

        let body = if self.loading {
            self.render_loading_skeleton(narrow, phase, &p, cx)
        } else {
            let name_focused = self
                .name_editor
                .as_ref()
                .is_some_and(|e| e.read(cx).focus.is_focused(w));
            let slug_focused = self
                .slug_editor
                .as_ref()
                .is_some_and(|e| e.read(cx).focus.is_focused(w));
            let ctx_focused = self
                .context_editor
                .as_ref()
                .is_some_and(|e| e.read(cx).focus.is_focused(w));
            let del_focused = self
                .delete_editor
                .as_ref()
                .is_some_and(|e| e.read(cx).focus.is_focused(w));

            let name_invalid = self.draft.name.trim().is_empty();
            let slug_invalid = WorkspaceValues::sanitize_slug(&self.draft.slug).is_empty();
            let ctx_chars = self.draft.context.chars().count();
            let ctx_over = ctx_chars > MAX_CONTEXT_CHARS;

            // Row 1: Workspace icon
            let row_icon = div()
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
                                .child(tr(cx, "Workspace icon", "工作区图标")),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(tr(
                                    cx,
                                    "Square, PNG or JPEG, up to 512 KB",
                                    "正方形 PNG 或 JPEG，最大 512 KB",
                                )),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(self.render_workspace_icon(&p, cx))
                        .child(pro_btn(
                            pro_eid(&self.id, "upload-icon"),
                            tr(cx, "Upload", "上传"),
                            Some(ProIcon::Upload),
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            |this, _, cx| {
                                let preset = ICON_PRESETS[this.icon_cycle % ICON_PRESETS.len()];
                                this.icon_cycle = this.icon_cycle.wrapping_add(1);
                                this.upload_icon(preset, cx);
                            },
                        )),
                );

            // Row 2: Workspace name
            let name_box_border = if name_invalid {
                p.red.opacity(0.65)
            } else if name_focused {
                p.focus
            } else {
                input_border
            };
            let hover_border = p.line_strong;
            let row_name = div()
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
                                .child(tr(cx, "Workspace name", "工作区名称")),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(tr(
                                    cx,
                                    "Shown in the sidebar and in invites",
                                    "显示在侧边栏与邀请链接中",
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
                        .border_color(name_box_border)
                        .when(!name_focused && !name_invalid, |d| {
                            d.hover(move |s| s.border_color(hover_border))
                        })
                        .flex()
                        .items_center()
                        .when_some(self.name_editor.clone(), |d, ed| d.child(ed)),
                );

            // Row 3: Workspace URL
            let slug_box_border = if slug_invalid {
                p.red.opacity(0.65)
            } else if slug_focused {
                p.focus
            } else {
                input_border
            };
            let row_slug = div()
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
                                .child(tr(cx, "Workspace URL", "工作区地址")),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(16.))
                                .text_color(p.muted)
                                .child(tr(
                                    cx,
                                    "Changing it breaks links people have saved",
                                    "修改后他人已收藏的链接将失效",
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
                        .border_color(slug_box_border)
                        .when(!slug_focused && !slug_invalid, |d| {
                            d.hover(move |s| s.border_color(hover_border))
                        })
                        .flex()
                        .items_center()
                        .gap(px(0.))
                        .child(
                            div()
                                .flex_none()
                                .text_size(px(13.))
                                .text_color(p.muted)
                                .child("app.workspace.ai/"),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(48.))
                                .h_full()
                                .when_some(self.slug_editor.clone(), |d, ed| d.child(ed)),
                        ),
                );

            let ws_card = div()
                .id(pro_eid(&self.id, "workspace-card"))
                .role(Role::Region)
                .aria_label(tr(cx, "Workspace", "工作区"))
                .mt(px(24.))
                .rounded(px(12.))
                .bg(p.card)
                .border_1()
                .border_color(p.line)
                .flex()
                .flex_col()
                .child(row_icon)
                .child(row_name)
                .child(row_slug);

            // Section 2: Company context
            let ctx_box_border = if ctx_over {
                p.red.opacity(0.65)
            } else if ctx_focused {
                p.focus
            } else {
                input_border
            };
            let expand_hover = p.hover;
            let expand_focus = p.focus;
            let expand_svg = if self.context_expanded {
                COLLAPSE_SVG
            } else {
                EXPAND_SVG
            };
            let expand_aria = if self.context_expanded {
                tr(cx, "Less room", "收起输入框")
            } else {
                tr(cx, "More room", "展开输入框")
            };
            let ctx_section = div()
                .mt(px(36.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.strong)
                        .child(tr(cx, "Company context", "公司背景")),
                )
                .child(
                    div()
                        .max_w(px(620.))
                        .mt(px(2.))
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.muted)
                        .child(tr(
                            cx,
                            "Added to every chat in this workspace, so the assistant already knows how your company works. Keep it short – it counts toward every message.",
                            "会自动附加到当前工作区的每次对话中，让助手了解你们的业务运作方式。请保持简短——它会计入每条消息的上下文。",
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
                                .relative()
                                .w_full()
                                .h(px(ctx_box_h))
                                .py(px(7.))
                                .pl(px(10.))
                                .pr(px(36.))
                                .rounded(px(8.))
                                .bg(input_surface)
                                .border_1()
                                .border_color(ctx_box_border)
                                .when(!ctx_focused && !ctx_over, |d| {
                                    d.hover(move |s| s.border_color(hover_border))
                                })
                                .when_some(self.context_editor.clone(), |d, ed| d.child(ed))
                                .child(activate(
                                    div()
                                        .id(pro_eid(&self.id, "expand-context"))
                                        .role(Role::Button)
                                        .aria_label(expand_aria)
                                        .tab_index(0)
                                        .absolute()
                                        .top(px(4.))
                                        .right(px(4.))
                                        .size(px(26.))
                                        .rounded(px(5.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .hover(move |s| s.bg(expand_hover))
                                        .focus_visible(move |s| s.border_1().border_color(expand_focus))
                                        .child(pro_svg("0 0 24 24", expand_svg, 15., p.muted)),
                                    cx,
                                    |this, _, cx| this.toggle_context_expanded(cx),
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_start()
                                .justify_between()
                                .gap(px(12.))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(if ctx_over { p.red_ink } else { p.muted })
                                        .child(if ctx_over {
                                            tr(
                                                cx,
                                                "Company context is over the 2,000-character limit.",
                                                "公司背景已超出 2,000 字上限。",
                                            )
                                        } else {
                                            tr(
                                                cx,
                                                "Two or three short paragraphs work best: what the company does, how it's organised and the terms people use. Leave out slogans and history.",
                                                "建议写两到三段简短说明：主营业务、组织架构及常用术语，无需口号与发展历史。",
                                            )
                                        }),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(if ctx_over {
                                            p.red
                                        } else if ctx_chars == MAX_CONTEXT_CHARS {
                                            p.muted
                                        } else {
                                            p.faint
                                        })
                                        .child(format!(
                                            "{} / 2,000",
                                            format_char_count(ctx_chars)
                                        )),
                                ),
                        ),
                );

            // Section 3: Danger zone
            let ws_display = if self.saved.name.trim().is_empty() {
                "Acme"
            } else {
                self.saved.name.as_ref()
            };
            let danger_sub = tr(
                cx,
                &format!(
                    "All chats, files and agents of {ws_display} go for good, for every member."
                ),
                &format!("{ws_display} 的全部对话、文件与智能体将对所有成员不可恢复删除。"),
            )
            .to_owned();

            let red = p.red;
            let red_hover = p.red.opacity(0.08);
            let danger_focus = p.focus;
            let danger_trigger = activate(
                div()
                    .id(pro_eid(&self.id, "delete-trigger"))
                    .role(Role::Button)
                    .aria_label(tr(cx, "Delete workspace", "删除工作区"))
                    .tab_index(0)
                    .h(px(30.))
                    .pl(px(10.))
                    .pr(px(12.))
                    .flex_none()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(red.opacity(0.35))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.))
                    .text_size(px(13.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(red)
                    .cursor_pointer()
                    .hover(move |s| s.bg(red_hover))
                    .focus_visible(move |s| s.border_1().border_color(danger_focus))
                    .child(pro_icon(ProIcon::Trash, 14., red))
                    .child(tr(cx, "Delete workspace", "删除工作区")),
                cx,
                |this, _, cx| {
                    if this.delete_open {
                        this.cancel_delete_confirm(None, cx);
                    } else {
                        this.open_delete_confirm(cx);
                    }
                },
            );

            let mut danger_card = div()
                .id(pro_eid(&self.id, "danger-card"))
                .role(Role::Region)
                .aria_label(tr(cx, "Danger zone", "危险操作"))
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
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .line_height(px(20.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.strong)
                                        .child(tr(cx, "Delete workspace", "删除工作区")),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(16.))
                                        .text_color(p.muted)
                                        .child(danger_sub),
                                ),
                        )
                        .child(danger_trigger),
                );

            let can_delete = self.can_confirm_delete();
            let del_border = if del_focused { p.focus } else { input_border };
            let solid_red: Hsla = rgb(0xf04438).into();
            let solid_red_hover: Hsla = rgb(0xc4320a).into();
            let confirm_btn_base = div()
                .id(pro_eid(&self.id, "confirm-delete-btn"))
                .role(Role::Button)
                .aria_label(tr(cx, "Confirm delete workspace", "确认不可恢复删除工作区"))
                .when(can_delete, |d| d.tab_index(0))
                .h(px(30.))
                .px(px(12.))
                .flex_none()
                .rounded(px(8.))
                .bg(solid_red)
                .flex()
                .items_center()
                .justify_center()
                .gap(px(6.))
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(0xffffff))
                .when(!can_delete, |d| d.opacity(0.4).cursor_default())
                .when(can_delete, |d| {
                    d.cursor_pointer()
                        .hover(move |s| s.bg(solid_red_hover))
                        .focus_visible(move |s| s.border_1().border_color(danger_focus))
                })
                .child(pro_icon(ProIcon::Trash, 14., rgb(0xffffff).into()))
                .child(format!("{} {}", tr(cx, "Delete", "删除"), ws_display));

            let confirm_btn = if can_delete {
                activate(confirm_btn_base, cx, |this, w, cx| {
                    if this.confirm_delete(cx)
                        && let Some(ed) = &this.delete_editor
                    {
                        ed.update(cx, |e, e_cx| e.set_value("", w, e_cx));
                    }
                })
                .into_any_element()
            } else {
                confirm_btn_base.into_any_element()
            };

            let delete_drawer = div()
                .pt(px(12.))
                .pb(px(14.))
                .pl(px(if narrow { 14. } else { 16. }))
                .pr(px(if narrow { 14. } else { 12. }))
                .border_t_1()
                .border_color(p.line)
                .flex()
                .when(narrow, |d| d.flex_col().items_stretch().gap(px(10.)))
                .when(!narrow, |d| d.items_end().justify_between().gap(px(16.)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .gap(px(4.))
                                .text_size(px(12.5))
                                .text_color(p.muted)
                                .child(tr(cx, "Type", "输入"))
                                .child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.ink)
                                        .child(ws_display.to_owned()),
                                )
                                .child(tr(cx, "to confirm", "以确认删除")),
                        )
                        .child(
                            div()
                                .when(narrow, |d| d.w_full())
                                .when(!narrow, |d| d.w(px(260.)))
                                .h(px(32.))
                                .px(px(10.))
                                .rounded(px(7.))
                                .bg(input_surface)
                                .border_1()
                                .border_color(del_border)
                                .flex()
                                .items_center()
                                .when_some(self.delete_editor.clone(), |d, ed| d.child(ed)),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(pro_btn(
                            pro_eid(&self.id, "cancel-delete-btn"),
                            tr(cx, "Cancel", "取消"),
                            None,
                            ProBtnStyle::Secondary,
                            true,
                            cx,
                            |this, w, cx| this.cancel_delete_confirm(Some(w), cx),
                        ))
                        .child(confirm_btn),
                )
                .into_any_element();

            danger_card = danger_card.child(pro_fold(
                pro_eid(&self.id, "delete-confirm-fold"),
                self.delete_open,
                78.,
                0.,
                move || delete_drawer,
            ));

            let danger_section = div()
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
                .child(danger_card);

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
                        .child(tr(cx, "General", "常规")),
                )
                .child(
                    div()
                        .mt(px(4.))
                        .text_size(px(13.))
                        .line_height(px(20.))
                        .text_color(p.text)
                        .child(tr(
                            cx,
                            "Set up the basics of your workspace.",
                            "设置工作区的基础信息。",
                        )),
                )
                .child(ws_card)
                .child(ctx_section)
                .child(danger_section)
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
                        |this, _, cx| {
                            this.commit_save(cx);
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
                if cmd && k == "s" {
                    this.commit_save(cx);
                    w.prevent_default();
                    cx.stop_propagation();
                } else if k == "escape" && this.delete_open {
                    this.cancel_delete_confirm(Some(w), cx);
                    w.prevent_default();
                    cx.stop_propagation();
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

impl Render for SettingsGeneralScreen {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.standalone {
            return self.render_page(w, cx).into_any_element();
        }
        self.shell.clone().into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{SettingsGeneralEvent, SettingsGeneralScreen, WorkspaceValues};
    use gpui::{AppContext as _, TestAppContext};
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn slug_sanitization_and_workspace_validation() {
        assert_eq!(
            WorkspaceValues::sanitize_slug("  Acme Labs_2026! "),
            "acme-labs-2026"
        );
        assert_eq!(WorkspaceValues::sanitize_slug("---"), "");

        let valid = WorkspaceValues::new("Acme", "acme").with_context("Three core teams.");
        assert!(valid.is_valid());

        let empty_name = WorkspaceValues::new("   ", "acme");
        assert!(!empty_name.is_valid());

        let over_limit = WorkspaceValues::new("Acme", "acme").with_context("a".repeat(2_001));
        assert!(!over_limit.is_valid());
    }

    #[test]
    fn dirty_save_discard_and_delete_guard_workflow() {
        struct EventSink;

        let app = TestAppContext::single();
        app.update(crate::init);

        let events: Rc<RefCell<Vec<SettingsGeneralEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let recorded = events.clone();

        let (screen, _sink) = app.update(|cx| {
            let screen = cx.new(|cx| SettingsGeneralScreen::new("test-sg", cx));
            let sink = cx.new(|sub_cx| {
                sub_cx
                    .subscribe(
                        &screen,
                        move |_: &mut EventSink, _, ev: &SettingsGeneralEvent, _| {
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
                assert_eq!(s.workspace().name.as_ref(), "Acme");
                assert!(!s.is_dirty());

                // Icon upload saves immediately
                s.upload_icon("indigo", s_cx);
                assert_eq!(s.workspace().icon.as_deref(), Some("indigo"));

                // Draft edits mark dirty; discard reverts
                s.set_draft_fields("Northwind", "northwind-hq", "Enterprise ops.", s_cx);
                assert!(s.is_dirty());
                s.discard_changes(None, s_cx);
                assert!(!s.is_dirty());
                assert_eq!(s.draft().name.as_ref(), "Acme");

                // Commit normalizes slug and emits Save
                s.set_draft_fields("Northwind", "Northwind HQ!", "Enterprise ops.", s_cx);
                assert!(s.commit_save(s_cx));
                assert!(!s.is_dirty());
                assert_eq!(s.workspace().slug.as_ref(), "northwind-hq");

                // Delete confirmation stays blocked until exact workspace name matches
                s.open_delete_confirm(s_cx);
                assert!(s.is_delete_open());
                s.set_delete_confirmation("Acme", s_cx);
                assert!(!s.can_confirm_delete());
                assert!(!s.confirm_delete(s_cx));

                s.set_delete_confirmation("Northwind", s_cx);
                assert!(s.can_confirm_delete());
                assert!(s.confirm_delete(s_cx));
                assert!(!s.is_delete_open());
            });
        });

        let emitted = events.borrow();
        assert_eq!(emitted.len(), 3);
        assert!(matches!(emitted[2], SettingsGeneralEvent::Delete));
    }
}
