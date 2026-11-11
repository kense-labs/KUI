//! Review generated edits in the context of their original source file.
use crate::primitives::{
    brand_mark::brand_mark,
    control::{UiLanguage, activate},
    editor::EditorScrollRegion,
    family::{Paint, PhaseClock, Tone, disabled_motion, ease, face},
    glyph::{Glyph, icon, spinner},
    nav::HorizontalScrollRegion,
    ops_surface::{Fold, btn, column, note, panel, row, title},
};
use gpui::{prelude::*, *};
use std::{cell::Cell, collections::HashMap, ops::Range, rc::Rc, time::Duration};

const LINE_HEIGHT: f32 = 20.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Clone, Debug)]
pub struct Hunk {
    /// One-based line in the original file.
    pub line: usize,
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub decision: Decision,
}

pub struct PatchEvent {
    pub hunks: Vec<usize>,
    pub decision: Decision,
}

/// Generation state supplied by the host.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PatchPhase {
    Thinking,
    Streaming,
    #[default]
    Ready,
    Error(SharedString),
}

/// Requests to stop or restart host generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchControlEvent {
    Stop,
    Retry,
    Regenerate,
}

#[derive(Clone, Copy)]
enum Syntax {
    Keyword,
    Literal,
    Number,
    Comment,
}

#[derive(Clone, Copy)]
enum LineKind {
    Context,
    Removed,
    Added,
}

type Tokens = Vec<(Range<usize>, Syntax)>;

#[derive(Clone, Copy)]
struct CodeStyle {
    paint: Paint,
    narrow: bool,
    scroll_x: Pixels,
}

pub struct PatchReview {
    id: SharedString,
    pub file: String,
    model: Option<SharedString>,
    pub summary: String,
    pub hunks: Vec<Hunk>,
    original: Option<Vec<String>>,
    newline: &'static str,
    phase: PatchPhase,
    phase_clock: PhaseClock,
    generation: u64,
    interactive: bool,
    tokens: HashMap<String, Tokens>,
    width: Rc<Cell<Pixels>>,
    code_height: Pixels,
    code_scroll: ScrollHandle,
    code_x: Pixels,
    max_code_x: Pixels,
    focus: FocusHandle,
    accept_focus: Vec<FocusHandle>,
}

