//! Inline AI prose diff viewer with word-level accept/reject, gliding popover, keyboard navigation, and undo.
//!
//! Computes word-level diffs (`diff_words`) or accepts precomputed [`DiffSegment`]s, rendering
//! struck red deletions beside green insertions inside a natural paragraph with `Tab` / `←` / `→`
//! navigation, `Enter` / `Y` accept, `Backspace` / `N` reject, `⌘Z` undo, and smooth collapse.

use crate::{
    components::pro_chrome::{
        ProBtnStyle, ProPalette, pro_btn, pro_card_shadow, pro_check_draw, pro_ease, pro_eid,
        pro_empty_state, pro_icon_btn, pro_popover_presence, pro_rolling_number, pro_skeleton_bar,
        pro_spring_f32, pro_tactile, pro_toast_banner, pro_view_transition, tr,
    },
    foundation::Theme,
    primitives::{
        control::UiLanguage,
        family::{PhaseClock, disabled_motion},
        pro_icons::{ProIcon, pro_icon, pro_svg},
    },
};
use gpui::{prelude::*, *};
use std::{cell::Cell, rc::Rc};
use web_time::Instant;

const TWIN_SPARK_SVG: &str = r#"<path d="M8 1.75c.3 2.9 1.35 3.95 4.25 4.25-2.9.3-3.95 1.35-4.25 4.25C7.7 7.35 6.65 6.3 3.75 6 6.65 5.7 7.7 4.65 8 1.75Z" fill="currentColor"></path><path d="M12.5 10.25c.15 1.25.6 1.7 1.75 1.75-1.15.15-1.6.6-1.75 1.75-.15-1.15-.6-1.6-1.75-1.75 1.15-.05 1.6-.5 1.75-1.75Z" fill="currentColor" opacity="0.7"></path>"#;

/// Resolution state of an individual [`DiffSegment::Change`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiffDecision {
    /// Awaiting user acceptance or rejection.
    #[default]
    Pending,
    /// Edit accepted (deletion collapses away, insertion settles into prose).
    Accepted,
    /// Edit rejected (insertion collapses away, original words settle back).
    Rejected,
}

/// Unchanged or changed prose run rendered inside [`InlineDiff`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffSegment {
    /// Unchanged text preserved verbatim.
    Equal(SharedString),
    /// Grouped word deletion and/or insertion at one point in the prose.
    Change {
        /// Original words to strike and remove if accepted (may be empty for pure insertions).
        remove: SharedString,
        /// Suggested replacement words to keep if accepted (may be empty for pure deletions).
        insert: SharedString,
    },
}

/// Compute a word-level diff between `original` and `suggested`, grouping adjacent edits into single [`DiffSegment::Change`] runs.
pub fn diff_words(original: &str, suggested: &str) -> Vec<DiffSegment> {
    let orig_toks = tokenize_prose(original);
    let sugg_toks = tokenize_prose(suggested);
    let n = orig_toks.len();
    let m = sugg_toks.len();

    // Dynamic programming LCS table over word/punctuation tokens
    let mut dp = vec![0u32; (n + 1) * (m + 1)];
    let stride = m + 1;
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i * stride + j] = if orig_toks[i] == sugg_toks[j] {
                dp[(i + 1) * stride + (j + 1)] + 1
            } else {
                dp[(i + 1) * stride + j].max(dp[i * stride + (j + 1)])
            };
        }
    }

    #[derive(PartialEq, Eq)]
    enum Op<'a> {
        Eq(&'a str),
        Del(&'a str),
        Ins(&'a str),
    }

    let mut ops = Vec::with_capacity(n + m);
    let (mut i, mut j) = (0usize, 0usize);
    while i < n && j < m {
        if orig_toks[i] == sugg_toks[j] {
            ops.push(Op::Eq(orig_toks[i]));
            i += 1;
            j += 1;
        } else if dp[(i + 1) * stride + j] >= dp[i * stride + (j + 1)] {
            ops.push(Op::Del(orig_toks[i]));
            i += 1;
        } else {
            ops.push(Op::Ins(sugg_toks[j]));
            j += 1;
        }
    }
    while i < n {
        ops.push(Op::Del(orig_toks[i]));
        i += 1;
    }
    while j < m {
        ops.push(Op::Ins(sugg_toks[j]));
        j += 1;
    }

    // Coalesce contiguous Eq runs and contiguous Del/Ins runs
    let mut segments = Vec::new();
    let mut eq_buf: Vec<&str> = Vec::new();
    let mut del_buf: Vec<&str> = Vec::new();
    let mut ins_buf: Vec<&str> = Vec::new();

    let flush_change =
        |del_b: &mut Vec<&str>, ins_b: &mut Vec<&str>, out: &mut Vec<DiffSegment>| {
            if !del_b.is_empty() || !ins_b.is_empty() {
                out.push(DiffSegment::Change {
                    remove: SharedString::from(join_prose_tokens(del_b)),
                    insert: SharedString::from(join_prose_tokens(ins_b)),
                });
                del_b.clear();
                ins_b.clear();
            }
        };
    let flush_eq = |eq_b: &mut Vec<&str>, out: &mut Vec<DiffSegment>| {
        if !eq_b.is_empty() {
            out.push(DiffSegment::Equal(SharedString::from(join_prose_tokens(
                eq_b,
            ))));
            eq_b.clear();
        }
    };

    for op in ops {
        match op {
            Op::Eq(tok) => {
                flush_change(&mut del_buf, &mut ins_buf, &mut segments);
                eq_buf.push(tok);
            }
            Op::Del(tok) => {
                flush_eq(&mut eq_buf, &mut segments);
                del_buf.push(tok);
            }
            Op::Ins(tok) => {
                flush_eq(&mut eq_buf, &mut segments);
                ins_buf.push(tok);
            }
        }
    }
    flush_change(&mut del_buf, &mut ins_buf, &mut segments);
    flush_eq(&mut eq_buf, &mut segments);
    segments
}

fn tokenize_prose(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for raw in s.split_whitespace() {
        // Split trailing punctuation (`.`, `,`, `!`, `?`, `;`, `:`) as separate tokens so word swaps stay crisp
        let trimmed = raw.trim_end_matches(['.', ',', '!', '?', ';', ':']);
        if trimmed.is_empty() {
            out.push(raw);
        } else {
            out.push(trimmed);
            let punct = &raw[trimmed.len()..];
            if !punct.is_empty() {
                out.push(punct);
            }
        }
    }
    out
}

fn is_punct_token(tok: &str) -> bool {
    !tok.is_empty()
        && tok
            .chars()
            .all(|c| matches!(c, '.' | ',' | '!' | '?' | ';' | ':'))
}

fn join_prose_tokens(toks: &[&str]) -> String {
    let mut out = String::new();
    for &t in toks {
        if !out.is_empty() && !is_punct_token(t) {
            out.push(' ');
        }
        out.push_str(t);
    }
    out
}

/// Typed events emitted by [`InlineDiff`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InlineDiffEvent {
    /// Emitted when an individual change is accepted or rejected.
    Decided {
        /// 0-based index among [`DiffSegment::Change`] entries.
        change_index: usize,
        /// Accepted or rejected verdict.
        decision: DiffDecision,
        /// Current resulting draft string (undecided changes retain original text).
        current_text: SharedString,
    },
    /// Emitted when `Accept all` or `Reject all` resolves every remaining pending edit.
    BulkDecided {
        /// Bulk verdict (`Accepted` or `Rejected`).
        decision: DiffDecision,
        /// Resulting text.
        current_text: SharedString,
    },
    /// Emitted when `Undo` (`⌘Z` / `Z`) reverts the previous action.
    Undone {
        /// Resulting text after undo.
        current_text: SharedString,
    },
    /// Emitted once the final pending change is resolved.
    Done(SharedString),
    /// Emitted when all changes are reset back to `Pending`.
    Reset,
}

