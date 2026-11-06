//! Simulated resolution and delivery; no request leaves this example.
use gpui::{prelude::*, *};
use kui::{
    components::link_unfurl::{LinkPreview, LinkUnfurl, LinkUnfurlEvent},
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, face},
    },
};
use std::{cell::Cell, rc::Rc, time::Duration};
pub const IDS: &[&str] = &["link-unfurl"];
pub fn story(id: &str, w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id)
        .then(|| cx.new(|cx| Demo::new(w, cx)).into())
}
struct Demo {
    composer: Entity<LinkUnfurl>,
}
impl Demo {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let lang = UiLanguage::current(cx);
        let composer =
            cx.new(|cx| LinkUnfurl::new(lang.text("Message", "消息"), w, cx).with_simulated(true));
        let fail_preview = Rc::new(Cell::new(true));
        let fail_send = Rc::new(Cell::new(true));
        cx.subscribe_in(
            &composer,
            w,
            move |_, composer, event: &LinkUnfurlEvent, w, cx| match event {
                LinkUnfurlEvent::Resolve { id, .. } => {
                    let id = *id;
                    let fail = fail_preview.replace(false);
                    let composer = composer.clone();
                    cx.spawn_in(w, async move |_, cx| {
                        cx.background_executor()
                            .timer(Duration::from_millis(550))
                            .await;
                        let _ = cx.update(|_, cx| {
                            composer.update(cx, |composer, cx| {
                                if fail {
                                    composer.resolve_preview(
                                        id,
                                        Err(lang
                                            .text(
                                                "Simulated preview failed; retry.",
                                                "模拟预览失败，请重试。",
                                            )
                                            .into()),
                                        cx,
                                    );
                                } else {
                                    composer.resolve_preview(
                                        id,
                                        Ok(Some(LinkPreview::new(
                                            lang.text("Sample site", "样例网站"),
                                            lang.text("A useful resource", "一份实用资料"),
                                            lang.text(
                                                "An illustrative preview supplied by the host.",
                                                "由宿主提供的模拟链接预览。",
                                            ),
                                        ))),
                                        cx,
                                    );
                                }
                            });
                        });
                    })
                    .detach();
                }
                LinkUnfurlEvent::Send { id, .. } => {
                    let id = *id;
                    let fail = fail_send.replace(false);
                    let composer = composer.clone();
                    cx.spawn_in(w, async move |_, cx| {
                        cx.background_executor()
                            .timer(Duration::from_millis(450))
                            .await;
                        let _ = cx.update(|w, cx| {
                            composer.update(cx, |composer, cx| {
                                if fail {
                                    composer.resolve_send(
                                        id,
                                        Err(lang
                                            .text(
                                                "Simulated send failed; draft kept.",
                                                "模拟发送失败，草稿已保留。",
                                            )
                                            .into()),
                                        w,
                                        cx,
                                    );
                                } else {
                                    composer.resolve_send(id, Ok(()), w, cx);
                                }
                            });
                        });
                    })
                    .detach();
                }
                _ => {}
            },
        )
        .detach();
        let visible = cx
            .try_global::<crate::StoryBridge>()
            .is_none_or(|bridge| (bridge.visible)());
        composer.update(cx, |s, cx| s.set_visible(visible, cx));
        let weak = composer.downgrade();
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(250))
                    .await;
                if weak
                    .update(cx, |s, cx| {
                        let visible = cx
                            .try_global::<crate::StoryBridge>()
                            .is_none_or(|bridge| (bridge.visible)());
                        s.set_visible(visible, cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        Self { composer }
    }
}
impl Render for Demo {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let lang = UiLanguage::current(cx);
        let label = lang.text("Insert sample link", "插入样例链接");
        div()
            .size_full()
            .min_h(px(520.))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .p(px(20.))
            .bg(p.stage)
            .text_color(p.ink)
            .child(self.composer.clone())
            .child(
                face(
                    "link-example",
                    label,
                    36.,
                    kui::foundation::surface_radius(cx).into(),
                    Tone::Surface,
                    cx,
                )
                .px(px(10.))
                .child(label)
                .on_click(cx.listener(move |s, _, w, cx| {
                    s.composer.update(cx, |composer, cx| {
                        composer.set_text(
                            lang.text(
                                "Take a look: https://example.test/guide",
                                "看看这个： https://example.test/guide",
                            ),
                            w,
                            cx,
                        )
                    })
                }))
                .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") && !e.is_held {
                        s.composer.update(cx, |composer, cx| {
                            composer.set_text(
                                lang.text(
                                    "Take a look: https://example.test/guide",
                                    "看看这个： https://example.test/guide",
                                ),
                                w,
                                cx,
                            )
                        });
                        cx.stop_propagation();
                    }
                })),
            )
    }
}