impl EventEmitter<PatchEvent> for PatchReview {}
impl EventEmitter<PatchControlEvent> for PatchReview {}

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
            original: None,
            newline: "\n",
            phase: PatchPhase::Ready,
            phase_clock: PhaseClock::default(),
            generation: 0,
            interactive: false,
            tokens: HashMap::new(),
            width: Rc::new(Cell::new(px(0.))),
            code_height: px(374.),
            code_scroll: ScrollHandle::new(),
            code_x: px(0.),
            max_code_x: px(0.),
            focus: cx.focus_handle().tab_stop(true),
            accept_focus: Vec::new(),
        }
    }

    /// Name the host-selected model in the review header.
    pub fn with_model(mut self, model: impl Into<SharedString>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Render the entire source and include unchanged lines between edits.
    pub fn with_original(mut self, source: &str) -> Self {
        self.newline = if source.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        self.original = Some(if source.is_empty() {
            Vec::new()
        } else {
            source.split(self.newline).map(str::to_owned).collect()
        });
        self
    }

    /// Show host generation controls and apply a maximum code viewport height.
    pub fn with_generation_controls(mut self, max_code_height: Pixels) -> Self {
        self.interactive = true;
        self.code_height = max_code_height.max(px(LINE_HEIGHT));
        self
    }

    /// Read the current host generation phase.
    pub fn phase(&self) -> &PatchPhase {
        &self.phase
    }

    /// Update the generation phase. Starting a new run clears prior proposals.
    pub fn set_phase(&mut self, phase: PatchPhase, cx: &mut Context<Self>) {
        if phase == PatchPhase::Thinking {
            self.hunks.clear();
            self.tokens.clear();
            self.generation = self.generation.wrapping_add(1);
        } else if self.phase == phase {
            return;
        }
        if !matches!(self.phase, PatchPhase::Thinking | PatchPhase::Streaming)
            || !matches!(phase, PatchPhase::Thinking | PatchPhase::Streaming)
        {
            self.phase_clock.restart(cx);
        }
        self.phase = phase;
        cx.notify();
    }

    /// The proposed file with only accepted changes applied.
    pub fn resolved_code(&self) -> Option<String> {
        let original = self.original.as_ref()?;
        let mut lines = Vec::new();
        let mut at = 0;
        for i in self.ordered_hunks() {
            let hunk = &self.hunks[i];
            let start = hunk.line - 1;
            if hunk.decision == Decision::Accepted {
                lines.extend(original[at..start].iter().map(String::as_str));
                lines.extend(hunk.after.iter().map(String::as_str));
                at = start + hunk.before.len();
            }
        }
        lines.extend(original[at..].iter().map(String::as_str));
        Some(lines.join(self.newline))
    }

    fn ordered_hunks(&self) -> Vec<usize> {
        let mut indices: Vec<_> = (0..self.hunks.len()).collect();
        let Some(original) = self.original.as_ref() else {
            return indices;
        };
        indices.sort_by_key(|&i| self.hunks[i].line);
        let mut end = 0;
        let mut last_start = None;
        indices.retain(|&i| {
            let hunk = &self.hunks[i];
            let Some(start) = hunk.line.checked_sub(1) else {
                return false;
            };
            let next = start.saturating_add(hunk.before.len());
            if start < end || last_start == Some(start) || next > original.len() {
                return false;
            }
            end = next;
            last_start = Some(start);
            true
        });
        indices
    }

    fn decide(
        &mut self,
        at: Option<usize>,
        decision: Decision,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.phase != PatchPhase::Ready {
            return;
        }
        let ordered = self.ordered_hunks();
        let mut changed = Vec::new();
        for &i in &ordered {
            let hunk = &mut self.hunks[i];
            if hunk.decision == Decision::Pending && at.is_none_or(|index| index == i) {
                hunk.decision = decision;
                changed.push(i);
            }
        }
        if changed.is_empty() {
            return;
        }
        let offset = at
            .and_then(|at| ordered.iter().position(|&i| i == at))
            .map_or(0, |i| i + 1);
        let next_focus = ordered[offset..]
            .iter()
            .chain(&ordered[..offset])
            .find(|&&i| self.hunks[i].decision == Decision::Pending)
            .and_then(|&i| self.accept_focus.get(i))
            .unwrap_or(&self.focus);
        next_focus.focus(w, cx);
        cx.emit(PatchEvent {
            hunks: changed,
            decision,
        });
        cx.notify();
    }

    fn scroll_code(&mut self, delta: Pixels, cx: &mut Context<Self>) -> bool {
        let next = (self.code_x + delta).clamp(px(0.), self.max_code_x);
        if self.code_x == next {
            return false;
        }
        self.code_x = next;
        cx.notify();
        true
    }

    fn code_row(
        cache: &mut HashMap<String, Tokens>,
        number: Option<usize>,
        text: &str,
        kind: LineKind,
        strength: f32,
        style: CodeStyle,
    ) -> Div {
        let CodeStyle {
            paint: p,
            narrow,
            scroll_x,
        } = style;
        let (mark, accent) = match kind {
            LineKind::Context => (" ", p.ink),
            LineKind::Removed => ("−", p.danger),
            LineKind::Added => ("+", p.success),
        };
        let dark = p.ink.to_rgb().r > 0.5;
        let tokens = cache
            .entry(text.to_owned())
            .or_insert_with(|| tokenize(text));
        let highlights = tokens
            .iter()
            .map(|(span, kind)| {
                let color = match kind {
                    Syntax::Keyword => rgb(if dark { 0xc7a3f0 } else { 0x7548a0 }).into(),
                    Syntax::Literal => rgb(if dark { 0xb5d99c } else { 0x6d6820 }).into(),
                    Syntax::Number => rgb(if dark { 0x98c6ed } else { 0x1b6294 }).into(),
                    Syntax::Comment => p.faint,
                };
                (
                    span.clone(),
                    HighlightStyle {
                        color: Some(color),
                        ..Default::default()
                    },
                )
            })
            .collect::<Vec<_>>();
        row()
            .gap_0()
            .w_full()
            .h(px(LINE_HEIGHT))
            .flex_none()
            .text_size(px(if narrow { 12. } else { 13. }))
            .line_height(px(LINE_HEIGHT))
            .font_family("Fira Code")
            .when(strength > 0., |line| {
                line.bg(accent.opacity(0.10 * strength))
            })
            .child(
                div()
                    .w(px(if narrow { 23. } else { 31. }))
                    .flex_none()
                    .pr(px(5.))
                    .text_right()
                    .text_color(p.faint)
                    .child(number.map_or_else(String::new, |n| n.to_string())),
            )
            .child(
                div()
                    .w(px(16.))
                    .flex_none()
                    .text_center()
                    .text_color(accent.opacity(strength))
                    .child(mark),
            )
            .child(
                div().min_w_0().flex_1().h_full().overflow_hidden().child(
                    div().relative().left(-scroll_x).whitespace_nowrap().child(
                        StyledText::new(SharedString::from(text.to_owned()))
                            .with_highlights(highlights),
                    ),
                ),
            )
    }
}

