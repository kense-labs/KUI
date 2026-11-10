//! Demonstrates feedback with a simulated submission and retry.
use gpui::{prelude::*, *};
use kui::{
    components::response_feedback::{ResponseFeedback, ResponseFeedbackEvent},
    primitives::{control::UiLanguage, family::Paint},
};
use std::time::Duration;

pub const IDS: &[&str] = &["response-feedback"];
pub fn story(id: &str, w: &mut Window, cx: &mut Context<crate::StoryHost>) -> Option<AnyView> {
    IDS.contains(&id)
        .then(|| cx.new(|cx| Example::new(w, cx)).into())
}
struct Example {
    feedback: Entity<ResponseFeedback>,
    language: UiLanguage,
    fail_next: bool,
    _subscription: Subscription,
}
impl Example {
    fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let feedback = cx.new(|cx| {
            ResponseFeedback::new("example-feedback", "example-message", w, cx)
                .with_response("Claude Sonnet 5.5", language.text(
                    "To prepare the release, verify the build in staging and run the checkout smoke test. Then publish the changelog and keep an eye on the first deployment.",
                    "发布前，先在预发布环境验证构建并测试结账流程。然后发布更新日志，留意首次部署的运行情况。",
                ), cx)
        });
        let subscription = cx.subscribe(
            &feedback,
            |this: &mut Self, _, event: &ResponseFeedbackEvent, cx| {
                if let ResponseFeedbackEvent::Submit(submission) = event {
                    let id = submission.request_id;
                    let fail = this.fail_next;
                    this.fail_next = false;
                    cx.spawn(async move |this, cx| {
                        cx.background_executor()
                            .timer(Duration::from_millis(550))
                            .await;
                        let _ = this.update(cx, |this, cx| {
                            this.feedback
                                .update(cx, |view, cx| view.complete(id, !fail, cx))
                        });
                    })
                    .detach();
                }
            },
        );
        Self {
            feedback,
            language,
            fail_next: true,
            _subscription: subscription,
        }
    }
}
impl Render for Example {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let paint = Paint::get(cx);
        div()
            .w(px(522.))
            .max_w_full()
            .flex()
            .flex_col()
            .gap(px(14.))
            .text_color(paint.ink)
            .child(self.feedback.clone())
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(paint.secondary)
                    .child(self.language.text(
                        "Sample only. The first submission fails so you can try again.",
                        "仅为演示。首次提交会模拟失败，可继续重试。",
                    )),
            )
    }
}