impl EventEmitter<InlineDiffEvent> for InlineDiff {}

#[derive(Clone)]
struct ChangeSlot {
    remove: SharedString,
    insert: SharedString,
    decision: DiffDecision,
    decided_at: Option<Instant>,
    accept_clock: PhaseClock,
}

/// Inline AI prose edit reviewer with per-change popover, keyboard shortcuts, and undo stack.
///
/// # Examples
///
/// ```no_run
/// # #[allow(unused_variables, unused_imports)]
/// # fn example(window: &mut gpui::Window, cx: &mut gpui::App) {
/// # use gpui::prelude::*;
/// use gpui::prelude::*;
/// use kui::components::inline_diff::{InlineDiff, InlineDiffEvent};
///
/// let review = cx.new(|cx| {
///     InlineDiff::new("prose-inline-diff", cx)
///         .with_label(Some("Suggested by AI · Make it shorter".into()))
/// });
/// cx.subscribe(&review, |_, event: &InlineDiffEvent, _| {
///     let _ = event;
/// }).detach();
/// # }
/// ```
pub struct InlineDiff {
    id: ElementId,
    focus: FocusHandle,
    label: Option<SharedString>,
    segments: Vec<DiffSegment>,
    changes: Vec<ChangeSlot>,
    history: Vec<Vec<(usize, DiffDecision)>>,
    active_change: Option<usize>,
    hovered_change: Option<usize>,
    popover_hovered: bool,
    keyboard_focused: bool,
    done_at: Option<Instant>,
    done_clock: PhaseClock,
    card_origin: Rc<Cell<Point<Pixels>>>,
    #[allow(clippy::type_complexity)]
    active_anchor: Rc<Cell<Option<(Point<Pixels>, Size<Pixels>)>>>,
    loading: bool,
    empty: bool,
    measured_width: Rc<Cell<f32>>,
    toast: Option<(SharedString, Instant)>,
}

