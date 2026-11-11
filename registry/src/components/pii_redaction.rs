//! Review masked messages and restore model replies using local values.
use crate::{
    components::textarea::{Textarea, TextareaStyle},
    primitives::{
        brand_mark::brand_mark,
        clipboard,
        control::UiLanguage,
        editor::{EditEvent, EditorShortcutEvent},
        family::{Paint, PhaseClock, Slide, Tone, face},
        field::{Field, FieldEvent},
        form_chrome::check_stroke,
        glyph::{Glyph, icon},
    },
};
use gpui::{prelude::*, *};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, OnceLock},
};
use unicode_segmentation::UnicodeSegmentation;

mod reply {
    use std::collections::HashMap;

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub enum PiiReplyPhase {
        #[default]
        Idle,
        Sending,
        Streaming,
        Complete,
        Failed,
        Stopped,
    }

    #[derive(Default)]
    pub(super) struct Reply {
        phase: PiiReplyPhase,
        active: Option<(u64, HashMap<String, String>)>,
        text: String,
        error: String,
    }

    impl Reply {
        pub(super) fn active_request(&self) -> Option<u64> {
            self.active.as_ref().map(|(id, _)| *id)
        }

        pub(super) fn phase(&self) -> PiiReplyPhase {
            self.phase
        }

        pub(super) fn text(&self) -> &str {
            &self.text
        }

        pub(super) fn error(&self) -> &str {
            &self.error
        }

        /// Opens a reply request and returns the only payload the model receives.
        /// Ranges and replacement tokens come from the approved redaction snapshot.
        pub(super) fn begin_masked(
            &mut self,
            id: u64,
            source: &str,
            redactions: impl IntoIterator<Item = (std::ops::Range<usize>, String)>,
        ) -> Option<String> {
            if self.active.is_some() {
                return None;
            }

            let mut masked = String::with_capacity(source.len());
            let mut values = HashMap::new();
            let mut previous = 0;
            for (range, token) in redactions {
                if range.start < previous || range.is_empty() {
                    return None;
                }
                let original = source.get(range.clone())?;
                masked.push_str(source.get(previous..range.start)?);
                masked.push_str(&token);
                match values.entry(token) {
                    std::collections::hash_map::Entry::Occupied(entry)
                        if entry.get() != original =>
                    {
                        return None;
                    }
                    std::collections::hash_map::Entry::Occupied(_) => {}
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        entry.insert(original.to_owned());
                    }
                }
                previous = range.end;
            }
            masked.push_str(&source[previous..]);
            self.begin(id, values).then_some(masked)
        }

        pub(super) fn begin(&mut self, id: u64, values: HashMap<String, String>) -> bool {
            if self.active.is_some() {
                return false;
            }
            self.active = Some((id, values));
            self.text.clear();
            self.error.clear();
            self.phase = PiiReplyPhase::Sending;
            true
        }

        pub(super) fn append(&mut self, id: u64, chunk: &str) -> bool {
            if !self.matches(id) {
                return false;
            }
            if !chunk.is_empty() {
                self.text.push_str(chunk);
                self.phase = PiiReplyPhase::Streaming;
            }
            true
        }

        pub(super) fn finish(&mut self, id: u64) -> bool {
            if !self.matches(id) {
                return false;
            }
            let (_, values) = self.active.take().expect("the request ID was checked");
            self.text = restore(&self.text, &values);
            self.phase = PiiReplyPhase::Complete;
            true
        }

        pub(super) fn fail(&mut self, id: u64, error: String) -> bool {
            if !self.matches(id) {
                return false;
            }
            self.active = None;
            self.text.clear();
            self.error = error;
            self.phase = PiiReplyPhase::Failed;
            true
        }

        /// Removes any values and partial or completed reply when the request is stopped.
        pub(super) fn clear(&mut self) -> Option<u64> {
            let id = self.active.take().map(|(id, _)| id);
            self.text.clear();
            self.error.clear();
            self.phase = if id.is_some() {
                PiiReplyPhase::Stopped
            } else {
                PiiReplyPhase::Idle
            };
            id
        }

        fn matches(&self, id: u64) -> bool {
            self.active
                .as_ref()
                .is_some_and(|(active, _)| *active == id)
        }
    }

    fn restore(reply: &str, values: &HashMap<String, String>) -> String {
        // Generated tokens have at most five letters and a usize counter.
        const MAX_TOKEN_BYTES: usize = 32;
        let mut restored = String::with_capacity(reply.len());
        let mut written = 0;
        for (start, _) in reply.match_indices('[') {
            if start < written {
                continue;
            }
            let Some(offset) = reply.as_bytes()[start..]
                .iter()
                .take(MAX_TOKEN_BYTES)
                .position(|&b| b == b']')
            else {
                continue;
            };
            let end = start + offset + 1;
            if let Some(original) = values.get(&reply[start..end]) {
                restored.push_str(&reply[written..start]);
                restored.push_str(original);
                written = end;
            }
        }
        restored.push_str(&reply[written..]);
        restored
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn host_payload_contains_only_masked_text_and_reuses_matching_tokens() {
            let source = "从 周宁 发送给 周宁";
            let redactions = source
                .match_indices("周宁")
                .map(|(start, name)| (start..start + name.len(), "[NAME_1]".to_owned()));
            let mut reply = Reply::default();
            let payload = reply.begin_masked(1, source, redactions).unwrap();
            assert_eq!(payload, "从 [NAME_1] 发送给 [NAME_1]");
            assert!(!payload.contains("周宁"));
            assert!(reply.append(1, "你好，[NAME_1]！"));
            assert!(reply.finish(1));
            assert_eq!(reply.text(), "你好，周宁！");
        }

        #[test]
        fn invalid_snapshot_cannot_start_a_request() {
            let mut reply = Reply::default();
            assert!(
                reply
                    .begin_masked(1, "周宁", [(1..3, "[NAME_1]".to_owned())],)
                    .is_none()
            );
            assert!(
                reply
                    .begin_masked(
                        2,
                        "Amy Bob",
                        [(0..3, "[NAME_1]".to_owned()), (4..7, "[NAME_1]".to_owned())],
                    )
                    .is_none()
            );
            assert_eq!(reply.phase(), PiiReplyPhase::Idle);
        }

        #[test]
        fn restores_split_tokens_only_on_completion_and_only_for_this_request() {
            let values = HashMap::from([
                ("[NAME_1]".to_owned(), "周宁".to_owned()),
                ("[EMAIL_1]".to_owned(), "zhou@example.test".to_owned()),
            ]);
            let mut reply = Reply::default();
            assert_eq!(reply.phase(), PiiReplyPhase::Idle);
            assert!(reply.begin(7, values));
            assert_eq!(reply.phase(), PiiReplyPhase::Sending);
            assert!(!reply.begin(8, HashMap::new()));
            assert!(reply.append(7, "你好，[NA"));
            assert!(reply.append(7, "ME_1]。[EMAIL_1] [EMAIL_9] [NAME_1]"));
            assert_eq!(reply.phase(), PiiReplyPhase::Streaming);
            assert_eq!(reply.text(), "你好，[NAME_1]。[EMAIL_1] [EMAIL_9] [NAME_1]");
            assert!(!reply.finish(8));
            assert!(reply.finish(7));
            assert_eq!(reply.phase(), PiiReplyPhase::Complete);
            assert_eq!(reply.text(), "你好，周宁。zhou@example.test [EMAIL_9] 周宁");
            assert!(!reply.finish(7));
        }

        #[test]
        fn invalidation_and_retry_reject_old_chunks_and_drop_original_values() {
            let mut reply = Reply::default();
            let values = HashMap::from([("[NAME_1]".to_owned(), "Priya".to_owned())]);
            assert!(reply.begin(1, values));
            assert!(reply.append(1, "Hello [NAME_1]"));
            assert_eq!(reply.clear(), Some(1));
            assert!(reply.text().is_empty());
            assert_eq!(reply.phase(), PiiReplyPhase::Stopped);
            assert!(reply.begin(2, HashMap::new()));
            assert!(!reply.fail(1, "stale".to_owned()));
            assert!(!reply.append(1, "stale"));
            assert!(!reply.finish(1));
            assert!(reply.fail(2, "sample failure".to_owned()));
            assert_eq!(reply.error(), "sample failure");
            assert!(reply.begin(3, HashMap::new()));
            assert!(reply.error().is_empty());
            assert!(reply.append(3, "[NAME_1]"));
            assert!(reply.finish(3));
            assert_eq!(reply.text(), "[NAME_1]");
        }

        #[test]
        fn restoration_does_not_interpret_substituted_values_as_tokens() {
            let values = HashMap::from([
                ("[NAME_1]".to_owned(), "[EMAIL_1]".to_owned()),
                ("[EMAIL_1]".to_owned(), "masked@example.test".to_owned()),
            ]);
            assert_eq!(
                restore("[NAME_1] [EMAIL_1] [[[OPEN]", &values),
                "[EMAIL_1] masked@example.test [[[OPEN]"
            );
        }
    }
}

