//! Host-driven inline completion, drawn in the same layout as the editable text.
use crate::primitives::{
    editor::{EditEvent, Editor, EditorCompletionEvent, InputMode},
    family::{Paint, Tone, face},
    nav::tr,
};
use gpui::{prelude::*, *};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GhostStatus {
    Idle,
    Waiting,
    Loading,
    Ready,
    Error,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GhostTextEvent {
    /// A host may abort work associated with this request ID.
    Cancel(u64),
    Request {
        id: u64,
        text: SharedString,
    },
    Changed(SharedString),
    Accepted {
        text: SharedString,
        word: bool,
    },
    Dismissed,
    /// A real focus or input interaction stopped the optional visual demo.
    PreviewStopped,
}
#[derive(Default)]
struct Generation {
    serial: u64,
    active: Option<u64>,
}
impl Generation {
    fn invalidate(&mut self) -> Option<u64> {
        self.serial = self.serial.wrapping_add(1);
        self.active.take()
    }
    fn start(&mut self) -> u64 {
        self.serial = self.serial.wrapping_add(1);
        self.active = Some(self.serial);
        self.serial
    }
    fn accepts(&self, id: u64, composing: bool) -> bool {
        !composing && self.active == Some(id)
    }
}
pub struct GhostText {
    editor: Entity<Editor>,
    label: SharedString,
    model: SharedString,
    previous: String,
    suggestion: String,
    request_value: String,
    status: GhostStatus,
    generation: Generation,
    focus: bool,
    action_pressed: bool,
    preview_allowed: bool,
    preview_active: bool,
    plain: bool,
    content_min_height: f32,
    delay: Duration,
    min_length: usize,
    timer: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<GhostTextEvent> for GhostText {}
impl GhostText {
    pub fn new(label: impl Into<SharedString>, w: &mut Window, cx: &mut Context<Self>) -> Self {
        let label = label.into();
        let help = tr(
            cx,
            "Tab accepts a suggestion, Command or Control plus Right accepts one word, and Escape dismisses it.",
            "按 Tab 接受建议，按 Command 或 Control 加右方向键逐词接受，按 Escape 取消建议。",
        );
        let editor = cx.new(|cx| {
            let mut editor = Editor::new(label.clone(), InputMode::Multi, w, cx);
            editor.description = help.into();
            editor
        });
        let text = cx.subscribe_in(
            &editor,
            w,
            |s: &mut Self, editor, event: &EditEvent, _, cx| match event {
                EditEvent::Changed => {
                    let current = editor.read(cx).value().to_string();
                    let composing = editor.read(cx).is_composing();
                    let unchanged = current == s.previous;
                    // A host may set the initial example and start its preview
                    // within one update, before the Changed event is delivered.
                    if s.focus || (s.preview_active && !unchanged) {
                        s.stop_preview(cx);
                    }
                    let added = current.strip_prefix(&s.previous).map(str::to_owned);
                    s.previous = current.clone();
                    cx.emit(GhostTextEvent::Changed(current.into()));
                    if composing {
                        s.clear(cx);
                        return;
                    }
                    if unchanged {
                        cx.notify();
                        return;
                    }
                    if let Some(added) = added
                        && !added.is_empty()
                        && s.suggestion.starts_with(&added)
                    {
                        // The request was made against the old source. Preserve the
                        // displayed remainder, but never append late chunks to new text.
                        if let Some(id) = s.generation.invalidate() {
                            cx.emit(GhostTextEvent::Cancel(id));
                        }
                        s.request_value.clear();
                        s.suggestion.drain(..added.len());
                        let suffix = s.suggestion.clone();
                        editor.update(cx, |e, cx| e.set_ghost(suffix, cx));
                        if s.suggestion.is_empty() {
                            s.status = GhostStatus::Idle;
                        }
                        cx.notify();
                        return;
                    }
                    s.clear(cx);
                    s.schedule(cx);
                }
                EditEvent::Focused => {
                    s.stop_preview(cx);
                    s.focus = true;
                    if s.status == GhostStatus::Idle {
                        s.schedule(cx);
                    }
                }
                EditEvent::Blurred => {
                    s.focus = false;
                    if !s.action_pressed && s.status != GhostStatus::Error {
                        s.clear_with_fade(cx, true);
                    }
                }
                _ => {}
            },
        );
        let selection = cx.subscribe_in(
            &editor,
            w,
            |s: &mut Self, editor, event: &EditorCompletionEvent, _, cx| match event {
                EditorCompletionEvent::SelectionChanged => {
                    s.clear(cx);
                    s.schedule(cx);
                }
                EditorCompletionEvent::CompositionEnded => {
                    s.clear(cx);
                    s.schedule(cx);
                }
                EditorCompletionEvent::Dismissed => {
                    s.clear_with_fade(cx, true);
                    cx.emit(GhostTextEvent::Dismissed);
                }
                EditorCompletionEvent::Accepted { text, word } => {
                    // Notifications may be delivered after multiple accepts or
                    // subsequent typing. Reconcile all text actually appended so
                    // far once; never replay individual chunks twice.
                    let current = editor.read(cx).value().to_owned();
                    let added = current.strip_prefix(&s.previous);
                    let matches = added.is_some_and(|added| s.suggestion.starts_with(added));
                    if matches {
                        let count = added.map_or(0, str::len);
                        s.suggestion.drain(..count);
                    } else {
                        s.suggestion.clear();
                    }
                    s.previous = current;
                    if let Some(id) = s.generation.invalidate() {
                        cx.emit(GhostTextEvent::Cancel(id));
                    }
                    s.request_value.clear();
                    let suffix = s.suggestion.clone();
                    editor.update(cx, |e, cx| e.set_ghost(suffix, cx));
                    s.status = if s.suggestion.is_empty() {
                        GhostStatus::Idle
                    } else {
                        GhostStatus::Ready
                    };
                    if !matches {
                        s.schedule(cx);
                    }
                    cx.emit(GhostTextEvent::Accepted {
                        text: text.clone(),
                        word: *word,
                    });
                    cx.notify();
                }
            },
        );
        Self {
            editor,
            label,
            model: "Autocomplete".into(),
            previous: String::new(),
            suggestion: String::new(),
            request_value: String::new(),
            status: GhostStatus::Idle,
            generation: Generation::default(),
            focus: false,
            action_pressed: false,
            preview_allowed: true,
            preview_active: false,
            plain: false,
            content_min_height: 84.,
            delay: Duration::from_millis(450),
            min_length: 3,
            timer: None,
            _subscriptions: vec![text, selection],
        }
    }
    /// Remove the outer card chrome and label when embedding in a host card.
    pub fn plain(mut self) -> Self {
        self.plain = true;
        self
    }
    /// Set the initial and minimum editor area height in logical pixels.
    pub fn min_height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.content_min_height = height.clamp(32., 2048.);
        }
        self
    }
    /// Set an explicitly simulated visual suffix without requesting a model,
    /// mutating the input, or adding it to the accessibility value. It can be
    /// shown only before real user interaction, and is not a real completion.
    pub fn preview_suggestion(
        &mut self,
        text: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.preview_allowed
            || self.focus
            || self.status != GhostStatus::Idle
            || self.generation.active.is_some()
        {
            return false;
        }
        let editor = self.editor.read(cx);
        if editor.is_composing()
            || editor.value().is_empty()
            || !editor.selection().is_empty()
            || editor.selection().end != editor.value().len()
        {
            return false;
        }
        self.previous = editor.value().to_owned();
        let text = text.into();
        self.preview_active = !text.is_empty();
        let preview_active = self.preview_active;
        self.editor.update(cx, |editor, cx| {
            editor.set_ghost_preview(preview_active, cx);
            editor.set_ghost(text, cx);
        });
        cx.notify();
        true
    }
    fn stop_preview(&mut self, cx: &mut Context<Self>) {
        if !self.preview_allowed {
            return;
        }
        self.preview_allowed = false;
        if self.preview_active {
            self.preview_active = false;
            self.editor.update(cx, |editor, cx| {
                editor.set_ghost_preview(false, cx);
                editor.set_ghost("", cx);
            });
        }
        cx.emit(GhostTextEvent::PreviewStopped);
    }
    pub fn model(mut self, label: impl Into<SharedString>) -> Self {
        self.model = label.into();
        self
    }
    pub fn placeholder(self, label: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        self.editor.update(cx, |e, _| e.set_placeholder(label));
        self
    }
    pub fn value(&self, cx: &App) -> String {
        self.editor.read(cx).value().to_owned()
    }
    pub fn set_value(&mut self, text: impl Into<String>, w: &Window, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |editor, cx| editor.set_value(text, w, cx));
    }
    pub fn status(&self) -> GhostStatus {
        self.status
    }
    fn clear(&mut self, cx: &mut Context<Self>) {
        self.clear_with_fade(cx, false);
    }
    fn clear_with_fade(&mut self, cx: &mut Context<Self>, fade: bool) {
        self.timer = None;
        if let Some(id) = self.generation.invalidate() {
            cx.emit(GhostTextEvent::Cancel(id));
        }
        self.suggestion.clear();
        self.request_value.clear();
        self.status = GhostStatus::Idle;
        let was_preview = self.preview_active;
        self.preview_active = false;
        self.editor.update(cx, |editor, cx| {
            editor.set_ghost_preview(false, cx);
            if fade && !was_preview {
                editor.dismiss_ghost(cx);
            } else {
                editor.set_ghost("", cx);
            }
        });
        cx.notify();
    }
    fn eligible(&self, cx: &App) -> bool {
        let editor = self.editor.read(cx);
        let selection = editor.selection();
        self.focus
            && !editor.is_composing()
            && selection.is_empty()
            && selection.end == editor.value().len()
            && editor.value().trim().chars().count() >= self.min_length
    }
    fn schedule(&mut self, cx: &mut Context<Self>) {
        if !self.eligible(cx) {
            return;
        }
        self.status = GhostStatus::Waiting;
        let delay = self.delay;
        self.timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update(cx, |s, cx| s.request(cx));
        }));
        cx.notify();
    }
    pub fn request(&mut self, cx: &mut Context<Self>) {
        // A manual retry supersedes any pending debounce; otherwise its timer
        // could start a second request and cancel the one just started.
        self.timer = None;
        if !self.eligible(cx) {
            return;
        }
        if let Some(id) = self.generation.invalidate() {
            cx.emit(GhostTextEvent::Cancel(id));
        }
        let id = self.generation.start();
        self.suggestion.clear();
        self.status = GhostStatus::Loading;
        self.request_value = self.editor.read(cx).value().to_owned();
        let text = self.request_value.clone().into();
        cx.emit(GhostTextEvent::Request { id, text });
        cx.notify();
    }
    /// Streaming chunks must carry the request ID; outdated or composing results are ignored.
    pub fn push_chunk(
        &mut self,
        id: u64,
        chunk: impl AsRef<str>,
        done: bool,
        cx: &mut Context<Self>,
    ) {
        if !self
            .generation
            .accepts(id, self.editor.read(cx).is_composing())
            || !self.eligible(cx)
            || self.editor.read(cx).value() != self.request_value
        {
            return;
        }
        let mut chunk = chunk.as_ref();
        if self.suggestion.is_empty() && self.editor.read(cx).value().ends_with(char::is_whitespace)
        {
            chunk = chunk.trim_start_matches(char::is_whitespace);
        }
        self.suggestion.push_str(chunk);
        let suffix = self.suggestion.clone();
        self.editor
            .update(cx, |editor, cx| editor.set_ghost(suffix, cx));
        self.status = if self.suggestion.is_empty() {
            GhostStatus::Loading
        } else {
            GhostStatus::Ready
        };
        if done {
            self.generation.active = None;
            if self.suggestion.is_empty() {
                self.status = GhostStatus::Idle;
            }
        }
        cx.notify();
    }
    pub fn set_error(&mut self, id: u64, cx: &mut Context<Self>) {
        if !self
            .generation
            .accepts(id, self.editor.read(cx).is_composing())
            || !self.eligible(cx)
            || self.editor.read(cx).value() != self.request_value
        {
            return;
        }
        self.generation.active = None;
        self.status = GhostStatus::Error;
        cx.notify();
    }
}
impl Render for GhostText {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Focus can change before deferred editor events reach the parent.
        // Retire a visual demo from the live focus state before drawing it.
        if self.preview_allowed && self.editor.read(cx).focus.is_focused(w) {
            self.stop_preview(cx);
            self.focus = true;
        }
        let p = Paint::get(cx);
        let r = crate::foundation::surface_radius(cx);
        let mut root = div()
            .id("ghost-text")
            .role(Role::Group)
            .aria_label(self.label.clone())
            .w_full()
            .max_w(px(680.))
            .when(!self.plain, |root| {
                root.rounded(r)
                    .border_1()
                    .border_color(if self.focus {
                        p.line.blend(p.ink.opacity(0.5))
                    } else {
                        p.line
                    })
                    .when(self.focus, |root| {
                        root.shadow(vec![gpui::BoxShadow {
                            color: p.bright.opacity(0.35),
                            offset: gpui::point(px(0.), px(0.)),
                            blur_radius: px(0.),
                            spread_radius: px(3.),
                            inset: false,
                        }])
                    })
                    .when(!self.focus, |root| {
                        root.hover(|s| s.border_color(p.line.blend(p.ink.opacity(0.22))))
                    })
                    .bg(p.panel)
            })
            .flex()
            .flex_col()
            .when(!self.plain, |root| {
                root.p_0().child(
                    div()
                        .pt(px(12.))
                        .px(px(16.))
                        .text_color(p.secondary)
                        .text_size(px(12.))
                        .line_height(px(20.))
                        .child(self.label.clone()),
                )
            })
            .child(
                div()
                    .h(px(self.content_min_height))
                    .min_h(px(self.content_min_height))
                    .w_full()
                    .pt(px(if self.plain { 0. } else { 14. }))
                    .px(px(if self.plain { 0. } else { 16. }))
                    .pb(px(if self.plain { 0. } else { 4. }))
                    .line_height(px(22.))
                    .child(self.editor.clone()),
            );
        let action = match self.status {
            GhostStatus::Loading => tr(cx, "Thinking…", "思考中…"),
            GhostStatus::Ready => tr(cx, "Tab to accept", "点击或按 Tab 接受"),
            GhostStatus::Error => tr(cx, "Retry", "重试"),
            _ => self.model.as_ref(),
        };
        let announcement = match self.status {
            GhostStatus::Ready => format!(
                "{} {} {}",
                tr(cx, "Suggestion:", "建议："),
                self.suggestion,
                tr(cx, "Press Tab to accept.", "按 Tab 接受。")
            ),
            GhostStatus::Loading => tr(cx, "Thinking", "正在生成建议").to_owned(),
            GhostStatus::Error => tr(cx, "Could not get a suggestion", "无法获取建议").to_owned(),
            _ => self.model.to_string(),
        };
        let mut footer = div()
            .flex()
            .justify_between()
            .items_center()
            .min_h(px(if self.plain { 0. } else { 30. }))
            .gap(px(10.))
            .when(!self.plain, |footer| {
                footer.pr(px(6.)).pb(px(6.)).pl(px(16.))
            })
            .child(
                div()
                    .id("ghost-status")
                    .role(Role::Status)
                    .aria_label(announcement)
                    .text_size(px(12.))
                    .text_color(p.secondary)
                    .child(self.model.clone()),
            );
        if matches!(self.status, GhostStatus::Ready | GhostStatus::Error) {
            let ready = self.status == GhostStatus::Ready;
            footer = footer.child(
                face("ghost-action", action.to_string(), 24., 8., Tone::Clear, cx)
                    .h(px(24.))
                    .px(px(9.))
                    .pl(px(6.))
                    .gap(px(6.))
                    .rounded(px(8.))
                    .text_size(px(12.))
                    .line_height(px(20.))
                    .text_color(p.secondary)
                    .hover(|s| s.bg(p.tray).text_color(p.ink))
                    .aria_label(if ready {
                        tr(cx, "Accept suggestion", "接受建议")
                    } else {
                        tr(cx, "Retry suggestion", "重试建议")
                    })
                    .tab_stop(!ready)
                    .px_2()
                    .child(action.to_string())
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|s, _, _, _| s.action_pressed = true),
                    )
                    .on_mouse_up_out(
                        MouseButton::Left,
                        cx.listener(|s, _, _, cx| {
                            s.action_pressed = false;
                            if !s.focus && s.status != GhostStatus::Error {
                                s.clear(cx);
                            }
                        }),
                    )
                    .on_click(cx.listener(move |s, _, w, cx| {
                        s.action_pressed = false;
                        if ready {
                            s.editor.update(cx, |e, cx| {
                                e.accept_ghost(false, w, cx);
                                e.focus(w, cx);
                            });
                        } else {
                            s.editor.update(cx, |e, cx| e.focus(w, cx));
                            s.focus = true;
                            s.request(cx);
                        }
                    })),
            );
        } else if self.status == GhostStatus::Loading {
            footer = footer.child(
                div()
                    .text_size(px(12.))
                    .text_color(p.secondary)
                    .child(action.to_string()),
            );
        }
        root = root.child(footer);
        root
    }
}
#[cfg(test)]
mod tests {
    use super::{Generation, GhostStatus, GhostText};
    use gpui::{EntityInputHandler, TestAppContext};
    #[test]
    fn stale_and_composing_responses_are_rejected() {
        let mut g = Generation::default();
        let first = g.start();
        assert!(!g.accepts(first, true));
        assert!(g.accepts(first, false));
        assert_eq!(g.invalidate(), Some(first));
        let second = g.start();
        assert!(!g.accepts(first, false));
        assert!(g.accepts(second, false));
    }
    #[test]
    fn visual_preview_never_changes_value_or_starts_a_request() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) =
            app.add_window_view(|w, cx| GhostText::new("Draft", w, cx).plain().min_height(140.));
        w.update(|w, cx| {
            let editor = view.read(cx).editor.clone();
            editor.update(cx, |e, cx| e.set_value("Existing", w, cx))
        });
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                assert!(s.preview_suggestion(" example", cx));
                assert_eq!(s.value(cx), "Existing");
                assert_eq!(s.status(), GhostStatus::Idle);
                assert!(s.generation.active.is_none());
                assert!(s.preview_active);
            })
        });
        w.update(|w, cx| {
            let editor = view.read(cx).editor.clone();
            editor.update(cx, |e, cx| {
                assert!(!e.accept_ghost(false, w, cx));
                e.focus(w, cx);
            })
        });
        w.update(|w, cx| w.simulate_next_frame(cx));
        w.run_until_parked();
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                assert_eq!(s.value(cx), "Existing");
                assert!(!s.preview_active);
                assert!(!s.preview_suggestion(" again", cx));
            })
        });
    }

    #[test]
    fn initial_host_text_and_preview_can_share_one_update() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|w, cx| GhostText::new("Draft", w, cx));
        w.update(|w, cx| {
            view.update(cx, |s, cx| {
                s.set_value("Example", w, cx);
                assert!(s.preview_suggestion(" completion", cx));
            })
        });
        w.update(|_, cx| {
            let s = view.read(cx);
            assert!(s.preview_active);
            assert!(s.preview_allowed);
            assert_eq!(s.value(cx), "Example");
        });
    }

    #[test]
    fn consecutive_word_accepts_keep_only_the_remaining_suffix() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|w, cx| GhostText::new("Draft", w, cx));
        w.update(|w, cx| {
            let editor = view.read(cx).editor.clone();
            editor.update(cx, |editor, cx| editor.set_value("Hi!", w, cx))
        });
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                s.focus = true;
                s.request(cx);
                let id = s.generation.active.unwrap();
                s.push_chunk(id, " first second third", false, cx);
            })
        });
        w.update(|w, cx| {
            let editor = view.read(cx).editor.clone();
            editor.update(cx, |editor, cx| {
                assert!(editor.accept_ghost(true, w, cx));
                assert!(editor.accept_ghost(true, w, cx));
            });
        });
        w.update(|_, cx| {
            let s = view.read(cx);
            assert_eq!(s.value(cx), "Hi! first second");
            assert_eq!(s.previous, "Hi! first second");
            assert_eq!(s.suggestion, " third");
            assert_eq!(s.status(), GhostStatus::Ready);
            assert!(s.generation.active.is_none());
        });
    }

    #[test]
    fn cancel_then_restart_rejects_previous_chunks() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|w, cx| GhostText::new("Draft", w, cx));
        w.update(|w, cx| {
            let editor = view.read(cx).editor.clone();
            editor.update(cx, |e, cx| e.set_value("abc", w, cx))
        });
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                s.focus = true;
                s.request(cx);
                let first = s.generation.active.unwrap();
                s.clear(cx);
                s.request(cx);
                let second = s.generation.active.unwrap();
                assert_ne!(first, second);
                s.push_chunk(first, " stale", false, cx);
                assert!(s.suggestion.is_empty());
                s.push_chunk(second, " fresh", true, cx);
                assert_eq!(s.suggestion, " fresh");
            })
        });
    }

    #[test]
    fn changed_input_and_ime_reject_old_stream_chunks() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(|w, cx| GhostText::new("Draft", w, cx));
        w.update(|w, cx| {
            let editor = view.read(cx).editor.clone();
            editor.update(cx, |e, cx| e.set_value("abc", w, cx))
        });
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                s.focus = true;
                s.request(cx);
            })
        });
        let old_id = w.update(|_, cx| view.read(cx).generation.active.unwrap());
        w.update(|w, cx| {
            view.update(cx, |s, cx| {
                s.editor.update(cx, |e, cx| e.set_value("abcd", w, cx));
                // The request is stale even before the Changed event is delivered.
                s.push_chunk(old_id, " stale", false, cx);
                assert!(s.suggestion.is_empty());
            })
        });
        w.update(|_, cx| view.update(cx, |s, cx| s.request(cx)));
        let new_id = w.update(|_, cx| view.read(cx).generation.active.unwrap());
        w.update(|w, cx| {
            let editor = view.read(cx).editor.clone();
            editor.update(cx, |e, cx| {
                e.focus(w, cx);
                e.replace_and_mark_text_in_range(None, "ni", Some(2..2), w, cx);
            });
        });
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                s.push_chunk(new_id, " stale", true, cx);
                assert!(s.suggestion.is_empty());
            })
        });
    }
}
