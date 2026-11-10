//! A local invitation program with a replayable sample funnel.
use gpui::{prelude::*, *};
use kui::{
    components::referral_card::{ReferralCard, ReferralCardOptions, ReferralMilestone},
    foundation::surface_radius,
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, face},
        glyph::{Glyph, icon},
    },
};

pub const IDS: &[&str] = &["referral-card"];

pub fn story(id: &str, w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id)
        .then(|| cx.new(|cx| Example::new(w, cx)).into())
}

struct Example(Entity<ReferralCard>);

impl Example {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        Self(cx.new(|cx| {
            ReferralCard::new(
                ReferralCardOptions {
                    link: "https://example.com/r/kense20".into(),
                    milestone: ReferralMilestone::new(
                        5,
                        lang.text("3 months free", "三个月免费使用"),
                    )
                    .with_detail(lang.text("Applied to your next invoice", "在下一张账单抵扣")),
                    ..Default::default()
                },
                w,
                cx,
            )
            .with_visibility(|cx| {
                cx.try_global::<crate::StoryBridge>()
                    .is_none_or(|bridge| (bridge.visible)())
            })
        }))
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
        let label = lang.text("Replay", "重播");
        div()
            .w_full()
            .px(px(8.))
            .py(px(22.))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(16.))
            .child(self.0.clone())
            .child(
                face(
                    "referral-replay",
                    label,
                    32.,
                    f32::from(surface_radius(cx)),
                    Tone::Surface,
                    cx,
                )
                .px(px(10.))
                .border_1()
                .border_color(p.line)
                .hover(|e| e.bg(p.hover))
                .child(icon(Glyph::Redo, 14., p.secondary))
                .child(label)
                .on_click(cx.listener(|s, _, w, cx| s.0.update(cx, |card, cx| card.replay(w, cx)))),
            )
    }
}
