//! Local sample reveal; no image is generated or sent.
use gpui::{prelude::*, *};
use kui::{
    components::image_reveal::{ImageReveal, ImageRevealEvent},
    primitives::{control::UiLanguage, family::Paint},
};
use std::{sync::Arc, time::Duration};
pub const IDS: &[&str] = &["image-reveal"];
pub fn story(id: &str, _: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id).then(|| cx.new(Example::new).into())
}
struct Example {
    view: Entity<ImageReveal>,
    language: UiLanguage,
    run: u64,
    _sub: Subscription,
}
fn image() -> ImageSource {
    Arc::new(Image::from_bytes(
        ImageFormat::Jpeg,
        include_bytes!("../../assets/navigation/room-0.jpg").to_vec(),
    ))
    .into()
}
fn schedule(
    view: &Entity<ImageReveal>,
    run: u64,
    fail: bool,
    language: UiLanguage,
    cx: &mut Context<Example>,
) {
    view.update(cx, |v, cx| {
        v.begin(run, cx);
        v.set_preview(run, image(), cx);
    });
    let weak = view.downgrade();
    cx.spawn(async move |_, cx| {
        for i in 1..=8 {
            cx.background_executor()
                .timer(Duration::from_millis(260))
                .await;
            if weak
                .update(cx, |v, cx| v.set_progress(run, i as f32 / 8., cx))
                .is_err()
            {
                return;
            }
        }
        let _ = weak.update(cx, |v, cx| {
            if fail {
                v.fail(
                    run,
                    language.text("Sample failed — retry", "样例失败，请重试"),
                    cx,
                );
            } else {
                v.finish(run, image(), cx);
            }
        });
    })
    .detach();
}
impl Example {
    fn new(cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let view = cx.new(|_| ImageReveal::new(language.text("A quiet room", "安静的房间"), 1.5));
        let sub = cx.subscribe(&view, |s: &mut Self, view, _: &ImageRevealEvent, cx| {
            s.run += 1;
            schedule(&view, s.run, false, s.language, cx);
        });
        schedule(&view, 1, true, language, cx);
        Self {
            view,
            language,
            run: 1,
            _sub: sub,
        }
    }
}
impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        div()
            .size_full()
            .min_h(px(540.))
            .p_4()
            .bg(p.stage)
            .text_color(p.ink)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(p.secondary)
                    .child(self.language.text(
                        "Local simulated reveal — nothing is generated",
                        "本地图像演示，不会生成图片",
                    )),
            )
            .child(div().w_full().max_w(px(560.)).child(self.view.clone()))
    }
}
