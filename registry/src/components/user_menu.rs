//! An account menu with identity, destinations, appearance and asynchronous sign-out.
use crate::{
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{Paint, Slide, Tone, face},
        glyph::{Glyph, icon, spinner},
        menu::{MenuAlign, MenuControl, MenuCore, MenuItem, render_menu},
    },
};
use gpui::{prelude::*, *};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Appearance {
    Light,
    Dark,
    #[default]
    System,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    Available,
    Busy,
    Away,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountEvent {
    Navigate(SharedString),
    Appearance(Appearance),
    Presence(Presence),
    SignOut,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub name: SharedString,
    pub email: SharedString,
    pub plan: Option<SharedString>,
    pub avatar: Option<SharedString>,
}
impl Account {
    pub fn new(name: impl Into<SharedString>, email: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
            plan: None,
            avatar: None,
        }
    }
}
pub struct UserMenu {
    core: MenuCore,
    account: Account,
    avatar_image: Option<ImageSource>,
    links: usize,
    theme: Appearance,
    status: Option<Presence>,
    theme_slide: Slide,
    status_slide: Slide,
    busy: bool,
    panel_width: f32,
    show_name: bool,
    compact: Option<bool>,
    spin: crate::primitives::family::PhaseClock,
}
impl EventEmitter<AccountEvent> for UserMenu {}
impl UserMenu {
    pub fn new(
        id: impl Into<ElementId>,
        account: Account,
        mut items: Vec<MenuItem>,
        cx: &App,
    ) -> Self {
        let l = UiLanguage::current(cx);
        let links = items.len();
        items.push(
            MenuItem::new("appearance", l.text("Theme", "主题"))
                .with_icon(Glyph::Sun)
                .with_separator(),
        );
        items.push(
            MenuItem::new("sign-out", l.text("Sign out", "退出登录"))
                .with_icon(Glyph::LogOut)
                .with_separator(),
        );
        Self {
            core: MenuCore::new(id.into(), account.name.clone(), items, cx),
            account,
            avatar_image: None,
            links,
            theme: Appearance::System,
            status: None,
            theme_slide: Slide::snappy(2.),
            status_slide: Slide::snappy(0.),
            busy: false,
            panel_width: 280.,
            show_name: false,
            compact: None,
            spin: Default::default(),
        }
    }
    /// Display a bundled or host-provided image, retaining initials while it loads.
    pub fn with_avatar(mut self, source: impl Into<ImageSource>) -> Self {
        self.avatar_image = Some(source.into());
        self
    }
    pub fn with_appearance(mut self, preference: Appearance) -> Self {
        self.theme = preference;
        self.theme_slide.reset(preference as u8 as f32);
        self
    }
    /// Current preference, including System independently from its resolved color.
    pub fn appearance(&self) -> Appearance {
        self.theme
    }
    /// Mirror a host preference without emitting a second user selection.
    pub fn set_appearance(&mut self, preference: Appearance, cx: &mut Context<Self>) {
        if self.theme != preference {
            self.theme = preference;
            self.theme_slide.set(preference as u8 as f32, cx);
            cx.notify()
        }
    }
    pub fn with_sign_out_shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        if let Some(item) = self.core.items.last_mut() {
            item.shortcut = shortcut.into()
        }
        self
    }
    pub fn with_align(mut self, align: MenuAlign) -> Self {
        self.core.align = align;
        self
    }
    pub fn with_panel_width(mut self, px: f32) -> Self {
        self.panel_width = px.max(192.);
        self
    }
    pub fn show_name(mut self) -> Self {
        self.show_name = true;
        self
    }
    pub fn initially_open(mut self) -> Self {
        self.core.open = true;
        self.core.appear.reset(1.);
        self
    }
    /// Use the host viewport breakpoint when rendering in an embedded canvas.
    pub fn set_compact(&mut self, compact: Option<bool>) {
        self.compact = compact
    }
    pub fn with_presence(mut self, status: Presence, cx: &App) -> Self {
        self.status = Some(status);
        self.status_slide.reset(status as u8 as f32);
        self.core.items.insert(
            self.links,
            MenuItem::new("presence", UiLanguage::current(cx).text("Status", "状态"))
                .with_icon(Glyph::User),
        );
        self
    }
    /// Resolve the sign-out request; failure keeps the same menu available for retry.
    pub fn finish_sign_out(&mut self, ok: bool, w: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            self.busy = false;
            if ok {
                self.core.close(w, cx)
            }
            cx.notify()
        }
    }
    fn choose(&mut self, i: usize, k: usize, cx: &mut Context<Self>) {
        if k > 2 {
            return;
        }
        if self.has_status(i) {
            let s = [Presence::Available, Presence::Busy, Presence::Away][k];
            if self.status != Some(s) {
                self.status = Some(s);
                self.status_slide.set(k as f32, cx);
                cx.emit(AccountEvent::Presence(s))
            }
        } else {
            let a = [Appearance::Light, Appearance::Dark, Appearance::System][k];
            if self.theme != a {
                self.theme = a;
                self.theme_slide.set(k as f32, cx);
                cx.emit(AccountEvent::Appearance(a))
            }
        }
        cx.notify()
    }
    fn has_status(&self, i: usize) -> bool {
        self.status.is_some() && i == self.links
    }
    fn avatar(&self, size: f32, p: Paint) -> AnyElement {
        let initials = self
            .account
            .name
            .split_whitespace()
            .filter_map(|s| s.chars().next())
            .take(2)
            .collect::<String>();
        let mut a = div()
            .relative()
            .size(px(size))
            .flex_none()
            .rounded_full()
            .bg(p.ink)
            .text_color(p.panel)
            .text_size(px(size * 0.36))
            .font_weight(FontWeight::MEDIUM)
            .flex()
            .items_center()
            .justify_center();
        a = a.child(initials);
        if let Some(src) = self
            .avatar_image
            .clone()
            .or_else(|| self.account.avatar.clone().map(Into::into))
        {
            a = a.child(
                img(src)
                    .absolute()
                    .inset_0()
                    .size_full()
                    .rounded_full()
                    .object_fit(ObjectFit::Cover),
            );
        }
        if let Some(status) = self.status {
            a = a.child(
                div()
                    .absolute()
                    .right_0()
                    .bottom_0()
                    .size(px(10.))
                    .rounded_full()
                    .border_1()
                    .border_color(p.panel)
                    .bg(match status {
                        Presence::Available => p.success,
                        Presence::Busy => p.danger,
                        Presence::Away => p.faint,
                    }),
            )
        }
        a.into_any_element()
    }
}
impl MenuControl for UserMenu {
    fn core(&mut self) -> &mut MenuCore {
        &mut self.core
    }
    fn header(&self, cx: &App) -> Option<AnyElement> {
        let p = Paint::get(cx);
        Some(
            div()
                .h(px(62.))
                .flex_none()
                .flex()
                .items_center()
                .gap(px(10.))
                .px(px(9.))
                .border_b_1()
                .border_color(p.line)
                .child(self.avatar(38., p))
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
                                .gap(px(6.))
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(14.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(self.account.name.clone()),
                                )
                                .when_some(self.account.plan.clone(), |e, s| {
                                    e.child(
                                        div()
                                            .rounded(px(4.))
                                            .bg(p.hover)
                                            .px(px(5.))
                                            .text_size(px(10.))
                                            .child(s),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_size(px(12.))
                                .text_color(p.secondary)
                                .child(self.account.email.clone()),
                        ),
                )
                .into_any_element(),
        )
    }
    fn selected(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        if i < self.links {
            self.core.close(w, cx);
            cx.emit(AccountEvent::Navigate(self.core.items[i].id.clone()))
        } else if i + 1 == self.core.items.len() {
            if !self.busy {
                self.busy = true;
                self.spin.restart(cx);
                cx.emit(AccountEvent::SignOut)
            }
        } else {
            self.horizontal(i, true, cx)
        }
        cx.notify()
    }
    fn horizontal(&mut self, i: usize, next: bool, cx: &mut Context<Self>) {
        if i < self.links || i + 1 == self.core.items.len() {
            return;
        }
        let k = if self.has_status(i) {
            self.status.map_or(0, |s| s as usize)
        } else {
            self.theme as usize
        };
        self.choose(i, (k + if next { 1 } else { 2 }) % 3, cx)
    }
    fn compact(&self, w: &Window) -> bool {
        self.compact.unwrap_or(w.viewport_size().width < px(640.))
    }
    fn accessory(&self, i: usize, cx: &Context<Self>, w: &Window) -> Option<AnyElement> {
        if i + 1 == self.core.items.len() && self.busy {
            if !Theme::global(cx).reduced_motion {
                w.request_animation_frame();
            }
            return Some(
                spinner(self.spin.age(cx) % 1., 14., Paint::get(cx).secondary).into_any_element(),
            );
        }
        if i < self.links || i + 1 == self.core.items.len() {
            return None;
        }
        let status = self.has_status(i);
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
        let compact = self.compact(w);
        let cell = if compact { 37. } else { 24. };
        let cell_h = if compact { 33. } else { 22. };
        let pos = if status {
            self.status_slide
        } else {
            self.theme_slide
        }
        .get(w, cx);
        let mut tray = div()
            .relative()
            .flex()
            .items_center()
            .p(px(2.))
            .rounded(px(4.))
            .bg(p.hover)
            .child(
                div()
                    .absolute()
                    .left(px(2. + pos * cell))
                    .top(px(2.))
                    .w(px(cell))
                    .h(px(cell_h))
                    .rounded(px(4.))
                    .bg(p.panel)
                    .shadow_sm(),
            );
        for k in 0..3 {
            let (en, zh, g) = if status {
                [
                    ("Available", "在线", Glyph::Check),
                    ("Busy", "忙碌", Glyph::Minus),
                    ("Away", "离开", Glyph::Clock),
                ][k]
            } else {
                [
                    ("Light", "浅色", Glyph::Sun),
                    ("Dark", "深色", Glyph::Moon),
                    ("System", "跟随系统", Glyph::Monitor),
                ][k]
            };
            let label = lang.text(en, zh);
            tray = tray.child(
                div()
                    .id(("choice", k))
                    .debug_selector(move || label.into())
                    .role(Role::MenuItemRadio)
                    .aria_label(label)
                    .aria_toggled(
                        (k == if status {
                            self.status.map_or(0, |s| s as usize)
                        } else {
                            self.theme as usize
                        })
                        .into(),
                    )
                    .w(px(cell))
                    .h(px(cell_h))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon(g, 14., p.ink))
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.choose(i, k, cx);
                        cx.stop_propagation()
                    })),
            );
        }
        Some(tray.into_any_element())
    }
}
impl Render for UserMenu {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let name = self.show_name && !self.compact(w);
        let label = format!(
            "{}, {}",
            UiLanguage::current(cx).text("Account menu", "账户菜单"),
            self.account.name
        );
        let btn = face(
            "account",
            label,
            40.,
            Theme::global(cx).tokens.radius.md.into(),
            Tone::Clear,
            cx,
        )
        .gap(px(7.))
        .child(self.avatar(36., p))
        .when(name, |e| {
            e.child(self.account.name.clone()).child(icon(
                if self.core.open {
                    Glyph::Up
                } else {
                    Glyph::Down
                },
                13.,
                p.secondary,
            ))
        });
        render_menu(self, btn, false, self.panel_width, w, cx)
    }
}
