//! Simulated tone changes; no text leaves the device.
use gpui::{prelude::*, *};
use kui::{
    components::tone_tuner::{ToneTuner, ToneTunerEvent, VoiceTone},
    primitives::{control::UiLanguage, family::Paint},
};
use std::time::Duration;
pub const IDS: &[&str] = &["tone-tuner"];
pub fn story(id: &str, w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id)
        .then(|| cx.new(|cx| Example::new(w, cx)).into())
}
struct Example {
    view: Entity<ToneTuner>,
    language: UiLanguage,
    fail_once: bool,
    _sub: Subscription,
}
impl Example {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let view=cx.new(|cx|ToneTuner::new(language.text("Reply draft","回复草稿"),language.text("Thanks for letting us know. We have received your request and will follow up soon.","感谢你告诉我们。我们已收到你的请求，会尽快跟进。"),w,cx));
        let sub = cx.subscribe_in(
            &view,
            w,
            |s: &mut Self, view, event: &ToneTunerEvent, w, cx| {
                if let ToneTunerEvent::Rewrite {
                    ticket,
                    text,
                    settings,
                } = event
                {
                    let ticket = *ticket;
                    let text = text.to_string();
                    let voice = settings.voice;
                    let language = s.language;
                    let fail = s.fail_once;
                    s.fail_once = false;
                    let weak = view.downgrade();
                    cx.spawn_in(w, async move |_, cx| {
                        cx.background_executor()
                            .timer(Duration::from_millis(850))
                            .await;
                        let _ = cx.update(|w, cx| {
                            let _ = weak.update(cx, |v, cx| {
                                if fail {
                                    v.fail(
                                        ticket,
                                        language.text(
                                            "Sample rewrite failed; retry.",
                                            "样例改写失败，请重试。",
                                        ),
                                        cx,
                                    );
                                } else {
                                    let suffix = match voice {
                                        VoiceTone::Friendly => language
                                            .text(" We're happy to help.", " 我们很乐意帮助你。"),
                                        VoiceTone::Confident => language
                                            .text(" We will take care of it.", " 我们会妥善处理。"),
                                        VoiceTone::Empathetic => language.text(
                                            " We understand this matters to you.",
                                            " 我们理解这对你很重要。",
                                        ),
                                        VoiceTone::Neutral => "",
                                    };
                                    v.finish(ticket, format!("{text}{suffix}"), w, cx);
                                }
                            });
                        });
                    })
                    .detach();
                }
            },
        );
        Self {
            view,
            language,
            fail_once: true,
            _sub: sub,
        }
    }
}
impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        div()
            .size_full()
            .min_h(px(600.))
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
                        "Simulated rewrite — text is not sent",
                        "模拟改写，文字不会发送",
                    )),
            )
            .child(div().w_full().max_w(px(640.)).child(self.view.clone()))
    }
}