impl InlineDiff {
    /// Create an [`InlineDiff`] preloaded with the `"Suggested by AI · Make it shorter"` announcement rewrite.
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<Theme>(|_, cx| cx.notify()).detach();
        cx.observe_global::<UiLanguage>(|_, cx| cx.notify())
            .detach();
        let (orig, sugg) = default_diff_pair();
        let segments = diff_words(orig, sugg);
        let changes = build_change_slots(&segments);
        Self {
            id: id.into(),
            focus: cx.focus_handle(),
            label: Some("Suggested by AI · Make it shorter".into()),
            segments,
            changes,
            history: Vec::new(),
            active_change: None,
            hovered_change: None,
            popover_hovered: false,
            keyboard_focused: false,
            done_at: None,
            done_clock: PhaseClock::default(),
            card_origin: Rc::new(Cell::new(point(px(0.), px(0.)))),
            active_anchor: Rc::new(Cell::new(None)),
            loading: false,
            empty: false,
            measured_width: Rc::new(Cell::new(520.0)),
            toast: None,
        }
    }

    /// Diff `original` against `suggested` word by word.
    pub fn with_texts(mut self, original: impl AsRef<str>, suggested: impl AsRef<str>) -> Self {
        self.segments = diff_words(original.as_ref(), suggested.as_ref());
        self.changes = build_change_slots(&self.segments);
        self.history.clear();
        self.done_at = None;
        self.active_change = None;
        self.hovered_change = None;
        self.popover_hovered = false;
        self.keyboard_focused = false;
        self
    }

    /// Supply pre-computed [`DiffSegment`]s directly.
    pub fn with_segments(mut self, segments: Vec<DiffSegment>) -> Self {
        self.changes = build_change_slots(&segments);
        self.segments = segments;
        self.history.clear();
        self.done_at = None;
        self.active_change = None;
        self.hovered_change = None;
        self.popover_hovered = false;
        self.keyboard_focused = false;
        self
    }

    /// Set or clear (`None`) the AI provenance caption above the paragraph.
    pub fn with_label(mut self, label: Option<SharedString>) -> Self {
        self.label = label;
        self
    }

    /// Toggle skeleton loading state.
    pub fn with_loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Toggle empty state.
    pub fn with_empty(mut self, empty: bool) -> Self {
        self.empty = empty;
        self
    }

    /// Return the number of undecided (`Pending`) suggestions remaining.
    pub fn pending_count(&self) -> usize {
        self.changes
            .iter()
            .filter(|c| c.decision == DiffDecision::Pending)
            .count()
    }

    /// Assemble the current plain text (accepted edits apply `insert`; pending and rejected keep `remove`).
    pub fn current_text(&self) -> SharedString {
        let mut tokens: Vec<&str> = Vec::new();
        let mut c_idx = 0usize;
        for seg in &self.segments {
            match seg {
                DiffSegment::Equal(txt) => {
                    for w in tokenize_prose(txt.as_ref()) {
                        tokens.push(w);
                    }
                }
                DiffSegment::Change { .. } => {
                    if let Some(slot) = self.changes.get(c_idx) {
                        let chosen = if slot.decision == DiffDecision::Accepted {
                            slot.insert.as_ref()
                        } else {
                            slot.remove.as_ref()
                        };
                        for w in tokenize_prose(chosen) {
                            tokens.push(w);
                        }
                    }
                    c_idx += 1;
                }
            }
        }
        SharedString::from(join_prose_tokens(&tokens))
    }

    fn next_pending_after(&self, from: usize) -> Option<usize> {
        if self.changes.is_empty() {
            return None;
        }
        let len = self.changes.len();
        for step in 1..=len {
            let idx = (from + step) % len;
            if self.changes[idx].decision == DiffDecision::Pending {
                return Some(idx);
            }
        }
        None
    }

    fn prev_pending_before(&self, from: usize) -> Option<usize> {
        if self.changes.is_empty() {
            return None;
        }
        let len = self.changes.len();
        for step in 1..=len {
            let idx = (from + len - step) % len;
            if self.changes[idx].decision == DiffDecision::Pending {
                return Some(idx);
            }
        }
        None
    }

    /// Highlight the next (`forward == true`) or previous pending change.
    pub fn step_highlight(&mut self, forward: bool, cx: &mut Context<Self>) {
        if self.pending_count() == 0 {
            self.active_change = None;
            self.keyboard_focused = false;
            cx.notify();
            return;
        }
        let next = match self.active_change {
            Some(cur) => {
                if forward {
                    self.next_pending_after(cur)
                } else {
                    self.prev_pending_before(cur)
                }
            }
            None => {
                if forward {
                    self.changes
                        .iter()
                        .position(|c| c.decision == DiffDecision::Pending)
                } else {
                    self.changes
                        .iter()
                        .rposition(|c| c.decision == DiffDecision::Pending)
                }
            }
        };
        self.active_change = next;
        self.keyboard_focused = next.is_some();
        cx.notify();
    }

    /// Resolve `change_index` as `Accepted` or `Rejected`.
    pub fn decide_change(
        &mut self,
        change_index: usize,
        decision: DiffDecision,
        cx: &mut Context<Self>,
    ) {
        let Some(slot) = self.changes.get_mut(change_index) else {
            return;
        };
        if slot.decision != DiffDecision::Pending || decision == DiffDecision::Pending {
            return;
        }
        let prev = slot.decision;
        slot.decision = decision;
        slot.decided_at = Some(Instant::now());
        if decision == DiffDecision::Accepted {
            slot.accept_clock.restart(cx);
        }
        self.history.push(vec![(change_index, prev)]);
        self.hovered_change = None;
        self.popover_hovered = false;
        self.active_change = if self.keyboard_focused {
            self.next_pending_after(change_index)
        } else {
            None
        };
        let text = self.current_text();
        cx.emit(InlineDiffEvent::Decided {
            change_index,
            decision,
            current_text: text.clone(),
        });
        if self.pending_count() == 0 {
            self.keyboard_focused = false;
            self.done_at = Some(Instant::now());
            self.done_clock.restart(cx);
            cx.emit(InlineDiffEvent::Done(text));
        }
        cx.notify();
    }

    /// Accept every remaining pending suggestion.
    pub fn accept_all(&mut self, cx: &mut Context<Self>) {
        self.decide_all(DiffDecision::Accepted, cx);
    }

    /// Reject every remaining pending suggestion.
    pub fn reject_all(&mut self, cx: &mut Context<Self>) {
        self.decide_all(DiffDecision::Rejected, cx);
    }

    fn decide_all(&mut self, verdict: DiffDecision, cx: &mut Context<Self>) {
        let now = Instant::now();
        let mut batch = Vec::new();
        for (idx, slot) in self.changes.iter_mut().enumerate() {
            if slot.decision == DiffDecision::Pending {
                batch.push((idx, DiffDecision::Pending));
                slot.decision = verdict;
                slot.decided_at = Some(now);
                if verdict == DiffDecision::Accepted {
                    slot.accept_clock.restart(cx);
                }
            }
        }
        if batch.is_empty() {
            return;
        }
        self.history.push(batch);
        self.active_change = None;
        self.hovered_change = None;
        self.popover_hovered = false;
        self.keyboard_focused = false;
        self.done_at = Some(now);
        self.done_clock.restart(cx);
        let text = self.current_text();
        cx.emit(InlineDiffEvent::BulkDecided {
            decision: verdict,
            current_text: text.clone(),
        });
        cx.emit(InlineDiffEvent::Done(text));
        cx.notify();
    }

    /// Undo the most recent individual or bulk decision (`⌘Z` or `Z`).
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        let Some(batch) = self.history.pop() else {
            return;
        };
        let mut first_restored = None;
        for (idx, prev) in batch {
            if let Some(slot) = self.changes.get_mut(idx) {
                slot.decision = prev;
                slot.decided_at = None;
                if first_restored.is_none() && prev == DiffDecision::Pending {
                    first_restored = Some(idx);
                }
            }
        }
        if self.pending_count() > 0 {
            self.done_at = None;
        }
        if self.keyboard_focused
            && let Some(idx) = first_restored
        {
            self.active_change = Some(idx);
        }
        let text = self.current_text();
        cx.emit(InlineDiffEvent::Undone { current_text: text });
        cx.notify();
    }

    /// Reset all changes back to `Pending`.
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        for slot in &mut self.changes {
            slot.decision = DiffDecision::Pending;
            slot.decided_at = None;
        }
        self.history.clear();
        self.done_at = None;
        self.active_change = None;
        self.hovered_change = None;
        self.popover_hovered = false;
        self.keyboard_focused = false;
        cx.emit(InlineDiffEvent::Reset);
        cx.notify();
    }

    fn handle_key_down(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        let cmd = ev.keystroke.modifiers.platform || ev.keystroke.modifiers.control;
        if cmd && key.eq_ignore_ascii_case("z") {
            self.undo(cx);
            return;
        }
        if cmd || ev.keystroke.modifiers.alt {
            return;
        }
        match key {
            "tab" => {
                self.step_highlight(!ev.keystroke.modifiers.shift, cx);
            }
            "right" => self.step_highlight(true, cx),
            "left" => self.step_highlight(false, cx),
            "enter" | "y" | "Y" => {
                if let Some(idx) = self.active_change {
                    self.decide_change(idx, DiffDecision::Accepted, cx);
                } else if let Some(first) = self
                    .changes
                    .iter()
                    .position(|c| c.decision == DiffDecision::Pending)
                {
                    self.decide_change(first, DiffDecision::Accepted, cx);
                }
            }
            "backspace" | "n" | "N" => {
                if let Some(idx) = self.active_change {
                    self.decide_change(idx, DiffDecision::Rejected, cx);
                } else if let Some(first) = self
                    .changes
                    .iter()
                    .position(|c| c.decision == DiffDecision::Pending)
                {
                    self.decide_change(first, DiffDecision::Rejected, cx);
                }
            }
            "z" | "Z" => self.undo(cx),
            "escape"
                if self.active_change.take().is_some()
                    || self.hovered_change.take().is_some()
                    || self.keyboard_focused =>
            {
                self.keyboard_focused = false;
                self.popover_hovered = false;
                cx.notify();
            }
            _ => {}
        }
    }
}

