//! Shared date reel with a simulated schedule operation.
use gpui::{prelude::*, *};
use kui::{
    components::date_reel::{DateReel, DateReelLocale},
    primitives::control::UiLanguage,
};
use std::time::Duration;

pub const IDS: &[&str] = &["date-reel"];

pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Demo::new).into())
}

struct Demo(Entity<DateReel>);

impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        crate::ensure_story_fonts(cx);
        let lang = UiLanguage::current(cx);
        let locale = match lang {
            UiLanguage::English => DateReelLocale::English,
            UiLanguage::Chinese => DateReelLocale::Chinese,
        };
        Self(cx.new(|cx| {
            let reel = DateReel::new(cx);
            let today = reel.value().date();
            let start = today
                .succ_opt()
                .and_then(|day| day.and_hms_opt(9, 0, 0))
                .unwrap_or_else(|| reel.value());
            reel.with_today(today)
                .with_default_value(start)
                .with_title(lang.text("Send later", "稍后发送"))
                .with_locale(locale)
                .with_visibility(|cx| {
                    cx.try_global::<crate::StoryBridge>()
                        .is_none_or(|bridge| (bridge.visible)())
                })
                .with_confirm(|_, cx| {
                    let timer = cx.background_executor().timer(Duration::from_millis(740));
                    cx.spawn(async move |_| {
                        timer.await;
                        Ok(())
                    })
                })
        }))
    }
}

impl Render for Demo {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .max_w(px(544.))
            .px(px(12.))
            .py(px(30.))
            .flex()
            .justify_center()
            .child(self.0.clone())
    }
}