pub use reply::PiiReplyPhase;

#[derive(Clone)]
pub enum PiiReplyEvent {
    Requested { id: u64, prompt: SharedString },
    Cancelled(u64),
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum PiiKind {
    Name,
    Email,
    Phone,
    Card,
    Other,
}
impl PiiKind {
    fn token(self) -> &'static str {
        match self {
            Self::Name => "NAME",
            Self::Email => "EMAIL",
            Self::Phone => "PHONE",
            Self::Card => "CARD",
            Self::Other => "OTHER",
        }
    }
    fn tint(self) -> Hsla {
        rgb(match self {
            Self::Name => 0x2b77b3,
            Self::Email => 0x8d54b4,
            Self::Phone => 0xb58329,
            Self::Card => 0x16836e,
            Self::Other => 0x6b7280,
        })
        .into()
    }

    fn label(self, lang: UiLanguage) -> &'static str {
        match self {
            Self::Name => lang.text("Name", "姓名"),
            Self::Email => lang.text("Email", "邮箱"),
            Self::Phone => lang.text("Phone", "电话"),
            Self::Card => lang.text("Card", "卡号"),
            Self::Other => lang.text("Other", "其他"),
        }
    }
}
// Lucide phone and at-sign artwork, ISC license.
// https://github.com/lucide-icons/lucide/tree/main/icons
fn kind_mark(kind: PiiKind) -> AnyElement {
    let (cache, body, stroke) = match kind {
        PiiKind::Email => {
            static EMAIL: OnceLock<Arc<Image>> = OnceLock::new();
            (
                &EMAIL,
                r#"<circle cx="12" cy="12" r="4"/><path d="M16 8v5a3 3 0 0 0 6 0v-1a10 10 0 1 0-4 8"/>"#,
                "#8d54b4",
            )
        }
        PiiKind::Phone => {
            static PHONE: OnceLock<Arc<Image>> = OnceLock::new();
            (
                &PHONE,
                r#"<path d="M13.832 16.568a1 1 0 0 0 1.213-.303l.355-.465A2 2 0 0 1 17 15h3a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2A18 18 0 0 1 2 4a2 2 0 0 1 2-2h3a2 2 0 0 1 2 2v3a2 2 0 0 1-.8 1.6l-.468.351a1 1 0 0 0-.292 1.233 14 14 0 0 0 6.392 6.384"/>"#,
                "#b58329",
            )
        }
        _ => {
            return icon(
                match kind {
                    PiiKind::Name => Glyph::User,
                    PiiKind::Card => Glyph::Card,
                    _ => Glyph::Lock,
                },
                14.,
                kind.tint(),
            )
            .into_any_element();
        }
    };
    let image = cache.get_or_init(|| {
        Arc::new(Image::from_bytes(
            ImageFormat::Svg,
            format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="{stroke}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{body}</svg>"#).into_bytes(),
        ))
    });
    img(image.clone())
        .size(px(14.))
        .flex_none()
        .into_any_element()
}
/// A half-open UTF-8 byte range in the exact text submitted to the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PiiSpan {
    pub start: usize,
    pub end: usize,
    pub kind: PiiKind,
}
impl PiiSpan {
    pub fn new(start: usize, end: usize, kind: PiiKind) -> Self {
        Self { start, end, kind }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PiiStatus {
    #[default]
    Idle,
    Scanning,
    Ready,
    Stopped,
    Failed,
}
#[derive(Clone, Debug)]
pub enum PiiEvent {
    ScanRequested { id: u64, text: SharedString },
    Cancelled(u64),
}
fn valid_spans(text: &str, spans: &mut [PiiSpan]) -> bool {
    spans.sort_by_key(|span| (span.start, span.end));
    let boundaries: Vec<usize> = text
        .grapheme_indices(true)
        .map(|(index, _)| index)
        .chain(std::iter::once(text.len()))
        .collect();
    let mut end = 0;
    for span in spans {
        if span.start < end
            || span.start >= span.end
            || span.end > text.len()
            || boundaries.binary_search(&span.start).is_err()
            || boundaries.binary_search(&span.end).is_err()
        {
            return false;
        }
        end = span.end;
    }
    true
}
fn redaction_tokens(text: &str, spans: &[PiiSpan]) -> Option<Vec<(PiiSpan, String)>> {
    let mut sorted = spans.to_vec();
    if !valid_spans(text, &mut sorted) {
        return None;
    }
    let mut counts: HashMap<PiiKind, usize> = HashMap::new();
    let mut tokens: HashMap<(PiiKind, &str), String> = HashMap::new();
    Some(
        sorted
            .into_iter()
            .map(|span| {
                let original = &text[span.start..span.end];
                let token = tokens.entry((span.kind, original)).or_insert_with(|| {
                    let count = counts.entry(span.kind).or_insert(0);
                    loop {
                        *count += 1;
                        let candidate = format!("[{}_{}]", span.kind.token(), count);
                        if !text.contains(&candidate) {
                            break candidate;
                        }
                    }
                });
                (span, token.clone())
            })
            .collect(),
    )
}

fn mask_text(text: &str, spans: &[PiiSpan]) -> Option<String> {
    let mut output = String::new();
    let mut previous = 0;
    for (span, token) in redaction_tokens(text, spans)? {
        output.push_str(&text[previous..span.start]);
        output.push_str(&token);
        previous = span.end;
    }
    output.push_str(&text[previous..]);
    Some(output)
}
/// Does not detect, transmit or send any content. The host supplies every scan result.
pub struct PiiRedaction {
    source: Entity<Textarea>,
    spans: Vec<PiiSpan>,
    painted: Rc<RefCell<Vec<PiiSpan>>>,
    snapshot: SharedString,
    model: Option<SharedString>,
    status: PiiStatus,
    active: Option<u64>,
    sequence: u64,
    reviewed: bool,
    selected_kind: PiiKind,
    history: Vec<Vec<PiiSpan>>,
    error: SharedString,
    copied: Option<bool>,
    copy_epoch: u64,
    action_focus: FocusHandle,
    reply_enabled: bool,
    reply: reply::Reply,
    reply_clock: PhaseClock,
    reply_before_restore: SharedString,
    show_original: bool,
    original_thumb: Slide,
    _sub: Subscription,
    _shortcuts: Subscription,
    _escape: Subscription,
}
impl EventEmitter<PiiEvent> for PiiRedaction {}
impl EventEmitter<PiiReplyEvent> for PiiRedaction {}
impl PiiRedaction {
    pub fn new(w: &mut Window, cx: &mut Context<Self>) -> Self {
        let l = UiLanguage::current(cx);
        let painted: Rc<RefCell<Vec<PiiSpan>>> = Rc::new(RefCell::new(Vec::new()));
        let marks = painted.clone();
        let source = cx.new(|cx| {
            Field::<TextareaStyle>::new("pii-source", l.text("Message", "原始文本"), w, cx)
        });
        source.read(cx).editor().update(cx, |editor, _| {
            editor.set_run_styler(Rc::new(move |text, base| {
                let mut spans = marks.borrow().clone();
                if !valid_spans(text, &mut spans) {
                    return vec![base.clone()];
                }
                let mut runs = Vec::new();
                let mut at = 0;
                for span in spans {
                    if span.start > at {
                        let mut run = base.clone();
                        run.len = span.start - at;
                        runs.push(run);
                    }
                    let mut marked = base.clone();
                    marked.len = span.end - span.start;
                    marked.background_color = Some(span.kind.tint().opacity(0.20));
                    runs.push(marked);
                    at = span.end;
                }
                if at < text.len() || runs.is_empty() {
                    let mut run = base.clone();
                    run.len = text.len() - at;
                    runs.push(run);
                }
                runs
            }));
        });
        let sub = cx.subscribe_in(&source, w, |s: &mut Self, _, event: &FieldEvent, _, cx| {
            if let FieldEvent::Changed(value) = event
                && value != &s.snapshot
            {
                s.invalidate(cx);
            }
        });
        let editor = source.read(cx).editor();
        editor.update(cx, |editor, _| editor.set_command_submit(true));
        let shortcuts = cx.subscribe_in(
            &editor,
            w,
            |s: &mut Self, _, _: &EditorShortcutEvent, w, cx| {
                if s.active.is_none() && s.reply.active_request().is_none() && s.scan(cx) {
                    s.action_focus.focus(w, cx);
                }
            },
        );
        let escape = cx.subscribe(&editor, |s: &mut Self, _, event: &EditEvent, cx| {
            if matches!(event, EditEvent::Escape) {
                s.stop(cx);
            }
        });
        Self {
            source,
            spans: Vec::new(),
            painted,
            snapshot: "".into(),
            model: None,
            status: PiiStatus::Idle,
            active: None,
            sequence: 0,
            reviewed: false,
            selected_kind: PiiKind::Other,
            history: Vec::new(),
            error: "".into(),
            copied: None,
            copy_epoch: 0,
            action_focus: cx.focus_handle().tab_stop(true),
            reply_enabled: false,
            reply: reply::Reply::default(),
            reply_clock: PhaseClock::default(),
            reply_before_restore: "".into(),
            show_original: false,
            original_thumb: Slide::new(0.),
            _sub: sub,
            _shortcuts: shortcuts,
            _escape: escape,
        }
    }
    /// Name the host-selected scan model in the status row.
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = Some(model.into());
        self
    }
    /// Enables the opt-in masked model reply workflow. Register a reply event subscriber first.
    pub fn with_reply(mut self) -> Self {
        self.reply_enabled = true;
        self
    }

    pub fn reply_phase(&self) -> PiiReplyPhase {
        self.reply.phase()
    }

    /// The visible reply. It contains placeholders until completion.
    pub fn reply_text(&self) -> &str {
        self.reply.text()
    }

    /// Starts a host request using only the approved masked snapshot.
    pub fn request_reply(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.reply_enabled
            || !self.reviewed
            || self.status != PiiStatus::Ready
            || self.composing(cx)
            || self.source_text(cx) != self.snapshot.as_ref()
        {
            return false;
        }
        let Some(redactions) = redaction_tokens(&self.snapshot, &self.spans) else {
            return false;
        };
        let id = self.sequence.wrapping_add(1).max(1);
        let Some(prompt) = self.reply.begin_masked(
            id,
            &self.snapshot,
            redactions
                .into_iter()
                .map(|(span, token)| (span.start..span.end, token)),
        ) else {
            return false;
        };
        self.sequence = id;
        self.reply_before_restore = "".into();
        self.reply_clock.restart(cx);
        cx.emit(PiiReplyEvent::Requested {
            id,
            prompt: prompt.into(),
        });
        cx.notify();
        true
    }

    /// Appends a model response chunk with placeholders intact.
    pub fn append_reply(&mut self, id: u64, chunk: &str, cx: &mut Context<Self>) -> bool {
        let was_sending = self.reply.phase() == PiiReplyPhase::Sending;
        let accepted = self.reply_is_current(id, cx) && self.reply.append(id, chunk);
        if accepted {
            if was_sending && self.reply.phase() == PiiReplyPhase::Streaming {
                self.reply_clock.restart(cx);
            }
            cx.notify();
        }
        accepted
    }

    /// Finishes the response and restores placeholders with this request's local values.
    pub fn finish_reply(&mut self, id: u64, cx: &mut Context<Self>) -> bool {
        if !self.reply_is_current(id, cx) {
            return false;
        }
        let previous = self.reply.text().to_owned();
        let accepted = self.reply.finish(id);
        if accepted {
            self.reply_before_restore = previous.into();
            self.reply_clock.restart(cx);
            cx.notify();
        }
        accepted
    }

    pub fn fail_reply(
        &mut self,
        id: u64,
        error: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        let accepted =
            self.reply_is_current(id, cx) && self.reply.fail(id, error.into().to_string());
        if accepted {
            cx.notify();
        }
        accepted
    }

    fn reply_is_current(&mut self, id: u64, cx: &mut Context<Self>) -> bool {
        if self.reply.active_request() != Some(id) {
            return false;
        }
        if !self.reviewed || self.composing(cx) || self.source_text(cx) != self.snapshot.as_ref() {
            self.clear_reply(cx);
            cx.notify();
            return false;
        }
        true
    }

    fn clear_reply(&mut self, cx: &mut Context<Self>) {
        self.reply_before_restore = "".into();
        if let Some(id) = self.reply.clear() {
            cx.emit(PiiReplyEvent::Cancelled(id));
        }
    }

    pub fn source(&self) -> Entity<Textarea> {
        self.source.clone()
    }
    pub fn status(&self) -> PiiStatus {
        self.status
    }
    pub fn spans(&self) -> &[PiiSpan] {
        &self.spans
    }
    pub fn active_request(&self) -> Option<u64> {
        self.active
    }
    fn source_text(&self, cx: &App) -> String {
        self.source.read(cx).value(cx).to_owned()
    }
    fn composing(&self, cx: &App) -> bool {
        self.source.read(cx).editor().read(cx).is_composing()
    }
    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.clear_reply(cx);
        self.reply = reply::Reply::default();
        if let Some(id) = self.active.take() {
            cx.emit(PiiEvent::Cancelled(id));
        }
    }
    fn repaint_source(&mut self, cx: &mut Context<Self>) {
        *self.painted.borrow_mut() = self.spans.clone();
        self.source
            .read(cx)
            .editor()
            .update(cx, |_, cx| cx.notify());
    }
    fn invalidate(&mut self, cx: &mut Context<Self>) {
        self.cancel(cx);
        self.show_original = false;
        self.original_thumb.reset(0.);
        self.spans.clear();
        self.history.clear();
        self.snapshot = "".into();
        self.status = PiiStatus::Idle;
        self.reviewed = false;
        self.copied = None;
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        self.error = "".into();
        self.repaint_source(cx);
        cx.notify();
    }
    /// Explicitly asks the host to scan the current text. It is never scanned or transmitted automatically.
    pub fn scan(&mut self, cx: &mut Context<Self>) -> bool {
        if self.composing(cx) {
            return false;
        }
        let text: SharedString = self.source_text(cx).into();
        if text.trim().is_empty() {
            self.error = UiLanguage::current(cx)
                .text("Enter text first", "请先输入文本")
                .into();
            cx.notify();
            return false;
        }
        self.cancel(cx);
        if self.snapshot != text {
            self.spans.clear();
            self.history.clear();
        }
        self.snapshot = text.clone();
        self.reviewed = false;
        self.copied = None;
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        self.error = "".into();
        self.sequence = self.sequence.wrapping_add(1).max(1);
        self.active = Some(self.sequence);
        self.status = PiiStatus::Scanning;
        self.repaint_source(cx);
        cx.emit(PiiEvent::ScanRequested {
            id: self.sequence,
            text,
        });
        cx.notify();
        true
    }
    pub fn stop(&mut self, cx: &mut Context<Self>) {
        if self.reply.active_request().is_some() {
            self.clear_reply(cx);
            cx.notify();
            return;
        }
        if self.active.is_some() {
            self.cancel(cx);
            self.status = PiiStatus::Stopped;
            self.reviewed = false;
            cx.notify();
        }
    }
    /// Reject the complete result if any range is overlapping, invalid or splits a Unicode grapheme.
    pub fn finish_scan(
        &mut self,
        id: u64,
        mut spans: Vec<PiiSpan>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.active != Some(id) || self.status != PiiStatus::Scanning {
            return false;
        }
        if self.composing(cx) || self.source_text(cx) != self.snapshot.as_ref() {
            self.invalidate(cx);
            return false;
        }
        self.active = None;
        if !valid_spans(self.snapshot.as_ref(), &mut spans) {
            self.status = PiiStatus::Failed;
            self.reviewed = false;
            self.error = UiLanguage::current(cx)
                .text(
                    "Invalid or overlapping ranges; nothing was applied.",
                    "识别范围无效或重叠，未应用结果。",
                )
                .into();
            cx.notify();
            return true;
        }
        self.history.push(self.spans.clone());
        self.spans = spans;
        self.status = PiiStatus::Ready;
        self.reviewed = false;
        self.repaint_source(cx);
        cx.notify();
        true
    }
    pub fn fail_scan(
        &mut self,
        id: u64,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.active != Some(id) {
            return false;
        }
        if self.composing(cx) || self.source_text(cx) != self.snapshot.as_ref() {
            self.invalidate(cx);
            return false;
        }
        self.active = None;
        self.status = PiiStatus::Failed;
        self.reviewed = false;
        self.error = message.into();
        cx.notify();
        true
    }
    /// Add a user-selected source range. It must not overlap an existing range.
    pub fn add_range(&mut self, span: PiiSpan, cx: &mut Context<Self>) -> bool {
        if self.composing(cx) {
            return false;
        }
        let current: SharedString = self.source_text(cx).into();
        if current.is_empty() {
            return false;
        }
        if self.snapshot != current {
            // A Changed event may not have reached us yet. Invalidate approval
            // before validating the selection, including on the failure path.
            self.invalidate(cx);
            self.snapshot = current;
        }
        let mut next = self.spans.clone();
        next.push(span);
        if !valid_spans(self.snapshot.as_ref(), &mut next) {
            self.error = UiLanguage::current(cx)
                .text(
                    "Select a non-overlapping text range.",
                    "请选择不重叠的有效文本范围。",
                )
                .into();
            cx.notify();
            return false;
        }
        self.cancel(cx);
        self.history.push(self.spans.clone());
        self.spans = next;
        self.status = PiiStatus::Ready;
        self.reviewed = false;
        self.error = "".into();
        self.copied = None;
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        self.repaint_source(cx);
        cx.notify();
        true
    }
    pub fn add_selection(&mut self, kind: PiiKind, cx: &mut Context<Self>) -> bool {
        if self.composing(cx) {
            return false;
        }
        let range = self.source.read(cx).editor().read(cx).selection();
        self.add_range(PiiSpan::new(range.start, range.end, kind), cx)
    }
    pub fn remove_range(&mut self, index: usize, cx: &mut Context<Self>) -> bool {
        if self.source_text(cx) != self.snapshot.as_ref()
            || index >= self.spans.len()
            || self.composing(cx)
        {
            return false;
        }
        self.cancel(cx);
        self.history.push(self.spans.clone());
        self.spans.remove(index);
        self.status = PiiStatus::Ready;
        self.reviewed = false;
        self.copied = None;
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        self.repaint_source(cx);
        cx.notify();
        true
    }
    pub fn undo(&mut self, cx: &mut Context<Self>) -> bool {
        if self.source_text(cx) != self.snapshot.as_ref() || self.composing(cx) {
            return false;
        }
        let Some(previous) = self.history.pop() else {
            return false;
        };
        self.cancel(cx);
        self.spans = previous;
        self.status = PiiStatus::Ready;
        self.reviewed = false;
        self.copied = None;
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        self.repaint_source(cx);
        cx.notify();
        true
    }
    /// A preview is available only for the exact text against which ranges were checked.
    pub fn preview(&self, cx: &App) -> Option<String> {
        if self.status != PiiStatus::Ready
            || self.composing(cx)
            || self.source_text(cx) != self.snapshot.as_ref()
        {
            return None;
        }
        mask_text(self.snapshot.as_ref(), &self.spans)
    }
    pub fn set_reviewed(&mut self, reviewed: bool, cx: &mut Context<Self>) -> bool {
        if self.preview(cx).is_none() {
            return false;
        }
        self.reviewed = reviewed;
        if !reviewed {
            self.clear_reply(cx);
            self.reply = reply::Reply::default();
        }
        self.copied = None;
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        cx.notify();
        true
    }
    /// Copy is enabled only after the reader explicitly reviews the current ranges.
    pub fn copy_preview(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.reviewed {
            return false;
        }
        let Some(text) = self.preview(cx) else {
            return false;
        };
        self.copy_epoch = self.copy_epoch.wrapping_add(1);
        let epoch = self.copy_epoch;
        let task = clipboard::write(&text, cx);
        cx.spawn(async move |this, cx| {
            let ok = task.await;
            let _ = this.update(cx, |s, cx| {
                if s.copy_epoch == epoch {
                    s.copied = Some(ok);
                    cx.notify();
                }
            });
            cx.background_executor()
                .timer(std::time::Duration::from_millis(1800))
                .await;
            let _ = this.update(cx, |s, cx| {
                if s.copy_epoch == epoch {
                    s.copied = None;
                    cx.notify();
                }
            });
        })
        .detach();
        true
    }
}
impl Render for PiiRedaction {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let l = UiLanguage::current(cx);
        let radius = crate::foundation::surface_radius(cx);
        let t = |en: &'static str, zh: &'static str| l.text(en, zh);
        let replying = self.reply.active_request().is_some();
        let can_send = self.reply_enabled && self.status == PiiStatus::Ready && self.reviewed;
        let status = match self.reply.phase() {
            PiiReplyPhase::Sending => t("Sending masked message", "正在发送脱敏文本"),
            PiiReplyPhase::Streaming => t("Receiving reply", "正在接收回复"),
            PiiReplyPhase::Complete => t("Reply restored", "回复已恢复"),
            PiiReplyPhase::Failed => t("Reply failed", "回复失败"),
            PiiReplyPhase::Stopped => t("Reply stopped", "已停止回复"),
            PiiReplyPhase::Idle => match self.status {
                PiiStatus::Idle => t("Ready to scan", "等待识别"),
                PiiStatus::Scanning => t("Scanning", "正在识别"),
                PiiStatus::Ready if can_send => t("Ready to send", "可以发送脱敏文本"),
                PiiStatus::Ready if self.reply_enabled => {
                    t("Review the masked message", "核对脱敏文本")
                }
                PiiStatus::Ready => t("Review all ranges before copying", "复制前请核对全部范围"),
                PiiStatus::Stopped => t("Stopped", "已停止"),
                PiiStatus::Failed => t("Scan failed", "识别失败"),
            },
        };
        let busy = self.active.is_some() || replying;
        let label = if busy {
            t("Stop", "停止")
        } else if can_send {
            match self.reply.phase() {
                PiiReplyPhase::Complete => t("Run again", "再试一次"),
                PiiReplyPhase::Failed | PiiReplyPhase::Stopped => t("Try again", "重试"),
                _ => t("Send masked message", "发送脱敏文本"),
            }
        } else if matches!(self.status, PiiStatus::Failed | PiiStatus::Stopped) {
            t("Try again", "重试")
        } else {
            t("Scan", "识别")
        };
        let mut actions = div().flex().flex_wrap().gap(px(8.));
        if self.reply_enabled && self.status != PiiStatus::Idle {
            actions = actions.child(
                face(
                    "pii-reset",
                    t("Reset", "重置"),
                    32.,
                    radius.into(),
                    Tone::Clear,
                    cx,
                )
                .px(px(8.))
                .child(t("Reset", "重置"))
                .on_click(cx.listener(|s, _, w, cx| {
                    s.invalidate(cx);
                    s.source.read(cx).focus_handle(cx).focus(w, cx);
                })),
            );
        }
        actions = actions.child(
            face(
                "pii-action",
                label,
                32.,
                radius.into(),
                if busy { Tone::Surface } else { Tone::Primary },
                cx,
            )
            .track_focus(&self.action_focus)
            .px(px(14.))
            .child(label)
            .on_click(cx.listener(move |s, _, _, cx| {
                if s.active.is_some() || s.reply.active_request().is_some() {
                    s.stop(cx);
                } else if can_send {
                    s.request_reply(cx);
                } else {
                    s.scan(cx);
                }
            })),
        );
        let masked = self.preview(cx);
        let show_masked = self.reply_enabled && !self.show_original && masked.is_some();
        let thumb_position = self.original_thumb.get(w, cx).clamp(0., 1.);
        let message = if self.reply_enabled {
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .child(t("Message", "消息")),
                        )
                        .child(
                            face(
                                "pii-show-original",
                                t("Show original", "显示原文"),
                                28.,
                                radius.into(),
                                Tone::Clear,
                                cx,
                            )
                            .px(px(8.))
                            .text_size(px(12.))
                            .aria_toggled(self.show_original.into())
                            .when(masked.is_none(), |button| {
                                button.opacity(0.45).tab_index(-1)
                            })
                            .child(t("Show original", "显示原文"))
                            .child(
                                div()
                                    .w(px(32.))
                                    .h(px(18.))
                                    .rounded_full()
                                    .relative()
                                    .bg(if self.show_original { p.bright } else { p.line })
                                    .child(
                                        div()
                                            .absolute()
                                            .left(px(2. + 14. * thumb_position))
                                            .top(px(2.))
                                            .size(px(14.))
                                            .rounded_full()
                                            .bg(if self.show_original {
                                                p.inverse
                                            } else {
                                                p.panel
                                            }),
                                    ),
                            )
                            .on_click(cx.listener(|s, _, _, cx| {
                                if s.preview(cx).is_some() {
                                    s.show_original = !s.show_original;
                                    s.original_thumb.duration(
                                        if s.show_original { 1. } else { 0. },
                                        0.16,
                                        cx,
                                    );
                                    cx.notify();
                                }
                            })),
                        ),
                )
                .when(show_masked, |panel| {
                    panel.child(
                        div()
                            .min_h(px(110.))
                            .rounded(radius)
                            .border_1()
                            .border_color(p.line)
                            .bg(p.stage)
                            .px(px(12.))
                            .py(px(10.))
                            .text_size(px(14.))
                            .line_height(px(22.))
                            .text_color(p.ink)
                            .child(masked.clone().unwrap_or_default()),
                    )
                })
                .when(!show_masked, |panel| {
                    panel.child(
                        div()
                            .h(px(110.))
                            .rounded(radius)
                            .bg(p.stage)
                            .px(px(12.))
                            .py(px(10.))
                            .text_size(px(14.))
                            .line_height(px(22.))
                            .child(self.source.read(cx).editor()),
                    )
                })
                .into_any_element()
        } else {
            self.source.clone().into_any_element()
        };
        let model_label = self.model.as_deref().unwrap_or(t("Model", "模型"));
        let can_edit_ranges = !self.reply_enabled;
        let redactions = redaction_tokens(&self.snapshot, &self.spans).unwrap_or_default();
        let mut kinds = div().flex().flex_wrap().gap(px(8.)).mt(px(4.));
        for (i, kind) in [
            PiiKind::Name,
            PiiKind::Email,
            PiiKind::Phone,
            PiiKind::Card,
            PiiKind::Other,
        ]
        .into_iter()
        .enumerate()
        {
            kinds = kinds.child(
                face(
                    ("pii-kind", i),
                    kind.label(l),
                    28.,
                    radius.into(),
                    Tone::Clear,
                    cx,
                )
                .px(px(2.))
                .text_size(px(14.))
                .line_height(px(20.))
                .aria_toggled((self.selected_kind == kind).into())
                .when(self.selected_kind == kind, |v| {
                    v.text_color(p.ink).font_weight(FontWeight::MEDIUM)
                })
                .when(self.selected_kind != kind, |v| {
                    v.text_color(p.secondary).hover(|s| s.text_color(p.ink))
                })
                .child(kind.label(l))
                .on_click(cx.listener(move |s, _, _, cx| {
                    s.selected_kind = kind;
                    cx.notify();
                })),
            );
        }
        let mut list = div()
            .id("pii-ranges")
            .role(Role::List)
            .aria_label(t("Redaction ranges", "脱敏范围"))
            .flex()
            .flex_wrap()
            .gap(px(8.));
        for (i, (span, token)) in redactions.iter().enumerate() {
            let label = format!("{} · {token}", span.kind.label(l));
            let remove = format!("{} {}", t("Remove", "移除"), label);
            list = list.child(
                div()
                    .id(("pii-range", i))
                    .role(Role::ListItem)
                    .flex()
                    .items_center()
                    .justify_between()
                    .min_w(px(140.))
                    .flex_1()
                    .p(px(6.))
                    .rounded(radius)
                    .bg(p.tray)
                    .min_h(px(28.))
                    .gap(px(12.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .line_height(px(20.))
                            .text_color(p.secondary)
                            .child(label),
                    )
                    .child(
                        face(
                            ("pii-remove", i),
                            remove,
                            32.,
                            radius.into(),
                            Tone::Clear,
                            cx,
                        )
                        .px(px(8.))
                        .child(t("Remove", "移除"))
                        .on_click(cx.listener(move |s, _, _, cx| {
                            s.remove_range(i, cx);
                        })),
                    ),
            );
        }
        let mut root = div()
            .id("pii-redaction")
            .role(Role::Group)
            .aria_label(t("Review redaction", "核对脱敏范围"))
            .w_full()
            .max_w(px(640.))
            .rounded(radius)
            .border_1()
            .border_color(p.line)
            .bg(p.panel)
            .p(px(16.))
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(message)
            .when(can_edit_ranges, |root| root.child(kinds))
            .when(can_edit_ranges, |root| {
                root.child(
                    face(
                        "pii-add",
                        t("Mask selected text", "脱敏所选文本"),
                        36.,
                        radius.into(),
                        Tone::Surface,
                        cx,
                    )
                    .px(px(10.))
                    .child(t("Mask selected text", "脱敏所选文本"))
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.add_selection(s.selected_kind, cx);
                    })),
                )
            })
            .when(can_edit_ranges, |root| root.child(list))
            .when(can_edit_ranges && !self.history.is_empty(), |root| {
                root.child(
                    face(
                        "pii-undo",
                        t("Undo range change", "撤销范围修改"),
                        32.,
                        radius.into(),
                        Tone::Clear,
                        cx,
                    )
                    .px(px(8.))
                    .child(t("Undo", "撤销"))
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.undo(cx);
                    })),
                )
            });
        let footer = div()
            .id("pii-status")
            .role(Role::Status)
            .min_h(px(32.))
            .pt(px(16.))
            .border_t_1()
            .border_color(p.line)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.))
            .text_size(px(14.))
            .line_height(px(22.))
            .text_color(p.secondary)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(7.))
                    .children(
                        self.model
                            .as_ref()
                            .filter(|_| !self.reply_enabled)
                            .and_then(|model| brand_mark(model, 14., cx)),
                    )
                    .child(status),
            )
            .child(actions);
        if self.reply_enabled {
            let cells = [PiiKind::Name, PiiKind::Email, PiiKind::Phone, PiiKind::Card]
                .into_iter()
                .chain(
                    self.spans
                        .iter()
                        .any(|span| span.kind == PiiKind::Other)
                        .then_some(PiiKind::Other),
                );
            root = root.child(div().flex().flex_wrap().children(cells.map(|kind| {
                let tokens = redactions
                    .iter()
                    .filter(|(span, _)| span.kind == kind)
                    .map(|(_, token)| token.as_str())
                    .collect::<Vec<_>>()
                    .join(" · ");
                let value = if tokens.is_empty() {
                    match self.status {
                        PiiStatus::Scanning => t("Checking", "正在检查"),
                        PiiStatus::Ready => t("None found", "未发现"),
                        _ => t("Not scanned", "尚未识别"),
                    }
                    .to_owned()
                } else {
                    tokens
                };
                div()
                    .w(relative(0.5))
                    .min_w_0()
                    .pb(px(6.))
                    .flex()
                    .items_start()
                    .gap(px(8.))
                    .child(kind_mark(kind))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(13.))
                            .line_height(px(18.))
                            .child(kind.label(l))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(p.secondary)
                                    .child(value),
                            ),
                    )
            })));
        }
        if !self.error.is_empty() {
            root = root.child(
                div()
                    .id("pii-error")
                    .role(Role::Alert)
                    .text_size(px(14.))
                    .line_height(px(22.))
                    .text_color(p.danger)
                    .child(self.error.clone()),
            );
        }
        if let Some(preview) = self.preview(cx) {
            root = root
                .when(!self.reply_enabled, |root| {
                    root.child(
                        div()
                            .min_h(px(90.))
                            .rounded(radius)
                            .bg(p.tray)
                            .px(px(14.))
                            .py(px(12.))
                            .text_size(px(14.))
                            .line_height(px(22.))
                            .text_color(p.ink)
                            .child(
                                div()
                                    .mb(px(6.))
                                    .text_size(px(12.))
                                    .text_color(p.secondary)
                                    .child(t("Masked preview", "脱敏预览")),
                            )
                            .child(preview),
                    )
                })
                .child(
                    face(
                        "pii-reviewed",
                        t("I reviewed these ranges", "我已核对脱敏范围"),
                        36.,
                        radius.into(),
                        Tone::Clear,
                        cx,
                    )
                    .px(px(8.))
                    .gap(px(8.))
                    .aria_toggled(self.reviewed.into())
                    .child(
                        div()
                            .size(px(16.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(if self.reviewed { p.bright } else { p.faint })
                            .bg(if self.reviewed { p.bright } else { p.panel })
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(check_stroke(
                                if self.reviewed { 1. } else { 0. },
                                0.,
                                12.,
                                p.inverse,
                            )),
                    )
                    .child(if self.reviewed {
                        t("Reviewed", "已核对")
                    } else {
                        t("I reviewed these ranges", "我已核对脱敏范围")
                    })
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.set_reviewed(!s.reviewed, cx);
                    })),
                );
            let label = match self.copied {
                Some(true) => t("Copied", "已复制"),
                Some(false) => t("Copy failed", "复制失败"),
                None => t("Copy masked text", "复制脱敏文本"),
            };
            let button = face("pii-copy", label, 36., radius.into(), Tone::Surface, cx)
                .px(px(10.))
                .child(label);
            root = root.child(if self.reviewed {
                button.on_click(cx.listener(|s, _, _, cx| {
                    s.copy_preview(cx);
                }))
            } else {
                button.opacity(0.45).tab_index(-1)
            });
        }
        if self.reply_enabled {
            let restore_age = if self.reply.phase() == PiiReplyPhase::Complete {
                self.reply_clock.time(0.26, w, cx)
            } else {
                f32::INFINITY
            };
            let reply_alpha = if restore_age < 0.10 {
                1. - restore_age / 0.10
            } else if restore_age < 0.26 {
                (restore_age - 0.10) / 0.16
            } else {
                1.
            };
            let text = match self.reply.phase() {
                PiiReplyPhase::Idle | PiiReplyPhase::Stopped => t(
                    "The reply will appear here with your details restored.",
                    "回复会显示在这里，并在本地恢复原始信息。",
                ),
                PiiReplyPhase::Sending => t("Waiting for the model", "正在等待模型"),
                PiiReplyPhase::Failed => self.reply.error(),
                PiiReplyPhase::Complete if restore_age < 0.10 => &self.reply_before_restore,
                _ => self.reply.text(),
            };
            let flow = if replying {
                self.reply_clock.time(0.46, w, cx) / 0.46
            } else {
                0.
            };
            let connector = |active: bool, reverse: bool| {
                let position = if reverse { 1. - flow } else { flow };
                div()
                    .flex_1()
                    .min_w(px(8.))
                    .h(px(1.))
                    .relative()
                    .bg(p.line)
                    .when(active, |line| {
                        line.child(
                            div()
                                .absolute()
                                .top(px(-2.))
                                .left(relative(position.clamp(0., 1.)))
                                .size(px(5.))
                                .rounded_full()
                                .bg(p.ink),
                        )
                    })
            };
            let reply_heading = match l {
                UiLanguage::Chinese => format!("{model_label} 的回复"),
                _ => format!("Reply from {model_label}"),
            };
            root = root
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .text_size(px(12.))
                        .text_color(p.secondary)
                        .child(icon(Glyph::User, 15., p.secondary))
                        .child(t("You", "你"))
                        .child(connector(
                            self.reply.phase() == PiiReplyPhase::Streaming,
                            true,
                        ))
                        .child(icon(Glyph::Lock, 15., p.secondary))
                        .child(t("PII guard", "隐私防护"))
                        .child(connector(
                            replying,
                            self.reply.phase() == PiiReplyPhase::Streaming,
                        ))
                        .children(
                            self.model
                                .as_ref()
                                .and_then(|model| brand_mark(model, 16., cx)),
                        )
                        .child(div().min_w_0().truncate().child(model_label.to_owned())),
                )
                .child(
                    div()
                        .id("pii-reply")
                        .role(Role::Group)
                        .aria_label(t("Model reply", "模型回复"))
                        .min_h(px(134.))
                        .rounded(radius)
                        .border_1()
                        .border_color(p.line)
                        .bg(p.stage)
                        .px(px(12.))
                        .py(px(10.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(7.))
                                .text_size(px(12.))
                                .text_color(p.secondary)
                                .children(
                                    self.model
                                        .as_ref()
                                        .and_then(|model| brand_mark(model, 16., cx)),
                                )
                                .child(reply_heading),
                        )
                        .child(
                            div()
                                .mt(px(6.))
                                .text_size(px(14.))
                                .line_height(px(22.))
                                .text_color(if self.reply.phase() == PiiReplyPhase::Failed {
                                    p.danger
                                } else {
                                    p.ink
                                })
                                .opacity(reply_alpha)
                                .child(text.to_owned()),
                        ),
                );
        }
        root.child(footer)
    }
}