impl Render for InlineDiff {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = ProPalette::get(cx);
        let reduced = disabled_motion(cx);
        let width = self.measured_width.get();
        let compact = width < 400.0;
        let measure_cell = self.measured_width.clone();
        let card_bg: Hsla = if p.dark { rgb(0x1c1c1f).into() } else { p.card };
        let card_border: Hsla = if p.dark {
            hsla(0., 0., 1., 0.10)
        } else {
            hsla(0., 0., 0., 0.075)
        };

        if self.loading {
            if !reduced {
                w.request_animation_frame();
            }
            return div()
                .id(self.id.clone())
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div()
                        .w_full()
                        .max_w(px(500.))
                        .rounded(px(12.))
                        .bg(card_bg)
                        .border_1()
                        .border_color(card_border)
                        .shadow(pro_card_shadow(p.dark))
                        .p(px(16.))
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .child(pro_skeleton_bar(190., 14., 4., 0.1, cx))
                        .child(pro_skeleton_bar(450., 68., 6., 0.35, cx))
                        .child(pro_skeleton_bar(460., 32., 6., 0.65, cx)),
                );
        }

        if self.empty || self.segments.is_empty() {
            return div()
                .id(self.id.clone())
                .w_full()
                .max_w(px(500.))
                .mx_auto()
                .rounded(px(12.))
                .bg(card_bg)
                .border_1()
                .border_color(card_border)
                .shadow(pro_card_shadow(p.dark))
                .child(pro_empty_state(
                    ProIcon::Sparkles,
                    tr(cx, "No prose edits to review", "暂无待审阅的文本润色"),
                    tr(
                        cx,
                        "Pass original and suggested text to inspect word-level deletions and insertions.",
                        "传入原文与建议修改稿即可逐处审阅增删词句。",
                    ),
                    None,
                    cx,
                ));
        }

        let any_collapsing = !reduced
            && self.changes.iter().any(|c| {
                c.decided_at
                    .map(|t| t.elapsed().as_secs_f32() < 0.42)
                    .unwrap_or(false)
            });
        let check_animating = !reduced
            && self
                .done_at
                .map(|t| t.elapsed().as_secs_f32() < 0.55)
                .unwrap_or(false);
        if any_collapsing || check_animating {
            w.request_animation_frame();
        }

        self.active_anchor.set(None);
        let popover_open =
            self.hovered_change.is_some() || self.popover_hovered || self.keyboard_focused;

        // Render flowing paragraph with inline red deletions + green insertions
        let mut flow = div()
            .w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(px(3.8))
            .gap_y(px(3.))
            .text_size(px(13.))
            .line_height(px(22.))
            .text_color(p.ink);

        let mut c_idx = 0usize;
        let total_changes = self.changes.len();
        for seg in &self.segments {
            match seg {
                DiffSegment::Equal(txt) => {
                    for word in txt.split_whitespace() {
                        flow = flow.child(word.to_owned());
                    }
                }
                DiffSegment::Change { .. } => {
                    let cur_i = c_idx;
                    c_idx += 1;
                    let Some(slot) = self.changes.get(cur_i) else {
                        continue;
                    };
                    let is_active = slot.decision == DiffDecision::Pending
                        && self.active_change == Some(cur_i)
                        && popover_open;
                    let settle_t = match slot.decided_at {
                        Some(t0) if !reduced => {
                            pro_ease((t0.elapsed().as_secs_f32() / 0.38).clamp(0.0, 1.0))
                        }
                        Some(_) => 1.0,
                        None => 0.0,
                    };

                    let anchor_cell = self.active_anchor.clone();
                    let del_bg = p.red.opacity(if is_active {
                        if p.dark { 0.25 } else { 0.16 }
                    } else if p.dark {
                        0.16
                    } else {
                        0.10
                    });
                    let add_bg = p.green.opacity(if is_active {
                        if p.dark { 0.25 } else { 0.18 }
                    } else if p.dark {
                        0.16
                    } else {
                        0.11
                    });
                    let strike_col = p.red.opacity(if p.dark { 0.76 } else { 0.62 });

                    match slot.decision {
                        DiffDecision::Pending => {
                            let is_anchor = slot.decision == DiffDecision::Pending
                                && self.active_change == Some(cur_i);
                            let mut chg_group = div()
                                .id(pro_eid(&self.id, format!("chg-pill-{cur_i}")))
                                .role(Role::Button)
                                .aria_label(SharedString::from(format!(
                                    "Suggestion {} of {}",
                                    cur_i + 1,
                                    total_changes
                                )))
                                .tab_index(0)
                                .relative()
                                .rounded(px(4.))
                                .when(is_active && self.keyboard_focused, |d| {
                                    d.border_1().border_color(p.focus.opacity(0.65))
                                })
                                .when(is_anchor, |d| {
                                    d.child(
                                        canvas(
                                            move |b, _, _| {
                                                anchor_cell.set(Some((b.origin, b.size)));
                                            },
                                            |_, _, _, _| (),
                                        )
                                        .absolute()
                                        .inset_0(),
                                    )
                                })
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .gap(px(3.))
                                .cursor_pointer()
                                .on_hover(cx.listener(move |this, inside: &bool, _, cx| {
                                    if *inside {
                                        this.hovered_change = Some(cur_i);
                                        this.active_change = Some(cur_i);
                                        cx.notify();
                                    } else if this.hovered_change == Some(cur_i) {
                                        this.hovered_change = None;
                                        cx.notify();
                                    }
                                }))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.decide_change(cur_i, DiffDecision::Accepted, cx);
                                }));

                            if !slot.remove.is_empty() {
                                chg_group = chg_group.child(
                                    div()
                                        .relative()
                                        .px(px(4.))
                                        .py(px(1.))
                                        .rounded(px(4.))
                                        .bg(del_bg)
                                        .text_color(if p.dark {
                                            p.red_ink.opacity(0.85)
                                        } else {
                                            p.red_ink
                                        })
                                        .line_through()
                                        .child(slot.remove.clone())
                                        .child(
                                            div()
                                                .absolute()
                                                .left(px(3.))
                                                .right(px(3.))
                                                .top(px(11.))
                                                .h(px(1.))
                                                .bg(strike_col),
                                        ),
                                );
                            }
                            if !slot.insert.is_empty() {
                                chg_group = chg_group.child(
                                    div()
                                        .px(px(4.))
                                        .py(px(1.))
                                        .rounded(px(4.))
                                        .bg(add_bg)
                                        .text_color(p.green_ink)
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(slot.insert.clone()),
                                );
                            }
                            flow = flow.child(chg_group);
                        }
                        DiffDecision::Accepted => {
                            let acc_drawn = reduced || slot.accept_clock.time(0.28, w, cx) > 0.012;
                            let rem_alpha = (1.0 - settle_t).clamp(0.0, 1.0);
                            if rem_alpha > 0.04 && !slot.remove.is_empty() {
                                flow = flow.child(
                                    div()
                                        .px(px(3.0 * rem_alpha))
                                        .rounded(px(4.))
                                        .bg(del_bg.opacity(rem_alpha))
                                        .opacity(rem_alpha)
                                        .text_size(px((13.0 * rem_alpha).max(1.0)))
                                        .text_color(p.muted)
                                        .line_through()
                                        .child(slot.remove.clone()),
                                );
                            }
                            let fade_tint = (1.0 - settle_t).clamp(0.0, 1.0);
                            if fade_tint > 0.02 {
                                flow = flow.child(
                                    div()
                                        .px(px((4.0 * fade_tint).max(2.0)))
                                        .py(px(1.))
                                        .rounded(px(4.))
                                        .bg(add_bg.opacity(fade_tint))
                                        .flex()
                                        .items_center()
                                        .gap(px(3.))
                                        .text_color(p.ink)
                                        .child(pro_check_draw(
                                            pro_eid(&self.id, format!("acc-chk-{cur_i}")),
                                            acc_drawn,
                                            12.,
                                            false,
                                            p.green_ink,
                                        ))
                                        .when(!slot.insert.is_empty(), |d| {
                                            d.child(slot.insert.clone())
                                        }),
                                );
                            } else if !slot.insert.is_empty() {
                                for w_str in slot.insert.split_whitespace() {
                                    flow = flow.child(w_str.to_owned());
                                }
                            }
                        }
                        DiffDecision::Rejected => {
                            if !slot.remove.is_empty() {
                                let fade_tint = (1.0 - settle_t).clamp(0.0, 1.0);
                                if fade_tint > 0.02 {
                                    flow = flow.child(
                                        div()
                                            .px(px(4.0 * fade_tint))
                                            .rounded(px(4.))
                                            .bg(del_bg.opacity(fade_tint))
                                            .text_color(p.ink)
                                            .child(slot.remove.clone()),
                                    );
                                } else {
                                    for w_str in slot.remove.split_whitespace() {
                                        flow = flow.child(w_str.to_owned());
                                    }
                                }
                            }
                            let ins_alpha = (1.0 - settle_t).clamp(0.0, 1.0);
                            if ins_alpha > 0.04 && !slot.insert.is_empty() {
                                flow = flow.child(
                                    div()
                                        .px(px(3.0 * ins_alpha))
                                        .rounded(px(4.))
                                        .bg(add_bg.opacity(ins_alpha))
                                        .opacity(ins_alpha)
                                        .text_size(px((13.0 * ins_alpha).max(1.0)))
                                        .text_color(p.ink)
                                        .child(slot.insert.clone()),
                                );
                            }
                        }
                    }
                }
            }
        }

        // Gliding popover (`✓ Accept [Y]` | `× Reject [N]`) shown cleanly on hunk hover / keyboard focus
        let pending_cnt = self.pending_count();
        let active_idx = self.active_change.filter(|&i| {
            self.changes
                .get(i)
                .map(|c| c.decision == DiffDecision::Pending)
                .unwrap_or(false)
        });
        let show_popover = active_idx.is_some() && popover_open;
        let card_origin_pt = self.card_origin.get();
        let (target_x, target_y) = match self.active_anchor.get() {
            Some((orig, sz)) => {
                let rel_x = f32::from(orig.x - card_origin_pt.x);
                let rel_y = f32::from(orig.y - card_origin_pt.y);
                let hunk_w = f32::from(sz.width);
                let hunk_h = f32::from(sz.height);
                let rx = rel_x + hunk_w * 0.5 - 92.0;
                let max_x = (width.min(500.0) - 196.0).max(10.0);
                // Keep top header `"Suggested by AI · Make it shorter"` unobstructed (`y >= 34`)
                let above_y = rel_y - 38.0;
                let ry = if above_y >= 34.0 {
                    above_y
                } else {
                    rel_y + hunk_h + 6.0
                };
                (rx.clamp(10.0, max_x), ry)
            }
            None => (16.0, 44.0),
        };
        let pop_x = pro_spring_f32(pro_eid(&self.id, "pop-x"), target_x, 0.20, w, cx);
        let pop_y = pro_spring_f32(pro_eid(&self.id, "pop-y"), target_y, 0.20, w, cx);

        let popover_el = {
            let chosen_idx = active_idx.unwrap_or(0);
            let base_id = self.id.clone();
            let acc_lbl = tr(cx, "Accept", "接受").to_owned();
            let rej_lbl = tr(cx, "Reject", "拒绝").to_owned();

            let accept_inner = div()
                .h(px(26.))
                .pl(px(8.))
                .pr(px(6.))
                .flex()
                .items_center()
                .gap(px(5.))
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.on_primary)
                .child(pro_icon(ProIcon::Check, 12.5, p.on_primary))
                .child(acc_lbl.clone())
                .child(
                    div()
                        .min_w(px(15.))
                        .h(px(15.))
                        .px(px(3.5))
                        .rounded(px(4.))
                        .bg(if p.dark {
                            hsla(0., 0., 0., 0.16)
                        } else {
                            hsla(0., 0., 1., 0.20)
                        })
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(9.5))
                        .child("Y"),
                );

            let accept_btn = pro_tactile(
                pro_eid(&base_id, "pop-accept-btn"),
                acc_lbl,
                6.,
                p.primary,
                p.primary.opacity(0.90),
                transparent_black(),
                transparent_black(),
                false,
                accept_inner,
                cx,
                move |this, _, cx| {
                    this.decide_change(chosen_idx, DiffDecision::Accepted, cx);
                },
            )
            .flex_none();

            let reject_rest = if p.dark { rgb(0x28282c).into() } else { p.card };
            let reject_inner = div()
                .h(px(26.))
                .pl(px(8.))
                .pr(px(6.))
                .flex()
                .items_center()
                .gap(px(5.))
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.strong)
                .child(pro_icon(ProIcon::Close, 12.5, p.muted))
                .child(rej_lbl.clone())
                .child(
                    div()
                        .min_w(px(15.))
                        .h(px(15.))
                        .px(px(3.5))
                        .rounded(px(4.))
                        .bg(p.chip)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(9.5))
                        .text_color(p.muted)
                        .child("N"),
                );

            let reject_btn = pro_tactile(
                pro_eid(&base_id, "pop-reject-btn"),
                rej_lbl,
                6.,
                reject_rest,
                p.chip_hover,
                transparent_black(),
                p.line_strong,
                false,
                reject_inner,
                cx,
                move |this, _, cx| {
                    this.decide_change(chosen_idx, DiffDecision::Rejected, cx);
                },
            )
            .flex_none();

            div()
                .id(pro_eid(&self.id, "diff-pop-hit"))
                .absolute()
                .left(px(pop_x))
                .top(px(pop_y))
                .py(px(4.))
                .on_hover(cx.listener(|this, inside: &bool, _, cx| {
                    if this.popover_hovered != *inside {
                        this.popover_hovered = *inside;
                        cx.notify();
                    }
                }))
                .child(pro_popover_presence(
                    pro_eid(&self.id, "diff-pop-pres"),
                    show_popover,
                    move || {
                        div()
                            .p(px(3.5))
                            .rounded(px(9.))
                            .bg(if p.dark { rgb(0x222226).into() } else { p.card })
                            .border_1()
                            .border_color(p.line_strong)
                            .shadow(pro_card_shadow(p.dark))
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .child(accept_btn)
                            .child(reject_btn)
                            .into_any_element()
                    },
                ))
        };

        // Footer bar crossfading between Pending toolbar and "All changes reviewed"
        let has_undo = !self.history.is_empty();
        let footer_content = if pending_cnt > 0 {
            let word_lbl = if pending_cnt == 1 {
                tr(cx, "suggestion", "处修改建议")
            } else {
                tr(cx, "suggestions", "处修改建议")
            };
            pro_view_transition(
                pro_eid(&self.id, "dfc-foot-tx"),
                "pending-bar",
                div()
                    .w_full()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .min_w(px(20.))
                                    .h(px(20.))
                                    .px(px(6.))
                                    .rounded(px(5.))
                                    .bg(p.chip)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(pro_rolling_number(
                                        pro_eid(&self.id, "pend-n"),
                                        pending_cnt as f64,
                                        0,
                                        "",
                                        "",
                                        12.,
                                        FontWeight::MEDIUM,
                                        p.ink,
                                    )),
                            )
                            .when(!compact, |d| {
                                d.child(
                                    div()
                                        .text_size(px(12.5))
                                        .text_color(p.muted)
                                        .child(word_lbl),
                                )
                            })
                            .when(has_undo, |d| {
                                d.child(pro_icon_btn(
                                    pro_eid(&self.id, "inline-undo-icon"),
                                    SharedString::from(tr(
                                        cx,
                                        "Undo last decision (⌘Z)",
                                        "撤销上一步 (⌘Z)",
                                    )),
                                    ProIcon::Undo,
                                    26.,
                                    false,
                                    cx,
                                    |this, _, cx| this.undo(cx),
                                ))
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .when(!compact, |d| {
                                d.child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(2.))
                                        .child(pro_icon_btn(
                                            pro_eid(&self.id, "nav-prev"),
                                            SharedString::from(tr(
                                                cx,
                                                "Previous suggestion",
                                                "上一处修改",
                                            )),
                                            ProIcon::ChevronLeft,
                                            26.,
                                            false,
                                            cx,
                                            |this, _, cx| this.step_highlight(false, cx),
                                        ))
                                        .child(pro_icon_btn(
                                            pro_eid(&self.id, "nav-next"),
                                            SharedString::from(tr(
                                                cx,
                                                "Next suggestion",
                                                "下一处修改",
                                            )),
                                            ProIcon::ChevronRight,
                                            26.,
                                            false,
                                            cx,
                                            |this, _, cx| this.step_highlight(true, cx),
                                        )),
                                )
                            })
                            .child(pro_btn(
                                pro_eid(&self.id, "btn-reject-all"),
                                tr(cx, "Reject all", "全部拒绝"),
                                None,
                                ProBtnStyle::Secondary,
                                true,
                                cx,
                                |this, _, cx| this.reject_all(cx),
                            ))
                            .child(pro_btn(
                                pro_eid(&self.id, "btn-accept-all"),
                                tr(cx, "Accept all", "全部接受"),
                                None,
                                ProBtnStyle::Primary,
                                true,
                                cx,
                                |this, _, cx| this.accept_all(cx),
                            )),
                    ),
            )
        } else {
            let done_checked = reduced || self.done_clock.time(0.28, w, cx) > 0.012;
            let animated_check = pro_check_draw(
                pro_eid(&self.id, "done-chk"),
                done_checked,
                16.,
                false,
                p.green_ink,
            );

            pro_view_transition(
                pro_eid(&self.id, "dfc-foot-tx"),
                "done-bar",
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.ink)
                            .child(animated_check)
                            .child(tr(cx, "All changes reviewed", "全部修改已审阅完成")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .when(has_undo, |d| {
                                d.child(pro_btn(
                                    pro_eid(&self.id, "done-undo-btn"),
                                    tr(cx, "Undo", "撤销"),
                                    Some(ProIcon::Undo),
                                    ProBtnStyle::Secondary,
                                    true,
                                    cx,
                                    |this, _, cx| this.undo(cx),
                                ))
                            })
                            .child(pro_btn(
                                pro_eid(&self.id, "done-reset-btn"),
                                tr(cx, "Reset", "重置"),
                                None,
                                ProBtnStyle::Ghost,
                                true,
                                cx,
                                |this, _, cx| this.reset(cx),
                            )),
                    ),
            )
        };

        let card_origin_writer = self.card_origin.clone();
        let card = div()
            .relative()
            .w_full()
            .max_w(px(500.))
            .child(
                canvas(
                    move |b, _, _| {
                        card_origin_writer.set(b.origin);
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .inset_0(),
            )
            .child(
                div()
                    .w_full()
                    .rounded(px(12.))
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .shadow(pro_card_shadow(p.dark))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .p(px(16.))
                            .pb(px(16.))
                            .flex()
                            .flex_col()
                            .gap(px(10.))
                            .when_some(self.label.clone(), |d, lbl| {
                                d.child(
                                    div()
                                        .w_full()
                                        .min_w_0()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.))
                                        .text_size(px(12.))
                                        .text_color(p.muted)
                                        .child(pro_svg("0 0 16 16", TWIN_SPARK_SVG, 14., p.blue))
                                        .child(div().truncate().child(lbl)),
                                )
                            })
                            .child(flow),
                    )
                    .child(
                        div()
                            .min_h(px(44.))
                            .pl(px(16.))
                            .pr(px(10.))
                            .py(px(8.))
                            .border_t_1()
                            .border_color(card_border)
                            .flex()
                            .items_center()
                            .child(footer_content),
                    ),
            )
            .child(popover_el);

        div()
            .id(self.id.clone())
            .role(Role::Region)
            .aria_label(SharedString::from(tr(
                cx,
                "Inline AI prose diff",
                "行内 AI 文本对比审阅",
            )))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key_down))
            .relative()
            .w_full()
            .min_w_0()
            .flex()
            .justify_center()
            .child(
                canvas(
                    move |b, window, _| {
                        let nw = f32::from(b.size.width);
                        if nw > 10.0 && (measure_cell.get() - nw).abs() > 1.0 {
                            measure_cell.set(nw);
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .h(px(0.)),
            )
            .child(card)
            .child(pro_toast_banner(self.toast.clone()))
    }
}

fn build_change_slots(segments: &[DiffSegment]) -> Vec<ChangeSlot> {
    segments
        .iter()
        .filter_map(|seg| match seg {
            DiffSegment::Change { remove, insert } => Some(ChangeSlot {
                remove: remove.clone(),
                insert: insert.clone(),
                decision: DiffDecision::Pending,
                decided_at: None,
                accept_clock: PhaseClock::default(),
            }),
            DiffSegment::Equal(_) => None,
        })
        .collect()
}

fn default_diff_pair() -> (&'static str, &'static str) {
    (
        "Hi everyone, the new dashboard is now available for all users. It loads a lot faster than before, and you can now export any report to CSV with just a single click. Let us know what you think!",
        "Hi everyone, the new dashboard is now available for all users. It loads twice as fast, and you can now export any report to CSV in one click. Let us know what you think!",
    )
}

#[cfg(test)]
mod tests {
    use super::{DiffDecision, DiffSegment, default_diff_pair, diff_words};

    #[test]
    fn diff_words_groups_adjacent_deletions_and_insertions_cleanly() {
        let (orig, sugg) = default_diff_pair();
        let segs = diff_words(orig, sugg);
        let changes: Vec<(&str, &str)> = segs
            .iter()
            .filter_map(|s| match s {
                DiffSegment::Change { remove, insert } => Some((remove.as_ref(), insert.as_ref())),
                DiffSegment::Equal(_) => None,
            })
            .collect();
        assert_eq!(
            changes,
            vec![
                ("a lot faster than before", "twice as fast"),
                ("with just a single", "in one"),
            ]
        );

        let multi = diff_words(
            "Hi everyone, we are very excited to announce that the new dashboard is now available.",
            "Hi everyone, the new dashboard is now live.",
        );
        let multi_changes: Vec<(&str, &str)> = multi
            .iter()
            .filter_map(|s| match s {
                DiffSegment::Change { remove, insert } => Some((remove.as_ref(), insert.as_ref())),
                DiffSegment::Equal(_) => None,
            })
            .collect();
        assert_eq!(
            multi_changes,
            vec![
                ("we are very excited to announce that", ""),
                ("available", "live"),
            ]
        );
        assert_eq!(DiffDecision::default(), DiffDecision::Pending);
    }
}
