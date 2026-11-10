use gpui::{prelude::*, *};
use kui::{
    components::charge_session::{ChargeSession, ChargeSessionOptions},
    primitives::control::UiLanguage,
};

pub const IDS: &[&str] = &["charge-session"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| {
        crate::ensure_story_fonts(cx);
        cx.new(|cx| {
            let lang = UiLanguage::current(cx);
            Demo(cx.new(|cx| {
                ChargeSession::new(
                    ChargeSessionOptions {
                        title: lang.text("Fast charger 4", "4 号快充桩").into(),
                        location: lang
                            .text("Harbor Street hub, 150 kW", "港湾街充电站 · 150 kW")
                            .into(),
                        ..Default::default()
                    },
                    cx,
                )
                .with_style(|view| view.max_w(px(520.)))
                .with_visibility(|cx| {
                    cx.try_global::<crate::StoryBridge>()
                        .is_none_or(|bridge| (bridge.visible)())
                })
            }))
        })
        .into()
    })
}

struct Demo(Entity<ChargeSession>);

impl Render for Demo {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .min_h(px(890.))
            .flex()
            .items_center()
            .justify_center()
            .px(px(12.))
            .py(px(24.))
            .child(self.0.clone())
    }
}