#[cfg(test)]
mod tests {
    use super::{PiiKind, PiiRedaction, PiiSpan, mask_text};
    use gpui::TestAppContext;

    fn span(text: &str, needle: &str, kind: PiiKind) -> PiiSpan {
        let start = text.find(needle).unwrap();
        PiiSpan::new(start, start + needle.len(), kind)
    }

    #[test]
    fn model_reply_requires_review_and_cancels_on_source_edit() {
        use super::{PiiReplyEvent, PiiReplyPhase};
        use std::{cell::RefCell, rc::Rc};

        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (control, window) = app.add_window_view(|w, cx| PiiRedaction::new(w, cx).with_reply());
        let events = Rc::new(RefCell::new(Vec::new()));
        let captured = events.clone();
        let _subscription = window.update(|_, cx| {
            cx.subscribe(&control, move |_, event: &PiiReplyEvent, _| {
                captured.borrow_mut().push(event.clone())
            })
        });
        window.update(|w, cx| {
            let source = control.read(cx).source();
            source.update(cx, |field, cx| field.set_value("Name: 张三", w, cx));
            control.update(cx, |s, cx| {
                assert!(s.add_range(span("Name: 张三", "张三", PiiKind::Name), cx));
                assert!(!s.request_reply(cx));
                assert!(s.set_reviewed(true, cx));
                assert!(s.request_reply(cx));
            });
        });
        let id = match events.borrow().first().cloned().expect("request emitted") {
            PiiReplyEvent::Requested { id, prompt } => {
                assert_eq!(prompt.as_ref(), "Name: [NAME_1]");
                id
            }
            PiiReplyEvent::Cancelled(_) => panic!("first event must be the request"),
        };
        window.update(|_, cx| {
            control.update(cx, |s, cx| {
                assert!(s.append_reply(id, "Hi [NAME_1]", cx));
                assert_eq!(s.reply_text(), "Hi [NAME_1]");
                assert!(s.finish_reply(id, cx));
                assert_eq!(s.reply_text(), "Hi 张三");
                assert_eq!(s.reply_phase(), PiiReplyPhase::Complete);
                assert!(s.request_reply(cx));
            });
        });
        let pending = match events
            .borrow()
            .last()
            .cloned()
            .expect("second request emitted")
        {
            PiiReplyEvent::Requested { id, .. } => id,
            PiiReplyEvent::Cancelled(_) => panic!("last event must be the second request"),
        };
        window.update(|w, cx| {
            control
                .read(cx)
                .source()
                .update(cx, |field, cx| field.set_value("Name: 李四", w, cx));
            control.update(cx, |s, cx| {
                assert!(!s.append_reply(pending, "late", cx));
                assert!(!s.finish_reply(pending, cx));
                assert!(s.reply_text().is_empty());
            });
        });
        assert!(
            matches!(events.borrow().last(), Some(PiiReplyEvent::Cancelled(id)) if *id == pending)
        );
    }

