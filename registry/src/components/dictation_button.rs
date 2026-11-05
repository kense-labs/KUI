//! Host-driven dictation controls; the host owns recognition and text insertion.
use crate::primitives::{
    family::{Paint, Slide, Tone, face},
    glyph::{Glyph, icon},
    nav::tr,
};
use gpui::{prelude::*, *};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DictationPhase {
    Idle,
    Requesting,
    Listening,
    Transcribing,
    ConfirmInsert,
    Error,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsertionTarget {
    Original,
    Current,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DictationEvent {
    Start,
    Cancel,
    Finalize,
    Insert {
        text: SharedString,
        target: InsertionTarget,
    },
    Retry,
}
fn current_session(active: bool, current: u64, incoming: u64) -> bool {
    active && current == incoming
}
fn current_insertion(
    active: bool,
    current_session_id: u64,
    session: u64,
    pending: bool,
    current_insertion_id: u64,
    insertion: u64,
) -> bool {
    current_session(active, current_session_id, session)
        && pending
        && current_insertion_id == insertion
}
fn final_is_pending(phase: DictationPhase, pending: bool, awaiting_final: bool) -> bool {
    phase == DictationPhase::Transcribing && pending && awaiting_final
}
pub struct DictationButton {
    phase: DictationPhase,
    interim: SharedString,
    final_text: SharedString,
    error: SharedString,
    elapsed: u64,
    level: f32,
    width: Slide,
    pending: bool,
    anchor_changed: bool,
    awaiting_final: bool,
    session_id: u64,
    session_active: bool,
    insertion_id: u64,
    insertion_pending: bool,
}
impl EventEmitter<DictationEvent> for DictationButton {}
impl Default for DictationButton {
    fn default() -> Self {
        Self::new()
    }
}
impl DictationButton {
    pub fn new() -> Self {
        Self {
            phase: DictationPhase::Idle,
            interim: "".into(),
            final_text: "".into(),
            error: "".into(),
            elapsed: 0,
            level: 0.,
            width: Slide::new(36.),
            pending: false,
            anchor_changed: false,
            awaiting_final: false,
            session_id: 0,
            session_active: false,
            insertion_id: 0,
            insertion_pending: false,
        }
    }
    /// Legacy unscoped update. Async hosts should use `set_phase_for` with a captured session ID.
    pub fn set_phase(&mut self, phase: DictationPhase, cx: &mut Context<Self>) {
        self.phase = phase;
        self.pending = false;
        if matches!(phase, DictationPhase::Idle | DictationPhase::Error) {
            self.session_active = false;
        }
        if phase == DictationPhase::Idle {
            self.anchor_changed = false;
            self.awaiting_final = false;
            self.insertion_pending = false;
            self.final_text = "".into();
            self.interim = "".into();
        }
        self.width.duration(
            if phase == DictationPhase::Idle {
                36.
            } else {
                216.
            },
            0.24,
            cx,
        );
        cx.notify();
    }
    /// Capture this ID in the Start or Retry event handler and attach it to asynchronous callbacks.
    pub fn session_id(&self) -> Option<u64> {
        self.session_active.then_some(self.session_id)
    }
    /// Returns false for a cancelled, finished, or superseded session.
    pub fn set_phase_for(
        &mut self,
        session: u64,
        phase: DictationPhase,
        cx: &mut Context<Self>,
    ) -> bool {
        if !current_session(self.session_active, self.session_id, session) {
            return false;
        }
        if phase == self.phase {
            return true;
        }
        let allowed = matches!(phase, DictationPhase::Idle | DictationPhase::Error)
            || matches!(
                (self.phase, phase),
                (DictationPhase::Requesting, DictationPhase::Listening)
            );
        if !allowed {
            return false;
        }
        self.set_phase(phase, cx);
        true
    }
    /// Only the active recognizer may update its provisional text.
    pub fn set_transcript_for(
        &mut self,
        session: u64,
        settled: impl Into<SharedString>,
        interim: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !current_session(self.session_active, self.session_id, session)
            || !(self.phase == DictationPhase::Listening
                || (self.phase == DictationPhase::Transcribing && self.awaiting_final))
        {
            return false;
        }
        self.set_transcript(settled, interim, cx);
        true
    }
    pub fn set_level_for(
        &mut self,
        session: u64,
        level: f32,
        elapsed: u64,
        cx: &mut Context<Self>,
    ) -> bool {
        if !current_session(self.session_active, self.session_id, session)
            || self.phase != DictationPhase::Listening
        {
            return false;
        }
        self.set_level(level, elapsed, cx);
        true
    }
    pub fn set_anchor_changed_for(
        &mut self,
        session: u64,
        changed: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        if !current_session(self.session_active, self.session_id, session) {
            return false;
        }
        self.set_anchor_changed(changed, cx);
        true
    }
    pub fn set_error_for(
        &mut self,
        session: u64,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !current_session(self.session_active, self.session_id, session)
            || !(matches!(
                self.phase,
                DictationPhase::Requesting | DictationPhase::Listening
            ) || (self.phase == DictationPhase::Transcribing && self.awaiting_final))
        {
            return false;
        }
        self.set_error(error, cx);
        true
    }
    /// Final callbacks must use the captured session ID to exclude an earlier recognizer.
    pub fn finish_recognition_for(
        &mut self,
        session: u64,
        text: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !current_session(self.session_active, self.session_id, session)
            || !final_is_pending(self.phase, self.pending, self.awaiting_final)
        {
            return false;
        }
        self.finish_recognition(text, cx);
        true
    }
    /// Capture this ID when handling Insert; it identifies that insertion attempt.
    pub fn insertion_id(&self) -> Option<u64> {
        self.insertion_pending.then_some(self.insertion_id)
    }
    /// Acknowledge insertion only for the active session and attempt.
    pub fn confirm_insert_for(
        &mut self,
        session: u64,
        insertion: u64,
        cx: &mut Context<Self>,
    ) -> bool {
        if !current_insertion(
            self.session_active,
            self.session_id,
            session,
            self.insertion_pending,
            self.insertion_id,
            insertion,
        ) {
            return false;
        }
        self.insertion_pending = false;
        self.set_phase(DictationPhase::Idle, cx);
        true
    }
    /// Return an insertion error without discarding the recognized text.
    pub fn fail_insert_for(
        &mut self,
        session: u64,
        insertion: u64,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !current_insertion(
            self.session_active,
            self.session_id,
            session,
            self.insertion_pending,
            self.insertion_id,
            insertion,
        ) {
            return false;
        }
        self.insertion_pending = false;
        self.error = error.into();
        self.set_phase(DictationPhase::ConfirmInsert, cx);
        true
    }
    fn emit_insert(&mut self, target: InsertionTarget, cx: &mut Context<Self>) {
        self.insertion_id = self.insertion_id.wrapping_add(1);
        self.insertion_pending = true;
        self.pending = true;
        self.error = "".into();
        cx.emit(DictationEvent::Insert {
            text: self.final_text.clone(),
            target,
        });
        cx.notify();
    }
    /// Legacy unscoped update; use `set_transcript_for` for asynchronous recognition.
    pub fn set_transcript(
        &mut self,
        settled: impl Into<SharedString>,
        interim: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.final_text = settled.into();
        self.interim = interim.into();
        cx.notify();
    }
    pub fn set_level(&mut self, level: f32, elapsed: u64, cx: &mut Context<Self>) {
        self.level = if level.is_finite() {
            level.clamp(0., 1.)
        } else {
            0.
        };
        self.elapsed = elapsed;
        cx.notify();
    }
    /// The host calls this when the editor's insertion position changes during recognition.
    pub fn set_anchor_changed(&mut self, changed: bool, cx: &mut Context<Self>) {
        self.anchor_changed = changed;
        cx.notify();
    }
    pub fn set_error(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.error = text.into();
        self.set_phase(DictationPhase::Error, cx);
    }
    /// Completes the host recognizer's finalization. Stale callbacks after cancel are ignored.
    /// Legacy unscoped finalization; use `finish_recognition_for` for asynchronous recognition.
    pub fn finish_recognition(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        if self.phase != DictationPhase::Transcribing || !self.pending || !self.awaiting_final {
            return;
        }
        self.awaiting_final = false;
        self.final_text = text.into();
        self.interim = "".into();
        if self.anchor_changed {
            self.set_phase(DictationPhase::ConfirmInsert, cx);
        } else {
            self.emit_insert(InsertionTarget::Original, cx);
        }
    }
    fn request(&mut self, event: DictationEvent, cx: &mut Context<Self>) {
        if event == DictationEvent::Cancel && self.phase != DictationPhase::Idle {
            self.phase = DictationPhase::Idle;
            self.session_active = false;
            self.pending = false;
            self.anchor_changed = false;
            self.awaiting_final = false;
            self.insertion_pending = false;
            self.final_text = "".into();
            self.interim = "".into();
            self.width.duration(36., 0.24, cx);
            cx.emit(event);
            cx.notify();
            return;
        }
        if self.pending {
            return;
        }
        match (&event, self.phase) {
            (DictationEvent::Start, DictationPhase::Idle)
            | (DictationEvent::Retry, DictationPhase::Error) => {
                self.session_id = self.session_id.wrapping_add(1);
                self.session_active = true;
                self.set_phase(DictationPhase::Requesting, cx);
                self.pending = true;
            }
            (DictationEvent::Finalize, DictationPhase::Listening) => {
                self.set_phase(DictationPhase::Transcribing, cx);
                self.pending = true;
                self.awaiting_final = true;
            }
            (DictationEvent::Insert { target, .. }, DictationPhase::ConfirmInsert) => {
                self.set_phase(DictationPhase::Transcribing, cx);
                self.emit_insert(*target, cx);
                return;
            }
            _ => return,
        }
        cx.emit(event);
        cx.notify();
    }
}
impl Render for DictationButton {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let r = crate::foundation::surface_radius(cx);
        let width = self.width.get(w, cx);

        let mut root = div()
            .id("dictation-root")
            .role(Role::Group)
            .aria_label(tr(cx, "Dictation", "语音输入"))
            .flex()
            .flex_col()
            .gap_2()
            .items_start()
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, _, cx| {
                if e.keystroke.key == "escape" && s.phase != DictationPhase::Idle {
                    s.request(DictationEvent::Cancel, cx);
                    cx.stop_propagation();
                }
            }));
        if !self.final_text.is_empty() || !self.interim.is_empty() {
            root = root.child(
                div()
                    .text_color(p.ink)
                    .child(self.final_text.clone())
                    .child(div().text_color(p.secondary).child(self.interim.clone())),
            );
        }
        if self.phase == DictationPhase::ConfirmInsert {
            root = root.child(div().text_color(p.secondary).child(tr(
                cx,
                "Insertion position changed. Choose where to insert.",
                "插入位置已改变，请选择插入位置。",
            )));
        }
        if self.phase == DictationPhase::Error
            || (self.phase == DictationPhase::ConfirmInsert && !self.error.is_empty())
        {
            root = root.child(
                div()
                    .id("dictation-error")
                    .role(Role::Alert)
                    .text_color(p.danger)
                    .child(self.error.clone()),
            );
        }
        let mut pill = div()
            .w(px(width))
            .h(px(36.))
            .rounded(r)
            .flex()
            .items_center()
            .justify_between()
            .overflow_hidden();
        match self.phase {
            DictationPhase::Idle => {
                pill = pill.child(
                    face(
                        "dictation-start",
                        tr(cx, "Dictate", "开始听写"),
                        36.,
                        18.,
                        Tone::Clear,
                        cx,
                    )
                    .size(px(36.))
                    .rounded_full()
                    .p_0()
                    .text_color(p.secondary)
                    .hover(|s| s.bg(p.tray).text_color(p.ink))
                    .child(icon(Glyph::Comment, 16., p.secondary))
                    .on_click(cx.listener(|s, _, _, cx| s.request(DictationEvent::Start, cx))),
                );
            }
            DictationPhase::Listening => {
                let mut bars = div().flex().items_center().gap(px(2.)).h(px(20.));
                for i in 0..16 {
                    let v = (i as f32 * 1.8).sin().abs();
                    bars = bars.child(
                        div()
                            .w(px(3.))
                            .h(px((3. + v * self.level * 17.).min(20.)))
                            .rounded_full()
                            .bg(p.ink),
                    );
                }
                pill = pill
                    .px(px(4.))
                    .gap(px(8.))
                    .child(
                        face(
                            "dictation-cancel",
                            tr(cx, "Cancel", "取消"),
                            28.,
                            14.,
                            Tone::Clear,
                            cx,
                        )
                        .size(px(28.))
                        .rounded_full()
                        .p_0()
                        .text_color(p.secondary)
                        .hover(|s| s.bg(p.ink.opacity(0.08)).text_color(p.ink))
                        .child(icon(Glyph::Close, 14., p.secondary))
                        .on_click(cx.listener(|s, _, _, cx| s.request(DictationEvent::Cancel, cx))),
                    )
                    .child(bars)
                    .child(
                        div()
                            .w(px(36.))
                            .text_center()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(p.secondary)
                            .child(format!("{}:{:02}", self.elapsed / 60, self.elapsed % 60)),
                    )
                    .child(
                        face(
                            "dictation-insert",
                            tr(cx, "Insert", "插入"),
                            28.,
                            14.,
                            Tone::Primary,
                            cx,
                        )
                        .size(px(28.))
                        .rounded_full()
                        .p_0()
                        .bg(p.bright)
                        .hover(|s| s.bg(p.bright.blend(p.ink.opacity(0.12))))
                        .child(icon(Glyph::Check, 16., p.inverse))
                        .on_click(
                            cx.listener(|s, _, _, cx| s.request(DictationEvent::Finalize, cx)),
                        ),
                    );
            }
            DictationPhase::ConfirmInsert => {
                let original = self.final_text.clone();
                let current = self.final_text.clone();
                pill = pill
                    .child(
                        face(
                            "dictation-original",
                            tr(cx, "Original position", "原位置"),
                            36.,
                            r.into(),
                            Tone::Clear,
                            cx,
                        )
                        .px_2()
                        .child(tr(cx, "Original", "原位置"))
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.request(
                                DictationEvent::Insert {
                                    text: original.clone(),
                                    target: InsertionTarget::Original,
                                },
                                cx,
                            )
                        })),
                    )
                    .child(
                        face(
                            "dictation-current",
                            tr(cx, "Current position", "当前位置"),
                            36.,
                            r.into(),
                            Tone::Clear,
                            cx,
                        )
                        .px_2()
                        .child(tr(cx, "Current", "当前位置"))
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.request(
                                DictationEvent::Insert {
                                    text: current.clone(),
                                    target: InsertionTarget::Current,
                                },
                                cx,
                            )
                        })),
                    );
            }
            DictationPhase::Error => {
                pill = pill.child(
                    face(
                        "dictation-retry",
                        tr(cx, "Try again", "重试"),
                        36.,
                        r.into(),
                        Tone::Clear,
                        cx,
                    )
                    .h(px(36.))
                    .px(px(14.))
                    .gap(px(6.))
                    .text_size(px(14.))
                    .line_height(px(20.))
                    .font_weight(FontWeight::MEDIUM)
                    .hover(|s| s.bg(p.ink.opacity(0.06)))
                    .child(tr(cx, "Try again", "重试"))
                    .on_click(cx.listener(|s, _, _, cx| s.request(DictationEvent::Retry, cx))),
                );
            }
            DictationPhase::Requesting | DictationPhase::Transcribing => {
                pill = pill.child(div().px_3().text_color(p.secondary).child(
                    if self.phase == DictationPhase::Requesting {
                        tr(cx, "Starting…", "正在启动…")
                    } else {
                        tr(cx, "Inserting…", "正在插入…")
                    },
                ));
            }
        }
        root.child(pill)
    }
}

#[cfg(test)]
mod session_tests {
    use super::{DictationPhase, current_insertion, current_session, final_is_pending};
    #[test]
    fn cancelled_and_old_recognizer_callbacks_are_rejected() {
        assert!(current_session(true, 7, 7));
        assert!(!current_session(false, 7, 7));
        assert!(!current_session(true, 8, 7));
        assert!(current_session(true, 8, 8));
    }
    #[test]
    fn stale_insert_attempts_cannot_acknowledge_newer_attempts() {
        assert!(current_insertion(true, 2, 2, true, 4, 4));
        assert!(!current_insertion(true, 2, 2, true, 5, 4));
        assert!(!current_insertion(true, 2, 1, true, 4, 4));
        assert!(!current_insertion(true, 2, 2, false, 4, 4));
    }
    #[test]
    fn final_text_is_accepted_only_once_while_finalizing() {
        assert!(final_is_pending(DictationPhase::Transcribing, true, true));
        assert!(!final_is_pending(DictationPhase::Transcribing, true, false));
        assert!(!final_is_pending(
            DictationPhase::ConfirmInsert,
            false,
            false
        ));
        assert!(!final_is_pending(DictationPhase::Idle, false, false));
    }
}
