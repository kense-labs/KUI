//! Sample generation only: image assets are local and no requests are made.
use gpui::{prelude::*, *};
use kui::{
    components::image_generation::{GeneratedImage, ImageGeneration, ImageGenerationEvent},
    primitives::{
        control::UiLanguage,
        family::{Paint, Tone, face},
    },
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
    fail_next: bool,
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
        let view = cx.new(|cx| {
            ImageGeneration::new(w, cx)
                .models(vec!["GPT Image".into(), "Gemini".into(), "Grok".into()])
                .with_suggestions(vec![
                    language
                        .text(
                            "Terracotta architecture under a clear blue sky",
                            "蓝天下的赤陶建筑",
                        )
                        .into(),
                    language
                        .text(
                            "Glazed cups on a pale table in soft daylight",
                            "柔光下浅色桌面上的釉色茶杯",
                        )
                        .into(),
                    language
                        .text(
                            "A reading corner with oak and woven linen",
                            "橡木与亚麻布布置的阅读角",
                        )
                        .into(),
                ])
        });
        view.update(cx, |v, cx| {
            v.set_prompt(
                language.text("Quiet interiors in soft morning light, with textured walls, oak, and warm shadows", "柔和晨光下的室内，带有纹理墙面、橡木与暖色阴影"),
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
                        let fail = (variation_of.is_some() || *index == 1) && s.fail_next;
                        if fail {
                            s.fail_next = false;
                            cx.notify();
                        }
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
        view.update(cx, |view, cx| view.generate(cx));
        Self {
            view,
            language,
            fail_next: false,
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
            .child(div().w_full().max_w(px(522.)).child(self.view.clone()))
            .child(
                face(
                    "image-example-fail",
                    self.language.text("Fail one image", "模拟一张图片失败"),
                    32.,
                    kui::foundation::surface_radius(cx).into(),
                    if self.fail_next {
                        Tone::Surface
                    } else {
                        Tone::Clear
                    },
                    cx,
                )
                .role(Role::Switch)
                .aria_toggled(self.fail_next.into())
                .px_2()
                .child(self.language.text("Fail one image", "模拟一张图片失败"))
                .on_click(cx.listener(|s, _, _, cx| {
                    s.fail_next = !s.fail_next;
                    cx.notify();
                })),
            )
    }
}