    #[test]
    fn replaces_unicode_ranges_and_reuses_non_colliding_tokens() {
        let text = "[EMAIL_1] 张三 lin@example.com 和 lin@example.com";
        let name = span(text, "张三", PiiKind::Name);
        let first = span(text, "lin@example.com", PiiKind::Email);
        let last_start = text.rfind("lin@example.com").unwrap();
        let last = PiiSpan::new(
            last_start,
            last_start + "lin@example.com".len(),
            PiiKind::Email,
        );
        assert_eq!(
            mask_text(text, &[last, name, first]).unwrap(),
            "[EMAIL_1] [NAME_1] [EMAIL_2] 和 [EMAIL_2]"
        );
    }

    #[test]
    fn rejects_overlapping_or_split_grapheme_ranges() {
        let text = "e\u{301}张三";
        assert!(mask_text(text, &[PiiSpan::new(0, 1, PiiKind::Other)]).is_none());
        assert!(mask_text(text, &[PiiSpan::new(1, 2, PiiKind::Other)]).is_none());
        assert!(mask_text(text, &[PiiSpan::new(0, text.len() + 1, PiiKind::Other)]).is_none());
        let name = span(text, "张三", PiiKind::Name);
        assert!(mask_text(text, &[name.clone(), name]).is_none());
    }