fn tokenize(line: &str) -> Tokens {
    let mut tokens = Vec::new();
    let mut chars = line.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        let kind = if line[start..].starts_with("//") {
            tokens.push((start..line.len(), Syntax::Comment));
            break;
        } else if ch == '"' {
            let mut escaped = false;
            for (_, next) in chars.by_ref() {
                if next == '"' && !escaped {
                    break;
                }
                escaped = next == '\\' && !escaped;
            }
            Syntax::Literal
        } else if ch.is_ascii_digit() {
            while chars
                .next_if(|(_, next)| next.is_ascii_alphanumeric() || *next == '_')
                .is_some()
            {}
            Syntax::Number
        } else if ch.is_ascii_alphabetic() || ch == '_' {
            while chars
                .next_if(|(_, next)| next.is_ascii_alphanumeric() || *next == '_')
                .is_some()
            {}
            let end = chars.peek().map_or(line.len(), |(i, _)| *i);
            if !matches!(
                &line[start..end],
                "as" | "async"
                    | "await"
                    | "break"
                    | "const"
                    | "else"
                    | "enum"
                    | "false"
                    | "fn"
                    | "for"
                    | "if"
                    | "impl"
                    | "in"
                    | "let"
                    | "match"
                    | "mod"
                    | "move"
                    | "mut"
                    | "pub"
                    | "return"
                    | "self"
                    | "struct"
                    | "trait"
                    | "true"
                    | "type"
                    | "use"
                    | "where"
                    | "while"
            ) {
                continue;
            }
            Syntax::Keyword
        } else {
            continue;
        };
        let end = chars.peek().map_or(line.len(), |(i, _)| *i);
        tokens.push((start..end, kind));
    }
    tokens
}

