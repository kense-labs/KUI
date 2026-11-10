//! Review a proposed edit and accept or reject its changes.
use crate::primitives::ops_surface::*;
use crate::primitives::{brand_mark::brand_mark, family::Paint};
use gpui::{prelude::*, *};
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Pending,
    Accepted,
    Rejected,
}
pub struct Hunk {
    pub line: usize,
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub decision: Decision,
}
pub struct PatchEvent {
    pub hunks: Vec<usize>,
    pub decision: Decision,
}
pub struct PatchReview {
    id: SharedString,
    pub file: String,
    model: Option<SharedString>,
    pub summary: String,
    pub hunks: Vec<Hunk>,
    focus: FocusHandle,
}
impl EventEmitter<PatchEvent> for PatchReview {}
impl PatchReview {
    pub fn new(
        id: impl Into<SharedString>,
        file: impl Into<String>,
        hunks: Vec<Hunk>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            id: id.into(),
            file: file.into(),
            model: None,
            summary: String::new(),
            hunks,
            focus: cx.focus_handle().tab_stop(true),
        }
    }
    /// Name the host-selected model in the review header.
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = Some(model.into());
        self
    }
    fn decide(&mut self, at: Option<usize>, d: Decision, cx: &mut Context<Self>) {
        let mut changed = vec![];
        for (i, h) in self.hunks.iter_mut().enumerate() {
            if h.decision == Decision::Pending && at.is_none_or(|j| j == i) {
                h.decision = d;
                changed.push(i)
            }
        }
        if !changed.is_empty() {
            cx.emit(PatchEvent {
                hunks: changed,
                decision: d,
            });
            cx.notify()
        }
    }
}
impl Render for PatchReview {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let mut body = column().id("patch").max_h(px(374.)).overflow_y_scroll();
        for (i, h) in self.hunks.iter().enumerate() {
            let mut part = column()
                .gap_0()
                .child(note(format!("@@ {} @@", h.line), cx));
            for (old, lines, visible) in [
                (true, &h.before, h.decision != Decision::Accepted),
                (false, &h.after, h.decision != Decision::Rejected),
            ] {
                let key = if old { "old-lines" } else { "new-lines" };
                let fold = Fold::get((key, i), visible, (lines.len() * 22) as f32, 0.26, w, cx);
                if fold.mounted() {
                    let label = if h.decision == Decision::Pending {
                        if old { "−" } else { "+" }
                    } else {
                        " "
                    };
                    let colour = if old { p.danger } else { p.success };
                    let text = column()
                        .gap_0()
                        .children(lines.iter().enumerate().map(|(n, s)| {
                            arrive(
                                div()
                                    .w_full()
                                    .h(px(22.))
                                    .flex()
                                    .items_center()
                                    .gap(px(7.))
                                    .px(px(8.))
                                    .text_size(px(12.))
                                    .font_family("monospace")
                                    .bg(colour.opacity(if h.decision == Decision::Pending {
                                        0.08
                                    } else {
                                        0.
                                    }))
                                    .child(
                                        div()
                                            .w(px(28.))
                                            .flex_none()
                                            .text_right()
                                            .text_color(p.faint)
                                            .child((h.line + n).to_string()),
                                    )
                                    .child(
                                        div()
                                            .w(px(10.))
                                            .flex_none()
                                            .text_color(colour)
                                            .child(label),
                                    )
                                    .child(div().min_w_0().truncate().child(s.clone())),
                                (key, i * 131 + n),
                                if old { 0 } else { 38 * n as u64 },
                                2.,
                                cx,
                            )
                        }));
                    part = part.child(fold.wrap(0., text))
                }
            }
            if h.decision == Decision::Pending {
                part = part.child(
                    row()
                        .child(btn(
                            ("accept", i),
                            tr(cx, "Accept", "接受"),
                            false,
                            cx,
                            move |s, _, cx| s.decide(Some(i), Decision::Accepted, cx),
                        ))
                        .child(btn(
                            ("reject", i),
                            tr(cx, "Reject", "拒绝"),
                            false,
                            cx,
                            move |s, _, cx| s.decide(Some(i), Decision::Rejected, cx),
                        )),
                )
            }
            body = body.child(part)
        }
        let pending = self
            .hunks
            .iter()
            .filter(|h| h.decision == Decision::Pending)
            .count();
        panel(&self.id, 522., cx)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                if e.keystroke.key == "enter"
                    && (e.keystroke.modifiers.platform || e.keystroke.modifiers.control)
                {
                    s.decide(None, Decision::Accepted, cx);
                    cx.stop_propagation()
                }
            }))
            .child(
                row()
                    .items_start()
                    .gap(px(8.))
                    .children(
                        self.model
                            .as_ref()
                            .and_then(|model| brand_mark(model, 16., cx)),
                    )
                    .child(
                        column()
                            .min_w_0()
                            .gap(px(2.))
                            .child(title(self.summary.clone()))
                            .when_some(self.model.clone(), |head, model| {
                                head.child(note(model, cx))
                            }),
                    ),
            )
            .child(
                column()
                    .gap_0()
                    .border_1()
                    .border_color(p.line)
                    .rounded(crate::foundation::surface_radius(cx))
                    .overflow_hidden()
                    .child(
                        row()
                            .justify_between()
                            .px(px(10.))
                            .py(px(6.))
                            .border_b_1()
                            .border_color(p.line)
                            .child(badge(self.file.clone(), cx))
                            .child(note(
                                format!(
                                    "+{}   −{}",
                                    self.hunks.iter().map(|h| h.after.len()).sum::<usize>(),
                                    self.hunks.iter().map(|h| h.before.len()).sum::<usize>()
                                ),
                                cx,
                            )),
                    )
                    .child(body),
            )
            .when(pending > 0, |r| {
                r.child(
                    row()
                        .child(btn(
                            "accept-all",
                            tr(cx, "Accept all", "全部接受"),
                            true,
                            cx,
                            |s, _, cx| s.decide(None, Decision::Accepted, cx),
                        ))
                        .child(btn(
                            "reject-all",
                            tr(cx, "Reject all", "全部拒绝"),
                            false,
                            cx,
                            |s, _, cx| s.decide(None, Decision::Rejected, cx),
                        )),
                )
            })
    }
}
