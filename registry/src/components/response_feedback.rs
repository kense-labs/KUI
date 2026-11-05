//! Collect a rating and optional structured feedback for a response.
use crate::primitives::{
    editor::{Editor, InputMode},
    family::{Paint, Slide, Tone, face},
    glyph::{Glyph, transformed},
    nav::tr,
};
use gpui::{prelude::*, *};
use std::time::Duration;

/// Whether a response was useful.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedbackRating {
    Positive,
    Negative,
}
/// A single submission. The host owns networking and reports its result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedbackSubmission {
    pub message_id: SharedString,
    pub request_id: u64,
    pub rating: FeedbackRating,
    pub reasons: Vec<SharedString>,
    pub note: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResponseFeedbackEvent {
    Submit(FeedbackSubmission),
    RatingChanged(Option<FeedbackRating>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FeedbackPhase {
    Form,
    Pending,
    Failed,
    ThankYou,
}

/// Inline feedback form with explicit host acknowledgement and retry.
pub struct ResponseFeedback {
    id: SharedString,
    message_id: SharedString,
    rating: Option<FeedbackRating>,
    reasons: Vec<SharedString>,
    editor: Entity<Editor>,
    phase: FeedbackPhase,
    expanded: bool,
    expansion: Slide,
    sequence: u64,
    focus: FocusHandle,
    first_reason_focus: FocusHandle,
    _editor_subscription: Subscription,
}
impl EventEmitter<ResponseFeedbackEvent> for ResponseFeedback {}
impl ResponseFeedback {
    pub fn new(
        id: impl Into<SharedString>,
        message_id: impl Into<SharedString>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let label = tr(cx, "Optional note", "补充说明（可选）");
        let editor = cx.new(|cx| Editor::new(label, InputMode::Multi, w, cx));
        let subscription = cx.subscribe(
            &editor,
            |_: &mut Self, _, _: &crate::primitives::editor::EditEvent, cx| cx.notify(),
        );
        Self {
            id: id.into(),
            message_id: message_id.into(),
            rating: None,
            reasons: Vec::new(),
            editor,
            phase: FeedbackPhase::Form,
            expanded: false,
            expansion: Slide::new(0.),
            sequence: 0,
            focus: cx.focus_handle(),
            first_reason_focus: cx.focus_handle(),
            _editor_subscription: subscription,
        }
    }
    pub fn rating(&self) -> Option<FeedbackRating> {
        self.rating
    }
    fn choose(&mut self, rating: FeedbackRating, cx: &mut Context<Self>) {
        if self.phase == FeedbackPhase::Pending {
            return;
        }
        if self.rating == Some(rating) && self.expanded {
            self.expanded = false;
            self.expansion.duration(0., 0.24, cx);
        } else {
            if self.rating != Some(rating) {
                self.reasons.clear();
            }
            self.rating = Some(rating);
            self.phase = FeedbackPhase::Form;
            self.expanded = true;
            self.expansion.duration(1., 0.24, cx);
            cx.emit(ResponseFeedbackEvent::RatingChanged(self.rating));
        }
        cx.notify();
    }
    fn submit(&mut self, cx: &mut Context<Self>) {
        let Some(rating) = self.rating else {
            return;
        };
        if self.phase == FeedbackPhase::Pending {
            return;
        }
        self.sequence = self.sequence.wrapping_add(1);
        self.phase = FeedbackPhase::Pending;
        cx.emit(ResponseFeedbackEvent::Submit(FeedbackSubmission {
            message_id: self.message_id.clone(),
            request_id: self.sequence,
            rating,
            reasons: self.reasons.clone(),
            note: self.editor.read(cx).value().to_owned(),
        }));
        cx.notify();
    }
    /// Complete only the current request; late callbacks cannot overwrite newer state.
    pub fn complete(&mut self, request_id: u64, success: bool, cx: &mut Context<Self>) {
        if self.phase != FeedbackPhase::Pending || request_id != self.sequence {
            return;
        }
        if success {
            self.phase = FeedbackPhase::ThankYou;
            let epoch = self.sequence;
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(1600))
                    .await;
                let _ = this.update(cx, |this, cx| {
                    if this.sequence == epoch && this.phase == FeedbackPhase::ThankYou {
                        this.expanded = false;
                        this.expansion.duration(0., 0.24, cx);
                        cx.notify();
                    }
                });
            })
            .detach();
        } else {
            self.phase = FeedbackPhase::Failed;
        }
        cx.notify();
    }
}
impl Render for ResponseFeedback {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let radius = crate::foundation::surface_radius(cx);
        let height = self.expansion.get(w, cx).clamp(0., 1.);
        let mut actions = div().flex().items_center().gap(px(2.));
        for (key, label, rating) in [
            (
                "positive",
                tr(cx, "Good response", "回答有帮助"),
                FeedbackRating::Positive,
            ),
            (
                "negative",
                tr(cx, "Bad response", "回答没有帮助"),
                FeedbackRating::Negative,
            ),
        ] {
            let active = self.rating == Some(rating);
            let button = face(key, label, 36., 8., Tone::Clear, cx)
            .size(px(36.))
            .rounded(px(8.))
            .p_0()
            .text_color(if active { p.ink } else { p.secondary })
            .hover(|s| s.bg(p.tray).text_color(p.ink))
            .aria_toggled(active.into())
            .child(transformed(
                Glyph::Like,
                16.,
                if active { p.ink } else { p.secondary },
                if rating == FeedbackRating::Positive {
                    0.
                } else {
                    std::f32::consts::PI
                },
                1.,
                (0., 0.),
                1.,
            ))
            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                this.choose(rating, cx);
                if this.expanded && matches!(event, ClickEvent::Keyboard(_)) {
                    this.first_reason_focus.focus(window, cx);
                }
            }));
            actions = actions.child(button);
        }
        let mut panel = div()
            .w_full()
            .overflow_hidden()
            .max_h(px(270. * height))
            .opacity(height);
        if height > 0. || self.expanded {
            let mut body = div()
                .flex()
                .flex_col()
                .gap(px(12.))
                .p(px(12.));
            if self.phase == FeedbackPhase::ThankYou {
                body = body.child(
                    div()
                        .relative()
                        .min_h(px(32.))
                        .pl(px(24.))
                        .text_size(px(14.))
                        .line_height(px(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(tr(cx, "Thank you for your feedback", "感谢你的反馈")),
                );
            } else {
                let positive = self.rating == Some(FeedbackRating::Positive);
                body = body.child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(if positive {
                            tr(cx, "What was helpful?", "哪些地方有帮助？")
                        } else {
                            tr(cx, "What could be better?", "哪些地方需要改进？")
                        }),
                );
                let choices = if positive {
                    [
                        tr(cx, "Accurate", "准确"),
                        tr(cx, "Clear", "清晰"),
                        tr(cx, "Useful", "有用"),
                    ]
                } else {
                    [
                        tr(cx, "Inaccurate", "不准确"),
                        tr(cx, "Incomplete", "不完整"),
                        tr(cx, "Unsafe", "不安全"),
                    ]
                };
                let mut chips = div().flex().flex_wrap().gap(px(8.));
                for (index, label) in choices.into_iter().enumerate() {
                    let selected = self.reasons.iter().any(|x| x.as_ref() == label);
                    let reason: SharedString = label.into();
                    chips = chips.child(
                        face(
                            ("feedback-reason", index),
                            label,
                            32.,
                            8.,
                            Tone::Clear,
                            cx,
                        )
                        .h(px(32.))
                        .px(px(12.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(if selected { p.ink } else { p.line })
                        .bg(if selected { p.ink } else { p.panel })
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .text_color(if selected { p.inverse } else { p.secondary })
                        .when(!selected, |b| {
                            b.hover(|s| s.bg(p.tray).border_color(p.faint).text_color(p.ink))
                        })
                        .aria_toggled(selected.into())
                        .when(index == 0, |button| {
                            button.track_focus(&self.first_reason_focus)
                        })
                        .child(label)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if this.phase == FeedbackPhase::Pending {
                                return;
                            }
                            if let Some(i) = this.reasons.iter().position(|x| *x == reason) {
                                this.reasons.remove(i);
                            } else {
                                this.reasons.push(reason.clone());
                            }
                            cx.notify();
                        })),
                    );
                }
                body = body.child(chips).child(
                    div()
                        .min_h(px(72.))
                        .h(px(72.))
                        .px(px(12.))
                        .py(px(8.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(p.line)
                        .child(self.editor.clone()),
                );
                if self.phase == FeedbackPhase::Failed {
                    body = body.child(div().text_color(p.danger).child(tr(
                        cx,
                        "Could not send. Please retry.",
                        "发送失败，请重试。",
                    )));
                }
                let label = match self.phase {
                    FeedbackPhase::Pending => tr(cx, "Sending…", "发送中…"),
                    FeedbackPhase::Failed => tr(cx, "Try again", "重试"),
                    _ => tr(cx, "Submit", "提交"),
                };
                body = body.child(
                    face(
                        "submit-feedback",
                        label,
                        32.,
                        radius.into(),
                        Tone::Primary,
                        cx,
                    )
                    .px(px(12.))
                    .child(label)
                    .on_click(cx.listener(|this, _, _, cx| this.submit(cx))),
                );
            }
            panel = panel.child(body);
        }
        div()
            .id(self.id.clone())
            .track_focus(&self.focus)
            .flex()
            .flex_col()
            .w_full()
            .max_w(px(600.))
            .text_color(p.ink)
            .child(actions)
            .child(panel)
    }
}

#[cfg(test)]
mod tests {
    use super::{FeedbackPhase, FeedbackRating, ResponseFeedback};
    use gpui::TestAppContext;

    #[test]
    fn failed_feedback_keeps_the_draft_and_ignores_old_completions() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, window) = app
            .add_window_view(|window, cx| ResponseFeedback::new("feedback", "message", window, cx));
        window.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.choose(FeedbackRating::Negative, cx);
                view.reasons.push("Inaccurate".into());
                view.editor
                    .update(cx, |editor, cx| editor.set_value("Details", window, cx));
                view.submit(cx);
                let first = view.sequence;
                view.submit(cx);
                assert_eq!(view.sequence, first);
                view.complete(first, false, cx);
                assert_eq!(view.phase, FeedbackPhase::Failed);
                assert_eq!(view.editor.read(cx).value(), "Details");
                assert_eq!(view.reasons[0].as_ref(), "Inaccurate");
                view.submit(cx);
                let retry = view.sequence;
                assert_ne!(retry, first);
                view.complete(first, true, cx);
                assert_eq!(view.phase, FeedbackPhase::Pending);
                view.complete(retry, true, cx);
                assert_eq!(view.phase, FeedbackPhase::ThankYou);
            });
        });
    }
}