    #[test]
    fn rejects_stale_results_and_invalidates_review_after_edit() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(PiiRedaction::new);
        w.update(|w, cx| {
            let source = view.read(cx).source();
            source.update(cx, |field, cx| field.set_value("Name: 张三", w, cx));
        });
        w.update(|_, cx| view.update(cx, |s, cx| assert!(s.scan(cx))));
        let id = w.update(|_, cx| view.read(cx).active_request().unwrap());
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                let name = span("Name: 张三", "张三", PiiKind::Name);
                assert!(s.finish_scan(id, vec![name], cx));
                assert_eq!(s.preview(cx).as_deref(), Some("Name: [NAME_1]"));
                assert!(s.set_reviewed(true, cx));
            })
        });
        w.update(|w, cx| {
            let source = view.read(cx).source();
            source.update(cx, |field, cx| field.set_value("Name: 李四", w, cx));
        });
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                assert!(s.preview(cx).is_none());
                assert!(!s.copy_preview(cx));
                assert!(!s.finish_scan(id, vec![], cx));
            })
        });
    }

    #[test]
    fn invalid_manual_selection_never_keeps_prior_approval_for_changed_text() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(PiiRedaction::new);
        w.update(|w, cx| {
            view.read(cx)
                .source()
                .update(cx, |field, cx| field.set_value("张三", w, cx))
        });
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                assert!(s.add_range(PiiSpan::new(0, "张三".len(), PiiKind::Name), cx));
                assert!(s.set_reviewed(true, cx));
            })
        });
        w.update(|w, cx| {
            view.read(cx)
                .source()
                .update(cx, |field, cx| field.set_value("李四", w, cx));
            view.update(cx, |s, cx| {
                assert!(!s.add_range(PiiSpan::new(0, 0, PiiKind::Name), cx));
                assert!(s.preview(cx).is_none());
                assert!(!s.copy_preview(cx));
            });
        });
    }

    #[test]
    fn blocks_scanning_during_ime_composition() {
        use gpui::EntityInputHandler;
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (view, w) = app.add_window_view(PiiRedaction::new);
        let editor = w.update(|_, cx| view.read(cx).source().read(cx).editor());
        w.update(|w, cx| {
            editor.update(cx, |editor, cx| {
                editor.focus(w, cx);
                editor.replace_and_mark_text_in_range(None, "ni", Some(2..2), w, cx);
            })
        });
        w.update(|_, cx| {
            view.update(cx, |s, cx| {
                assert!(!s.scan(cx));
                assert!(!s.add_selection(PiiKind::Name, cx));
                assert!(s.preview(cx).is_none());
            })
        });
    }
}
