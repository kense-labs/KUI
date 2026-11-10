//! The share sheet example uses fictitious recipients and a simulated delivery service.
use gpui::{prelude::*, *};
use kui::{
    components::share_sheet::{
        ShareChannel, SharePerson, ShareSheet, ShareSheetAccess, ShareSheetAlign,
    },
    foundation::surface_radius,
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, face},
        glyph::Glyph,
    },
};
use std::{cell::Cell, rc::Rc, time::Duration};

pub const IDS: &[&str] = &["share-sheet"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}

struct Example {
    sheet: Entity<ShareSheet>,
    failure: Rc<Cell<bool>>,
}

impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let t = |en, zh| lang.text(en, zh);
        let people = [
            ("riley", "Riley Morgan", "莱利", [0x9167e7, 0x54cdbc]),
            ("avery", "Avery Tran", "艾弗里", [0x50dfaf, 0xee68a2]),
            ("morgan", "Morgan Lee", "摩根", [0x86edac, 0xd481df]),
            ("noah", "Noah Kim", "诺亚", [0x89d3ce, 0x6bdce5]),
            ("jules", "Jules Chen", "朱尔斯", [0xa1df49, 0x8266d4]),
        ]
        .into_iter()
        .map(|(id, en, zh, accent)| SharePerson::new(id, t(en, zh)).with_accent(accent))
        .collect();
        let access = vec![
            ShareSheetAccess::new("invited", t("Only people invited", "仅受邀成员"))
                .with_description(t("Nobody else can open it", "其他人无法打开"))
                .with_icon(Glyph::Lock),
            ShareSheetAccess::new("team", t("Anyone at Northbank", "北岸团队成员"))
                .with_description(t("Can view and comment", "可查看和评论"))
                .with_icon(Glyph::User),
            ShareSheetAccess::new("link", t("Anyone with the link", "知道链接的任何人"))
                .with_description(t("Can view", "可查看"))
                .with_icon(Glyph::Globe),
        ];
        let channels = vec![
            ShareChannel::new("email", t("Email", "邮件"))
                .with_icon(Glyph::Comment)
                .with_done_label(t("Sent", "已发送")),
            ShareChannel::new("slack", "Slack")
                .with_icon(Glyph::Comment)
                .with_done_label(t("Posted", "已发布")),
            ShareChannel::new("x", "X")
                .with_icon(Glyph::Close)
                .with_done_label(t("Posted", "已发布")),
        ];
        let failure = Rc::new(Cell::new(false));
        let send_failure = failure.clone();
        let sheet = cx.new(|cx| {
            ShareSheet::new(
                t("Release plan", "发布计划"),
                "https://example.com/docs/release-plan",
                people,
                access,
                cx,
            )
            .with_align(ShareSheetAlign::Center)
            .with_default_access("team")
            .with_channels(channels)
            .on_send(move |_, _, cx| {
                let fail = send_failure.get();
                let delay = cx.background_executor().timer(Duration::from_millis(780));
                cx.spawn(async move |_| {
                    delay.await;
                    if fail { Err("".into()) } else { Ok(()) }
                })
            })
            .on_channel(|_, _, cx| {
                let delay = cx.background_executor().timer(Duration::from_millis(680));
                cx.spawn(async move |_| {
                    delay.await;
                    Ok(())
                })
            })
        });
        Self { sheet, failure }
    }
}

impl Render for Example {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
        let radius = f32::from(surface_radius(cx));
        let mut choices = div().flex().items_center().justify_center().gap(px(4.));
        for (failed, label) in [
            (false, lang.text("Send works", "发送成功")),
            (true, lang.text("Send fails", "发送失败")),
        ] {
            let active = failed == self.failure.get();
            choices = choices.child(
                face(
                    ("share-demo-mode", usize::from(failed)),
                    label,
                    32.,
                    radius,
                    Tone::Clear,
                    cx,
                )
                .px(px(10.))
                .aria_toggled(active.into())
                .when(active, |e| e.bg(p.hover))
                .hover(|s| s.bg(p.hover))
                .child(label)
                .on_click(cx.listener(move |s, _, _, cx| {
                    s.failure.set(failed);
                    cx.notify();
                })),
            );
        }
        let height = if w.viewport_size().width <= px(520.) {
            420.
        } else {
            600.
        };
        div()
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .child(
                div()
                    .w_full()
                    .h(px(height))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(self.sheet.clone()),
            )
            .child(div().h(px(60.)).flex().items_center().child(choices))
    }
}
