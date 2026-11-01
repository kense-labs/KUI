//! Review a proposed edit and accept or reject its changes.
use crate::primitives::family::Paint;
use crate::primitives::ops_surface::*;
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
            summary: String::new(),
            hunks,
            focus: cx.focus_handle().tab_stop(true),
        }
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
        let mut body = column().id("patch").max_h(px(330.)).overflow_y_scroll();
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
                                    .p(px(3.))
                                    .text_size(px(12.))
                                    .font_family("monospace")
                                    .bg(colour.opacity(if h.decision == Decision::Pending {
                                        0.08
                                    } else {
                                        0.
                                    }))
                                    .child(format!("{label} {s}")),
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
            .child(title(self.summary.clone()))
            .child(row().child(badge(self.file.clone(), cx)).child(note(
                format!(
                    "+{}   −{}",
                    self.hunks.iter().map(|h| h.after.len()).sum::<usize>(),
                    self.hunks.iter().map(|h| h.before.len()).sum::<usize>()
                ),
                cx,
            )))
            .child(body)
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
