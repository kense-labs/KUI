//! Four sections demonstrating the three label modes of LiquidTabBar.
use gpui::{prelude::*, *};
use kui::{
    components::liquid_tab_bar::{
        LiquidTabBar, LiquidTabChanged, LiquidTabIcon, LiquidTabItem, LiquidTabLabels,
        liquid_panel_id,
    },
    foundation::{Theme, ThemeMode},
    primitives::{
        control::UiLanguage,
        family::{Paint, PhaseClock, Tone, ease, face},
        presentation::{column, row},
    },
};

pub const IDS: &[&str] = &["liquid-tab-bar"];
const BAR_ID: &str = "sections-example";

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}

struct Example {
    bar: Entity<LiquidTabBar>,
    value: SharedString,
    previous: Option<SharedString>,
    labels: LiquidTabLabels,
    panel_clock: PhaseClock,
    mode_focus: [FocusHandle; 3],
    _selected: Subscription,
}

impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let bar = cx.new(|cx| {
            LiquidTabBar::new(
                vec![
                    LiquidTabItem::new("home", lang.text("Home", "首页"), LiquidTabIcon::Home),
                    LiquidTabItem::new(
                        "saved",
                        lang.text("Saved", "收藏"),
                        LiquidTabIcon::Bookmark,
                    ),
                    LiquidTabItem::new("inbox", lang.text("Inbox", "收件箱"), LiquidTabIcon::Inbox)
                        .with_badge(2)
                        .with_badge_label(lang.text("2 unread", "2 条未读")),
                    LiquidTabItem::new(
                        "profile",
                        lang.text("Profile", "我的"),
                        LiquidTabIcon::Profile,
                    ),
                ],
                "home",
                cx,
            )
            .with_id(BAR_ID)
            .with_label(lang.text("Sections", "栏目"))
        });
        let selected = cx.subscribe(&bar, |s, bar, e: &LiquidTabChanged, cx| {
            s.previous = Some(std::mem::replace(&mut s.value, e.0.clone()));
            s.panel_clock.restart(cx);
            bar.update(cx, |bar, cx| bar.set_value(e.0.clone(), cx));
            cx.notify();
        });
        Self {
            bar,
            value: "home".into(),
            previous: None,
            labels: LiquidTabLabels::Active,
            panel_clock: PhaseClock::default(),
            mode_focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            _selected: selected,
        }
    }

    fn set_mode(&mut self, mode: LiquidTabLabels, cx: &mut Context<Self>) {
        if mode != self.labels {
            self.labels = mode;
            self.bar.update(cx, |bar, cx| bar.set_labels(mode, cx));
            cx.notify();
        }
    }
}

impl Render for Example {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let lang = UiLanguage::current(cx);
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let narrow = w.viewport_size().width < px(420.);
        if narrow && self.labels == LiquidTabLabels::Always {
            self.set_mode(LiquidTabLabels::Active, cx);
        }
        let p = Paint::get(cx);
        let time = self.panel_clock.time(0.26, w, cx);
        let leaving = self.previous.as_ref().filter(|_| time < 0.10);
        let shown = leaving.unwrap_or(&self.value);
        let (section, note) = match shown.as_ref() {
            "saved" => (
                lang.text("Saved", "收藏"),
                lang.text("3 saved places", "3 个收藏地点"),
            ),
            "inbox" => (
                lang.text("Inbox", "收件箱"),
                lang.text("2 unread messages", "2 条未读消息"),
            ),
            "profile" => (
                lang.text("Profile", "我的"),
                lang.text("Your profile", "你的个人资料"),
            ),
            _ => (
                lang.text("Home", "首页"),
                lang.text("Your feed", "你的动态"),
            ),
        };
        let alpha = if leaving.is_some() {
            1. - ease(time / 0.10)
        } else {
            ease((time - 0.10) / 0.16)
        };
        let y = 4. * (1. - alpha) * if leaving.is_some() { -1. } else { 1. };
        let panel_id = liquid_panel_id(BAR_ID, shown);
        let panel = row()
            .id(panel_id.clone())
            .accessibility_id(panel_id)
            .role(Role::TabPanel)
            .aria_label(section)
            .h(px(20.))
            .justify_center()
            .text_size(px(14.))
            .text_color(p.secondary)
            .child(div().relative().top(px(y)).opacity(alpha).child(note));
        let mut modes = row()
            .id("liquid-label-modes")
            .role(Role::Group)
            .aria_label(lang.text("Tab labels", "标签显示"))
            .max_w_full()
            .h(px(44.))
            .p(px(3.))
            .gap(px(2.))
            .rounded_full()
            .border_1()
            .border_color(p.line)
            .bg(p.stage);
        for (i, (mode, label)) in [
            (
                LiquidTabLabels::Active,
                lang.text("Active label", "当前标签"),
            ),
            (LiquidTabLabels::Always, lang.text("All labels", "全部标签")),
            (LiquidTabLabels::None, lang.text("Icons only", "仅图标")),
        ]
        .into_iter()
        .enumerate()
        {
            if narrow && mode == LiquidTabLabels::Always {
                continue;
            }
            let selected = self.labels == mode;
            modes = modes.child(
                face(("liquid-label-mode", i), label, 36., 18., Tone::Clear, cx)
                    .px(px(12.))
                    .track_focus(&self.mode_focus[i])
                    .aria_toggled(selected.into())
                    .font_weight(FontWeight::NORMAL)
                    .text_color(if selected { p.ink } else { p.secondary })
                    .when(selected, |b| {
                        b.bg(if dark { rgb(0x181818) } else { rgb(0xffffff) })
                    })
                    .child(label)
                    .on_click(cx.listener(move |s, _, w, cx| {
                        s.set_mode(mode, cx);
                        s.mode_focus[i].focus(w, cx);
                    }))
                    .on_key_down(cx.listener(move |s, e: &KeyDownEvent, _, cx| {
                        if !e.is_held && matches!(e.keystroke.key.as_str(), "enter" | "space") {
                            s.set_mode(mode, cx);
                            cx.stop_propagation();
                        }
                    })),
            );
        }
        column()
            .w_full()
            .max_w(px(522.))
            .px(px(6.))
            .items_center()
            .gap(px(16.))
            .child(
                column()
                    .w_full()
                    .h(px(200.))
                    .pb(px(2.5))
                    .justify_center()
                    .items_center()
                    .gap(px(16.))
                    .child(self.bar.clone())
                    .child(panel),
            )
            .child(modes)
    }
}