impl Render for PatchReview {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Paint::get(cx);
        let language = UiLanguage::current(cx);
        let t = |en: &'static str, zh: &'static str| language.text(en, zh);
        let radius = crate::foundation::surface_radius(cx);
        let measured = self.width.get();
        let width = if measured > px(0.) {
            measured
        } else {
            w.viewport_size().width.min(px(720.))
        };
        let narrow = width < px(440.);
        let writing = self.phase == PatchPhase::Streaming;
        let generating = matches!(self.phase, PatchPhase::Thinking | PatchPhase::Streaming);
        let reduced = disabled_motion(cx);
        let order = self.ordered_hunks();
        let pending = order
            .iter()
            .filter(|&&i| self.hunks[i].decision == Decision::Pending)
            .count();
        let accepted = order
            .iter()
            .any(|&i| self.hunks[i].decision == Decision::Accepted);
        self.accept_focus.truncate(self.hunks.len());
        self.accept_focus
            .resize_with(self.hunks.len(), || cx.focus_handle().tab_stop(true));
        let mut widest = self
            .original
            .iter()
            .flatten()
            .chain(
                self.hunks
                    .iter()
                    .flat_map(|h| h.before.iter().chain(&h.after)),
            )
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0) as f32;
        widest *= if narrow { 7.4 } else { 8.0 };
        self.max_code_x = (px(widest + 48.) - width).max(px(0.));
        self.code_x = self.code_x.min(self.max_code_x);
        let style = CodeStyle {
            paint: p,
            narrow,
            scroll_x: self.code_x,
        };
        let mut body = column()
            .id("patch-code")
            .role(Role::Region)
            .aria_label(self.file.clone())
            .track_focus(&self.focus)
            .w_full()
            .max_h(self.code_height)
            .gap_0()
            .overflow_y_scroll()
            .track_scroll(&self.code_scroll)
            .on_scroll(cx.listener(|s, e: &ScrollEvent, w, cx| {
                let delta = e.delta.pixel_delta(px(LINE_HEIGHT));
                if !e.control
                    && !e.platform
                    && delta.x.abs() > delta.y.abs()
                    && s.scroll_code(-delta.x, cx)
                {
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }));
        let original = self.original.as_deref();
        let mut at = 0;
        let mut display_line = 1;
        for &i in &order {
            let hunk = &self.hunks[i];
            if let Some(source) = original {
                let start = hunk.line - 1;
                for line in &source[at..start] {
                    body = body.child(Self::code_row(
                        &mut self.tokens,
                        Some(display_line),
                        line,
                        LineKind::Context,
                        0.,
                        style,
                    ));
                    display_line += 1;
                }
                at = start + hunk.before.len();
            } else {
                display_line = hunk.line;
            }
            let base = display_line;
            let before = original.map_or(hunk.before.as_slice(), |source| {
                &source[at - hunk.before.len()..at]
            });
            let base_key = format!("{}-{}-{i}", self.id, self.generation);
            let settled = hunk.decision != Decision::Pending;
            let ready = self.phase == PatchPhase::Ready;
            let mut part = column().gap_0().w_full();
            for (old, lines, visible) in [
                (true, before, hunk.decision != Decision::Accepted),
                (
                    false,
                    hunk.after.as_slice(),
                    hunk.decision != Decision::Rejected,
                ),
            ] {
                let key = format!("{base_key}-{}", if old { "old" } else { "new" });
                let fold = Fold::get(
                    key.clone(),
                    visible,
                    lines.len() as f32 * LINE_HEIGHT,
                    0.26,
                    w,
                    cx,
                );
                if lines.is_empty() || !fold.mounted() {
                    continue;
                }
                let kept = if old {
                    hunk.decision == Decision::Rejected
                } else {
                    hunk.decision == Decision::Accepted
                };
                let kind = if kept {
                    LineKind::Context
                } else if old {
                    LineKind::Removed
                } else {
                    LineKind::Added
                };
                let mut text = column().gap_0().w_full();
                for (n, line) in lines.iter().enumerate() {
                    let number = (!(old && !kept && original.is_some())).then_some(base + n);
                    let row = Self::code_row(
                        &mut self.tokens,
                        number,
                        line,
                        kind,
                        f32::from(!kept),
                        style,
                    );
                    if writing && !old && !reduced {
                        text =
                            text.child(div().w_full().overflow_hidden().child(row).with_animation(
                                format!("{key}-{n}"),
                                Animation::new(Duration::from_millis(120)),
                                |row, t| row.h(px(LINE_HEIGHT * ease(t))).opacity(t),
                            ));
                    } else {
                        text = text.child(row);
                    }
                }
                part = part.child(fold.wrap(0., text));
            }
            let actions = Fold::get(
                format!("{base_key}-actions"),
                ready && !settled,
                34.,
                if settled { 0.26 } else { 0.24 },
                w,
                cx,
            );
            if actions.mounted() {
                let mut toolbar = row()
                    .w_full()
                    .justify_end()
                    .gap(px(3.))
                    .px(px(4.))
                    .min_h(px(34.));
                if !settled && ready {
                    let decision_button = |decision, label, enabled| {
                        let accept = decision == Decision::Accepted;
                        let kind = if accept {
                            "patch-accept"
                        } else {
                            "patch-reject"
                        };
                        let control = face(
                            (kind, i),
                            format!("{label} {} {}", t("change at line", "变更行"), hunk.line),
                            28.,
                            radius.into(),
                            if enabled { Tone::Primary } else { Tone::Clear },
                            cx,
                        )
                        .px(px(8.))
                        .text_size(px(12.))
                        .child(label)
                        .when(accept, |control| control.track_focus(&self.accept_focus[i]));
                        activate(control, cx, move |s, w, cx| {
                            s.decide(Some(i), decision, w, cx)
                        })
                    };
                    toolbar = toolbar
                        .child(decision_button(
                            Decision::Rejected,
                            t("Reject", "拒绝"),
                            false,
                        ))
                        .child(decision_button(
                            Decision::Accepted,
                            t("Accept", "接受"),
                            false,
                        ));
                }
                part = part.child(actions.wrap(0., toolbar));
            }
            body = body.child(part);
            display_line += if hunk.decision == Decision::Rejected {
                before.len()
            } else {
                hunk.after.len()
            };
        }
        if let Some(source) = original {
            for line in &source[at..] {
                body = body.child(Self::code_row(
                    &mut self.tokens,
                    Some(display_line),
                    line,
                    LineKind::Context,
                    0.,
                    style,
                ));
                display_line += 1;
            }
        }

        let (label, glyph, state_color) = match &self.phase {
            PatchPhase::Thinking => (
                format!("{} {}", t("Reading", "正在读取"), self.file),
                Glyph::Spinner,
                p.secondary,
            ),
            PatchPhase::Streaming => (
                t("Writing changes", "正在写入改动").to_owned(),
                Glyph::Spinner,
                p.secondary,
            ),
            PatchPhase::Error(error) => (error.to_string(), Glyph::Alert, p.danger),
            PatchPhase::Ready if pending > 0 => (
                format!("{pending} {}", t("changes to review", "项改动待审阅")),
                Glyph::Edit,
                p.secondary,
            ),
            PatchPhase::Ready if order.is_empty() => (
                t("No changes needed", "无需修改").to_owned(),
                Glyph::Check,
                p.secondary,
            ),
            PatchPhase::Ready if accepted => (
                t("Changes accepted", "已接受改动").to_owned(),
                Glyph::Check,
                p.success,
            ),
            PatchPhase::Ready => (
                t("Changes rejected", "已拒绝改动").to_owned(),
                Glyph::Close,
                p.secondary,
            ),
        };
        let mut status = row().flex_1().min_w_0().gap(px(6.));
        if generating && !reduced {
            let seconds = self.phase_clock.age(cx);
            if w.is_visible() {
                w.request_animation_frame();
            }
            let ink = state_color.opacity(if seconds.is_finite() {
                0.62 + 0.38 * (seconds * 4.).sin().abs()
            } else {
                1.
            });
            status = status.child(div().flex_none().child(spinner(
                if seconds.is_finite() { seconds } else { 0. },
                14.,
                ink,
            )));
        } else {
            status = status.child(div().flex_none().child(icon(glyph, 14., state_color)));
        }
        status = status.child(
            div()
                .id("patch-status")
                .role(Role::Status)
                .text_color(state_color)
                .text_size(px(12.))
                .line_height(px(17.))
                .child(label),
        );
        let mut footer = row()
            .w_full()
            .flex_wrap()
            .justify_between()
            .gap(px(6.))
            .when(narrow, |row| row.items_start().flex_col())
            .child(status);
        if self.phase == PatchPhase::Ready && pending > 0 {
            footer = footer.child(
                row()
                    .flex_wrap()
                    .gap(px(4.))
                    .child(btn(
                        "reject-all",
                        t("Reject all", "全部拒绝"),
                        false,
                        cx,
                        |s, w, cx| s.decide(None, Decision::Rejected, w, cx),
                    ))
                    .child(btn(
                        "accept-all",
                        t("Accept all", "全部接受"),
                        true,
                        cx,
                        |s, w, cx| s.decide(None, Decision::Accepted, w, cx),
                    )),
            );
        } else if self.interactive {
            let (id, label, event) = match self.phase {
                PatchPhase::Thinking | PatchPhase::Streaming => {
                    ("patch-stop", t("Stop", "停止"), PatchControlEvent::Stop)
                }
                PatchPhase::Error(_) => {
                    ("patch-retry", t("Retry", "重试"), PatchControlEvent::Retry)
                }
                PatchPhase::Ready => (
                    "patch-regenerate",
                    t("Regenerate", "重新生成"),
                    PatchControlEvent::Regenerate,
                ),
            };
            footer = footer.child(
                btn(id, label, false, cx, move |s, w, cx| {
                    s.focus.focus(w, cx);
                    cx.emit(event);
                })
                .border_1()
                .border_color(p.line),
            );
        }
        let plus: usize = order
            .iter()
            .filter(|&&i| self.hunks[i].decision != Decision::Rejected)
            .map(|&i| self.hunks[i].after.len())
            .sum();
        let minus: usize = order
            .iter()
            .filter(|&&i| self.hunks[i].decision != Decision::Rejected)
            .map(|&i| self.hunks[i].before.len())
            .sum();
        let code_scroll = self.code_scroll.clone();
        let code_x = self.code_x;
        let max_code_x = self.max_code_x;
        let body = div().relative().w_full().child(body).child(
            canvas(
                |_, _, _| (),
                move |bounds, _, _, cx| {
                    if let Some(host) = cx.try_global::<EditorScrollRegion>() {
                        let top = -code_scroll.offset().y;
                        (host.0)(
                            bounds,
                            top > px(0.1),
                            top + px(0.1) < code_scroll.max_offset().y,
                        );
                    }
                    if let Some(host) = cx.try_global::<HorizontalScrollRegion>() {
                        (host.0)(bounds, code_x > px(0.1), code_x + px(0.1) < max_code_x);
                    }
                },
            )
            .absolute()
            .inset_0(),
        );
        let measured_width = self.width.clone();
        let weak = cx.weak_entity();
        let card = panel(&self.id, 720., cx)
            .role(Role::Group)
            .aria_label(format!(
                "{}: {}",
                t("Code suggestion", "代码改动审阅"),
                self.summary
            ))
            .p(px(if narrow { 9. } else { 12. }))
            .gap(px(9.))
            .on_key_down(cx.listener(|s, e: &KeyDownEvent, w, cx| {
                let key = e.keystroke.key.as_str();
                let changed = if !e.is_held
                    && (e.keystroke.modifiers.platform || e.keystroke.modifiers.control)
                {
                    match key {
                        "enter" => {
                            s.decide(None, Decision::Accepted, w, cx);
                            true
                        }
                        "backspace" => {
                            s.decide(None, Decision::Rejected, w, cx);
                            true
                        }
                        _ => false,
                    }
                } else if s.focus.is_focused(w) {
                    match key {
                        "left" => s.scroll_code(px(-36.), cx),
                        "right" => s.scroll_code(px(36.), cx),
                        _ => false,
                    }
                } else {
                    false
                };
                if changed {
                    w.prevent_default();
                    cx.stop_propagation();
                }
            }))
            .child(
                row()
                    .items_start()
                    .gap(px(8.))
                    .pl(px(4.))
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
                    .w_full()
                    .border_1()
                    .border_color(p.line)
                    .rounded(radius)
                    .overflow_hidden()
                    .child(
                        row()
                            .w_full()
                            .justify_between()
                            .px(px(8.))
                            .min_h(px(34.))
                            .border_b_1()
                            .border_color(p.line)
                            .child(
                                row()
                                    .min_w_0()
                                    .gap(px(6.))
                                    .child(div().flex_none().child(icon(
                                        Glyph::FileText,
                                        14.,
                                        p.secondary,
                                    )))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .truncate()
                                            .text_size(px(12.))
                                            .child(self.file.clone()),
                                    ),
                            )
                            .child(
                                row()
                                    .flex_none()
                                    .gap(px(9.))
                                    .text_size(px(12.))
                                    .child(
                                        div()
                                            .text_color(if plus == 0 { p.faint } else { p.success })
                                            .child(format!("+{plus}")),
                                    )
                                    .child(
                                        div()
                                            .text_color(if minus == 0 { p.faint } else { p.danger })
                                            .child(format!("−{minus}")),
                                    ),
                            ),
                    )
                    .child(body),
            )
            .child(footer);
        div()
            .w_full()
            .max_w(px(720.))
            .child(card)
            .on_children_prepainted(move |bounds, w, _| {
                if let Some(bounds) = bounds.first() {
                    let width = bounds.size.width;
                    if (measured_width.replace(width) - width).abs() > px(0.5) {
                        let weak = weak.clone();
                        w.on_next_frame(move |_, cx| {
                            let _ = weak.update(cx, |_, cx| cx.notify());
                        });
                        w.request_animation_frame();
                    }
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{Decision, Hunk, PatchPhase, PatchReview};
    use gpui::{Modifiers, TestAppContext};

    fn hunk(line: usize, before: &[&str], after: &[&str]) -> Hunk {
        Hunk {
            line,
            before: before.iter().map(|line| (*line).to_owned()).collect(),
            after: after.iter().map(|line| (*line).to_owned()).collect(),
            decision: Decision::Pending,
        }
    }

    #[test]
    fn acceptance_keeps_unedited_lines_and_rejects_overlapping_ranges() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (patch, window) = app.add_window_view(|_, cx| {
            PatchReview::new(
                "patch",
                "source.rs",
                vec![
                    hunk(4, &["four"], &["different"]),
                    hunk(2, &["two", "three"], &["replacement"]),
                    hunk(3, &["three"], &["overlapping"]),
                    hunk(19, &[], &["outside"]),
                ],
                cx,
            )
            .with_original("one\r\ntwo\r\nthree\r\nfour\r\n")
        });
        window.update(|w, cx| {
            patch.update(cx, |patch, cx| {
                patch.set_phase(PatchPhase::Streaming, cx);
                patch.decide(None, Decision::Accepted, w, cx);
                assert_eq!(
                    patch.resolved_code().as_deref(),
                    Some("one\r\ntwo\r\nthree\r\nfour\r\n")
                );
                patch.set_phase(PatchPhase::Ready, cx);
                patch.decide(Some(1), Decision::Accepted, w, cx);
                patch.decide(Some(0), Decision::Rejected, w, cx);
                patch.decide(None, Decision::Accepted, w, cx);
                assert_eq!(
                    patch.resolved_code().as_deref(),
                    Some("one\r\nreplacement\r\nfour\r\n")
                );
                assert_eq!(patch.hunks[2].decision, Decision::Pending);
                assert_eq!(patch.hunks[3].decision, Decision::Pending);
            })
        });
    }

    #[test]
    fn focus_moves_to_the_next_hunk_and_then_back_to_the_file() {
        let mut app = TestAppContext::single();
        app.update(crate::init);
        let (patch, window) = app.add_window_view(|_, cx| {
            PatchReview::new(
                "patch",
                "source.rs",
                vec![
                    hunk(1, &["one"], &["first"]),
                    hunk(2, &["two"], &["second"]),
                ],
                cx,
            )
            .with_original("one\ntwo")
        });
        window.run_until_parked();
        let first = window.debug_bounds("Accept change at line 1").unwrap();
        window.simulate_click(first.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|w, cx| {
            let patch = patch.read(cx);
            assert!(patch.accept_focus[1].is_focused(w));
            assert_eq!(patch.hunks[0].decision, Decision::Accepted);
        });
        let last = window.debug_bounds("Reject change at line 2").unwrap();
        window.simulate_click(last.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|w, cx| {
            let patch = patch.read(cx);
            assert!(patch.focus.is_focused(w));
            assert_eq!(patch.resolved_code().as_deref(), Some("first\ntwo"));
        });
    }
}
