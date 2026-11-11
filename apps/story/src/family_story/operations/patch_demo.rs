//! Local streaming and recovery fixtures for the patch review.
use gpui::{prelude::*, *};
use kui::{
    components::code_suggestion::{Decision, Hunk, PatchControlEvent, PatchPhase, PatchReview},
    primitives::{control::UiLanguage, family::disabled_motion},
};
use std::time::Duration;

const ORIGINAL: &str = "use std::time::Duration;\nuse crate::billing::{Invoice, Request, send};\n\npub async fn fetch_invoice(request: Request) -> Result<Invoice> {\n    let response = send(request).await?;\n    response.body()\n}";

fn hunks() -> Vec<Hunk> {
    [
        (
            3,
            &[][..],
            &[
                "use crate::time::sleep;",
                "",
                "const MAX_ATTEMPTS: usize = 4;",
                "const RETRY_DELAY_MS: u64 = 250;",
            ][..],
        ),
        (
            5,
            &["    let response = send(request).await?;"][..],
            &[
                "    let mut response = send(request.clone()).await?;",
                "    for attempt in 1..MAX_ATTEMPTS {",
                "        if !response.is_retryable() { break; }",
                "        sleep(Duration::from_millis(RETRY_DELAY_MS << (attempt - 1))).await;",
                "        response = send(request.clone()).await?;",
                "    }",
            ][..],
        ),
        (
            6,
            &["    response.body()"][..],
            &["    response.ensure_success()?.body()"][..],
        ),
    ]
    .into_iter()
    .map(|(line, before, after)| Hunk {
        line,
        before: before.iter().map(|line| (*line).to_owned()).collect(),
        after: after.iter().map(|line| (*line).to_owned()).collect(),
        decision: Decision::Pending,
    })
    .collect()
}

pub(super) struct PatchDemo {
    control: Entity<PatchReview>,
    language: UiLanguage,
    task: Option<Task<()>>,
    completed: usize,
    _subscription: Subscription,
}

impl PatchDemo {
    #[cfg(test)]
    pub(super) fn review(&self) -> &Entity<PatchReview> {
        &self.control
    }

    pub(super) fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = UiLanguage::current(cx);
        let control = cx.new(|cx| {
            let mut patch = PatchReview::new("code-suggestion", "fetch_invoice.rs", vec![], cx)
                .with_model("Claude Sonnet 5.5")
                .with_original(ORIGINAL)
                .with_generation_controls(px(374.));
            patch.summary = language
                .text("Add retry with backoff", "添加退避重试")
                .into();
            patch
        });
        let subscription = cx.subscribe_in(
            &control,
            w,
            |s: &mut Self, _, event: &PatchControlEvent, _, cx| match event {
                PatchControlEvent::Stop => s.stop(cx),
                PatchControlEvent::Retry | PatchControlEvent::Regenerate => s.start(cx),
            },
        );
        cx.defer_in(w, |s, _, cx| s.start(cx));
        Self {
            control,
            language,
            task: None,
            completed: 0,
            _subscription: subscription,
        }
    }

    pub(super) fn start(&mut self, cx: &mut Context<Self>) {
        self.task = None;
        self.completed = 0;
        let reduced = disabled_motion(cx);
        self.control.update(cx, |control, cx| {
            control.set_phase(PatchPhase::Thinking, cx);
            if reduced {
                control.hunks = hunks();
                control.set_phase(PatchPhase::Ready, cx);
            }
        });
        if reduced {
            return;
        }
        let control = self.control.downgrade();
        self.task = Some(cx.spawn(async move |this, cx| {
            wait_for_preview(cx).await;
            cx.background_executor()
                .timer(Duration::from_millis(800))
                .await;
            wait_for_preview(cx).await;
            if control
                .update(cx, |control, cx| {
                    control.set_phase(PatchPhase::Streaming, cx)
                })
                .is_err()
            {
                return;
            }
            for (i, mut hunk) in hunks().into_iter().enumerate() {
                let additions = std::mem::take(&mut hunk.after);
                if control
                    .update(cx, |control, cx| {
                        control.hunks.push(hunk);
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
                for line in additions {
                    cx.background_executor()
                        .timer(Duration::from_millis(38))
                        .await;
                    wait_for_preview(cx).await;
                    if control
                        .update(cx, |control, cx| {
                            if let Some(hunk) = control.hunks.get_mut(i) {
                                hunk.after.push(line);
                                cx.notify();
                            }
                        })
                        .is_err()
                    {
                        return;
                    }
                }
                if this.update(cx, |s, _| s.completed = i + 1).is_err() {
                    return;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(120))
                    .await;
                wait_for_preview(cx).await;
            }
            let _ = control.update(cx, |control, cx| control.set_phase(PatchPhase::Ready, cx));
        }));
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        self.task = None;
        self.control.update(cx, |control, cx| {
            control.hunks.truncate(self.completed);
            control.set_phase(PatchPhase::Ready, cx);
        });
    }

    pub(super) fn show_empty(&mut self, cx: &mut Context<Self>) {
        self.task = None;
        self.control.update(cx, |control, cx| {
            control.set_phase(PatchPhase::Thinking, cx);
            control.set_phase(PatchPhase::Ready, cx);
        });
    }

    pub(super) fn show_error(&mut self, cx: &mut Context<Self>) {
        self.task = None;
        let error = self.language.text(
            "The model timed out. The original code is intact.",
            "模型响应超时，原始代码保持不变。",
        );
        self.control.update(cx, |control, cx| {
            control.set_phase(PatchPhase::Thinking, cx);
            control.set_phase(PatchPhase::Error(error.into()), cx);
        });
    }
}

impl Render for PatchDemo {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = kui::primitives::family::Paint::get(cx);
        div()
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(8.))
            .child(self.control.clone())
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(p.secondary)
                    .child(self.language.text(
                        "Sample review. Nothing is written to disk.",
                        "样例审阅，不会写入文件。",
                    )),
            )
    }
}

async fn wait_for_preview(cx: &AsyncApp) {
    while !cx.update(|cx| {
        cx.try_global::<crate::StoryBridge>()
            .is_none_or(|bridge| (bridge.visible)())
    }) {
        cx.background_executor()
            .timer(Duration::from_millis(250))
            .await;
    }
}
