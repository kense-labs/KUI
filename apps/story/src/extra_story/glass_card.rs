//! Localized profile example for the glass disclosure.
use gpui::{prelude::*, *};
use kui::{
    components::glass_card::GlassCard,
    foundation::{Theme, ThemeMode, surface_radius},
    primitives::control::UiLanguage,
};
use std::sync::Arc;

pub const IDS: &[&str] = &["glass-card"];

// The demo uses the rasterized profile.svg to avoid blocking the first WebKit paint.
const PROFILE_ART: &[u8] = include_bytes!("../../assets/glass/profile.png");

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id)
        .then(|| cx.new(ProfileExample::new).into())
}

struct ProfileExample(Entity<GlassCard>);

impl ProfileExample {
    fn new(cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let detail = cx.new(|cx| ProfileDetails {
            lang,
            following: false,
            booked: false,
            follow_focus: cx.focus_handle().tab_stop(true),
            book_focus: cx.focus_handle().tab_stop(true),
        });
        let image = Arc::new(Image::from_bytes(ImageFormat::Png, PROFILE_ART.to_vec()));
        let card = cx.new(|cx| {
            GlassCard::new(image, "", "Leon Zhu", cx)
                .with_id("profile-card")
                .with_subtitle(Box::new(move |_, _| {
                    div()
                        .child(lang.text("Design lead, Lisbon", "设计负责人 · 里斯本"))
                        .into_any_element()
                }))
                .with_aside(Box::new(|_, cx| {
                    let dark = Theme::global(cx).mode == ThemeMode::Dark;
                    let star = rgb(if dark { 0xb5b5b5 } else { 0x585858 });
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(div().text_color(star).child("★"))
                        .child("4.9")
                        .into_any_element()
                }))
                .with_body(Box::new(move |_, _| detail.clone().into_any_element()))
        });
        Self(card)
    }
}

impl Render for ProfileExample {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .px_1()
            .flex()
            .justify_center()
            .child(self.0.clone())
    }
}

struct ProfileDetails {
    lang: UiLanguage,
    following: bool,
    booked: bool,
    follow_focus: FocusHandle,
    book_focus: FocusHandle,
}

impl Render for ProfileDetails {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let lang = self.lang;
        let dark = Theme::global(cx).mode == ThemeMode::Dark;
        let ink: Hsla = rgb(if dark { 0xf2f2f2 } else { 0x0b0b0b }).into();
        let secondary: Hsla = rgb(if dark { 0xb5b5b5 } else { 0x585858 }).into();
        let faint = rgb(if dark { 0x8c8c8c } else { 0x7d7d7d });
        let primary_bg = rgb(if dark { 0xbebebe } else { 0x373737 });
        let primary_text = rgb(if dark { 0x141414 } else { 0xffffff });
        let t = |en, zh| lang.text(en, zh);
        let action = |id, label: &'static str, primary, focus: &FocusHandle| {
            div()
                .id(id)
                .track_focus(focus)
                .role(Role::Button)
                .h(px(44.))
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .justify_center()
                .px(px(8.))
                .rounded(surface_radius(cx))
                .cursor_pointer()
                .border_1()
                .border_color(transparent_black())
                .focus_visible(move |s| s.border_color(ink))
                .text_size(px(14.))
                .line_height(px(20.))
                .when_else(
                    primary,
                    |d| {
                        d.bg(primary_bg)
                            .text_color(primary_text)
                            .hover(move |s| s.bg(primary_bg.blend(white().opacity(0.12).into())))
                    },
                    |d| {
                        d.bg(ink.opacity(0.08))
                            .hover(move |s| s.bg(ink.opacity(0.15)))
                    },
                )
                .child(label)
        };
        let follow = action(
            "glass-follow",
            if self.following {
                t("Following", "已关注")
            } else {
                t("Follow", "关注")
            },
            true,
            &self.follow_focus,
        )
        .aria_toggled(self.following.into());
        let book = action(
            "glass-book",
            if self.booked {
                t("Requested", "已申请")
            } else {
                t("Book a session", "预约会谈")
            },
            false,
            &self.book_focus,
        )
        .aria_toggled(self.booked.into());
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(14.))
            .child(
                div()
                    // Compensate for the preview font's wider Latin advances.
                    .text_size(px(if lang == UiLanguage::English { 13.5 } else { 14. }))
                    .line_height(px(19.6))
                    .text_color(secondary)
                    .child(t(
                        "Leads design systems at Meridian. Mentors on motion, type, and shipping small teams fast.",
                        "负责 Meridian 的设计系统。指导小团队打磨动效、字体和发布流程。",
                    )),
            )
            .child(
                div()
                    .flex()
                    .gap(px(8.))
                    .children(
                        [
                            (t("Sessions", "场次"), "128"),
                            (t("Followers", "关注者"), "12.4k"),
                            (t("Replies in", "回复用时"), t("2 h", "2 小时")),
                        ]
                        .map(|(label, value)| {
                            div()
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .line_height(px(17.))
                                        .text_color(faint)
                                        .child(label),
                                )
                                .child(div().text_size(px(16.)).line_height(px(22.)).child(value))
                        }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(8.))
                    .child(follow.on_click(cx.listener(|s, _, _, cx| {
                        s.following = !s.following;
                        cx.notify();
                    })))
                    .child(book.on_click(cx.listener(|s, _, _, cx| {
                        s.booked = !s.booked;
                        cx.notify();
                    }))),
            )
    }
}
