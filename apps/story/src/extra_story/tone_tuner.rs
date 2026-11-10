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
    _sub: Subscription,
}
impl Example {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let view = cx.new(|cx| {
            ToneTuner::new(
                language.text("Reply draft", "回复草稿"),
                language.text(
                    "Hi Sofia,\n\nWe found a duplicate charge of $1,240 on invoice INV-20481. We have issued a refund to your original payment method. It may take 5–10 business days to appear.\n\nPlease reply if you need anything else.\n\nBest,\nJordan",
                    "Sofia，你好。\n\n我们查到发票 INV-20481 上重复扣了 1,240 美元，已原路退款。款项预计在 5–10 个工作日内到账。\n\n如需其他帮助，请直接回复。\n\nJordan",
                ),
                w, cx,
            ).with_recipient("Sofia Ramirez", language.text("Re: Duplicate charge on INV-20481", "回复：发票 INV-20481 重复扣款"))
        });
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
                    let settings = *settings;
                    let voice = settings.voice;
                    let language = s.language;
                    let weak = view.downgrade();
                    cx.spawn_in(w, async move |_, cx| {
                        cx.background_executor()
                            .timer(Duration::from_millis(850))
                            .await;
                        let _ = cx.update(|w, cx| {
                            let _ = weak.update(cx, |v, cx| {
                                let greeting = match (settings.formality, voice) {
                                    (2, _) => language.text("Dear Sofia,", "Sofia，您好："),
                                    (_, VoiceTone::Friendly) => {
                                        language.text("Hi Sofia!", "Sofia，你好！")
                                    }
                                    (_, VoiceTone::Empathetic) => language.text(
                                        "Hi Sofia, I'm sorry for the duplicate charge.",
                                        "Sofia，你好。很抱歉给你带来了重复扣款的困扰。",
                                    ),
                                    _ => language.text("Hi Sofia,", "Sofia，你好。"),
                                };
                                let body = text
                                    .strip_prefix(language.text("Hi Sofia,", "Sofia，你好。"))
                                    .unwrap_or(&text)
                                    .trim_start();
                                let detail = match settings.detail {
                                    0 => body
                                        .split_inclusive(language.text(". ", "。"))
                                        .take(2)
                                        .collect::<String>(),
                                    2 => format!(
                                        "{} {}",
                                        body,
                                        language.text(
                                            "You do not need to take any further action.",
                                            "你无需执行其他操作。",
                                        )
                                    ),
                                    _ => body.to_owned(),
                                };
                                let output = format!("{greeting}\n\n{}", detail.trim_end());
                                v.finish(ticket, output, w, cx);
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
