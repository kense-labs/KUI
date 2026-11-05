//! Sample generation only: image assets are local and no requests are made.
use gpui::{prelude::*, *};
use kui::{
    components::image_generation::{GeneratedImage, ImageGeneration, ImageGenerationEvent},
    primitives::{control::UiLanguage, family::Paint},
};
use std::{collections::HashMap, sync::Arc, time::Duration};
pub const IDS: &[&str] = &["image-generation"];
pub fn story(id: &str, w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id)
        .then(|| cx.new(|cx| Example::new(w, cx)).into())
}
struct Example {
    view: Entity<ImageGeneration>,
    language: UiLanguage,
    failed: bool,
    variation_failed: bool,
    images: HashMap<String, usize>,
    _subscription: Subscription,
}
fn sample(index: usize) -> ImageSource {
    let bytes: &[u8] = match index % 4 {
        0 => include_bytes!("../../assets/navigation/room-0.jpg"),
        1 => include_bytes!("../../assets/navigation/room-1.jpg"),
        2 => include_bytes!("../../assets/navigation/room-2.jpg"),
        _ => include_bytes!("../../assets/navigation/room-3.jpg"),
    };
    Arc::new(Image::from_bytes(ImageFormat::Jpeg, bytes.to_vec())).into()
}
impl Example {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let view = cx.new(|cx| ImageGeneration::new(w, cx));
        view.update(cx, |v, cx| {
            v.set_prompt(
                language.text("A quiet room in morning light", "晨光中的安静房间"),
                w,
                cx,
            )
        });
        let subscription = cx.subscribe(
            &view,
            |s: &mut Self, view, event: &ImageGenerationEvent, cx| {
                let (ticket, index, fail) = match event {
                    ImageGenerationEvent::Generate {
                        ticket,
                        index,
                        variation_of,
                        ..
                    } => {
                        let fail = if variation_of.is_some() && !s.variation_failed {
                            s.variation_failed = true;
                            true
                        } else if variation_of.is_none() && *index == 1 && !s.failed {
                            s.failed = true;
                            true
                        } else {
                            false
                        };
                        (*ticket, *index, fail)
                    }
                    ImageGenerationEvent::Upscale { ticket, image_id } => (
                        *ticket,
                        *s.images.get(image_id.as_ref()).unwrap_or(&0),
                        false,
                    ),
                    _ => return,
                };
                s.images.insert(format!("sample-{ticket}"), index);
                let weak = view.downgrade();
                let error = s
                    .language
                    .text("Sample failure — retry", "样例失败，请重试");
                cx.spawn(async move |_, cx| {
                    for step in 1..=4 {
                        cx.background_executor()
                            .timer(Duration::from_millis(240))
                            .await;
                        if weak
                            .update(cx, |v, cx| v.set_progress(ticket, step as f32 / 4., cx))
                            .is_err()
                        {
                            return;
                        }
                    }
                    let _ = weak.update(cx, |v, cx| {
                        if fail {
                            v.fail(ticket, error, cx);
                        } else {
                            let (width, height) =
                                [(338, 460), (460, 366), (460, 313), (354, 460)][index % 4];
                            v.finish(
                                ticket,
                                GeneratedImage::new(
                                    format!("sample-{ticket}"),
                                    sample(index),
                                    "Sample image",
                                    width,
                                    height,
                                ),
                                cx,
                            );
                        }
                    });
                })
                .detach();
            },
        );
        Self {
            view,
            language,
            failed: false,
            variation_failed: false,
            images: HashMap::new(),
            _subscription: subscription,
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
                        "Simulated generation — no data is sent",
                        "模拟生成，不会发送数据",
                    )),
            )
            .child(div().w_full().max_w(px(640.)).child(self.view.clone()))
    }
}
