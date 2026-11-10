//! Simulated resolution and delivery; no request leaves this example.
use gpui::{prelude::*, *};
use kui::{
    components::link_unfurl::{LinkPreview, LinkSample, LinkUnfurl, LinkUnfurlEvent},
    primitives::{control::UiLanguage, family::Paint},
};
use std::{cell::Cell, rc::Rc, sync::Arc, time::Duration};

const AIRBNB: &str = "https://airbnb.com/rooms/482139";
const FIGMA: &str = "https://figma.com/design/kui-workshop";
const SPOTIFY: &str = "https://open.spotify.com/playlist/kui-studio";
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
        let examples = [
            (
                "Airbnb",
                lang.text(
                    "Found a place for the offsite, take a look",
                    "找到了一处团建场地，来看看",
                ),
                AIRBNB,
                include_bytes!("../../assets/link-unfurl/airbnb.svg").as_slice(),
            ),
            (
                "Figma",
                lang.text("Here is the design for review", "这份设计稿可以评审了"),
                FIGMA,
                include_bytes!("../../assets/orbit-logos/figma.svg").as_slice(),
            ),
            (
                "Spotify",
                lang.text("A playlist for the studio", "工作室的歌单"),
                SPOTIFY,
                include_bytes!("../../assets/link-unfurl/spotify.svg").as_slice(),
            ),
        ];
        let samples = examples
            .iter()
            .map(|(label, text, url, bytes)| {
                LinkSample::new(
                    *label,
                    format!("{text}\n{url}"),
                    Arc::new(Image::from_bytes(ImageFormat::Svg, bytes.to_vec())),
                )
            })
            .collect();
        let composer = cx.new(|cx| {
            LinkUnfurl::new(lang.text("Message", "消息"), w, cx)
                .with_samples(samples)
                .with_collapsed_previews()
        });
        let fail_preview = Rc::new(Cell::new(true));
        let fail_send = Rc::new(Cell::new(true));
        cx.subscribe_in(
            &composer,
            w,
            move |_, composer, event: &LinkUnfurlEvent, w, cx| match event {
                LinkUnfurlEvent::Resolve { id, url } => {
                    let id = *id;
                    let fail = url.as_ref() != AIRBNB && fail_preview.replace(false);
                    let (site, title, description) = match url.as_ref() {
                        AIRBNB => (
                            "Airbnb",
                            lang.text("A quiet house for the team", "适合团队聚会的安静小屋"),
                            lang.text(
                                "A bright getaway with room for the whole team.",
                                "明亮的度假小屋，可容纳整个团队。",
                            ),
                        ),
                        FIGMA => (
                            "Figma",
                            lang.text("KUI workshop", "KUI 工作坊"),
                            lang.text(
                                "Design explorations and notes for review.",
                                "供评审的设计探索与说明。",
                            ),
                        ),
                        SPOTIFY => (
                            "Spotify",
                            lang.text("Studio listening", "工作室歌单"),
                            lang.text(
                                "A shared playlist for focused work.",
                                "一份适合专注工作的共享歌单。",
                            ),
                        ),
                        _ => (
                            lang.text("Sample site", "样例网站"),
                            lang.text("A useful resource", "一份实用资料"),
                            lang.text(
                                "An illustrative preview supplied by the host.",
                                "由宿主提供的模拟链接预览。",
                            ),
                        ),
                    };
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
                                        Ok(Some(LinkPreview::new(site, title, description))),
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
        composer.update(cx, |s, cx| {
            s.set_visible(visible, cx);
            s.set_text(format!("{}\n{AIRBNB}", examples[0].1), w, cx);
        });
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
                div().text_size(px(12.)).text_color(p.faint).child(lang.text(
                    "Demo composer. Paste a link or try a sample to open its preview. Nothing is sent.",
                    "演示编辑器。粘贴链接或试用样例可查看预览，消息不会发送。",
                )),
            )
    }
}
